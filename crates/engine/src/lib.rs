//! UI-independent MTR-style tracing engine.
//!
//! Wraps `trippy-core` and exposes a small, serializable snapshot model so the
//! UI layer never touches trippy types directly.

pub mod analysis;
mod dns;
pub mod report;
mod session;

pub use analysis::{Verdict, VerdictLevel};
pub use dns::{AsnInfo, Resolver};
pub use session::{PrivilegeInfo, TraceSession};

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Icmp,
    Udp,
    Tcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AddrFamily {
    /// Prefer IPv4, fall back to IPv6.
    #[default]
    Any,
    V4,
    V6,
}

/// ECMP (equal-cost multipath) strategy. Paris/Dublin are UDP-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Multipath {
    #[default]
    Classic,
    Paris,
    Dublin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TraceOptions {
    pub target: String,
    pub protocol: Protocol,
    pub family: AddrFamily,
    /// Minimum time per round (mtr's "interval").
    pub interval_ms: u64,
    pub first_ttl: u8,
    pub max_ttl: u8,
    pub packet_size: u16,
    /// Stop after this many rounds; `None` = run until stopped.
    pub max_rounds: Option<usize>,
    /// Destination port for UDP/TCP (ignored for ICMP).
    pub port: Option<u16>,
    pub source_addr: Option<String>,
    pub interface: Option<String>,
    /// DSCP code point (0-63), sent in the IP TOS byte.
    pub dscp: u8,
    pub multipath: Multipath,
    pub resolve_hostnames: bool,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            target: String::new(),
            protocol: Protocol::Icmp,
            family: AddrFamily::Any,
            interval_ms: 1000,
            first_ttl: 1,
            max_ttl: 30,
            packet_size: 84,
            max_rounds: None,
            port: None,
            source_addr: None,
            interface: None,
            dscp: 0,
            multipath: Multipath::Classic,
            resolve_hostnames: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEntry {
    pub ip: IpAddr,
    pub hostname: Option<String>,
    pub asn: Option<AsnInfo>,
    /// RFC1918 / ULA / CGNAT / link-local etc.
    pub private: bool,
    /// How many replies came from this address (ECMP paths show >1 host).
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HopStats {
    pub ttl: u8,
    pub hosts: Vec<HostEntry>,
    pub sent: usize,
    pub recv: usize,
    pub loss_pct: f64,
    pub fwd_loss_pct: f64,
    pub bwd_loss_pct: f64,
    pub last_ms: Option<f64>,
    pub best_ms: Option<f64>,
    pub worst_ms: Option<f64>,
    pub avg_ms: f64,
    pub stddev_ms: f64,
    pub jitter_ms: Option<f64>,
    pub javg_ms: f64,
    pub jmax_ms: Option<f64>,
    pub jinta: f64,
    /// e.g. "TimeExceeded", "EchoReply", "Unreachable", "NotApplicable".
    pub last_reply: Option<String>,
    /// `None` = not applicable (non-UDP / IPv6), else whether NAT was detected.
    pub nat: Option<bool>,
    pub mpls_labels: Vec<u32>,
    pub dscp: Option<u8>,
    pub ecn: Option<u8>,
    /// Recent RTT samples, oldest first. `None` = probe lost.
    pub samples: Vec<Option<f64>>,
    pub is_target: bool,
}

impl HopStats {
    pub fn lost(&self) -> usize {
        self.sent.saturating_sub(self.recv)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceStatus {
    Running,
    Paused,
    Finished,
    Stopped,
    Failed,
}

/// A stable change of the responding address at one hop.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathEvent {
    /// Unix time in milliseconds.
    pub at: i64,
    pub ttl: u8,
    pub from: IpAddr,
    pub to: IpAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceSnapshot {
    pub id: u64,
    pub target: String,
    pub target_addr: IpAddr,
    pub options: TraceOptions,
    pub status: TraceStatus,
    /// Unix time in milliseconds.
    pub started_at: i64,
    pub rounds: usize,
    pub hops: Vec<HopStats>,
    pub path_events: Vec<PathEvent>,
    pub verdict: Verdict,
    pub error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("could not resolve '{0}'")]
    Resolve(String),
    #[error("no {1} address found for '{0}'")]
    NoAddress(String, &'static str),
    #[error("tracer error: {0}")]
    Tracer(#[from] trippy_core::Error),
    #[error("{0}")]
    Other(String),
}

/// Human-readable advice for when raw sockets are denied on this OS.
pub fn permission_hint() -> &'static str {
    if cfg!(target_os = "linux") {
        "Tracing needs raw sockets. Grant the binary CAP_NET_RAW once per build: \
         sudo setcap cap_net_raw+ep <path-to-binary>"
    } else if cfg!(target_os = "windows") {
        "Raw sockets need Administrator. Restart the app with \"Run as administrator\"."
    } else if cfg!(target_os = "macos") {
        "ICMP works unprivileged on macOS; UDP/TCP modes may need to be run with sudo."
    } else {
        "Raw sockets need elevated privileges on this platform."
    }
}

pub(crate) fn decorate_error(msg: String) -> String {
    let lower = msg.to_lowercase();
    if lower.contains("permission denied")
        || lower.contains("os error 13")
        || lower.contains("access is denied")
        || lower.contains("os error 10013")
        || lower.contains("operation not permitted")
    {
        format!("{msg}\n\n{}", permission_hint())
    } else {
        msg
    }
}

/// Address ranges that are never globally routed (used for "your network" labels).
pub fn is_private(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || (o[0] == 100 && (o[1] & 0xC0) == 64) // 100.64/10 CGNAT
        }
        IpAddr::V6(v6) => {
            let s = v6.segments()[0];
            v6.is_loopback() || (s & 0xfe00) == 0xfc00 || (s & 0xffc0) == 0xfe80
        }
    }
}
