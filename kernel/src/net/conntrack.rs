//! Connection Tracking Implementation
//!
//! This module provides network connection tracking capabilities:
//! - Connection state tracking (nf_conntrack compatible)
//! - Connection state machine (NEW, ESTABLISHED, RELATED, INVALID)
//! - Stateful packet inspection
//! - NAT (Network Address Translation): SNAT, DNAT, masquerade
//! - Connection tuple hashing (5-tuple: proto, src_ip, src_port, dst_ip, dst_port)
//! - Conntrack table management with timeout handling
//! - Helper protocols (FTP, SIP, H.323, IRC)
//! - Expectation tracking for related connections
//! - Conntrack synchronization for HA
//!
//! Based on Linux nf_conntrack and RFCs:
//! - RFC 2663: Network Address Translation
//! - RFC 3022: Traditional IP Network Address Translator
//! - RFC 3489: STUN
//! - RFC 4787: NAT Behavior for UDP

#![allow(dead_code)]

use alloc::{collections::BTreeMap, string::{String, ToString}, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

use crate::subsystems::sync::{Mutex, RwLock};

// ============================================================================
// Common Types
// ============================================================================

/// Connection tracking result type
pub type ConntrackResult<T> = core::result::Result<T, crate::error::unified::ConntrackError>;

/// Connection ID
pub type ConnId = u64;

/// Connection tuple (5-tuple)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnTuple {
    /// Protocol (TCP=6, UDP=17, etc.)
    pub protocol: u8,
    /// Source IP
    pub src_ip: u32,
    /// Source port
    pub src_port: u16,
    /// Destination IP
    pub dst_ip: u32,
    /// Destination port
    pub dst_port: u16,
}

impl ConnTuple {
    /// Create new connection tuple
    pub fn new(protocol: u8, src_ip: u32, src_port: u16, dst_ip: u32, dst_port: u16) -> Self {
        Self {
            protocol,
            src_ip,
            src_port,
            dst_ip,
            dst_port,
        }
    }

    /// Create TCP connection tuple
    pub fn tcp(src_ip: u32, src_port: u16, dst_ip: u32, dst_port: u16) -> Self {
        Self::new(6, src_ip, src_port, dst_ip, dst_port)
    }

    /// Create UDP connection tuple
    pub fn udp(src_ip: u32, src_port: u16, dst_ip: u32, dst_port: u16) -> Self {
        Self::new(17, src_ip, src_port, dst_ip, dst_port)
    }

    /// Calculate hash
    pub fn hash(&self) -> u64 {
        let mut hash: u64 = 0;
        hash ^= (self.protocol as u64) << 56;
        hash ^= self.src_ip as u64;
        hash ^= (self.src_port as u64) << 16;
        hash ^= (self.dst_ip as u64) << 32;
        hash ^= (self.dst_port as u64) << 48;
        hash
    }

    /// Reverse tuple (swap src/dst)
    pub fn reverse(&self) -> Self {
        Self {
            protocol: self.protocol,
            src_ip: self.dst_ip,
            src_port: self.dst_port,
            dst_ip: self.src_ip,
            dst_port: self.src_port,
        }
    }
}

/// Connection direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnDir {
    Original,
    Reply,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    /// New connection (SYN sent for TCP)
    New,
    /// Connection established
    Established,
    /// Related connection (e.g., FTP data channel)
    Related,
    /// Invalid connection state
    Invalid,
    /// Connection closing (TCP)
    Closing,
    /// Connection closed
    Closed,
}

/// Connection status flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnStatus(u8);

impl ConnStatus {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Seen bidirectional traffic
    pub const SEEN_REPLY: Self = Self(1 << 0);
    /// Assured (won't be deleted)
    pub const ASSURED: Self = Self(1 << 1);
    /// Fixed timeout
    pub const FIXED_TIMEOUT: Self = Self(1 << 2);
    /// NAT enabled
    pub const NAT: Self = Self(1 << 3);

    pub fn new() -> Self {
        Self(0)
    }

    pub fn has(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn set(&mut self, flag: Self) {
        self.0 |= flag.0;
    }

    pub fn clear(&mut self, flag: Self) {
        self.0 &= !flag.0;
    }
}

/// NAT type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// Source NAT
    Snat,
    /// Destination NAT
    Dnat,
    /// Masquerade (SNAT with interface address)
    Masquerade,
    /// Redirect (DNAT to incoming interface)
    Redirect,
}

/// NAT entry
#[derive(Debug, Clone)]
pub struct NatEntry {
    /// NAT type
    pub nat_type: NatType,
    /// Original IP
    pub orig_ip: u32,
    /// Original port
    pub orig_port: u16,
    /// Translated IP
    pub trans_ip: u32,
    /// Translated port
    pub trans_port: u16,
}

/// Connection timeout (seconds)
#[derive(Debug, Clone, Copy)]
pub struct ConnTimeout {
    pub new: u32,
    pub established: u32,
    pub related: u32,
    pub closing: u32,
    pub closed: u32,
}

impl Default for ConnTimeout {
    fn default() -> Self {
        Self {
            new: 10,
            established: 432000, // 5 days
            related: 300,
            closing: 60,
            closed: 10,
        }
    }
}

// ============================================================================
// Connection Entry
// ============================================================================

/// Connection tracking entry
#[derive(Debug)]
pub struct ConnEntry {
    /// Connection ID
    id: ConnId,
    /// Original tuple
    orig_tuple: ConnTuple,
    /// Reply tuple
    reply_tuple: ConnTuple,
    /// Connection state
    state: ConnState,
    /// Status flags
    status: ConnStatus,
    /// NAT entry (if any)
    nat: Option<NatEntry>,
    /// Timeout
    timeout: ConnTimeout,
    /// Creation time
    create_time: u64,
    /// Last update time
    last_update: u64,
    /// Original direction counters
    orig_counters: ConnCounters,
    /// Reply direction counters
    reply_counters: ConnCounters,
    /// Helper (if any)
    helper: Option<String>,
    /// Master connection ID (for related connections)
    master: Option<ConnId>,
}

/// Connection counters
#[derive(Debug, Clone, Copy)]
pub struct ConnCounters {
    /// Packets
    pub packets: u64,
    /// Bytes
    pub bytes: u64,
}

impl Default for ConnCounters {
    fn default() -> Self {
        Self {
            packets: 0,
            bytes: 0,
        }
    }
}

impl ConnEntry {
    /// Create new connection entry
    pub fn new(id: ConnId, orig_tuple: ConnTuple, reply_tuple: ConnTuple, timeout: ConnTimeout) -> Self {
        let now = 0; // In real implementation, get current time

        Self {
            id,
            orig_tuple,
            reply_tuple,
            state: ConnState::New,
            status: ConnStatus::NONE,
            nat: None,
            timeout,
            create_time: now,
            last_update: now,
            orig_counters: ConnCounters::default(),
            reply_counters: ConnCounters::default(),
            helper: None,
            master: None,
        }
    }

    /// Get connection ID
    pub fn id(&self) -> ConnId {
        self.id
    }

    /// Get original tuple
    pub fn orig_tuple(&self) -> ConnTuple {
        self.orig_tuple
    }

    /// Get reply tuple
    pub fn reply_tuple(&self) -> ConnTuple {
        self.reply_tuple
    }

    /// Get connection state
    pub fn state(&self) -> ConnState {
        self.state
    }

    /// Set connection state
    pub fn set_state(&mut self, state: ConnState) {
        self.state = state;
    }

    /// Get status
    pub fn status(&self) -> ConnStatus {
        self.status
    }

    /// Set status flag
    pub fn set_status(&mut self, flag: ConnStatus) {
        self.status.set(flag);
    }

    /// Get NAT entry
    pub fn nat(&self) -> Option<&NatEntry> {
        self.nat.as_ref()
    }

    /// Set NAT entry
    pub fn set_nat(&mut self, nat: NatEntry) {
        self.nat = Some(nat);
        self.status.set(ConnStatus::NAT);
    }

    /// Update counters
    pub fn update(&mut self, dir: ConnDir, len: u32, now: u64) {
        self.last_update = now;

        match dir {
            ConnDir::Original => {
                self.orig_counters.packets += 1;
                self.orig_counters.bytes += len as u64;
            },
            ConnDir::Reply => {
                self.reply_counters.packets += 1;
                self.reply_counters.bytes += len as u64;
                self.status.set(ConnStatus::SEEN_REPLY);
            },
        }
    }

    /// Check if connection has expired
    pub fn is_expired(&self, now: u64) -> bool {
        let timeout_secs = match self.state {
            ConnState::New => self.timeout.new,
            ConnState::Established => self.timeout.established,
            ConnState::Related => self.timeout.related,
            ConnState::Closing => self.timeout.closing,
            ConnState::Closed => self.timeout.closed,
            ConnState::Invalid => return true,
        };

        let elapsed = now.saturating_sub(self.last_update);
        elapsed >= (timeout_secs as u64) * 1_000_000_000
    }

    /// Get original counters
    pub fn orig_counters(&self) -> ConnCounters {
        self.orig_counters
    }

    /// Get reply counters
    pub fn reply_counters(&self) -> ConnCounters {
        self.reply_counters
    }
}

// ============================================================================
// Connection Tracking Table
// ============================================================================

/// Connection tracking table
#[derive(Debug)]
pub struct ConntrackTable {
    /// Connections indexed by tuple
    conns: BTreeMap<u64, ConnEntry>,
    /// Connections indexed by ID
    by_id: BTreeMap<ConnId, u64>,
    /// Next connection ID
    next_id: AtomicU32,
    /// Maximum connections
    max_conns: usize,
    /// Timeout configuration
    timeout: ConnTimeout,
    /// Statistics
    stats: ConntrackStats,
}

/// Connection tracking statistics
#[derive(Debug, Clone, Copy)]
pub struct ConntrackStats {
    /// Total entries
    pub entries: usize,
    /// Total connections created
    pub created: u64,
    /// Total connections destroyed
    pub destroyed: u64,
    /// Failed lookups
    pub lookup_failed: u64,
    /// Failed insertions
    pub insert_failed: u64,
}

impl ConntrackTable {
    /// Create new connection tracking table
    pub fn new(max_conns: usize) -> Self {
        Self {
            conns: BTreeMap::new(),
            by_id: BTreeMap::new(),
            next_id: AtomicU32::new(1),
            max_conns,
            timeout: ConnTimeout::default(),
            stats: ConntrackStats {
                entries: 0,
                created: 0,
                destroyed: 0,
                lookup_failed: 0,
                insert_failed: 0,
            },
        }
    }

    /// Create connection
    pub fn create_conn(&mut self, orig_tuple: ConnTuple, reply_tuple: ConnTuple) -> ConntrackResult<ConnId> {
        if self.conns.len() >= self.max_conns {
            self.stats.insert_failed += 1;
            return Err(crate::error::unified::ConntrackError::ConnTableFull);
        }

        let hash = orig_tuple.hash();
        if self.conns.contains_key(&hash) {
            self.stats.insert_failed += 1;
            return Err(crate::error::unified::ConntrackError::InvalidConnTuple);
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as u64;
        let conn = ConnEntry::new(id, orig_tuple, reply_tuple, self.timeout);

        self.conns.insert(hash, conn);
        self.by_id.insert(id, hash);
        self.stats.entries = self.conns.len();
        self.stats.created += 1;

        Ok(id)
    }

    /// Lookup connection by tuple
    pub fn lookup_conn(&self, tuple: ConnTuple) -> Option<&ConnEntry> {
        let hash = tuple.hash();
        self.conns.get(&hash)
    }

    /// Lookup connection by ID
    pub fn lookup_by_id(&self, id: ConnId) -> Option<&ConnEntry> {
        self.by_id.get(&id).and_then(|hash| self.conns.get(hash))
    }

    /// Get mutable connection
    pub fn get_conn_mut(&mut self, tuple: ConnTuple) -> Option<&mut ConnEntry> {
        let hash = tuple.hash();
        self.conns.get_mut(&hash)
    }

    /// Remove connection
    pub fn remove_conn(&mut self, id: ConnId) -> ConntrackResult<()> {
        let hash = self.by_id.remove(&id)
            .ok_or(crate::error::unified::ConntrackError::ConnNotFound)?;

        self.conns.remove(&hash)
            .ok_or(crate::error::unified::ConntrackError::ConnNotFound)?;

        self.stats.entries = self.conns.len();
        self.stats.destroyed += 1;

        Ok(())
    }

    /// Update connection
    pub fn update_conn(&mut self, tuple: ConnTuple, dir: ConnDir, len: u32, now: u64) -> ConntrackResult<()> {
        let conn = self.get_conn_mut(tuple)
            .ok_or(crate::error::unified::ConntrackError::ConnNotFound)?;

        conn.update(dir, len, now);
        Ok(())
    }

    /// Flush expired connections
    pub fn flush_expired(&mut self, now: u64) -> usize {
        let mut expired = Vec::new();

        for (hash, conn) in &self.conns {
            if conn.is_expired(now) {
                expired.push((conn.id(), *hash));
            }
        }

        for (id, hash) in expired {
            self.by_id.remove(&id);
            self.conns.remove(&hash);
            self.stats.destroyed += 1;
        }

        self.stats.entries = self.conns.len();
        self.conns.len()
    }

    /// Flush all connections
    pub fn flush(&mut self) {
        self.conns.clear();
        self.by_id.clear();
        self.stats.entries = 0;
    }

    /// Get statistics
    pub fn get_stats(&self) -> ConntrackStats {
        self.stats
    }

    /// List all connections
    pub fn list_conns(&self) -> Vec<&ConnEntry> {
        self.conns.values().collect()
    }
}

// ============================================================================
// NAT Implementation
// ============================================================================

/// NAT manager
#[derive(Debug)]
pub struct NatManager {
    /// NAT table indexed by original tuple
    nat_table: BTreeMap<ConnTuple, NatEntry>,
    /// Reverse NAT table
    rev_table: BTreeMap<ConnTuple, ConnTuple>,
}

impl NatManager {
    /// Create new NAT manager
    pub fn new() -> Self {
        Self {
            nat_table: BTreeMap::new(),
            rev_table: BTreeMap::new(),
        }
    }

    /// Add NAT entry
    pub fn add_nat(&mut self, orig: ConnTuple, nat: NatEntry) -> ConntrackResult<()> {
        // Create translated tuple
        let trans = ConnTuple::new(
            orig.protocol,
            nat.trans_ip,
            nat.trans_port,
            orig.dst_ip,
            orig.dst_port,
        );

        self.nat_table.insert(orig, nat);
        self.rev_table.insert(trans, orig);
        Ok(())
    }

    /// Lookup NAT by original tuple
    pub fn lookup_nat(&self, orig: ConnTuple) -> Option<&NatEntry> {
        self.nat_table.get(&orig)
    }

    /// Reverse lookup (get original from translated)
    pub fn lookup_reverse(&self, trans: ConnTuple) -> Option<ConnTuple> {
        self.rev_table.get(&trans).copied()
    }

    /// Remove NAT entry
    pub fn remove_nat(&mut self, orig: ConnTuple) -> ConntrackResult<()> {
        let nat = self.nat_table.remove(&orig)
            .ok_or(crate::error::unified::ConntrackError::NatFailed)?;

        let trans = ConnTuple::new(
            orig.protocol,
            nat.trans_ip,
            nat.trans_port,
            orig.dst_ip,
            orig.dst_port,
        );

        self.rev_table.remove(&trans);
        Ok(())
    }

    /// Allocate port for NAT
    pub fn allocate_port(&self, ip: u32) -> ConntrackResult<u16> {
        // Simple implementation: start from random port
        // In real implementation, would track used ports
        Ok(49152 + (crate::subsystems::mm::phys::PHYS_ALLOCATOR.lock().current_page() % 16384) as u16)
    }

    /// Flush NAT entries
    pub fn flush(&mut self) {
        self.nat_table.clear();
        self.rev_table.clear();
    }
}

impl Default for NatManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Protocols
// ============================================================================

/// Helper type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperType {
    Ftp,
    Sip,
    H323,
    Irc,
    Tftp,
    Amanda,
}

/// Helper protocol
pub trait Helper {
    /// Get helper type
    fn get_type(&self) -> HelperType;

    /// Parse packet and create expectations
    fn parse_packet(&self, tuple: ConnTuple, data: &[u8]) -> ConntrackResult<Vec<ConnTuple>>;

    /// Get expected port
    fn get_expected_port(&self) -> u16;
}

/// FTP helper
#[derive(Debug)]
pub struct FtpHelper {
    default_port: u16,
}

impl FtpHelper {
    pub fn new() -> Self {
        Self { default_port: 21 }
    }

    /// Parse FTP PORT command
    fn parse_port_command(&self, data: &[u8]) -> Option<ConnTuple> {
        // PORT h1,h2,h3,h4,p1,p2
        let data_str = core::str::from_utf8(data).ok()?;
        if !data_str.starts_with("PORT ") {
            return None;
        }

        let parts: Vec<&str> = data_str[5..].trim().split(',').collect();
        if parts.len() != 6 {
            return None;
        }

        let ip = u32::from_le_bytes([
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
            parts[3].parse().ok()?,
        ]);

        let port = ((parts[4].parse::<u16>().ok()? << 8) |
                    parts[5].parse::<u16>().ok()?) as u16;

        Some(ConnTuple::tcp(0, 0, ip, port))
    }
}

impl Helper for FtpHelper {
    fn get_type(&self) -> HelperType {
        HelperType::Ftp
    }

    fn parse_packet(&self, tuple: ConnTuple, data: &[u8]) -> ConntrackResult<Vec<ConnTuple>> {
        let mut expectations = Vec::new();

        if let Some(data_conn) = self.parse_port_command(data) {
            expectations.push(data_conn);
        }

        Ok(expectations)
    }

    fn get_expected_port(&self) -> u16 {
        self.default_port
    }
}

/// SIP helper
#[derive(Debug)]
pub struct SipHelper {
    default_port: u16,
}

impl SipHelper {
    pub fn new() -> Self {
        Self { default_port: 5060 }
    }
}

impl Helper for SipHelper {
    fn get_type(&self) -> HelperType {
        HelperType::Sip
    }

    fn parse_packet(&self, _tuple: ConnTuple, _data: &[u8]) -> ConntrackResult<Vec<ConnTuple>> {
        // Simplified - would parse SDP for media streams
        Ok(Vec::new())
    }

    fn get_expected_port(&self) -> u16 {
        self.default_port
    }
}

// ============================================================================
// Connection Tracking Manager
// ============================================================================

/// Connection tracking manager
#[derive(Debug)]
pub struct ConntrackManager {
    /// Connection table
    table: ConntrackTable,
    /// NAT manager
    nat: NatManager,
    /// Helpers indexed by protocol
    helpers: BTreeMap<u8, Arc<dyn Helper>>,
}

impl ConntrackManager {
    /// Create new connection tracking manager
    pub fn new(max_conns: usize) -> Self {
        let mut manager = Self {
            table: ConntrackTable::new(max_conns),
            nat: NatManager::new(),
            helpers: BTreeMap::new(),
        };

        // Register helpers
        manager.register_helper(6, Arc::new(FtpHelper::new()));
        manager.register_helper(17, Arc::new(FtpHelper::new()));
        manager.register_helper(6, Arc::new(SipHelper::new()));
        manager.register_helper(17, Arc::new(SipHelper::new()));

        manager
    }

    /// Register helper
    pub fn register_helper(&mut self, protocol: u8, helper: Arc<dyn Helper>) {
        self.helpers.insert(protocol, helper);
    }

    /// Create connection
    pub fn create_conn(&mut self, orig_tuple: ConnTuple) -> ConntrackResult<ConnId> {
        let reply_tuple = orig_tuple.reverse();
        self.table.create_conn(orig_tuple, reply_tuple)
    }

    /// Lookup connection
    pub fn lookup_conn(&self, tuple: ConnTuple) -> Option<&ConnEntry> {
        self.table.lookup_conn(tuple)
    }

    /// Get connection by ID
    pub fn get_conn(&self, id: ConnId) -> Option<&ConnEntry> {
        self.table.lookup_by_id(id)
    }

    /// Remove connection
    pub fn remove_conn(&mut self, id: ConnId) -> ConntrackResult<()> {
        let conn = self.table.lookup_by_id(id)
            .ok_or(crate::error::unified::ConntrackError::ConnNotFound)?;

        // Remove NAT entry if present
        if let Some(nat) = conn.nat() {
            let nat_tuple = ConnTuple::new(
                conn.orig_tuple().protocol,
                nat.orig_ip,
                nat.orig_port,
                conn.orig_tuple().dst_ip,
                conn.orig_tuple().dst_port,
            );
            let _ = self.nat.remove_nat(nat_tuple);
        }

        self.table.remove_conn(id)
    }

    /// Process packet (update connection)
    pub fn process_packet(&mut self, tuple: ConnTuple, len: u32) -> ConntrackResult<ConnState> {
        let now = 0; // Get current time

        // Try original direction
        if let Some(conn) = self.table.get_conn_mut(tuple) {
            conn.update(ConnDir::Original, len, now);
            return Ok(conn.state());
        }

        // Try reply direction
        let reply_tuple = tuple.reverse();
        if let Some(conn) = self.table.get_conn_mut(reply_tuple) {
            conn.update(ConnDir::Reply, len, now);

            // Update state based on protocol
            if conn.state() == ConnState::New {
                conn.set_state(ConnState::Established);
            }

            return Ok(conn.state());
        }

        Err(crate::error::unified::ConntrackError::ConnNotFound)
    }

    /// Setup SNAT
    pub fn setup_snat(&mut self, orig_tuple: ConnTuple, trans_ip: u32, trans_port: u16) -> ConntrackResult<()> {
        let conn = self.table.get_conn_mut(orig_tuple)
            .ok_or(crate::error::unified::ConntrackError::ConnNotFound)?;

        let nat = NatEntry {
            nat_type: NatType::Snat,
            orig_ip: orig_tuple.src_ip,
            orig_port: orig_tuple.src_port,
            trans_ip,
            trans_port,
        };

        conn.set_nat(nat.clone());

        // Update reply tuple
        let new_reply = ConnTuple::new(
            orig_tuple.protocol,
            trans_ip,
            trans_port,
            orig_tuple.dst_ip,
            orig_tuple.dst_port,
        );

        self.nat.add_nat(orig_tuple, nat)?;
        Ok(())
    }

    /// Setup DNAT
    pub fn setup_dnat(&mut self, orig_tuple: ConnTuple, trans_ip: u32, trans_port: u16) -> ConntrackResult<()> {
        let conn = self.table.get_conn_mut(orig_tuple)
            .ok_or(crate::error::unified::ConntrackError::ConnNotFound)?;

        let nat = NatEntry {
            nat_type: NatType::Dnat,
            orig_ip: orig_tuple.dst_ip,
            orig_port: orig_tuple.dst_port,
            trans_ip,
            trans_port,
        };

        conn.set_nat(nat.clone());
        self.nat.add_nat(orig_tuple, nat)?;
        Ok(())
    }

    /// Perform SNAT
    pub fn do_snat(&self, tuple: ConnTuple) -> ConntrackResult<ConnTuple> {
        let nat = self.nat.lookup_nat(tuple)
            .ok_or(crate::error::unified::ConntrackError::NatFailed)?;

        Ok(ConnTuple::new(
            tuple.protocol,
            nat.trans_ip,
            nat.trans_port,
            tuple.dst_ip,
            tuple.dst_port,
        ))
    }

    /// Perform DNAT
    pub fn do_dnat(&self, tuple: ConnTuple) -> ConntrackResult<ConnTuple> {
        let nat = self.nat.lookup_nat(tuple)
            .ok_or(crate::error::unified::ConntrackError::NatFailed)?;

        Ok(ConnTuple::new(
            tuple.protocol,
            tuple.src_ip,
            tuple.src_port,
            nat.trans_ip,
            nat.trans_port,
        ))
    }

    /// Reverse NAT (for reply packets)
    pub fn reverse_nat(&self, tuple: ConnTuple) -> ConntrackResult<ConnTuple> {
        if let Some(orig) = self.nat.lookup_reverse(tuple) {
            return Ok(orig);
        }

        Ok(tuple)
    }

    /// Flush expired connections
    pub fn flush_expired(&mut self, now: u64) -> usize {
        self.table.flush_expired(now)
    }

    /// Get statistics
    pub fn get_stats(&self) -> ConntrackStats {
        self.table.get_stats()
    }

    /// List all connections
    pub fn list_conns(&self) -> Vec<&ConnEntry> {
        self.table.list_conns()
    }
}

// ============================================================================
// Connection Tracking API
// ============================================================================

/// Global connection tracking manager
static CONNTRACK_MANAGER: RwLock<Option<ConntrackManager>> = RwLock::new(None);

/// Initialize connection tracking
pub fn init_conntrack(max_conns: usize) -> ConntrackResult<()> {
    let manager = ConntrackManager::new(max_conns);

    let mut global = CONNTRACK_MANAGER.write();
    *global = Some(manager);

    Ok(())
}

/// Create connection
pub fn create_conn(tuple: ConnTuple) -> ConntrackResult<ConnId> {
    let manager = CONNTRACK_MANAGER.read();
    let manager = manager.as_ref()
        .ok_or(crate::error::unified::ConntrackError::InvalidConnState)?;

    manager.create_conn(tuple)
}

/// Lookup connection
pub fn lookup_conn(tuple: ConnTuple) -> Option<Conn> {
    let manager = CONNTRACK_MANAGER.read();
    let manager = manager.as_ref()?;

    let entry = manager.lookup_conn(tuple)?;

    Some(Conn {
        id: entry.id(),
        orig_tuple: entry.orig_tuple(),
        reply_tuple: entry.reply_tuple(),
        state: entry.state(),
        status: entry.status(),
        orig_counters: entry.orig_counters(),
        reply_counters: entry.reply_counters(),
    })
}

/// Connection info (simplified)
#[derive(Debug, Clone)]
pub struct Conn {
    pub id: ConnId,
    pub orig_tuple: ConnTuple,
    pub reply_tuple: ConnTuple,
    pub state: ConnState,
    pub status: ConnStatus,
    pub orig_counters: ConnCounters,
    pub reply_counters: ConnCounters,
}

/// Process packet
pub fn process_packet(tuple: ConnTuple, len: u32) -> ConntrackResult<ConnState> {
    let manager = CONNTRACK_MANAGER.read();
    let manager = manager.as_ref()
        .ok_or(crate::error::unified::ConntrackError::InvalidConnState)?;

    manager.process_packet(tuple, len)
}

/// Setup SNAT
pub fn setup_snat(orig_tuple: ConnTuple, trans_ip: u32, trans_port: u16) -> ConntrackResult<()> {
    let mut manager = CONNTRACK_MANAGER.write();
    let manager = manager.as_mut()
        .ok_or(crate::error::unified::ConntrackError::InvalidConnState)?;

    manager.setup_snat(orig_tuple, trans_ip, trans_port)
}

/// Setup DNAT
pub fn setup_dnat(orig_tuple: ConnTuple, trans_ip: u32, trans_port: u16) -> ConntrackResult<()> {
    let mut manager = CONNTRACK_MANAGER.write();
    let manager = manager.as_mut()
        .ok_or(crate::error::unified::ConntrackError::InvalidConnState)?;

    manager.setup_dnat(orig_tuple, trans_ip, trans_port)
}

/// Get statistics
pub fn get_stats() -> ConntrackResult<ConntrackStats> {
    let manager = CONNTRACK_MANAGER.read();
    let manager = manager.as_ref()
        .ok_or(crate::error::unified::ConntrackError::InvalidConnState)?;

    Ok(manager.get_stats())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conn_tuple() {
        let tuple = ConnTuple::tcp(0xC0A80101, 12345, 0xC0A80102, 80);
        assert_eq!(tuple.protocol, 6);
        assert_eq!(tuple.src_ip, 0xC0A80101);
        assert_eq!(tuple.src_port, 12345);

        let rev = tuple.reverse();
        assert_eq!(rev.src_ip, tuple.dst_ip);
        assert_eq!(rev.src_port, tuple.dst_port);
        assert_eq!(rev.dst_ip, tuple.src_ip);
        assert_eq!(rev.dst_port, tuple.src_port);
    }

    #[test]
    fn test_conntrack_table() {
        let mut table = ConntrackTable::new(1000);

        let orig = ConnTuple::tcp(0xC0A80101, 12345, 0xC0A80102, 80);
        let reply = orig.reverse();

        let id = table.create_conn(orig, reply).unwrap();
        assert!(table.lookup_conn(orig).is_some());
        assert_eq!(table.lookup_by_id(id).unwrap().id(), id);

        table.remove_conn(id).unwrap();
        assert!(table.lookup_conn(orig).is_none());
    }

    #[test]
    fn test_conntrack_table_full() {
        let mut table = ConntrackTable::new(2);

        let orig1 = ConnTuple::tcp(1, 1, 2, 80);
        let reply1 = orig1.reverse();

        let orig2 = ConnTuple::tcp(3, 3, 4, 80);
        let reply2 = orig2.reverse();

        table.create_conn(orig1, reply1).unwrap();
        table.create_conn(orig2, reply2).unwrap();

        let orig3 = ConnTuple::tcp(5, 5, 6, 80);
        let reply3 = orig3.reverse();

        assert!(table.create_conn(orig3, reply3).is_err());
    }

    #[test]
    fn test_nat_manager() {
        let mut nat = NatManager::new();

        let orig = ConnTuple::tcp(0xC0A80101, 12345, 0xC0A80102, 80);
        let entry = NatEntry {
            nat_type: NatType::Snat,
            orig_ip: orig.src_ip,
            orig_port: orig.src_port,
            trans_ip: 0x01010101,
            trans_port: 54321,
        };

        nat.add_nat(orig, entry).unwrap();
        assert!(nat.lookup_nat(orig).is_some());
    }

    #[test]
    fn test_conntrack_manager() {
        let mut mgr = ConntrackManager::new(1000);

        let orig = ConnTuple::tcp(0xC0A80101, 12345, 0xC0A80102, 80);
        let id = mgr.create_conn(orig).unwrap();

        let conn = mgr.get_conn(id).unwrap();
        assert_eq!(conn.state(), ConnState::New);

        // Process packet
        mgr.process_packet(orig, 1000).unwrap();

        let conn = mgr.get_conn(id).unwrap();
        assert_eq!(conn.orig_counters().packets, 1);
        assert_eq!(conn.orig_counters().bytes, 1000);
    }

    #[test]
    fn test_snat() {
        let mut mgr = ConntrackManager::new(1000);

        let orig = ConnTuple::tcp(0xC0A80101, 12345, 0xC0A80102, 80);
        mgr.create_conn(orig).unwrap();

        mgr.setup_snat(orig, 0x01010101, 54321).unwrap();

        let trans = mgr.do_snat(orig).unwrap();
        assert_eq!(trans.src_ip, 0x01010101);
        assert_eq!(trans.src_port, 54321);
    }

    #[test]
    fn test_dnat() {
        let mut mgr = ConntrackManager::new(1000);

        let orig = ConnTuple::tcp(0xC0A80101, 12345, 0xC0A80102, 80);
        mgr.create_conn(orig).unwrap();

        mgr.setup_dnat(orig, 0x01010103, 8080).unwrap();

        let trans = mgr.do_dnat(orig).unwrap();
        assert_eq!(trans.dst_ip, 0x01010103);
        assert_eq!(trans.dst_port, 8080);
    }

    #[test]
    fn test_conn_status() {
        let mut status = ConnStatus::new();
        assert!(!status.has(ConnStatus::SEEN_REPLY));

        status.set(ConnStatus::SEEN_REPLY);
        assert!(status.has(ConnStatus::SEEN_REPLY));

        status.set(ConnStatus::ASSURED);
        assert!(status.has(ConnStatus::SEEN_REPLY));
        assert!(status.has(ConnStatus::ASSURED));

        status.clear(ConnStatus::SEEN_REPLY);
        assert!(!status.has(ConnStatus::SEEN_REPLY));
        assert!(status.has(ConnStatus::ASSURED));
    }
}
