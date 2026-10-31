//! Command-line flags: `tracelens 1.1.1.1 --udp` opens straight into a trace.
use clap::{ArgGroup, Parser, ValueEnum};
use mtr_engine::{AddrFamily, Multipath, Protocol, TraceOptions};

#[derive(Copy, Clone, ValueEnum)]
enum MultipathArg {
    Classic,
    Paris,
    Dublin,
}

#[derive(Parser)]
#[command(
    name = "tracelens",
    version,
    about = "TraceLens: MTR-style traceroute and network path diagnostics"
)]
#[command(group(ArgGroup::new("proto").args(["icmp", "udp", "tcp"])))]
#[command(group(ArgGroup::new("fam").args(["ipv4", "ipv6"])))]
pub struct Cli {
    /// Host or IP to trace on startup
    pub target: Option<String>,
    /// Use ICMP echo probes (default)
    #[arg(long)]
    icmp: bool,
    /// Use UDP probes
    #[arg(long, short = 'u')]
    udp: bool,
    /// Use TCP SYN probes
    #[arg(long, short = 'T')]
    tcp: bool,
    /// Force IPv4
    #[arg(short = '4')]
    ipv4: bool,
    /// Force IPv6
    #[arg(short = '6')]
    ipv6: bool,
    /// Destination port (UDP/TCP)
    #[arg(long, short = 'P')]
    port: Option<u16>,
    /// Seconds between rounds
    #[arg(long, short = 'i', default_value_t = 1.0)]
    interval: f64,
    /// Stop after this many rounds
    #[arg(long, short = 'c')]
    rounds: Option<usize>,
    /// First hop (TTL) to probe
    #[arg(long, short = 'f', default_value_t = 1)]
    first_ttl: u8,
    /// Max hops (TTL)
    #[arg(long, short = 'm', default_value_t = 30)]
    max_ttl: u8,
    /// Packet size in bytes
    #[arg(long, short = 's', default_value_t = 84)]
    size: u16,
    /// Source address to send from
    #[arg(long)]
    source: Option<String>,
    /// Network interface to send from
    #[arg(long, short = 'I')]
    interface: Option<String>,
    /// DSCP value (0-63)
    #[arg(long, default_value_t = 0)]
    dscp: u8,
    /// ECMP strategy (paris/dublin need --udp)
    #[arg(long, value_enum, default_value = "classic")]
    multipath: MultipathArg,
    /// Don't resolve hostnames / ASNs
    #[arg(long, short = 'n')]
    no_dns: bool,
    /// Run without a window and print an mtr-style report (uses --rounds, default 10)
    #[arg(long, short = 'r', requires = "target")]
    pub report: bool,
}

impl Cli {
    pub fn trace_options(&self) -> Option<TraceOptions> {
        let target = self.target.clone()?;
        Some(TraceOptions {
            target,
            protocol: if self.udp {
                Protocol::Udp
            } else if self.tcp {
                Protocol::Tcp
            } else {
                Protocol::Icmp
            },
            family: if self.ipv4 {
                AddrFamily::V4
            } else if self.ipv6 {
                AddrFamily::V6
            } else {
                AddrFamily::Any
            },
            interval_ms: (self.interval * 1000.0).round().max(20.0) as u64,
            first_ttl: self.first_ttl,
            max_ttl: self.max_ttl,
            packet_size: self.size,
            max_rounds: self.rounds.or(self.report.then_some(10)),
            port: self.port,
            source_addr: self.source.clone(),
            interface: self.interface.clone(),
            dscp: self.dscp,
            multipath: match self.multipath {
                MultipathArg::Classic => Multipath::Classic,
                MultipathArg::Paris => Multipath::Paris,
                MultipathArg::Dublin => Multipath::Dublin,
            },
            resolve_hostnames: !self.no_dns,
        })
    }
}

/// Headless `--report` mode: trace to completion and print the table + verdict.
pub fn run_report(opts: TraceOptions) -> i32 {
    use mtr_engine::{report, Resolver, TraceSession, TraceStatus};
    let session = match TraceSession::start(opts, Resolver::new()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return 1;
        }
    };
    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if session.status() != TraceStatus::Running {
            break;
        }
    }
    // let outstanding reverse DNS / ASN lookups land (bounded)
    let _ = session.snapshot(); // queues lookups for every hop
    for _ in 0..40 {
        if session.resolver().pending() == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    let snap = session.snapshot();
    if let Some(e) = &snap.error {
        eprintln!("error: {e}");
        return 1;
    }
    print!("{}", report::to_text(&snap));
    for d in &snap.verdict.details {
        println!("  - {d}");
    }
    if !snap.path_events.is_empty() {
        println!("Path changes:");
        for e in &snap.path_events {
            println!("  hop {}: {} -> {}", e.ttl, e.from, e.to);
        }
    }
    0
}
