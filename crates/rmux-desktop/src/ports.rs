//! Explicit local socket snapshots. No subprocess, background polling or shell input.
use anyhow::{Result, ensure};
use std::{collections::BTreeSet, net::SocketAddr, sync::atomic::{AtomicBool, Ordering}};
static SCANNING: AtomicBool = AtomicBool::new(false);
pub struct Permit;
impl Permit {
    pub fn acquire() -> Result<Self> {
        ensure!(SCANNING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_ok(), "Another port scan is running; retry shortly");
        Ok(Self)
    }
}
impl Drop for Permit { fn drop(&mut self) { SCANNING.store(false, Ordering::Release); } }
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Listener { pub address: SocketAddr, pub pid: u32 }
/// An explicit HTTP launch target, not protocol detection. Scoped IPv6
/// addresses are copy-only because browser support for zone IDs varies.
pub fn http_url(mut address: SocketAddr) -> Option<String> {
    if address.port() == 0 { return None; }
    if let SocketAddr::V6(ipv6) = address { if ipv6.scope_id() != 0 { return None; } }
    if address.ip().is_unspecified() {
        address.set_ip(if address.is_ipv4() { std::net::Ipv4Addr::LOCALHOST.into() } else { std::net::Ipv6Addr::LOCALHOST.into() });
    }
    Some(format!("http://{address}/"))
}

pub struct Report { pub listeners: Vec<Listener>, pub truncated: bool, pub ownership_changed: bool }
use crate::process_tree::{process_snapshot, stable_owned, descendants};

impl Report {
    pub fn sidebar_summary(&self) -> String {
        let ports: BTreeSet<_> = self.listeners.iter().map(|listener| listener.address.port()).collect();
        let partial = if self.truncated || self.ownership_changed { " · partial" } else { "" };
        if ports.is_empty() { return format!("Ports snapshot: none{partial}"); }
        let list = ports.iter().take(4).map(u16::to_string).collect::<Vec<_>>().join(", ");
        let more = if ports.len() > 4 { format!(" +{}", ports.len() - 4) } else { String::new() };
        format!("Ports snapshot: {list}{more}{partial}")
    }
}

pub fn scan(root: u32, canceled: &AtomicBool) -> Result<Option<Report>> {
    use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
    if canceled.load(Ordering::Acquire) { return Ok(None); }
    let before = process_snapshot();
    if canceled.load(Ordering::Acquire) { return Ok(None); }
    ensure!(before.contains_key(&root), "Terminal process is no longer running");
    let owned = descendants(root, before.iter().map(|(pid, identity)| (*pid, identity.parent)))?;
    let mut listeners = BTreeSet::new();
    let mut truncated = false;
    if canceled.load(Ordering::Acquire) { return Ok(None); }
    let mut sockets = netstat2::iterate_sockets_info(AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6, ProtocolFlags::TCP)?;
    let mut index = 0;
    loop {
        if canceled.load(Ordering::Acquire) { return Ok(None); }
        if index == 100_000 { truncated = true; break; }
        let Some(socket) = sockets.next() else { break; };
        let socket = socket?;
        index += 1;
        let ProtocolSocketInfo::Tcp(tcp) = socket.protocol_socket_info else { continue; };
        if tcp.state != TcpState::Listen { continue; }
        for pid in socket.associated_pids.into_iter().filter(|pid| owned.contains(pid)) {
            listeners.insert(Listener { address: SocketAddr::new(tcp.local_addr, tcp.local_port), pid });
            if listeners.len() > 128 { listeners.pop_last(); truncated = true; break; }
        }
        if truncated { break; }
    }
    if canceled.load(Ordering::Acquire) { return Ok(None); }
    let after = process_snapshot();
    if canceled.load(Ordering::Acquire) { return Ok(None); }
    let stable = stable_owned(root, &before, &after)?;
    let count = listeners.len();
    listeners.retain(|listener| stable.contains(&listener.pid));
    let ownership_changed = listeners.len() != count;
    Ok(Some(Report { listeners: listeners.into_iter().collect(), truncated, ownership_changed }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sidebar_snapshot_deduplicates_ports_and_marks_partial_results() {
        let listeners = [3000,3000,4000,5000,6000,7000].into_iter().enumerate().map(|(index,port)| Listener {
            address: SocketAddr::from(([127,0,0,1],port)), pid: index as u32,
        }).collect();
        let mut report = Report { listeners, truncated: false, ownership_changed: false };
        assert_eq!(report.sidebar_summary(), "Ports snapshot: 3000, 4000, 5000, 6000 +1");
        report.ownership_changed = true;
        assert!(report.sidebar_summary().ends_with(" · partial"));
        report.listeners.clear();
        assert_eq!(report.sidebar_summary(), "Ports snapshot: none · partial");
    }
    #[test]
    fn canceled_scan_does_not_require_a_live_process() -> Result<()> {
        assert!(scan(0, &AtomicBool::new(true))?.is_none());
        Ok(())
    }
    #[test]
    fn snapshot_finds_current_process_loopback_listener() -> Result<()> {
        let socket = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = socket.local_addr()?;
        let report = scan(std::process::id(), &AtomicBool::new(false))?.expect("active scan");
        assert!(report.listeners.iter().any(|row| row.address == address && row.pid == std::process::id()));
        Ok(())
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    #[test]
    fn browser_targets_map_wildcards_and_preserve_explicit_addresses() {
        for (address, expected) in [
            ("0.0.0.0:3000", "http://127.0.0.1:3000/"),
            ("[::]:8080", "http://[::1]:8080/"),
            ("127.0.0.1:443", "http://127.0.0.1:443/"),
            ("192.168.1.2:9000", "http://192.168.1.2:9000/"),
            ("[::1]:4000", "http://[::1]:4000/"),
        ] { assert_eq!(http_url(address.parse().unwrap()).as_deref(), Some(expected)); }
        assert!(http_url("127.0.0.1:0".parse().unwrap()).is_none());
        assert!(http_url("[fe80::1%2]:8000".parse().unwrap()).is_none());
    }
}
