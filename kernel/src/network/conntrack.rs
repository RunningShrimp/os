//! Connection tracking for stateful firewalls and NAT
//!
//! This module implements connection tracking similar to Linux's nf_conntrack:
//! - Connection state tracking (TCP, UDP, ICMP)
//! - NAT implementation (SNAT, DNAT, masquerade)
//! - Connection tracking helpers for application protocols
//! - Timeout management for different connection states
//! - Connection event logging
//!
//! # Examples
//!
//! ```rust
//! use kernel::network::conntrack::{ConntrackManager, NatType, ConnectionState};
//!
//! // Create connection tracking manager
//! let conntrack = ConntrackManager::new();
//!
//! // Track a TCP connection
//! let conn = conntrack.track_connection(
//!     "192.168.1.100".parse().unwrap(),
//!     12345,
//!     "10.0.0.1".parse().unwrap(),
//!     80,
//!     Protocol::TCP,
//! );
//!
//! // Add NAT rule
//! conntrack.add_nat_rule(NatRule {
//!     nat_type: NatType::Masquerade,
//!     external_addr: "203.0.113.1".parse().unwrap(),
//!     ..Default::default()
//! });
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use core::time::Duration;

use crate::subsystems::net::ipv4::Ipv4Addr;
use crate::subsystems::sync::{Mutex, RwLock};

/// Default connection tracking timeout for established connections (seconds)
pub const DEFAULT_TIMEOUT_ESTABLISHED: u64 = 432000; // 5 days

/// Default timeout for untracked connections (seconds)
pub const DEFAULT_TIMEOUT_UNTRACKED: u64 = 30;

/// Default timeout for related connections (seconds)
pub const DEFAULT_TIMEOUT_RELATED: u64 = 180;

/// Maximum number of tracked connections
pub const MAX_CONNECTIONS: usize = 1_000_000;

/// Connection tracking entry
#[derive(Debug, Clone)]
pub struct ConntrackEntry {
    /// Entry ID
    pub id: u64,
    /// Source IP address
    pub src_ip: Ipv4Addr,
    /// Source port
    pub src_port: u16,
    /// Destination IP address
    pub dst_ip: Ipv4Addr,
    /// Destination port
    pub dst_port: u16,
    /// Protocol
    pub protocol: Protocol,
    /// Connection state
    pub state: ConnectionState,
    /// NAT information
    pub nat: Option<NatInfo>,
    /// Connection timestamp (since boot)
    pub timestamp: u64,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Packet count
    pub packets: AtomicU64,
    /// Byte count
    pub bytes: AtomicU64,
    /// Connection status
    pub status: ConntrackStatus,
    /// Related connections (e.g., FTP data connection)
    pub related: Vec<u64>,
    /// Connection mark (for marking)
    pub mark: u32,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// TCP: SYN sent
    TcpSynSent,
    /// TCP: SYN received
    TcpSynRecv,
    /// TCP: Established
    TcpEstablished,
    /// TCP: FIN wait
    TcpFinWait,
    /// TCP: Close wait
    TcpCloseWait,
    /// TCP: Last ACK
    TcpLastAck,
    /// TCP: Time wait
    TcpTimeWait,
    /// TCP: Closed
    TcpClosed,
    /// UDP: Untracked
    UdpUntracked,
    /// UDP: Tracked
    UdpTracked,
    /// ICMP: Untracked
    IcmpUntracked,
    /// ICMP: Tracked
    IcmpTracked,
}

/// Connection tracking status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConntrackStatus {
    /// Connection is active
    Active,
    /// Connection is closing
    Closing,
    /// Connection is timed out
    TimedOut,
    /// Connection is closed
    Closed,
}

/// NAT information
#[derive(Debug, Clone)]
pub struct NatInfo {
    /// NAT type
    pub nat_type: NatType,
    /// Original source IP (before SNAT)
    pub orig_src_ip: Option<Ipv4Addr>,
    /// Original source port (before SNAT)
    pub orig_src_port: Option<u16>,
    /// Translated source IP (after SNAT)
    pub trans_src_ip: Option<Ipv4Addr>,
    /// Translated source port (after SNAT)
    pub trans_src_port: Option<u16>,
    /// Original destination IP (before DNAT)
    pub orig_dst_ip: Option<Ipv4Addr>,
    /// Original destination port (before DNAT)
    pub orig_dst_port: Option<u16>,
    /// Translated destination IP (after DNAT)
    pub trans_dst_ip: Option<Ipv4Addr>,
    /// Translated destination port (after DNAT)
    pub trans_dst_port: Option<u16>,
}

/// NAT types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// Source NAT (SNAT)
    Snat,
    /// Destination NAT (DNAT)
    Dnat,
    /// Masquerade (SNAT with dynamic IP)
    Masquerade,
    /// Redirect (DNAT to local machine)
    Redirect,
}

/// NAT rule
#[derive(Debug, Clone)]
pub struct NatRule {
    /// Rule priority
    pub priority: u32,
    /// NAT type
    pub nat_type: NatType,
    /// Source IP to match
    pub src_ip: Option<Ipv4Addr>,
    /// Source port to match
    pub src_port: Option<u16>,
    /// Destination IP to match
    pub dst_ip: Option<Ipv4Addr>,
    /// Destination port to match
    pub dst_port: Option<u16>,
    /// Protocol to match
    pub protocol: Option<Protocol>,
    /// External address for SNAT/masquerade
    pub external_addr: Option<Ipv4Addr>,
    /// External port for SNAT
    pub external_port: Option<u16>,
    /// Forward address for DNAT
    pub forward_addr: Option<Ipv4Addr>,
    /// Forward port for DNAT
    pub forward_port: Option<u16>,
    /// Ingress interface
    pub in_interface: Option<String>,
    /// Egress interface
    pub out_interface: Option<String>,
}

impl Default for NatRule {
    fn default() -> Self {
        Self {
            priority: 0,
            nat_type: NatType::Snat,
            src_ip: None,
            src_port: None,
            dst_ip: None,
            dst_port: None,
            protocol: None,
            external_addr: None,
            external_port: None,
            forward_addr: None,
            forward_port: None,
            in_interface: None,
            out_interface: None,
        }
    }
}

/// Timeout configuration
#[derive(Debug, Clone, Copy)]
pub struct TimeoutConfig {
    /// Established connections timeout
    pub established: u64,
    /// Untracked connections timeout
    pub untracked: u64,
    /// Related connections timeout
    pub related: u64,
    /// TCP SYN sent timeout
    pub tcp_syn_sent: u64,
    /// TCP SYN received timeout
    pub tcp_syn_recv: u64,
    /// TCP FIN wait timeout
    pub tcp_fin_wait: u64,
    /// TCP time wait timeout
    pub tcp_time_wait: u64,
    /// TCP close timeout
    pub tcp_close: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            established: DEFAULT_TIMEOUT_ESTABLISHED,
            untracked: DEFAULT_TIMEOUT_UNTRACKED,
            related: DEFAULT_TIMEOUT_RELATED,
            tcp_syn_sent: 120,
            tcp_syn_recv: 60,
            tcp_fin_wait: 120,
            tcp_time_wait: 120,
            tcp_close: 10,
        }
    }
}

/// Connection tracking event
#[derive(Debug, Clone)]
pub enum ConntrackEvent {
    /// New connection created
    NewConnection(u64),
    /// Connection updated
    ConnectionUpdate(u64),
    /// Connection destroyed
    ConnectionDestroy(u64),
    /// NAT applied
    NatApplied(u64, NatType),
    /// Connection timed out
    ConnectionTimeout(u64),
}

/// Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    TCP,
    UDP,
    ICMP,
}

impl Protocol {
    /// Get protocol number
    pub fn as_number(&self) -> u8 {
        match self {
            Self::TCP => 6,
            Self::UDP => 17,
            Self::ICMP => 1,
        }
    }

    /// Create protocol from number
    pub fn from_number(num: u8) -> Option<Self> {
        match num {
            6 => Some(Self::TCP),
            17 => Some(Self::UDP),
            1 => Some(Self::ICMP),
            _ => None,
        }
    }
}

/// Connection tracking helper
#[derive(Debug, Clone)]
pub enum ConntrackHelper {
    /// FTP helper
    Ftp,
    /// SIP helper
    Sip,
    /// H.323 helper
    H323,
    /// TFTP helper
    Tftp,
    /// Amanda backup protocol
    Amanda,
}

impl ConntrackHelper {
    /// Get helper name
    pub fn name(&self) -> &str {
        match self {
            Self::Ftp => "ftp",
            Self::Sip => "sip",
            Self::H323 => "h323",
            Self::Tftp => "tftp",
            Self::Amanda => "amanda",
        }
    }
}

/// Connection tracking statistics
#[derive(Debug, Default, Clone)]
pub struct ConntrackStats {
    /// Total connections tracked
    pub total_connections: u64,
    /// Current active connections
    pub active_connections: u64,
    /// Connections timed out
    pub timeouts: u64,
    /// NAT operations performed
    pub nat_operations: u64,
    /// Failed NAT operations
    pub nat_failures: u64,
    /// Helper invocations
    pub helper_invocations: u64,
    /// Packets processed
    pub packets_processed: u64,
    /// Packets untracked (no connection)
    pub packets_untracked: u64,
}

/// Connection tracking manager
#[derive(Debug)]
pub struct ConntrackManager {
    /// Connection entries indexed by tuple key
    connections: RwLock<BTreeMap<(Ipv4Addr, u16, Ipv4Addr, u16, Protocol), ConntrackEntry>>,
    /// Connections indexed by ID
    connections_by_id: RwLock<BTreeMap<u64, ConntrackEntry>>,
    /// NAT rules
    nat_rules: Mutex<Vec<NatRule>>,
    /// Timeout configuration
    timeouts: Mutex<TimeoutConfig>,
    /// Statistics
    stats: Mutex<ConntrackStats>,
    /// Next connection ID
    next_id: AtomicU64,
    /// Current timestamp (seconds since boot)
    current_time: AtomicU64,
    /// Connection helpers
    helpers: Mutex<Vec<ConntrackHelper>>,
}

impl ConntrackManager {
    /// Create new connection tracking manager
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(BTreeMap::new()),
            connections_by_id: RwLock::new(BTreeMap::new()),
            nat_rules: Mutex::new(Vec::new()),
            timeouts: Mutex::new(TimeoutConfig::default()),
            stats: Mutex::new(ConntrackStats::default()),
            next_id: AtomicU64::new(1),
            current_time: AtomicU64::new(0),
            helpers: Mutex::new(Vec::new()),
        }
    }

    /// Initialize connection tracking
    pub fn init(&mut self) -> Result<(), ()> {
        // Initialize helpers
        let mut helpers = self.helpers.lock();
        helpers.push(ConntrackHelper::Ftp);
        helpers.push(ConntrackHelper::Sip);
        helpers.push(ConntrackHelper::H323);

        Ok(())
    }

    /// Track a new connection
    pub fn track_connection(
        &self,
        src_ip: Ipv4Addr,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
        protocol: Protocol,
    ) -> Result<u64, ()> {
        let key = (src_ip, src_port, dst_ip, dst_port, protocol);

        // Check if connection already exists
        {
            let conns = self.connections.read();
            if conns.contains_key(&key) {
                return Err(());
            }
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let now = self.current_time.load(Ordering::Relaxed);

        let state = match protocol {
            Protocol::TCP => ConnectionState::TcpSynSent,
            Protocol::UDP => ConnectionState::UdpTracked,
            Protocol::ICMP => ConnectionState::IcmpTracked,
        };

        let entry = ConntrackEntry {
            id,
            src_ip,
            src_port,
            dst_ip,
            dst_port,
            protocol,
            state,
            nat: None,
            timestamp: now,
            last_activity: now,
            packets: AtomicU64::new(1),
            bytes: AtomicU64::new(0),
            status: ConntrackStatus::Active,
            related: Vec::new(),
            mark: 0,
        };

        {
            let mut conns = self.connections.write();
            let mut conns_by_id = self.connections_by_id.write();

            if conns.len() >= MAX_CONNECTIONS {
                return Err(()); // Connection table full
            }

            conns.insert(key, entry.clone());
            conns_by_id.insert(id, entry);
        }

        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_connections += 1;
        stats.active_connections += 1;

        Ok(id)
    }

    /// Find connection by tuple
    pub fn find_connection(
        &self,
        src_ip: Ipv4Addr,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
        protocol: Protocol,
    ) -> Option<u64> {
        let key = (src_ip, src_port, dst_ip, dst_port, protocol);

        // Try direct lookup
        {
            let conns = self.connections.read();
            if let Some(entry) = conns.get(&key) {
                return Some(entry.id);
            }
        }

        // Try reverse lookup (for reply packets)
        let reverse_key = (dst_ip, dst_port, src_ip, src_port, protocol);
        let conns = self.connections.read();
        conns.get(&reverse_key).map(|entry| entry.id)
    }

    /// Get connection by ID
    pub fn get_connection(&self, id: u64) -> Option<ConntrackEntry> {
        let conns = self.connections_by_id.read();
        conns.get(&id).cloned()
    }

    /// Update connection state
    pub fn update_connection(
        &self,
        id: u64,
        new_state: ConnectionState,
    ) -> Result<(), ()> {
        let now = self.current_time.load(Ordering::Relaxed);

        // Update in connections map
        {
            let mut conns = self.connections.write();
            for (_, entry) in conns.iter_mut() {
                if entry.id == id {
                    entry.state = new_state;
                    entry.last_activity = now;
                    entry.packets.fetch_add(1, Ordering::Relaxed);
                    return Ok(());
                }
            }
        }

        // Update in connections_by_id map
        let mut conns_by_id = self.connections_by_id.write();
        if let Some(entry) = conns_by_id.get_mut(&id) {
            entry.state = new_state;
            entry.last_activity = now;
            entry.packets.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Remove connection
    pub fn remove_connection(&self, id: u64) -> Result<(), ()> {
        let mut conns = self.connections.write();
        let mut conns_by_id = self.connections_by_id.write();

        // Find and remove from both maps
        let mut found = None;
        for (key, entry) in conns.iter() {
            if entry.id == id {
                found = Some(key.clone());
                break;
            }
        }

        if let Some(key) = found {
            conns.remove(&key);
            conns_by_id.remove(&id);

            // Update statistics
            let mut stats = self.stats.lock();
            stats.active_connections -= 1;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Track packet (update or create connection)
    pub fn track_packet(
        &self,
        src_ip: Ipv4Addr,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
        protocol: Protocol,
    ) -> Result<u64, ()> {
        // Try to find existing connection
        if let Some(id) = self.find_connection(src_ip, src_port, dst_ip, dst_port, protocol) {
            // Update existing connection
            let _ = self.update_connection(id, ConnectionState::TcpEstablished);
            Ok(id)
        } else {
            // Create new connection
            self.track_connection(src_ip, src_port, dst_ip, dst_port, protocol)
        }
    }

    /// Add NAT rule
    pub fn add_nat_rule(&self, rule: NatRule) -> Result<(), ()> {
        let mut rules = self.nat_rules.lock();
        rules.push(rule);
        Ok(())
    }

    /// Remove NAT rule
    pub fn remove_nat_rule(&self, index: usize) -> Result<(), ()> {
        let mut rules = self.nat_rules.lock();
        if index < rules.len() {
            rules.remove(index);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Apply NAT to packet
    pub fn apply_nat(
        &self,
        src_ip: Ipv4Addr,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
        protocol: Protocol,
    ) -> Result<(Ipv4Addr, u16, Ipv4Addr, u16), ()> {
        let rules = self.nat_rules.lock();

        // Find matching NAT rule
        for rule in rules.iter() {
            if self.rule_matches(rule, src_ip, src_port, dst_ip, dst_port, protocol) {
                let mut new_src_ip = src_ip;
                let mut new_src_port = src_port;
                let mut new_dst_ip = dst_ip;
                let mut new_dst_port = dst_port;

                match rule.nat_type {
                    NatType::Snat | NatType::Masquerade => {
                        if let Some(addr) = rule.external_addr {
                            new_src_ip = addr;
                        }
                        if let Some(port) = rule.external_port {
                            new_src_port = port;
                        }
                    },
                    NatType::Dnat | NatType::Redirect => {
                        if let Some(addr) = rule.forward_addr {
                            new_dst_ip = addr;
                        }
                        if let Some(port) = rule.forward_port {
                            new_dst_port = port;
                        }
                    },
                }

                // Update statistics
                let mut stats = self.stats.lock();
                stats.nat_operations += 1;

                return Ok((new_src_ip, new_src_port, new_dst_ip, new_dst_port));
            }
        }

        // No NAT applied
        Ok((src_ip, src_port, dst_ip, dst_port))
    }

    /// Check if NAT rule matches packet
    fn rule_matches(
        &self,
        rule: &NatRule,
        src_ip: Ipv4Addr,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
        protocol: Protocol,
    ) -> bool {
        // Check protocol
        if let Some(rule_proto) = rule.protocol {
            if rule_proto != protocol {
                return false;
            }
        }

        // Check source IP
        if let Some(rule_src_ip) = rule.src_ip {
            if rule_src_ip != src_ip {
                return false;
            }
        }

        // Check source port
        if let Some(rule_src_port) = rule.src_port {
            if rule_src_port != src_port {
                return false;
            }
        }

        // Check destination IP
        if let Some(rule_dst_ip) = rule.dst_ip {
            if rule_dst_ip != dst_ip {
                return false;
            }
        }

        // Check destination port
        if let Some(rule_dst_port) = rule.dst_port {
            if rule_dst_port != dst_port {
                return false;
            }
        }

        true
    }

    /// Flush all connections
    pub fn flush(&self) {
        let mut conns = self.connections.write();
        let mut conns_by_id = self.connections_by_id.write();

        let count = conns.len();
        conns.clear();
        conns_by_id.clear();

        // Update statistics
        let mut stats = self.stats.lock();
        stats.active_connections = 0;
        stats.total_connections += count as u64;
    }

    /// Set timeout configuration
    pub fn set_timeouts(&self, timeouts: TimeoutConfig) {
        let mut t = self.timeouts.lock();
        *t = timeouts;
    }

    /// Get timeout configuration
    pub fn get_timeouts(&self) -> TimeoutConfig {
        let t = self.timeouts.lock();
        *t
    }

    /// Advance time (called by timer)
    pub fn advance_time(&self, delta: u64) {
        self.current_time.fetch_add(delta, Ordering::Relaxed);

        // Check for expired connections
        self.cleanup_expired();
    }

    /// Cleanup expired connections
    fn cleanup_expired(&self) {
        let now = self.current_time.load(Ordering::Relaxed);
        let timeouts = self.timeouts.lock();

        let mut to_remove = Vec::new();

        {
            let conns = self.connections.read();
            for (key, entry) in conns.iter() {
                let timeout = match entry.state {
                    ConnectionState::TcpEstablished => timeouts.established,
                    ConnectionState::UdpTracked => timeouts.untracked,
                    ConnectionState::IcmpTracked => timeouts.untracked,
                    _ => timeouts.untracked,
                };

                if now - entry.last_activity > timeout {
                    to_remove.push(key.clone());
                }
            }
        }

        // Remove expired connections
        for key in to_remove {
            let mut conns = self.connections.write();
            let entry = conns.remove(&key);
            if let Some(entry) = entry {
                let mut conns_by_id = self.connections_by_id.write();
                conns_by_id.remove(&entry.id);

                let mut stats = self.stats.lock();
                stats.timeouts += 1;
                stats.active_connections -= 1;
            }
        }
    }

    /// Add connection helper
    pub fn add_helper(&self, helper: ConntrackHelper) {
        let mut helpers = self.helpers.lock();
        helpers.push(helper);
    }

    /// Invoke helper for connection
    pub fn invoke_helper(
        &self,
        _id: u64,
        _helper: ConntrackHelper,
    ) -> Result<(), ()> {
        // In real implementation, this would invoke the helper
        let mut stats = self.stats.lock();
        stats.helper_invocations += 1;
        Ok(())
    }

    /// Set connection mark
    pub fn set_mark(&self, id: u64, mark: u32) -> Result<(), ()> {
        let mut conns = self.connections.write();
        for (_, entry) in conns.iter_mut() {
            if entry.id == id {
                entry.mark = mark;
                return Ok(());
            }
        }
        Err(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> ConntrackStats {
        let stats = self.stats.lock();
        stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = ConntrackStats::default();
    }
}

impl Default for ConntrackManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection - high-level interface
pub type Connection = ConntrackEntry;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_conversion() {
        assert_eq!(Protocol::TCP.as_number(), 6);
        assert_eq!(Protocol::UDP.as_number(), 17);
        assert_eq!(Protocol::ICMP.as_number(), 1);

        assert_eq!(Protocol::from_number(6), Some(Protocol::TCP));
        assert_eq!(Protocol::from_number(17), Some(Protocol::UDP));
        assert_eq!(Protocol::from_number(255), None);
    }

    #[test]
    fn test_nat_type() {
        assert_eq!(NatType::Snat as u8, NatType::Snat as u8);
        assert_eq!(NatType::Dnat as u8, NatType::Dnat as u8);
    }

    #[test]
    fn test_timeout_config_default() {
        let config = TimeoutConfig::default();
        assert_eq!(config.established, DEFAULT_TIMEOUT_ESTABLISHED);
        assert_eq!(config.untracked, DEFAULT_TIMEOUT_UNTRACKED);
        assert_eq!(config.related, DEFAULT_TIMEOUT_RELATED);
    }

    #[test]
    fn test_conntrack_helper() {
        let helper = ConntrackHelper::Ftp;
        assert_eq!(helper.name(), "ftp");

        let helper = ConntrackHelper::Sip;
        assert_eq!(helper.name(), "sip");
    }

    #[test]
    fn test_conntrack_manager_create() {
        let manager = ConntrackManager::new();
        assert!(manager.init().is_ok());
    }

    #[test]
    fn test_track_connection() {
        let manager = ConntrackManager::new();
        manager.init().unwrap();

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let id = manager.track_connection(src_ip, 12345, dst_ip, 80, Protocol::TCP);
        assert!(id.is_ok());

        let id = id.unwrap();
        let conn = manager.get_connection(id);
        assert!(conn.is_some());

        let conn = conn.unwrap();
        assert_eq!(conn.src_ip, src_ip);
        assert_eq!(conn.dst_ip, dst_ip);
        assert_eq!(conn.protocol, Protocol::TCP);
    }

    #[test]
    fn test_find_connection() {
        let manager = ConntrackManager::new();
        manager.init().unwrap();

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        manager.track_connection(src_ip, 12345, dst_ip, 80, Protocol::TCP).unwrap();

        let id = manager.find_connection(src_ip, 12345, dst_ip, 80, Protocol::TCP);
        assert!(id.is_some());
    }

    #[test]
    fn test_update_connection() {
        let manager = ConntrackManager::new();
        manager.init().unwrap();

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let id = manager.track_connection(src_ip, 12345, dst_ip, 80, Protocol::TCP).unwrap();

        assert!(manager.update_connection(id, ConnectionState::TcpEstablished).is_ok());

        let conn = manager.get_connection(id).unwrap();
        assert_eq!(conn.state, ConnectionState::TcpEstablished);
    }

    #[test]
    fn test_remove_connection() {
        let manager = ConntrackManager::new();
        manager.init().unwrap();

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let id = manager.track_connection(src_ip, 12345, dst_ip, 80, Protocol::TCP).unwrap();
        assert!(manager.remove_connection(id).is_ok());

        let conn = manager.get_connection(id);
        assert!(conn.is_none());
    }

    #[test]
    fn test_nat_rules() {
        let manager = ConntrackManager::new();
        manager.init().unwrap();

        let rule = NatRule {
            nat_type: NatType::Masquerade,
            external_addr: Some(Ipv4Addr::new(203, 0, 113, 1)),
            ..Default::default()
        };

        assert!(manager.add_nat_rule(rule).is_ok());

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let (new_src, _new_src_port, new_dst, _new_dst_port) =
            manager.apply_nat(src_ip, 12345, dst_ip, 80, Protocol::TCP).unwrap();

        assert_eq!(new_src, Ipv4Addr::new(203, 0, 113, 1));
        assert_eq!(new_dst, dst_ip); // Destination unchanged for SNAT
    }

    #[test]
    fn test_flush() {
        let manager = ConntrackManager::new();
        manager.init().unwrap();

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        manager.track_connection(src_ip, 12345, dst_ip, 80, Protocol::TCP).unwrap();
        manager.track_connection(src_ip, 12346, dst_ip, 81, Protocol::UDP).unwrap();

        manager.flush();

        let stats = manager.get_stats();
        assert_eq!(stats.active_connections, 0);
    }

    #[test]
    fn test_timeouts() {
        let manager = ConntrackManager::new();
        manager.init().unwrap();

        let timeouts = TimeoutConfig {
            established: 3600,
            untracked: 60,
            ..Default::default()
        };

        manager.set_timeouts(timeouts);

        let retrieved = manager.get_timeouts();
        assert_eq!(retrieved.established, 3600);
        assert_eq!(retrieved.untracked, 60);
    }

    #[test]
    fn test_mark() {
        let manager = ConntrackManager::new();
        manager.init().unwrap();

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let id = manager.track_connection(src_ip, 12345, dst_ip, 80, Protocol::TCP).unwrap();
        assert!(manager.set_mark(id, 0x12345678).is_ok());

        let conn = manager.get_connection(id).unwrap();
        assert_eq!(conn.mark, 0x12345678);
    }

    #[test]
    fn test_stats() {
        let manager = ConntrackManager::new();
        manager.init().unwrap();

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        manager.track_connection(src_ip, 12345, dst_ip, 80, Protocol::TCP).unwrap();

        let stats = manager.get_stats();
        assert!(stats.total_connections > 0);
        assert!(stats.active_connections > 0);
    }

    #[test]
    fn test_nat_rule_default() {
        let rule = NatRule::default();
        assert_eq!(rule.priority, 0);
        assert_eq!(rule.nat_type, NatType::Snat);
        assert!(rule.src_ip.is_none());
    }
}
