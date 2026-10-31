use std::net::IpAddr;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::Serialize;
use trippy_core::{Builder, PrivilegeMode, Tracer};

use crate::dns::{resolve_target, Resolver};
use crate::{
    EngineError, HopStats, HostEntry, Protocol, TraceOptions, TraceSnapshot, TraceStatus,
};

/// Max RTT samples kept per hop (drives sparkline width).
const MAX_SAMPLES: usize = 120;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
/// trippy distinguishes concurrent traces by an ICMP identifier; keep them unique.
static NEXT_TRACE_IDENT: AtomicU16 = AtomicU16::new(0);

/// Marker payload used to unwind the tracer thread when the user stops a trace.
/// `resume_unwind` skips the panic hook, so nothing is printed.
struct Cancelled;

#[derive(Debug, Clone, Copy, Serialize)]
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

pub struct TraceSession {
    id: u64,
    opts: TraceOptions,
    target_addr: IpAddr,
    tracer: Tracer,
    cancel: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
    outcome: Arc<Mutex<Option<Result<(), String>>>>,
    resolver: Resolver,
}

impl TraceSession {
    /// Resolve the target, build the tracer and start it on a background thread.
    pub fn start(opts: TraceOptions, resolver: Resolver) -> Result<Self, EngineError> {
        let target_addr = resolve_target(&opts.target, opts.family)?;
        let privs = PrivilegeInfo::discover();
        let privilege_mode = if privs.privileged || !privs.needs_privileges {
            PrivilegeMode::Privileged
        } else {
            PrivilegeMode::Unprivileged
        };
        if privilege_mode == PrivilegeMode::Unprivileged && opts.protocol == Protocol::Tcp {
            return Err(EngineError::Other(
                "TCP tracing requires elevated privileges (raw sockets)".into(),
            ));
        }

        let ident = NEXT_TRACE_IDENT
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(std::process::id() as u16);

        let mut builder = Builder::new(target_addr)
            .privilege_mode(privilege_mode)
            .protocol(match opts.protocol {
                Protocol::Icmp => trippy_core::Protocol::Icmp,
                Protocol::Udp => trippy_core::Protocol::Udp,
                Protocol::Tcp => trippy_core::Protocol::Tcp,
            })
            .trace_identifier(ident)
            .max_ttl(opts.max_ttl.clamp(1, 64))
            .packet_size(opts.packet_size)
            .min_round_duration(Duration::from_millis(opts.interval_ms.max(100)))
            .max_round_duration(Duration::from_millis(opts.interval_ms.max(100)).max(Duration::from_secs(1)))
            .max_rounds(opts.max_rounds)
            .max_samples(MAX_SAMPLES);
        if let Some(port) = opts.port {
            builder = builder.port_direction(trippy_core::PortDirection::FixedDest(
                trippy_core::Port(port),
            ));
        }
        let tracer = builder.build()?;

        let cancel = Arc::new(AtomicBool::new(false));
        let outcome: Arc<Mutex<Option<Result<(), String>>>> = Arc::default();

        let run_tracer = tracer.clone();
        let run_cancel = cancel.clone();
        let run_outcome = outcome.clone();
        let handle = thread::Builder::new()
            .name(format!("trace-{ident}"))
            .spawn(move || {
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    run_tracer.run_with(|_round| {
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
            tracer,
            cancel,
            handle: Mutex::new(Some(handle)),
            outcome,
            resolver,
        })
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

    pub fn clear(&self) {
        self.tracer.clear();
    }

    pub fn status(&self) -> TraceStatus {
        match &*self.outcome.lock().unwrap() {
            None => TraceStatus::Running,
            Some(Err(_)) => TraceStatus::Failed,
            Some(Ok(())) if self.cancel.load(Ordering::Relaxed) => TraceStatus::Stopped,
            Some(Ok(())) => TraceStatus::Finished,
        }
    }

    pub fn snapshot(&self) -> TraceSnapshot {
        let state = self.tracer.snapshot();
        let flow = trippy_core::State::default_flow_id();
        let resolve = self.opts.resolve_hostnames;

        let hops = state
            .hops()
            .iter()
            .map(|hop| HopStats {
                ttl: hop.ttl(),
                hosts: hop
                    .addrs_with_counts()
                    .map(|(ip, count)| HostEntry {
                        ip: *ip,
                        hostname: resolve.then(|| self.resolver.lookup(*ip)).flatten(),
                        count: *count,
                    })
                    .collect(),
                sent: hop.total_sent(),
                recv: hop.total_recv(),
                loss_pct: hop.loss_pct(),
                last_ms: hop.last_ms(),
                best_ms: hop.best_ms(),
                worst_ms: hop.worst_ms(),
                avg_ms: hop.avg_ms(),
                stddev_ms: hop.stddev_ms(),
                jitter_ms: hop.jitter_ms(),
                samples: hop
                    .samples()
                    .iter()
                    .rev()
                    .map(|d| (!d.is_zero()).then(|| d.as_secs_f64() * 1000.0))
                    .collect(),
                is_target: state.is_target(hop, flow),
            })
            .collect();

        let status = self.status();
        let error = match &*self.outcome.lock().unwrap() {
            Some(Err(e)) => Some(e.clone()),
            _ => state.error().map(str::to_string),
        }
        .map(crate::decorate_error);

        TraceSnapshot {
            id: self.id,
            target: self.opts.target.clone(),
            target_addr: self.target_addr,
            protocol: self.opts.protocol,
            status,
            rounds: state.round_count(flow),
            hops,
            error,
        }
    }
}

impl Drop for TraceSession {
    fn drop(&mut self) {
        self.stop();
    }
}
