use std::collections::HashMap;
use std::net::IpAddr;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use trippy_core::{
    Builder, IcmpExtensionParseMode, IcmpPacketType, MultipathStrategy, NatStatus, Port,
    PortDirection, PrivilegeMode, ProbeStatus, Round, Tracer,
};

use crate::dns::{resolve_target, Resolver};
use crate::{
    analysis, is_private, EngineError, HopStats, HostEntry, Multipath, PathEvent, Protocol,
    TraceOptions, TraceSnapshot, TraceStatus,
};

/// Max RTT samples kept per hop (15 min at a 1 s interval). Drives the latency chart.
pub const MAX_SAMPLES: usize = 900;
const MAX_PATH_EVENTS: usize = 500;
/// A new address must answer this many consecutive rounds before it counts as a path change.
const PATH_CHANGE_CONFIRM_ROUNDS: u8 = 2;
/// Addresses seen at a hop within this many rounds are treated as ECMP siblings.
const ECMP_WINDOW_ROUNDS: u64 = 30;
/// Rounds after a hop first answers during which its addresses are learned, not logged.
/// Load-balanced siblings usually all show up in this window.
const PATH_WARMUP_ROUNDS: u64 = 10;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
/// trippy distinguishes concurrent traces by an ICMP identifier; keep them unique.
static NEXT_TRACE_IDENT: AtomicU16 = AtomicU16::new(0);

/// Marker payload used to unwind the tracer thread when the user stops a trace.
/// `resume_unwind` skips the panic hook, so nothing is printed.
struct Cancelled;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegeInfo {
    /// Process can open raw sockets (root / CAP_NET_RAW / Administrator).
    pub privileged: bool,
    /// Whether this platform requires privileges at all for raw tracing.
    pub needs_privileges: bool,
}

impl PrivilegeInfo {
    pub fn discover() -> Self {
        match trippy_privilege::Privilege::discover() {
            Ok(p) => Self {
                privileged: p.has_privileges(),
                needs_privileges: p.needs_privileges(),
            },
            Err(_) => Self {
                privileged: false,
                needs_privileges: true,
            },
        }
    }
}

/// Per-TTL path-change detector fed from each completed round.
///
/// A change is logged when a hop's address switches to one that hasn't answered at that hop
/// within the last [`ECMP_WINDOW_ROUNDS`] rounds, and keeps answering for
/// [`PATH_CHANGE_CONFIRM_ROUNDS`] consecutive rounds. Alternating between addresses already
/// seen there is load balancing (ECMP, very common with classic UDP), not a route change.
/// Nothing is logged for a hop during its first [`PATH_WARMUP_ROUNDS`] rounds.
#[derive(Default)]
struct PathTracker {
    round: u64,
    current: HashMap<u8, IpAddr>,
    /// ttl -> round the hop first answered (for the warm-up)
    first_round: HashMap<u8, u64>,
    /// ttl -> (candidate address, consecutive rounds, candidate was recently seen)
    pending: HashMap<u8, (IpAddr, u8, bool)>,
    /// (ttl, addr) -> last round that address answered at that hop
    last_seen: HashMap<(u8, IpAddr), u64>,
    events: Vec<PathEvent>,
}

impl PathTracker {
    fn observe(&mut self, ttl: u8, host: IpAddr, now_ms: i64) {
        let recently_seen = self
            .last_seen
            .insert((ttl, host), self.round)
            .is_some_and(|r| self.round.saturating_sub(r) <= ECMP_WINDOW_ROUNDS);
        let first = *self.first_round.entry(ttl).or_insert(self.round);
        let warming_up = self.round.saturating_sub(first) < PATH_WARMUP_ROUNDS;
        let Some(&cur) = self.current.get(&ttl) else {
            self.current.insert(ttl, host);
            return;
        };
        if cur == host {
            self.pending.remove(&ttl);
            return;
        }
        let entry = self.pending.entry(ttl).or_insert((host, 0, recently_seen));
        if entry.0 != host {
            *entry = (host, 0, recently_seen);
        }
        entry.1 += 1;
        if entry.1 >= PATH_CHANGE_CONFIRM_ROUNDS {
            let known = entry.2;
            self.pending.remove(&ttl);
            self.current.insert(ttl, host);
            if !known && !warming_up {
                self.events.push(PathEvent {
                    at: now_ms,
                    ttl,
                    from: cur,
                    to: host,
                });
                if self.events.len() > MAX_PATH_EVENTS {
                    self.events.remove(0);
                }
            }
        }
    }

    fn on_round(&mut self, round: &Round<'_>) {
        let now = chrono::Utc::now().timestamp_millis();
        self.round += 1;
        for p in round.probes {
            if let ProbeStatus::Complete(c) = p {
                self.observe(c.ttl.0, c.host, now);
            }
        }
    }
}

pub struct TraceSession {
    id: u64,
    opts: TraceOptions,
    target_addr: IpAddr,
    started_at: i64,
    tracer: Tracer,
    cancel: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
    outcome: Arc<Mutex<Option<Result<(), String>>>>,
    paths: Arc<Mutex<PathTracker>>,
    resolver: Resolver,
}

fn validate(opts: &TraceOptions) -> Result<(), EngineError> {
    let bad = |m: &str| Err(EngineError::Other(m.into()));
    if opts.first_ttl == 0 || opts.first_ttl > opts.max_ttl {
        return bad("First hop must be between 1 and the max hop");
    }
    if opts.max_ttl > 64 {
        return bad("Max hop must be 64 or less");
    }
    if !(28..=1500).contains(&opts.packet_size) {
        return bad("Packet size must be between 28 and 1500 bytes");
    }
    if opts.dscp > 63 {
        return bad("DSCP must be between 0 and 63");
    }
    if opts.multipath != Multipath::Classic && opts.protocol != Protocol::Udp {
        return bad("Paris and Dublin multipath strategies only work with UDP");
    }
    Ok(())
}

impl TraceSession {
    /// Resolve the target, build the tracer and start it on a background thread.
    pub fn start(opts: TraceOptions, resolver: Resolver) -> Result<Self, EngineError> {
        validate(&opts)?;
        let target_addr = resolve_target(&opts.target, opts.family)?;
        let privs = PrivilegeInfo::discover();
        let privilege_mode = if privs.privileged || !privs.needs_privileges {
            PrivilegeMode::Privileged
        } else {
            PrivilegeMode::Unprivileged
        };
        // trippy-core's unprivileged mode only works on macOS: on Linux the IPv4 ICMP path sets
        // IP_HDRINCL on a datagram socket (ENOPROTOOPT) and UDP never sees time-exceeded replies.
        if privilege_mode == PrivilegeMode::Unprivileged && cfg!(target_os = "linux") {
            return Err(EngineError::Other(crate::permission_hint().into()));
        }
        if privilege_mode == PrivilegeMode::Unprivileged && opts.protocol == Protocol::Tcp {
            return Err(EngineError::Other(
                "TCP tracing requires elevated privileges (raw sockets)".into(),
            ));
        }

        let ident = NEXT_TRACE_IDENT
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(std::process::id() as u16);
        let interval = Duration::from_millis(opts.interval_ms.max(20));
        let source_addr = match opts
            .source_addr
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(s) => Some(
                s.parse::<IpAddr>()
                    .map_err(|_| EngineError::Other(format!("Invalid source address '{s}'")))?,
            ),
            None => None,
        };
        let interface = opts
            .interface
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let tracer = Builder::new(target_addr)
            .privilege_mode(privilege_mode)
            .protocol(match opts.protocol {
                Protocol::Icmp => trippy_core::Protocol::Icmp,
                Protocol::Udp => trippy_core::Protocol::Udp,
                Protocol::Tcp => trippy_core::Protocol::Tcp,
            })
            .trace_identifier(ident)
            .source_addr(source_addr)
            .interface(interface)
            .first_ttl(opts.first_ttl)
            .max_ttl(opts.max_ttl)
            .packet_size(opts.packet_size)
            .tos(opts.dscp << 2)
            .multipath_strategy(match opts.multipath {
                Multipath::Classic => MultipathStrategy::Classic,
                Multipath::Paris => MultipathStrategy::Paris,
                Multipath::Dublin => MultipathStrategy::Dublin,
            })
            .icmp_extension_parse_mode(IcmpExtensionParseMode::Enabled)
            .min_round_duration(interval)
            .max_round_duration(interval.max(Duration::from_secs(1)))
            .max_rounds(opts.max_rounds)
            .max_samples(MAX_SAMPLES)
            .port_direction(match (opts.protocol, opts.port) {
                (Protocol::Icmp, _) => PortDirection::None,
                (_, Some(port)) => PortDirection::FixedDest(Port(port)),
                // classic traceroute: fixed source port, destination port varies per probe
                (Protocol::Udp, None) => PortDirection::FixedSrc(Port(50000 + ident % 10000)),
                (Protocol::Tcp, None) => PortDirection::FixedDest(Port(80)),
            })
            .build()?;

        let cancel = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let outcome: Arc<Mutex<Option<Result<(), String>>>> = Arc::default();
        let paths: Arc<Mutex<PathTracker>> = Arc::default();

        let (run_tracer, run_cancel, run_paused, run_outcome, run_paths) = (
            tracer.clone(),
            cancel.clone(),
            paused.clone(),
            outcome.clone(),
            paths.clone(),
        );
        let handle = thread::Builder::new()
            .name(format!("trace-{ident}"))
            .spawn(move || {
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    run_tracer.run_with(|round| {
                        run_paths.lock().unwrap().on_round(round);
                        // Pausing blocks the tracer between rounds; stats are kept intact.
                        while run_paused.load(Ordering::Relaxed)
                            && !run_cancel.load(Ordering::Relaxed)
                        {
                            thread::sleep(Duration::from_millis(50));
                        }
                        if run_cancel.load(Ordering::Relaxed) {
                            panic::resume_unwind(Box::new(Cancelled));
                        }
                    })
                }));
                let res = match result {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(e.to_string()),
                    Err(payload) if payload.is::<Cancelled>() => Ok(()),
                    Err(_) => Err("tracer thread panicked".into()),
                };
                *run_outcome.lock().unwrap() = Some(res);
            })
            .map_err(|e| EngineError::Other(e.to_string()))?;

        Ok(Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            opts,
            target_addr,
            started_at: chrono::Utc::now().timestamp_millis(),
            tracer,
            cancel,
            paused,
            handle: Mutex::new(Some(handle)),
            outcome,
            paths,
            resolver,
        })
    }

    pub fn resolver(&self) -> &Resolver {
        &self.resolver
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn options(&self) -> &TraceOptions {
        &self.opts
    }

    /// Ask the tracer to stop. It exits at the end of the current round.
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Stop and wait for the tracer thread to exit (releases its sockets).
    pub fn stop_and_join(&self) {
        self.stop();
        if let Some(h) = self.handle.lock().unwrap().take() {
            let _ = h.join();
        }
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn clear(&self) {
        self.tracer.clear();
        let mut p = self.paths.lock().unwrap();
        // start learning from scratch, including the warm-up
        *p = PathTracker::default();
    }

    pub fn status(&self) -> TraceStatus {
        match &*self.outcome.lock().unwrap() {
            None if self.paused.load(Ordering::Relaxed) => TraceStatus::Paused,
            None => TraceStatus::Running,
            Some(Err(_)) => TraceStatus::Failed,
            Some(Ok(())) if self.cancel.load(Ordering::Relaxed) => TraceStatus::Stopped,
            Some(Ok(())) => TraceStatus::Finished,
        }
    }

    /// Full snapshot (all retained samples).
    pub fn snapshot(&self) -> TraceSnapshot {
        self.snapshot_with(MAX_SAMPLES)
    }

    /// Snapshot keeping only the most recent `max_samples` RTT samples per hop
    /// (cheap to send for tabs that only render sparklines).
    pub fn snapshot_with(&self, max_samples: usize) -> TraceSnapshot {
        let state = self.tracer.snapshot();
        let flow = trippy_core::State::default_flow_id();
        let resolve = self.opts.resolve_hostnames;

        let hops: Vec<HopStats> = state
            .hops()
            .iter()
            .map(|hop| {
                let tos = hop.tos().map(|t| t.0);
                HopStats {
                    ttl: hop.ttl(),
                    hosts: hop
                        .addrs_with_counts()
                        .map(|(ip, count)| {
                            let info = if resolve {
                                self.resolver.lookup(*ip)
                            } else {
                                Default::default()
                            };
                            HostEntry {
                                ip: *ip,
                                hostname: info.hostname,
                                asn: info.asn,
                                private: is_private(ip),
                                count: *count,
                            }
                        })
                        .collect(),
                    sent: hop.total_sent(),
                    recv: hop.total_recv(),
                    loss_pct: hop.loss_pct(),
                    fwd_loss_pct: hop.forward_loss_pct(),
                    bwd_loss_pct: hop.backward_loss_pct(),
                    last_ms: hop.last_ms(),
                    best_ms: hop.best_ms(),
                    worst_ms: hop.worst_ms(),
                    avg_ms: hop.avg_ms(),
                    stddev_ms: hop.stddev_ms(),
                    jitter_ms: hop.jitter_ms(),
                    javg_ms: hop.javg_ms(),
                    jmax_ms: hop.jmax_ms(),
                    jinta: hop.jinta(),
                    last_reply: hop.last_icmp_packet_type().map(|t| {
                        match t {
                            IcmpPacketType::TimeExceeded(_) => "TimeExceeded",
                            IcmpPacketType::EchoReply(_) => "EchoReply",
                            IcmpPacketType::Unreachable(_) => "Unreachable",
                            IcmpPacketType::NotApplicable => "NotApplicable",
                        }
                        .to_string()
                    }),
                    nat: match hop.last_nat_status() {
                        NatStatus::NotApplicable => None,
                        NatStatus::NotDetected => Some(false),
                        NatStatus::Detected => Some(true),
                    },
                    mpls_labels: hop
                        .extensions()
                        .map(|e| {
                            e.extensions
                                .iter()
                                .filter_map(|x| match x {
                                    trippy_core::Extension::Mpls(stack) => {
                                        Some(stack.members.iter().map(|m| m.label))
                                    }
                                    trippy_core::Extension::Unknown(_) => None,
                                })
                                .flatten()
                                .collect()
                        })
                        .unwrap_or_default(),
                    dscp: tos.map(|t| t >> 2),
                    ecn: tos.map(|t| t & 0b11),
                    samples: {
                        let s = hop.samples();
                        s[..s.len().min(max_samples)]
                            .iter()
                            .rev()
                            .map(|d| (!d.is_zero()).then_some(d.as_secs_f64() * 1000.0))
                            .collect()
                    },
                    // trippy flags the last responding hop as the target even when the
                    // destination never answers, so decide by address instead.
                    is_target: hop.addrs().any(|a| *a == self.target_addr),
                }
            })
            .collect();

        let status = self.status();
        let error = match &*self.outcome.lock().unwrap() {
            Some(Err(e)) => Some(e.clone()),
            _ => state.error().map(str::to_string),
        }
        .map(crate::decorate_error);
        let rounds = state.round_count(flow);
        let verdict = analysis::verdict(&hops, rounds, &self.opts);

        TraceSnapshot {
            id: self.id,
            target: self.opts.target.clone(),
            target_addr: self.target_addr,
            options: self.opts.clone(),
            status,
            started_at: self.started_at,
            rounds,
            hops,
            path_events: self.paths.lock().unwrap().events.clone(),
            verdict,
            error,
        }
    }
}

impl Drop for TraceSession {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(p: &mut PathTracker, ttl: u8, seq: &[IpAddr]) {
        for &h in seq {
            p.round += 1;
            p.observe(ttl, h, p.round as i64);
        }
    }

    fn ips() -> (IpAddr, IpAddr, IpAddr, IpAddr) {
        let ip = |s: &str| s.parse().unwrap();
        (
            ip("10.0.0.1"),
            ip("10.0.0.2"),
            ip("10.0.0.3"),
            ip("10.0.0.4"),
        )
    }

    #[test]
    fn path_change_needs_confirmation() {
        let (a, b, ..) = ips();
        let mut p = PathTracker::default();
        feed(&mut p, 3, &[a; 12]);
        feed(&mut p, 3, &[b, a, a]); // single blip
        assert!(p.events.is_empty());
        feed(&mut p, 3, &[b; 40]);
        let mut q = PathTracker::default();
        feed(&mut q, 3, &[a; 12]);
        feed(&mut q, 3, &[b, b]);
        assert_eq!(q.events.len(), 1);
        assert_eq!(
            (q.events[0].from, q.events[0].to, q.events[0].at),
            (a, b, 14)
        );
    }

    #[test]
    fn ecmp_alternation_is_not_a_path_change() {
        let (a, b, ..) = ips();
        let mut p = PathTracker::default();
        feed(&mut p, 6, &[a; 12]);
        // b shows up once (load-balanced sibling), then traffic flips between them
        feed(&mut p, 6, &[b, a, b, b, a, a, b, b, a, a]);
        assert!(p.events.is_empty(), "{:?}", p.events);
    }

    #[test]
    fn sibling_first_seen_during_warmup_is_not_logged() {
        let (a, b, ..) = ips();
        let mut p = PathTracker::default();
        // the real-world case: the second ECMP address first takes over in round 4
        feed(&mut p, 6, &[a, a, a, b, b, a]);
        // ...and keeps alternating long after the warm-up
        for _ in 0..5 {
            feed(&mut p, 6, &[a, a, b, b]);
        }
        assert!(p.events.is_empty(), "{:?}", p.events);
    }

    #[test]
    fn warmup_is_per_hop() {
        let (a, b, c, d) = ips();
        let mut p = PathTracker::default();
        feed(&mut p, 1, &[a; 20]);
        // hop 9 first answers in round 21: its early switch is still warm-up
        feed(&mut p, 9, &[b, c, c]);
        assert!(p.events.is_empty());
        feed(&mut p, 9, &[c; 10]);
        feed(&mut p, 9, &[d, d]);
        assert_eq!(p.events.len(), 1);
        assert_eq!((p.events[0].ttl, p.events[0].to), (9, d));
    }

    #[test]
    fn returning_after_a_long_absence_is_logged() {
        let (a, b, ..) = ips();
        let mut p = PathTracker::default();
        feed(&mut p, 2, &[a; 12]);
        feed(&mut p, 2, &[b; 40]);
        feed(&mut p, 2, &[a, a]);
        assert_eq!(p.events.len(), 2);
        assert_eq!((p.events[1].from, p.events[1].to), (b, a));
    }
}
