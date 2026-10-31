//! Quick headless smoke test: `cargo run -p mtr-engine --example cli -- 1.1.1.1`
use mtr_engine::{Resolver, TraceOptions, TraceSession};
use std::time::Duration;

fn main() {
    let target = std::env::args().nth(1).unwrap_or_else(|| "1.1.1.1".into());
    println!("{:?}", mtr_engine::PrivilegeInfo::discover());
    let proto = match std::env::args().nth(2).as_deref() { Some("udp") => mtr_engine::Protocol::Udp, _ => mtr_engine::Protocol::Icmp };
    let opts = TraceOptions { target, protocol: proto, max_rounds: Some(3), ..Default::default() };
    let s = TraceSession::start(opts, Resolver::new()).expect("start");
    std::thread::sleep(Duration::from_secs(5));
    let snap = s.snapshot();
    println!("status={:?} rounds={} err={:?}", snap.status, snap.rounds, snap.error);
    for h in &snap.hops {
        let host = h.hosts.first().map(|e| format!("{} {}", e.ip, e.hostname.clone().unwrap_or_default())).unwrap_or("???".into());
        println!("{:>2} {:<50} loss {:>5.1}% avg {:>6.1} samples {}", h.ttl, host, h.loss_pct, h.avg_ms, h.samples.len());
    }
}
