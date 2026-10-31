//! Plain-text / CSV / JSON exports shared by both frontends.
use crate::TraceSnapshot;
use std::fmt::Write;

fn host_label(h: &crate::HopStats) -> String {
    match h.hosts.first() {
        None => "???".into(),
        Some(e) => {
            let mut s = match &e.hostname {
                Some(n) => format!("{n} ({})", e.ip),
                None => e.ip.to_string(),
            };
            if h.hosts.len() > 1 {
                let _ = write!(s, " +{}", h.hosts.len() - 1);
            }
            s
        }
    }
}

fn ms(v: Option<f64>) -> String {
    v.map_or_else(|| "-".into(), |v| format!("{v:.1}"))
}

/// `mtr --report` style table.
pub fn to_text(s: &TraceSnapshot) -> String {
    let mut out = format!(
        "HOST: {} ({}) via {:?}, {} rounds\n",
        s.target, s.target_addr, s.protocol, s.rounds
    );
    let width = s.hops.iter().map(|h| host_label(h).len()).max().unwrap_or(10).max(10);
    let _ = writeln!(
        out,
        "{:>3}  {:<width$}  {:>6} {:>5} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "#", "Host", "Loss%", "Snt", "Last", "Avg", "Best", "Wrst", "StDev"
    );
    for h in &s.hops {
        let _ = writeln!(
            out,
            "{:>3}  {:<width$}  {:>5.1}% {:>5} {:>7} {:>7.1} {:>7} {:>7} {:>7.1}",
            h.ttl,
            host_label(h),
            h.loss_pct,
            h.sent,
            ms(h.last_ms),
            h.avg_ms,
            ms(h.best_ms),
            ms(h.worst_ms),
            h.stddev_ms
        );
    }
    out
}

pub fn to_csv(s: &TraceSnapshot) -> String {
    let mut out = String::from("ttl,ip,hostname,loss_pct,sent,recv,last_ms,avg_ms,best_ms,worst_ms,stddev_ms,jitter_ms\n");
    for h in &s.hops {
        let (ip, name) = h
            .hosts
            .first()
            .map(|e| (e.ip.to_string(), e.hostname.clone().unwrap_or_default()))
            .unwrap_or_default();
        let opt = |v: Option<f64>| v.map(|v| format!("{v:.3}")).unwrap_or_default();
        let _ = writeln!(
            out,
            "{},{},{},{:.2},{},{},{},{:.3},{},{},{:.3},{}",
            h.ttl, ip, name, h.loss_pct, h.sent, h.recv, opt(h.last_ms), h.avg_ms,
            opt(h.best_ms), opt(h.worst_ms), h.stddev_ms, opt(h.jitter_ms)
        );
    }
    out
}

pub fn to_json(s: &TraceSnapshot) -> String {
    serde_json::to_string_pretty(s).unwrap_or_default()
}
