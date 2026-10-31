use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, ToSocketAddrs};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::{AddrFamily, EngineError};

#[derive(Clone)]
enum Entry {
    Pending,
    Resolved(Option<String>),
}

/// Non-blocking reverse-DNS cache. Lookups run on a background worker;
/// `lookup` returns `None` until the answer arrives.
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

impl Resolver {
    pub fn new() -> Self {
        let cache: Arc<Mutex<HashMap<IpAddr, Entry>>> = Arc::default();
        let (tx, rx) = channel::<IpAddr>();
        let worker_cache = cache.clone();
        thread::Builder::new()
            .name("rdns".into())
            .spawn(move || {
                let mut seen = HashSet::new();
                while let Ok(ip) = rx.recv() {
                    if !seen.insert(ip) {
                        continue;
                    }
                    let name = dns_lookup::lookup_addr(&ip)
                        .ok()
                        .filter(|n| n.parse::<IpAddr>().is_err());
                    worker_cache
                        .lock()
                        .unwrap()
                        .insert(ip, Entry::Resolved(name));
                }
            })
            .expect("spawn rdns thread");
        Self { cache, tx }
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<String> {
        let mut cache = self.cache.lock().unwrap();
        match cache.get(&ip) {
            Some(Entry::Resolved(name)) => name.clone(),
            Some(Entry::Pending) => None,
            None => {
                cache.insert(ip, Entry::Pending);
                let _ = self.tx.send(ip);
                None
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
