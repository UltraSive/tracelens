//! Headless smoke test: `cargo run -p mtr-engine --example cli -- 1.1.1.1 [icmp|udp|tcp]`
use mtr_engine::{report, Protocol, Resolver, TraceOptions, TraceSession};
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1);
    let target = args.next().unwrap_or_else(|| "1.1.1.1".into());
    let protocol = match args.next().as_deref() {
        Some("udp") => Protocol::Udp,
        Some("tcp") => Protocol::Tcp,
        _ => Protocol::Icmp,
    };
    println!("{:?}", mtr_engine::PrivilegeInfo::discover());
    let opts = TraceOptions {
        target,
        protocol,
        max_rounds: Some(8),
        ..Default::default()
    };
    let s = TraceSession::start(opts, Resolver::new()).expect("start");
    std::thread::sleep(Duration::from_secs(10));
    let snap = s.snapshot();
    println!("status={:?} err={:?}", snap.status, snap.error);
    print!("{}", report::to_text(&snap));
    for d in &snap.verdict.details {
        println!("  - {d}");
    }
}
