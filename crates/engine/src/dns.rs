//! Reverse DNS + ASN (Team Cymru) lookups on a small worker pool.
use std::collections::HashMap;
use std::net::{IpAddr, ToSocketAddrs};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use trippy_dns::{
    Config, DnsEntry, DnsResolver, IpAddrFamily, ResolveMethod, Resolved, Unresolved,
};

use crate::{AddrFamily, EngineError};

const WORKERS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsnInfo {
    pub asn: String,
    /// Organisation name, e.g. "Cloudflare, Inc." (falls back to the handle).
    pub name: String,
    /// Registry handle, e.g. "CLOUDFLARENET".
    pub handle: String,
    pub prefix: String,
    pub country: String,
}

#[derive(Clone, Default)]
pub struct HostInfo {
    pub hostname: Option<String>,
    pub asn: Option<AsnInfo>,
}

#[derive(Clone)]
enum Entry {
    Pending,
    Done(HostInfo),
}

/// Non-blocking reverse-DNS/ASN cache. `lookup` returns what is known so far
/// and queues unknown addresses for the background workers.
#[derive(Clone)]
pub struct Resolver {
    cache: Arc<Mutex<HashMap<IpAddr, Entry>>>,
    tx: Sender<IpAddr>,
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

fn make_resolver() -> Option<DnsResolver> {
    let cfg = |m| {
        Config::new(
            m,
            IpAddrFamily::System,
            Duration::from_secs(5),
            Duration::from_secs(3600),
        )
    };
    DnsResolver::start(cfg(ResolveMethod::Resolv))
        .or_else(|_| DnsResolver::start(cfg(ResolveMethod::Cloudflare)))
        .ok()
}

fn to_asn(info: &trippy_dns::AsInfo) -> Option<AsnInfo> {
    if info.asn.is_empty() || info.asn == "NA" {
        return None;
    }
    // Cymru names look like "CLOUDFLARENET - Cloudflare, Inc., AU".
    let full = info
        .name
        .rsplit_once(", ")
        .map_or(info.name.as_str(), |(n, _)| n)
        .trim();
    let (handle, org) = full.split_once(" - ").unwrap_or((full, ""));
    let org = org.trim();
    Some(AsnInfo {
        asn: info.asn.clone(),
        name: if org.is_empty() {
            handle.trim().to_string()
        } else {
            org.to_string()
        },
        handle: handle.trim().to_string(),
        prefix: info.prefix.clone(),
        country: info.cc.clone(),
    })
}

fn resolve_one(resolver: Option<&DnsResolver>, ip: IpAddr) -> HostInfo {
    use trippy_dns::Resolver as _;
    let Some(r) = resolver else {
        let hostname = dns_lookup::lookup_addr(&ip)
            .ok()
            .filter(|n| n.parse::<IpAddr>().is_err());
        return HostInfo {
            hostname,
            asn: None,
        };
    };
    let lookup_asn = !crate::is_private(&ip);
    let entry = if lookup_asn {
        r.reverse_lookup_with_asinfo(ip)
    } else {
        r.reverse_lookup(ip)
    };
    match entry {
        DnsEntry::Resolved(Resolved::WithAsInfo(_, names, info)) => HostInfo {
            hostname: names.into_iter().next(),
            asn: to_asn(&info),
        },
        DnsEntry::Resolved(Resolved::Normal(_, names)) => HostInfo {
            hostname: names.into_iter().next(),
            asn: None,
        },
        DnsEntry::NotFound(Unresolved::WithAsInfo(_, info)) => HostInfo {
            hostname: None,
            asn: to_asn(&info),
        },
        _ => HostInfo::default(),
    }
}

impl Resolver {
    pub fn new() -> Self {
        let cache: Arc<Mutex<HashMap<IpAddr, Entry>>> = Arc::default();
        let (tx, rx) = channel::<IpAddr>();
        let rx: Arc<Mutex<Receiver<IpAddr>>> = Arc::new(Mutex::new(rx));
        for i in 0..WORKERS {
            let rx = rx.clone();
            let cache = cache.clone();
            thread::Builder::new()
                .name(format!("rdns-{i}"))
                .spawn(move || {
                    // trippy-dns' resolver is !Send, so each worker owns one.
                    let resolver = make_resolver();
                    loop {
                        let ip = match rx.lock().unwrap().recv() {
                            Ok(ip) => ip,
                            Err(_) => return,
                        };
                        let info = resolve_one(resolver.as_ref(), ip);
                        cache.lock().unwrap().insert(ip, Entry::Done(info));
                    }
                })
                .expect("spawn rdns worker");
        }
        Self { cache, tx }
    }

    /// Number of addresses still waiting for a lookup.
    pub fn pending(&self) -> usize {
        self.cache
            .lock()
            .unwrap()
            .values()
            .filter(|e| matches!(e, Entry::Pending))
            .count()
    }

    pub fn lookup(&self, ip: IpAddr) -> HostInfo {
        let mut cache = self.cache.lock().unwrap();
        match cache.get(&ip) {
            Some(Entry::Done(info)) => info.clone(),
            Some(Entry::Pending) => HostInfo::default(),
            None => {
                cache.insert(ip, Entry::Pending);
                let _ = self.tx.send(ip);
                HostInfo::default()
            }
        }
    }
}

/// Resolve a target (hostname or literal IP) to a single address.
pub(crate) fn resolve_target(target: &str, family: AddrFamily) -> Result<IpAddr, EngineError> {
    let target = target.trim();
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(ip);
    }
    let addrs: Vec<IpAddr> = (target, 0)
        .to_socket_addrs()
        .map_err(|_| EngineError::Resolve(target.to_string()))?
        .map(|sa| sa.ip())
        .collect();
    let pick = |v4: bool| addrs.iter().copied().find(|a| a.is_ipv4() == v4);
    match family {
        AddrFamily::V4 => pick(true).ok_or(EngineError::NoAddress(target.into(), "IPv4")),
        AddrFamily::V6 => pick(false).ok_or(EngineError::NoAddress(target.into(), "IPv6")),
        AddrFamily::Any => pick(true)
            .or_else(|| pick(false))
            .ok_or(EngineError::Resolve(target.into())),
    }
}
