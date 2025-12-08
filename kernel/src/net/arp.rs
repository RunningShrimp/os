//! Address Resolution Protocol (ARP) implementation
//!
//! This module provides ARP functionality for mapping IPv4 addresses to MAC addresses.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

use super::device::MacAddr;
use super::packet::{Packet, PacketType};
use super::ipv4::Ipv4Addr;

/// ARP hardware types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ArpHardwareType {
    /// Ethernet
    Ethernet = 1,
    /// Experimental Ethernet
    ExperimentalEthernet = 2,
    /// AX.25
    Ax25 = 3,
    /// Proteon ProNET Token Ring
    ProNetTokenRing = 4,
    /// Chaos
    Chaos = 5,
    /// IEEE 802 Networks
    Ieee802 = 6,
    /// ARCNET
    Arcnet = 7,
    /// Hyperchannel
    Hyperchannel = 8,
    /// AppleTalk
    AppleTalk = 9,
    /// Lanstar
    Lanstar = 10,
    /// Unassigned
    Unassigned = 11,
    /// Unassigned
    Unassigned2 = 12,
    /// Unassigned
    Unassigned3 = 13,
    /// Unassigned
    Unassigned4 = 14,
    /// Unassigned
    Unassigned5 = 15,
    /// Frame Relay
    FrameRelay = 16,
    /// ATM
    Atm = 17,
    /// HDLC
    Hdlc = 18,
    /// Fibre Channel
    FibreChannel = 19,
    /// ATM (adapted)
    Atm2 = 20,
    /// Serial Line
    SerialLine = 21,
}

impl From<u16> for ArpHardwareType {
    fn from(value: u16) -> Self {
        match value {
            1 => ArpHardwareType::Ethernet,
            2 => ArpHardwareType::ExperimentalEthernet,
            3 => ArpHardwareType::Ax25,
            4 => ArpHardwareType::ProNetTokenRing,
            5 => ArpHardwareType::Chaos,
            6 => ArpHardwareType::Ieee802,
            7 => ArpHardwareType::Arcnet,
            8 => ArpHardwareType::Hyperchannel,
            9 => ArpHardwareType::AppleTalk,
            10 => ArpHardwareType::Lanstar,
            16 => ArpHardwareType::FrameRelay,
            17 => ArpHardwareType::Atm,
            18 => ArpHardwareType::Hdlc,
            19 => ArpHardwareType::FibreChannel,
            20 => ArpHardwareType::Atm2,
            21 => ArpHardwareType::SerialLine,
            _ => ArpHardwareType::Unassigned,
        }
    }
}

/// ARP protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ArpProtocolType {
    /// IPv4
    Ipv4 = 0x0800,
    /// ARP
    Arp = 0x0806,
    /// RARP
    Rarp = 0x8035,
}

impl From<u16> for ArpProtocolType {
    fn from(value: u16) -> Self {
        match value {
            0x0800 => ArpProtocolType::Ipv4,
            0x0806 => ArpProtocolType::Arp,
            0x8035 => ArpProtocolType::Rarp,
            _ => ArpProtocolType::Ipv4, // Default fallback
        }
    }
}

/// ARP operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ArpOperation {
    /// Request
    Request = 1,
    /// Reply
    Reply = 2,
    /// RARP Request
    RarpRequest = 3,
    /// RARP Reply
    RarpReply = 4,
    /// DRARP Request
    DrarpRequest = 5,
    /// DRARP Reply
    DrarpReply = 6,
    /// DRARP Error
    DrarpError = 7,
    /// InARP Request
    InarpRequest = 8,
    /// InARP Reply
    InarpReply = 9,
}

impl From<u16> for ArpOperation {
    fn from(value: u16) -> Self {
        match value {
            1 => ArpOperation::Request,
            2 => ArpOperation::Reply,
            3 => ArpOperation::RarpRequest,
            4 => ArpOperation::RarpReply,
            5 => ArpOperation::DrarpRequest,
            6 => ArpOperation::DrarpReply,
            7 => ArpOperation::DrarpError,
            8 => ArpOperation::InarpRequest,
            9 => ArpOperation::InarpReply,
            _ => ArpOperation::Request, // Default fallback
        }
    }
}

/// ARP packet header
#[derive(Debug, Clone)]
pub struct ArpHeader {
    /// Hardware type
    pub hardware_type: ArpHardwareType,
    /// Protocol type
    pub protocol_type: ArpProtocolType,
    /// Hardware address length
    pub hardware_len: u8,
    /// Protocol address length
    pub protocol_len: u8,
    /// Operation
    pub operation: ArpOperation,
    /// Sender hardware address
    pub sender_hardware_addr: MacAddr,
    /// Sender protocol address
    pub sender_protocol_addr: Ipv4Addr,
    /// Target hardware address
    pub target_hardware_addr: MacAddr,
    /// Target protocol address
    pub target_protocol_addr: Ipv4Addr,
}

impl ArpHeader {
    /// Size of ARP header in bytes
    pub const SIZE: usize = 28;

    /// Create a new ARP header
    pub fn new(
        hardware_type: ArpHardwareType,
        protocol_type: ArpProtocolType,
        operation: ArpOperation,
        sender_hardware_addr: MacAddr,
        sender_protocol_addr: Ipv4Addr,
        target_hardware_addr: MacAddr,
        target_protocol_addr: Ipv4Addr,
    ) -> Self {
        Self {
            hardware_type,
            protocol_type,
            hardware_len: 6, // MAC address size
            protocol_len: 4, // IPv4 address size
            operation,
            sender_hardware_addr,
            sender_protocol_addr,
            target_hardware_addr,
            target_protocol_addr,
        }
    }

    /// Serialize ARP header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);

        // Hardware type (2 bytes)
        bytes.extend_from_slice(&(self.hardware_type as u16).to_be_bytes());

        // Protocol type (2 bytes)
        bytes.extend_from_slice(&(self.protocol_type as u16).to_be_bytes());

        // Hardware length (1 byte)
        bytes.push(self.hardware_len);

        // Protocol length (1 byte)
        bytes.push(self.protocol_len);

        // Operation (2 bytes)
        bytes.extend_from_slice(&(self.operation as u16).to_be_bytes());

        // Sender hardware address (6 bytes)
        bytes.extend_from_slice(&self.sender_hardware_addr.bytes());

        // Sender protocol address (4 bytes)
        bytes.extend_from_slice(&self.sender_protocol_addr.to_be_bytes());

        // Target hardware address (6 bytes)
        bytes.extend_from_slice(&self.target_hardware_addr.bytes());

        // Target protocol address (4 bytes)
        bytes.extend_from_slice(&self.target_protocol_addr.to_be_bytes());

        bytes
    }

    /// Parse ARP header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ArpError> {
        if bytes.len() < Self::SIZE {
            return Err(ArpError::InvalidPacket);
        }

        let hardware_type = ArpHardwareType::from(u16::from_be_bytes([bytes[0], bytes[1]]));
        let protocol_type = ArpProtocolType::from(u16::from_be_bytes([bytes[2], bytes[3]]));
        let hardware_len = bytes[4];
        let protocol_len = bytes[5];
        let operation = ArpOperation::from(u16::from_be_bytes([bytes[6], bytes[7]]));

        // Validate hardware and protocol lengths
        if hardware_len != 6 || protocol_len != 4 {
            return Err(ArpError::InvalidPacket);
        }

        let sender_mac = MacAddr::new([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13]
        ]);
        let sender_ip = Ipv4Addr::from_be_bytes([bytes[14], bytes[15], bytes[16], bytes[17]]);

        let target_mac = MacAddr::new([
            bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23]
        ]);
        let target_ip = Ipv4Addr::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);

        Ok(Self {
            hardware_type,
            protocol_type,
            hardware_len,
            protocol_len,
            operation,
            sender_hardware_addr: sender_mac,
            sender_protocol_addr: sender_ip,
            target_hardware_addr: target_mac,
            target_protocol_addr: target_ip,
        })
    }

    /// Create ARP request
    pub fn request(
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    ) -> Self {
        Self::new(
            ArpHardwareType::Ethernet,
            ArpProtocolType::Ipv4,
            ArpOperation::Request,
            sender_mac,
            sender_ip,
            MacAddr::zero(), // Target MAC is unknown
            target_ip,
        )
    }

    /// Create ARP reply
    pub fn reply(
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_mac: MacAddr,
        target_ip: Ipv4Addr,
    ) -> Self {
        Self::new(
            ArpHardwareType::Ethernet,
            ArpProtocolType::Ipv4,
            ArpOperation::Reply,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        )
    }
}

/// ARP cache entry
#[derive(Debug, Clone)]
pub struct ArpEntry {
    /// IP address
    pub ip_addr: Ipv4Addr,
    /// MAC address
    pub mac_addr: MacAddr,
    /// Entry creation time
    pub created_at: u64,
    /// Last access time
    pub last_accessed: u64,
    /// Entry is permanent (doesn't expire)
    pub permanent: bool,
    /// Number of times this entry was accessed
    pub access_count: u64,
}

impl ArpEntry {
    /// Create a new ARP entry
    pub fn new(ip_addr: Ipv4Addr, mac_addr: MacAddr, permanent: bool) -> Self {
        let now = Self::current_time();
        Self {
            ip_addr,
            mac_addr,
            created_at: now,
            last_accessed: now,
            permanent,
            access_count: 0,
        }
    }

    /// Get current time (placeholder - should use system time)
    fn current_time() -> u64 {
        // In a real implementation, this would use system time
        // For now, return a simple counter
        use core::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Check if entry has expired
    pub fn is_expired(&self, timeout: Duration) -> bool {
        if self.permanent {
            return false;
        }

        let now = Self::current_time();
        let elapsed = Duration::from_secs(now.saturating_sub(self.last_accessed));
        elapsed > timeout
    }

    /// Update access time and count
    pub fn access(&mut self) {
        self.last_accessed = Self::current_time();
        self.access_count += 1;
    }
}

/// ARP cache
pub struct ArpCache {
    /// Cache entries mapped by IP address
    entries: BTreeMap<Ipv4Addr, ArpEntry>,
    /// Maximum cache size
    max_size: usize,
    /// Entry timeout
    timeout: Duration,
    /// Cleanup interval
    cleanup_interval: u64,
    /// Last cleanup time
    last_cleanup: AtomicU64,
}

impl ArpCache {
    /// Create a new ARP cache
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            max_size: 1024, // Configurable
            timeout: Duration::from_secs(60 * 20), // 20 minutes default
            cleanup_interval: 60, // Clean up every 60 seconds
            last_cleanup: AtomicU64::new(0),
        }
    }

    /// Add or update an ARP entry
    pub fn insert(&mut self, ip_addr: Ipv4Addr, mac_addr: MacAddr, permanent: bool) {
        let entry = ArpEntry::new(ip_addr, mac_addr, permanent);
        self.entries.insert(ip_addr, entry);

        // Remove oldest entries if cache is full
        while self.entries.len() > self.max_size {
            self.remove_oldest();
        }
    }

    /// Look up MAC address for IP address
    pub fn lookup(&mut self, ip_addr: Ipv4Addr) -> Option<MacAddr> {
        if let Some(entry) = self.entries.get_mut(&ip_addr) {
            entry.access();
            Some(entry.mac_addr)
        } else {
            None
        }
    }

    /// Remove an entry by IP address
    pub fn remove(&mut self, ip_addr: Ipv4Addr) -> Option<ArpEntry> {
        self.entries.remove(&ip_addr)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove expired entries
    pub fn cleanup(&mut self) {
        let now = ArpEntry::current_time();
        let threshold = now.saturating_sub(self.timeout.as_secs());

        self.entries.retain(|_, entry| {
            entry.permanent || entry.last_accessed >= threshold
        });
    }

    /// Get cache statistics
    pub fn stats(&self) -> ArpCacheStats {
        ArpCacheStats {
            entries: self.entries.len(),
            max_size: self.max_size,
            permanent_entries: self.entries.values()
                .filter(|entry| entry.permanent)
                .count(),
        }
    }

    /// Get all entries (for debugging)
    pub fn entries(&self) -> impl Iterator<Item = &ArpEntry> {
        self.entries.values()
    }

    /// Remove the oldest entry
    fn remove_oldest(&mut self) {
        // Find the oldest entry key first
        let oldest_key = self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(ip, _)| *ip);

        if let Some(oldest_ip) = oldest_key {
            self.entries.remove(&oldest_ip);
        }
    }

    /// Check if cleanup is needed and perform it
    pub fn maybe_cleanup(&mut self) {
        let now = ArpEntry::current_time();
        let last_cleanup = self.last_cleanup.load(Ordering::Relaxed);

        if now.saturating_sub(last_cleanup) >= self.cleanup_interval {
            self.cleanup();
            self.last_cleanup.store(now, Ordering::Relaxed);
        }
    }
}

/// ARP cache statistics
#[derive(Debug, Clone)]
pub struct ArpCacheStats {
    /// Total number of entries
    pub entries: usize,
    /// Maximum cache size
    pub max_size: usize,
    /// Number of permanent entries
    pub permanent_entries: usize,
}

/// ARP packet processor
pub struct ArpProcessor {
    /// Cache timeout
    cache_timeout: Duration,
}

impl ArpProcessor {
    /// Create a new ARP processor
    pub fn new() -> Self {
        Self {
            cache_timeout: Duration::from_secs(60 * 20), // 20 minutes
        }
    }

    /// Process an incoming ARP packet
    pub fn process_packet(
        &self,
        packet: &[u8],
        local_ip: Ipv4Addr,
        local_mac: MacAddr,
        cache: &mut ArpCache,
    ) -> Result<Option<ArpHeader>, ArpError> {
        // Parse ARP header
        let header = ArpHeader::from_bytes(packet)?;

        // Update cache with sender information
        cache.insert(
            header.sender_protocol_addr,
            header.sender_hardware_addr,
            false, // Not permanent
        );

        match header.operation {
            ArpOperation::Request => {
                // Check if the request is for our IP
                if header.target_protocol_addr == local_ip {
                    // Send ARP reply
                    let reply = ArpHeader::reply(
                        local_mac,
                        local_ip,
                        header.sender_hardware_addr,
                        header.sender_protocol_addr,
                    );
                    return Ok(Some(reply));
                }
            }
            ArpOperation::Reply => {
                // Cache the reply information (already done above)
            }
            _ => {
                // Other operations not supported yet
            }
        }

        Ok(None)
    }
}

impl Default for ArpProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// ARP errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArpError {
    /// Invalid packet format
    InvalidPacket,
    /// Invalid hardware type
    InvalidHardwareType,
    /// Invalid protocol type
    InvalidProtocolType,
    /// Invalid operation
    InvalidOperation,
    /// Buffer too small
    BufferTooSmall,
}

/// ARP packet type alias for compatibility
pub type ArpPacket = ArpHeader;

/// Use atomic operations for thread safety
const ARP_TABLE_SIZE: usize = 256;