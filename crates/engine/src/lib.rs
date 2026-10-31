//! Shared MTR-style tracing engine used by both the Tauri and native GUIs.
//!
//! Wraps `trippy-core` and exposes a small, serializable snapshot model so the
//! UI layers never touch trippy types directly.

mod dns;
pub mod report;
mod session;

pub use dns::Resolver;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TraceOptions {
    pub target: String,
    pub protocol: Protocol,
    pub family: AddrFamily,
    /// Minimum time per round (mtr's "interval").
    pub interval_ms: u64,
    pub max_ttl: u8,
    pub packet_size: u16,
    /// Stop after this many rounds; `None` = run until stopped.
    pub max_rounds: Option<usize>,
    /// Destination port for UDP/TCP (ignored for ICMP).
    pub port: Option<u16>,
    pub resolve_hostnames: bool,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            target: String::new(),
            protocol: Protocol::Icmp,
            family: AddrFamily::Any,
            interval_ms: 1000,
            max_ttl: 30,
            packet_size: 84,
            max_rounds: None,
            port: None,
            resolve_hostnames: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEntry {
    pub ip: IpAddr,
    pub hostname: Option<String>,
    /// How many replies came from this address (ECMP paths show >1 host).
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HopStats {
    pub ttl: u8,
    pub hosts: Vec<HostEntry>,
    pub sent: usize,
    pub recv: usize,
    pub loss_pct: f64,
    pub last_ms: Option<f64>,
    pub best_ms: Option<f64>,
    pub worst_ms: Option<f64>,
    pub avg_ms: f64,
    pub stddev_ms: f64,
    pub jitter_ms: Option<f64>,
    /// Recent RTT samples, oldest first. `None` = probe lost.
    pub samples: Vec<Option<f64>>,
    pub is_target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceStatus {
    Running,
    Finished,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceSnapshot {
    pub id: u64,
    pub target: String,
    pub target_addr: IpAddr,
    pub protocol: Protocol,
    pub status: TraceStatus,
    pub rounds: usize,
    pub hops: Vec<HopStats>,
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
        "Raw sockets need privileges. Either grant the binary CAP_NET_RAW \
         (sudo setcap cap_net_raw+ep <path-to-binary>) or allow unprivileged ICMP \
         (sudo sysctl -w net.ipv4.ping_group_range=\"0 2147483647\")."
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
    if lower.contains("permission denied") || lower.contains("os error 13")
        || lower.contains("access is denied") || lower.contains("os error 10013")
        || lower.contains("operation not permitted")
    {
        format!("{msg}\n\n{}", permission_hint())
    } else {
        msg
    }
}
