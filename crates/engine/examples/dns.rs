//! Check reverse DNS + ASN lookups: `cargo run -p mtr-engine --example dns -- 1.1.1.1 8.8.8.8`
use mtr_engine::Resolver;
fn main() {
    let r = Resolver::new();
    let ips: Vec<std::net::IpAddr> = std::env::args()
        .skip(1)
        .map(|a| a.parse().unwrap())
        .collect();
    for ip in &ips {
        r.lookup(*ip);
    }
    std::thread::sleep(std::time::Duration::from_secs(8));
    for ip in &ips {
        let i = r.lookup(*ip);
        println!("{ip}: host={:?} asn={:?}", i.hostname, i.asn);
    }
}
