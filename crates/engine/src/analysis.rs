//! Plain-English verdict over a trace.
//!
//! The main job is to avoid mtr's classic misreading: loss at an intermediate hop that does
//! not continue to later hops is just a router deprioritising probe replies, not a problem.
use serde::{Deserialize, Serialize};

use crate::{HopStats, Protocol, TraceOptions};

/// Rounds before we commit to anything beyond "collecting data".
const MIN_ROUNDS: usize = 5;
/// Destination loss below this (or fewer than 2 lost probes) is treated as noise.
const LOSS_NOISE_PCT: f64 = 2.0;
/// Intermediate-hop loss worth mentioning even when it does not continue.
const NOTABLE_HOP_LOSS_PCT: f64 = 5.0;
/// Average-latency increase between consecutive hops worth pointing out.
const LATENCY_JUMP_MS: f64 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerdictLevel {
    Pending,
    Ok,
    Info,
    Warn,
    Bad,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    pub level: VerdictLevel,
    pub headline: String,
    pub details: Vec<String>,
    /// TTL the verdict points at (for highlighting), if any.
    pub focus_ttl: Option<u8>,
}

impl Verdict {
    fn new(level: VerdictLevel, headline: impl Into<String>) -> Self {
        Self {
            level,
            headline: headline.into(),
            details: vec![],
            focus_ttl: None,
        }
    }
}

/// Short "who owns this hop" label: "your router", "a private network", "CLOUDFLARENET (AS13335)".
pub fn owner_label(hop: &HopStats) -> Option<String> {
    let host = hop.hosts.first()?;
    if host.private {
        return Some(if hop.ttl == 1 {
            "your router".into()
        } else {
            "a private network".into()
        });
    }
    host.asn
        .as_ref()
        .map(|a| format!("{} (AS{})", a.name, a.asn))
}

fn hop_ref(hop: &HopStats) -> String {
    match owner_label(hop) {
        Some(o) => format!("hop {} ({o})", hop.ttl),
        None => format!("hop {}", hop.ttl),
    }
}

fn responding(h: &HopStats) -> bool {
    h.recv > 0
}

pub fn verdict(hops: &[HopStats], rounds: usize, opts: &TraceOptions) -> Verdict {
    if rounds < MIN_ROUNDS {
        return Verdict::new(VerdictLevel::Pending, "Collecting data…");
    }
    if !hops.iter().any(responding) {
        let mut v = Verdict::new(VerdictLevel::Bad, "No replies from any hop");
        v.details
            .push("Check your network connection, VPN or firewall.".into());
        if opts.protocol != Protocol::Icmp {
            v.details
                .push("Some networks block UDP/TCP probes; try ICMP.".into());
        }
        return v;
    }

    let Some(target) = hops.iter().find(|h| h.is_target && responding(h)) else {
        let last = hops
            .iter()
            .rev()
            .find(|h| responding(h))
            .expect("checked above");
        let mut v = Verdict::new(VerdictLevel::Bad, "The destination isn't responding");
        v.focus_ttl = Some(last.ttl);
        v.details.push(format!(
            "The last reply came from {}. Beyond that, probes get no answer: the path may be broken there, or the destination may block {} probes.",
            hop_ref(last),
            format!("{:?}", opts.protocol).to_uppercase()
        ));
        if opts.protocol == Protocol::Icmp {
            v.details
                .push("Try TCP (port 443) to check whether only ICMP is filtered.".into());
        }
        return v;
    };

    let before_target: Vec<&HopStats> = hops.iter().filter(|h| h.ttl < target.ttl).collect();
    let t_loss = target.loss_pct;
    let loss_is_real = target.lost() >= 2 && t_loss >= LOSS_NOISE_PCT;

    let mut v;
    // Hops whose loss is explained by the loss that continues to the destination.
    let mut origin_ttl = target.ttl;

    if loss_is_real {
        // Walk back from the destination while hops keep showing comparable loss.
        // Hops that never reply at all ("???") are neutral and don't break the run.
        let threshold = (t_loss * 0.5).max(1.0);
        for h in before_target.iter().rev() {
            if !responding(h) {
                continue;
            }
            if h.loss_pct >= threshold {
                origin_ttl = h.ttl;
            } else {
                break;
            }
        }
        let level = if t_loss >= 10.0 {
            VerdictLevel::Bad
        } else {
            VerdictLevel::Warn
        };
        if origin_ttl == target.ttl {
            v = Verdict::new(
                level,
                format!("The destination is dropping {t_loss:.0}% of probes"),
            );
            v.details.push(
                "Every hop before it is clean, so this is either real loss on the last link or the server rate-limiting replies to probes.".into(),
            );
        } else {
            let origin = hops
                .iter()
                .find(|h| h.ttl == origin_ttl)
                .expect("origin is a hop");
            v = Verdict::new(
                level,
                format!("{t_loss:.0}% packet loss starting at {}", hop_ref(origin)),
            );
            v.details.push(format!(
                "Loss begins at hop {origin_ttl} and continues all the way to the destination, so it is real loss rather than a router ignoring probes."
            ));
            if origin.hosts.first().is_some_and(|h| h.private) {
                v.details.push(
                    "That is inside your own network: check Wi-Fi signal, cables or the router."
                        .into(),
                );
            }
        }
        v.focus_ttl = Some(origin_ttl);
    } else {
        v = Verdict::new(
            VerdictLevel::Ok,
            format!(
                "Looks healthy: no meaningful packet loss to {}",
                opts.target
            ),
        );
        v.details.push(format!(
            "Average round-trip time to the destination is {:.1} ms.",
            target.avg_ms
        ));
    }

    // Intermediate loss that does not continue: explicitly reassure.
    let mut misleading: Vec<&HopStats> = before_target
        .iter()
        .copied()
        .filter(|h| h.ttl < origin_ttl && responding(h) && h.loss_pct >= NOTABLE_HOP_LOSS_PCT)
        .collect();
    misleading.sort_by(|a, b| b.loss_pct.total_cmp(&a.loss_pct));
    if let Some(h) = misleading.first() {
        let others = misleading.len() - 1;
        v.details.push(format!(
            "{} shows {:.0}% loss{}, but later hops are fine. Routers often deprioritise replies to probes, so this is normal and not a problem.",
            capitalize(&hop_ref(h)),
            h.loss_pct,
            if others > 0 { format!(" (and {others} other hop{})", if others > 1 { "s" } else { "" }) } else { String::new() }
        ));
    }

    // Biggest sustained latency jump.
    let resp: Vec<&HopStats> = hops
        .iter()
        .filter(|h| responding(h) && h.ttl <= target.ttl)
        .collect();
    let jump = resp
        .windows(2)
        .map(|w| (w[1], w[1].avg_ms - w[0].avg_ms))
        .filter(|(h, d)| *d >= LATENCY_JUMP_MS && target.avg_ms >= h.avg_ms - 10.0)
        .max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((h, d)) = jump {
        v.details.push(format!(
            "Latency rises by about {d:.0} ms at {} and stays higher. That's normal if the link covers a long distance (e.g. crossing an ocean).",
            hop_ref(h)
        ));
    }

    // Unstable latency at the destination.
    if target.recv > 5 && target.stddev_ms > 30.0 && target.stddev_ms > target.avg_ms * 0.25 {
        v.details.push(format!(
            "Latency to the destination is unstable (±{:.0} ms), which can cause choppy calls or games.",
            target.stddev_ms
        ));
        if v.level == VerdictLevel::Ok {
            v.level = VerdictLevel::Info;
        }
    }

    v
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AsnInfo, HostEntry};

    fn hop(ttl: u8, ip: &str, sent: usize, recv: usize, avg: f64) -> HopStats {
        let ip = ip.parse().unwrap();
        HopStats {
            ttl,
            hosts: if recv > 0 {
                vec![HostEntry {
                    ip,
                    hostname: None,
                    asn: None,
                    private: crate::is_private(&ip),
                    count: recv,
                }]
            } else {
                vec![]
            },
            sent,
            recv,
            loss_pct: if sent > 0 {
                (sent - recv) as f64 / sent as f64 * 100.0
            } else {
                0.0
            },
            fwd_loss_pct: 0.0,
            bwd_loss_pct: 0.0,
            last_ms: Some(avg),
            best_ms: Some(avg),
            worst_ms: Some(avg),
            avg_ms: avg,
            stddev_ms: 1.0,
            jitter_ms: None,
            javg_ms: 0.0,
            jmax_ms: None,
            jinta: 0.0,
            last_reply: None,
            nat: None,
            mpls_labels: vec![],
            dscp: None,
            ecn: None,
            samples: vec![],
            is_target: false,
        }
    }

    fn opts() -> TraceOptions {
        TraceOptions {
            target: "example.com".into(),
            ..Default::default()
        }
    }

    fn target(mut h: HopStats) -> HopStats {
        h.is_target = true;
        h
    }

    #[test]
    fn pending_until_enough_rounds() {
        let v = verdict(&[target(hop(1, "1.1.1.1", 2, 2, 5.0))], 2, &opts());
        assert_eq!(v.level, VerdictLevel::Pending);
    }

    #[test]
    fn healthy_with_misleading_intermediate_loss() {
        let hops = vec![
            hop(1, "192.168.1.1", 20, 20, 1.0),
            hop(2, "8.8.4.4", 20, 10, 8.0), // 50% but does not continue
            hop(3, "9.9.9.9", 20, 20, 9.0),
            target(hop(4, "1.1.1.1", 20, 20, 10.0)),
        ];
        let v = verdict(&hops, 20, &opts());
        assert_eq!(v.level, VerdictLevel::Ok, "{v:?}");
        assert!(
            v.details
                .iter()
                .any(|d| d.starts_with("Hop 2") && d.contains("normal")),
            "{v:?}"
        );
    }

    #[test]
    fn loss_that_continues_points_at_origin() {
        let mut h3 = hop(3, "9.9.9.9", 20, 15, 9.0);
        h3.hosts[0].asn = Some(AsnInfo {
            asn: "64500".into(),
            name: "EXAMPLE-ISP".into(),
            handle: String::new(),
            prefix: String::new(),
            country: String::new(),
        });
        let hops = vec![
            hop(1, "192.168.1.1", 20, 20, 1.0),
            hop(2, "8.8.4.4", 20, 20, 8.0),
            h3,
            hop(4, "0.0.0.0", 20, 0, 0.0), // ??? hop, neutral
            target(hop(5, "1.1.1.1", 20, 15, 10.0)),
        ];
        let v = verdict(&hops, 20, &opts());
        assert_eq!(v.level, VerdictLevel::Bad);
        assert_eq!(v.focus_ttl, Some(3));
        assert!(
            v.headline.contains("hop 3 (EXAMPLE-ISP (AS64500))"),
            "{}",
            v.headline
        );
    }

    #[test]
    fn loss_only_at_destination() {
        let hops = vec![
            hop(1, "192.168.1.1", 20, 20, 1.0),
            target(hop(2, "1.1.1.1", 20, 18, 10.0)),
        ];
        let v = verdict(&hops, 20, &opts());
        assert_eq!(v.level, VerdictLevel::Bad);
        assert!(v.headline.starts_with("The destination is dropping"));
    }

    #[test]
    fn local_network_loss_is_called_out() {
        let hops = vec![
            hop(1, "192.168.1.1", 20, 14, 1.0),
            target(hop(2, "1.1.1.1", 20, 14, 10.0)),
        ];
        let v = verdict(&hops, 20, &opts());
        assert!(v.headline.contains("your router"), "{}", v.headline);
        assert!(v
            .details
            .iter()
            .any(|d| d.contains("inside your own network")));
    }

    #[test]
    fn unreachable_destination() {
        let hops = vec![
            hop(1, "192.168.1.1", 20, 20, 1.0),
            hop(2, "8.8.4.4", 20, 20, 5.0),
            hop(3, "0.0.0.0", 20, 0, 0.0),
        ];
        let v = verdict(&hops, 20, &opts());
        assert_eq!(v.level, VerdictLevel::Bad);
        assert_eq!(v.focus_ttl, Some(2));
    }

    #[test]
    fn single_lost_probe_is_noise() {
        let hops = vec![
            hop(1, "192.168.1.1", 20, 20, 1.0),
            target(hop(2, "1.1.1.1", 20, 19, 10.0)),
        ];
        assert_eq!(verdict(&hops, 20, &opts()).level, VerdictLevel::Ok);
    }

    #[test]
    fn latency_jump_is_explained() {
        let hops = vec![
            hop(1, "192.168.1.1", 20, 20, 1.0),
            hop(2, "8.8.4.4", 20, 20, 10.0),
            hop(3, "9.9.9.9", 20, 20, 90.0),
            target(hop(4, "1.1.1.1", 20, 20, 92.0)),
        ];
        let v = verdict(&hops, 20, &opts());
        assert!(
            v.details
                .iter()
                .any(|d| d.contains("rises by about 80 ms at hop 3")),
            "{v:?}"
        );
    }
}
