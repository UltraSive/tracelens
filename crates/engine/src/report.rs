//! Text / Markdown / CSV / trippy-compatible JSON exports.
use std::fmt::Write;

use chrono::{TimeZone, Utc};
use serde::Serialize;

use crate::{HopStats, TraceSnapshot};

fn host_label(h: &HopStats) -> String {
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

fn asn_label(h: &HopStats) -> String {
    match h.hosts.first() {
        Some(e) if e.private => "private".into(),
        Some(e) => e
            .asn
            .as_ref()
            .map(|a| format!("AS{} {}", a.asn, a.name))
            .unwrap_or_default(),
        None => String::new(),
    }
}

fn ms(v: Option<f64>) -> String {
    v.map_or_else(|| "-".into(), |v| format!("{v:.1}"))
}

fn header_line(s: &TraceSnapshot) -> String {
    let when = Utc
        .timestamp_millis_opt(s.started_at)
        .single()
        .map(|t| t.to_rfc3339())
        .unwrap_or_default();
    format!(
        "{} ({}) via {}, {} rounds, started {when}",
        s.target,
        s.target_addr,
        s.protocol_name(),
        s.rounds
    )
}

impl TraceSnapshot {
    fn protocol_name(&self) -> &'static str {
        match self.options.protocol {
            crate::Protocol::Icmp => "ICMP",
            crate::Protocol::Udp => "UDP",
            crate::Protocol::Tcp => "TCP",
        }
    }
}

/// `mtr --report` style table.
pub fn to_text(s: &TraceSnapshot) -> String {
    let mut out = format!("HOST: {}\n", header_line(s));
    let _ = writeln!(out, "Verdict: {}", s.verdict.headline);
    let width = s
        .hops
        .iter()
        .map(|h| host_label(h).len())
        .max()
        .unwrap_or(10)
        .max(10);
    let aw = s
        .hops
        .iter()
        .map(|h| asn_label(h).len())
        .max()
        .unwrap_or(3)
        .max(3);
    let _ = writeln!(
        out,
        "{:>3}  {:<width$}  {:<aw$}  {:>6} {:>5} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "#", "Host", "ASN", "Loss%", "Snt", "Last", "Avg", "Best", "Wrst", "StDev"
    );
    for h in &s.hops {
        let _ = writeln!(
            out,
            "{:>3}  {:<width$}  {:<aw$}  {:>5.1}% {:>5} {:>7} {:>7.1} {:>7} {:>7} {:>7.1}",
            h.ttl,
            host_label(h),
            asn_label(h),
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

/// GitHub-flavoured Markdown, for tickets and chat.
pub fn to_markdown(s: &TraceSnapshot) -> String {
    let mut out = format!(
        "**Trace to `{}`**: {}\n\n",
        s.target,
        header_line(s).split_once(") ").map_or("", |x| x.1)
    );
    let _ = writeln!(out, "> **{}**", s.verdict.headline);
    for d in &s.verdict.details {
        let _ = writeln!(out, "> - {d}");
    }
    out.push_str("\n| # | Host | Network | Loss | Sent | Last | Avg | Best | Worst | StDev |\n");
    out.push_str("|--:|------|---------|-----:|-----:|-----:|----:|-----:|------:|------:|\n");
    for h in &s.hops {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {:.1}% | {} | {} | {:.1} | {} | {} | {:.1} |",
            h.ttl,
            host_label(h).replace('|', "\\|"),
            asn_label(h).replace('|', "\\|"),
            h.loss_pct,
            h.sent,
            ms(h.last_ms),
            h.avg_ms,
            ms(h.best_ms),
            ms(h.worst_ms),
            h.stddev_ms
        );
    }
    if !s.path_events.is_empty() {
        out.push_str("\n**Path changes**\n\n");
        for e in &s.path_events {
            let t = Utc
                .timestamp_millis_opt(e.at)
                .single()
                .map(|t| t.format("%H:%M:%S UTC").to_string())
                .unwrap_or_default();
            let _ = writeln!(
                out,
                "- {t}: hop {} changed from `{}` to `{}`",
                e.ttl, e.from, e.to
            );
        }
    }
    out
}

pub fn to_csv(s: &TraceSnapshot) -> String {
    let mut out = String::from(
        "ttl,ip,hostname,asn,network,loss_pct,sent,recv,last_ms,avg_ms,best_ms,worst_ms,stddev_ms,jitter_ms\n",
    );
    let q = |v: &str| {
        if v.contains([',', '"']) {
            format!("\"{}\"", v.replace('"', "\"\""))
        } else {
            v.to_string()
        }
    };
    for h in &s.hops {
        let first = h.hosts.first();
        let ip = first.map(|e| e.ip.to_string()).unwrap_or_default();
        let name = first.and_then(|e| e.hostname.clone()).unwrap_or_default();
        let asn = first.and_then(|e| e.asn.as_ref());
        let opt = |v: Option<f64>| v.map(|v| format!("{v:.3}")).unwrap_or_default();
        let _ = writeln!(
            out,
            "{},{},{},{},{},{:.2},{},{},{},{:.3},{},{},{:.3},{}",
            h.ttl,
            ip,
            q(&name),
            asn.map(|a| a.asn.as_str()).unwrap_or(""),
            q(asn.map(|a| a.name.as_str()).unwrap_or("")),
            h.loss_pct,
            h.sent,
            h.recv,
            opt(h.last_ms),
            h.avg_ms,
            opt(h.best_ms),
            opt(h.worst_ms),
            h.stddev_ms,
            opt(h.jitter_ms)
        );
    }
    out
}

// ---- trippy `--mode json` compatible report ----
// Mirrors trippy-tui 0.13 `report::types` so tools that parse trippy output can read it.

fn fixed<S: serde::Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&format!("{v:.2}"))
}

#[derive(Serialize)]
struct TReport {
    info: TInfo,
    hops: Vec<THop>,
}
#[derive(Serialize)]
struct TInfo {
    target: THost,
    start_timestamp: String,
    end_timestamp: String,
}
#[derive(Serialize)]
struct THost {
    ip: std::net::IpAddr,
    hostname: String,
}
#[derive(Serialize)]
struct TMplsMember {
    label: u32,
}
#[derive(Serialize)]
struct TMpls {
    members: Vec<TMplsMember>,
}
#[derive(Serialize)]
enum TExt {
    #[serde(rename = "mpls")]
    Mpls(TMpls),
}
#[derive(Serialize)]
struct THop {
    ttl: u8,
    hosts: Vec<THost>,
    extensions: Vec<TExt>,
    #[serde(serialize_with = "fixed")]
    loss_pct: f64,
    sent: usize,
    #[serde(serialize_with = "fixed")]
    last: f64,
    recv: usize,
    #[serde(serialize_with = "fixed")]
    avg: f64,
    #[serde(serialize_with = "fixed")]
    best: f64,
    #[serde(serialize_with = "fixed")]
    worst: f64,
    #[serde(serialize_with = "fixed")]
    stddev: f64,
    #[serde(serialize_with = "fixed")]
    jitter: f64,
    #[serde(serialize_with = "fixed")]
    javg: f64,
    #[serde(serialize_with = "fixed")]
    jmax: f64,
    #[serde(serialize_with = "fixed")]
    jinta: f64,
    nat: Option<bool>,
    tos: u8,
}

/// JSON in the same shape as `trip --mode json`.
pub fn to_json(s: &TraceSnapshot) -> String {
    let ts = |ms: i64| {
        Utc.timestamp_millis_opt(ms)
            .single()
            .unwrap_or_default()
            .to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
    };
    let report = TReport {
        info: TInfo {
            target: THost {
                ip: s.target_addr,
                hostname: s.target.clone(),
            },
            start_timestamp: ts(s.started_at),
            end_timestamp: ts(Utc::now().timestamp_millis()),
        },
        hops: s
            .hops
            .iter()
            .map(|h| THop {
                ttl: h.ttl,
                hosts: h
                    .hosts
                    .iter()
                    .map(|e| THost {
                        ip: e.ip,
                        hostname: e.hostname.clone().unwrap_or_else(|| e.ip.to_string()),
                    })
                    .collect(),
                extensions: if h.mpls_labels.is_empty() {
                    vec![]
                } else {
                    vec![TExt::Mpls(TMpls {
                        members: h
                            .mpls_labels
                            .iter()
                            .map(|&label| TMplsMember { label })
                            .collect(),
                    })]
                },
                loss_pct: h.loss_pct,
                sent: h.sent,
                last: h.last_ms.unwrap_or_default(),
                recv: h.recv,
                avg: h.avg_ms,
                best: h.best_ms.unwrap_or_default(),
                worst: h.worst_ms.unwrap_or_default(),
                stddev: h.stddev_ms,
                jitter: h.jitter_ms.unwrap_or_default(),
                javg: h.javg_ms,
                jmax: h.jmax_ms.unwrap_or_default(),
                jinta: h.jinta,
                nat: h.nat,
                tos: (h.dscp.unwrap_or(0) << 2) | h.ecn.unwrap_or(0),
            })
            .collect(),
    };
    serde_json::to_string_pretty(&report).unwrap_or_default()
}

/// Dispatch by format name used by the UI.
pub fn render(s: &TraceSnapshot, format: &str) -> Option<String> {
    Some(match format {
        "text" => to_text(s),
        "markdown" => to_markdown(s),
        "csv" => to_csv(s),
        "json" => to_json(s),
        _ => return None,
    })
}
