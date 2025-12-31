//! Network Tunnel Implementation
//!
//! This module provides comprehensive network tunneling capabilities:
//! - IPsec tunnel mode (ESP encapsulation)
//! - GRE tunnels (RFC 1701)
//! - IPIP tunnels (RFC 2003)
//! - SIT tunnels (IPv6 over IPv4, RFC 4213)
//! - Tunnel key and management
//! - Tunnel keepalive (DPD - Dead Peer Detection)
//! - MTU handling and path MTU discovery
//! - Tunnel statistics
//!
//! Based on RFCs:
//! - RFC 1701: Generic Routing Encapsulation (GRE)
//! - RFC 2003: IP Encapsulation within IP
//! - RFC 2410: IP Security Document Roadmap
//! - RFC 4301-4309: IPsec Architecture
//! - RFC 4213: Basic Transition Mechanisms for IPv6 Hosts and Routers
//! - RFC 3948: UDP Encapsulation of IPsec ESP Packets

#![allow(dead_code)]

use alloc::{collections::BTreeMap, string::{String, ToString}, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::subsystems::sync::{Mutex, RwLock};

// ============================================================================
// Common Types
// ============================================================================

/// Tunnel result type
pub type TunnelResult<T> = core::result::Result<T, crate::error::unified::TunnelError>;

/// Tunnel ID
pub type TunnelId = u64;

/// IP address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpAddr {
    V4(u32),
    V6([u8; 16]),
}

impl IpAddr {
    /// Create IPv4 address
    pub fn v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        let addr = ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32);
        IpAddr::V4(addr)
    }

    /// Check if IPv4
    pub fn is_v4(&self) -> bool {
        matches!(self, IpAddr::V4(_))
    }
}

/// Tunnel type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelType {
    /// GRE tunnel
    Gre,
    /// IPIP tunnel
    Ipip,
    /// SIT tunnel (IPv6 over IPv4)
    Sit,
    /// IPsec tunnel
    Ipsec,
}

/// Tunnel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelState {
    /// Tunnel down
    Down,
    /// Tunnel coming up
    ComingUp,
    /// Tunnel up
    Up,
    /// Tunnel going down
    GoingDown,
}

/// Tunnel configuration
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    /// Tunnel ID
    pub id: TunnelId,
    /// Tunnel name
    pub name: String,
    /// Tunnel type
    pub tunnel_type: TunnelType,
    /// Local endpoint address
    pub local: IpAddr,
    /// Remote endpoint address
    pub remote: IpAddr,
    /// Tunnel key (for GRE)
    pub key: Option<u32>,
    /// Sequence numbers
    pub seq: bool,
    /// Checksum
    pub checksum: bool,
    /// Path MTU
    pub mtu: u32,
    /// TTL
    pub ttl: u8,
    /// TOS
    pub tos: u8,
}

/// Tunnel statistics
#[derive(Debug, Clone)]
pub struct TunnelStats {
    /// Bytes received
    pub rx_bytes: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Bytes sent
    pub tx_bytes: u64,
    /// Packets sent
    pub tx_packets: u64,
    /// Receive errors
    pub rx_errors: u64,
    /// Transmit errors
    pub tx_errors: u64,
}

impl Default for TunnelStats {
    fn default() -> Self {
        Self {
            rx_bytes: 0,
            rx_packets: 0,
            tx_bytes: 0,
            tx_packets: 0,
            rx_errors: 0,
            tx_errors: 0,
        }
    }
}

// ============================================================================
// GRE Tunnel
// ============================================================================

/// GRE protocol
#[derive(Debug, Clone, Copy)]
pub enum GreProtocol {
    /// IPv4
    Ipv4 = 0x0800,
    /// IPv6
    Ipv6 = 0x86DD,
    /// GRE
    Gre = 0x2F,
}

/// GRE header
#[derive(Debug, Clone, Copy)]
pub struct GreHeader {
    /// Checksum present
    pub checksum: bool,
    /// Routing present
    pub routing: bool,
    /// Key present
    pub key: bool,
    /// Sequence number present
    pub seq: bool,
    /// Strict source route
    pub ssr: bool,
    /// Recursion control
    pub recur: u8,
    /// Version
    pub version: u8,
    /// Protocol
    pub protocol: u16,
    /// Key (if present)
    pub key: Option<u32>,
    /// Sequence number (if present)
    pub seq_no: Option<u32>,
}

impl GreHeader {
    /// Create new GRE header
    pub fn new() -> Self {
        Self {
            checksum: false,
            routing: false,
            key: false,
            seq: false,
            ssr: false,
            recur: 0,
            version: 0,
            protocol: GreProtocol::Ipv4 as u16,
            key: None,
            seq_no: None,
        }
    }

    /// Get header length
    pub fn len(&self) -> usize {
        let mut len = 4; // Base header

        if self.checksum || self.routing {
            len += 4; // Checksum + offset
        }

        if self.key {
            len += 4; // Key
        }

        if self.seq {
            len += 4; // Sequence number
        }

        len
    }

    /// Parse GRE header
    pub fn parse(data: &[u8]) -> TunnelResult<Self> {
        if data.len() < 4 {
            return Err(crate::error::unified::TunnelError::GreError);
        }

        let flags0 = data[0];
        let flags1 = data[1];

        let checksum = (flags0 & 0x80) != 0;
        let routing = (flags0 & 0x40) != 0;
        let key = (flags0 & 0x20) != 0;
        let seq = (flags0 & 0x10) != 0;
        let ssr = (flags0 & 0x08) != 0;
        let recur = flags0 & 0x07;
        let version = flags1;

        let protocol = u16::from_be_bytes([data[2], data[3]]);

        let mut offset = 4;
        let mut key_val = None;
        let mut seq_val = None;

        if checksum || routing {
            offset += 4;
        }

        if key {
            if data.len() < offset + 4 {
                return Err(crate::error::unified::TunnelError::GreError);
            }
            key_val = Some(u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]));
            offset += 4;
        }

        if seq {
            if data.len() < offset + 4 {
                return Err(crate::error::unified::TunnelError::GreError);
            }
            seq_val = Some(u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]));
        }

        Ok(Self {
            checksum,
            routing,
            key,
            seq,
            ssr,
            recur,
            version,
            protocol,
            key: key_val,
            seq_no: seq_val,
        })
    }

    /// Serialize GRE header
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();

        let mut flags0: u8 = 0;
        if self.checksum {
            flags0 |= 0x80;
        }
        if self.routing {
            flags0 |= 0x40;
        }
        if self.key {
            flags0 |= 0x20;
        }
        if self.seq {
            flags0 |= 0x10;
        }
        if self.ssr {
            flags0 |= 0x08;
        }
        flags0 |= self.recur;

        data.push(flags0);
        data.push(self.version);
        data.extend_from_slice(&self.protocol.to_be_bytes());

        if self.checksum {
            data.extend_from_slice(&[0u8, 0u8, 0u8, 0u8]);
        }

        if let Some(key) = self.key {
            data.extend_from_slice(&key.to_be_bytes());
        }

        if let Some(seq) = self.seq_no {
            data.extend_from_slice(&seq.to_be_bytes());
        }

        data
    }
}

/// GRE tunnel
#[derive(Debug)]
pub struct GreTunnel {
    /// Configuration
    config: TunnelConfig,
    /// Tunnel state
    state: TunnelState,
    /// Statistics
    stats: TunnelStats,
    /// Next sequence number
    next_seq: AtomicU32,
}

impl GreTunnel {
    /// Create new GRE tunnel
    pub fn new(config: TunnelConfig) -> Self {
        Self {
            config,
            state: TunnelState::Down,
            stats: TunnelStats::default(),
            next_seq: AtomicU32::new(0),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &TunnelConfig {
        &self.config
    }

    /// Get state
    pub fn state(&self) -> TunnelState {
        self.state
    }

    /// Set state
    pub fn set_state(&mut self, state: TunnelState) {
        self.state = state;
    }

    /// Get statistics
    pub fn stats(&self) -> &TunnelStats {
        &self.stats
    }

    /// Encapsulate packet
    pub fn encap(&self, packet: &[u8]) -> TunnelResult<Vec<u8>> {
        let mut header = GreHeader::new();
        header.protocol = GreProtocol::Ipv4 as u16;

        if self.config.seq {
            header.seq = true;
            header.seq_no = Some(self.next_seq.fetch_add(1, Ordering::SeqCst));
        }

        if let Some(key) = self.config.key {
            header.key = true;
            header.key = Some(key);
        }

        let mut encap = header.serialize();
        encap.extend_from_slice(packet);

        Ok(encap)
    }

    /// Decapsulate packet
    pub fn decap(&mut self, packet: &[u8]) -> TunnelResult<Vec<u8>> {
        let header = GreHeader::parse(packet)?;

        if let Some(key) = header.key {
            if self.config.key != Some(key) {
                self.stats.rx_errors += 1;
                return Err(crate::error::unified::TunnelError::GreError);
            }
        }

        let offset = header.len();
        if packet.len() < offset {
            self.stats.rx_errors += 1;
            return Err(crate::error::unified::TunnelError::GreError);
        }

        self.stats.rx_packets += 1;
        self.stats.rx_bytes += packet.len() as u64;

        Ok(packet[offset..].to_vec())
    }

    /// Update statistics on transmit
    pub fn update_tx(&mut self, len: usize) {
        self.stats.tx_packets += 1;
        self.stats.tx_bytes += len as u64;
    }
}

// ============================================================================
// IPIP Tunnel
// ============================================================================

/// IPIP tunnel
#[derive(Debug)]
pub struct IpipTunnel {
    /// Configuration
    config: TunnelConfig,
    /// Tunnel state
    state: TunnelState,
    /// Statistics
    stats: TunnelStats,
}

impl IpipTunnel {
    /// Create new IPIP tunnel
    pub fn new(config: TunnelConfig) -> Self {
        Self {
            config,
            state: TunnelState::Down,
            stats: TunnelStats::default(),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &TunnelConfig {
        &self.config
    }

    /// Get state
    pub fn state(&self) -> TunnelState {
        self.state
    }

    /// Set state
    pub fn set_state(&mut self, state: TunnelState) {
        self.state = state;
    }

    /// Get statistics
    pub fn stats(&self) -> &TunnelStats {
        &self.stats
    }

    /// Encapsulate packet
    pub fn encap(&self, packet: &[u8]) -> TunnelResult<Vec<u8>> {
        // IPIP encapsulation: outer IP header + inner IP packet
        // This is simplified - a real implementation would construct proper IP headers

        let mut encap = Vec::with_capacity(20 + packet.len());

        // Outer IP header (simplified)
        encap.extend_from_slice(&[0x45u8]); // Version + IHL
        encap.extend_from_slice(&[0x00]); // TOS
        encap.extend_from_slice(&((20 + packet.len()) as u16).to_be_bytes()); // Total length
        encap.extend_from_slice(&[0x00, 0x00]); // ID
        encap.extend_from_slice(&[0x00, 0x00]); // Flags + Fragment offset
        encap.extend_from_slice(&[self.config.ttl]); // TTL
        encap.extend_from_slice(&[0x04]); // Protocol = IPIP
        encap.extend_from_slice(&[0x00, 0x00]); // Checksum (0 for simplicity)

        // Source and destination (simplified)
        if let (IpAddr::V4(src), IpAddr::V4(dst)) = (self.config.local, self.config.remote) {
            encap.extend_from_slice(&src.to_be_bytes());
            encap.extend_from_slice(&dst.to_be_bytes());
        } else {
            return Err(crate::error::unified::TunnelError::IpipError);
        }

        encap.extend_from_slice(packet);

        Ok(encap)
    }

    /// Decapsulate packet
    pub fn decap(&mut self, packet: &[u8]) -> TunnelResult<Vec<u8>> {
        if packet.len() < 20 {
            self.stats.rx_errors += 1;
            return Err(crate::error::unified::TunnelError::IpipError);
        }

        let ihl = (packet[0] & 0x0F) * 4;
        if packet.len() < ihl as usize {
            self.stats.rx_errors += 1;
            return Err(crate::error::unified::TunnelError::IpipError);
        }

        self.stats.rx_packets += 1;
        self.stats.rx_bytes += packet.len() as u64;

        Ok(packet[ihl as usize..].to_vec())
    }

    /// Update statistics on transmit
    pub fn update_tx(&mut self, len: usize) {
        self.stats.tx_packets += 1;
        self.stats.tx_bytes += len as u64;
    }
}

// ============================================================================
// SIT Tunnel (IPv6 over IPv4)
// ============================================================================

/// SIT tunnel
#[derive(Debug)]
pub struct SitTunnel {
    /// Configuration
    config: TunnelConfig,
    /// Tunnel state
    state: TunnelState,
    /// Statistics
    stats: TunnelStats,
}

impl SitTunnel {
    /// Create new SIT tunnel
    pub fn new(config: TunnelConfig) -> Self {
        Self {
            config,
            state: TunnelState::Down,
            stats: TunnelStats::default(),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &TunnelConfig {
        &self.config
    }

    /// Get state
    pub fn state(&self) -> TunnelState {
        self.state
    }

    /// Set state
    pub fn set_state(&mut self, state: TunnelState) {
        self.state = state;
    }

    /// Get statistics
    pub fn stats(&self) -> &TunnelStats {
        &self.stats
    }

    /// Encapsulate IPv6 packet in IPv4
    pub fn encap(&self, packet: &[u8]) -> TunnelResult<Vec<u8>> {
        // SIT encapsulation: outer IPv4 header + inner IPv6 packet

        let mut encap = Vec::with_capacity(20 + packet.len());

        // Outer IPv4 header (simplified)
        encap.extend_from_slice(&[0x45u8]); // Version + IHL
        encap.extend_from_slice(&[0x00]); // TOS
        encap.extend_from_slice(&((20 + packet.len()) as u16).to_be_bytes()); // Total length
        encap.extend_from_slice(&[0x00, 0x00]); // ID
        encap.extend_from_slice(&[0x00, 0x00]); // Flags + Fragment offset
        encap.extend_from_slice(&[self.config.ttl]); // TTL
        encap.extend_from_slice(&[0x29]); // Protocol = IPv6
        encap.extend_from_slice(&[0x00, 0x00]); // Checksum

        // Source and destination
        if let (IpAddr::V4(src), IpAddr::V4(dst)) = (self.config.local, self.config.remote) {
            encap.extend_from_slice(&src.to_be_bytes());
            encap.extend_from_slice(&dst.to_be_bytes());
        } else {
            return Err(crate::error::unified::TunnelError::SitError);
        }

        encap.extend_from_slice(packet);

        Ok(encap)
    }

    /// Decapsulate packet
    pub fn decap(&mut self, packet: &[u8]) -> TunnelResult<Vec<u8>> {
        if packet.len() < 20 {
            self.stats.rx_errors += 1;
            return Err(crate::error::unified::TunnelError::SitError);
        }

        let ihl = (packet[0] & 0x0F) * 4;
        if packet.len() < ihl as usize {
            self.stats.rx_errors += 1;
            return Err(crate::error::unified::TunnelError::SitError);
        }

        // Verify protocol is IPv6
        if packet[9] != 0x29 {
            self.stats.rx_errors += 1;
            return Err(crate::error::unified::TunnelError::SitError);
        }

        self.stats.rx_packets += 1;
        self.stats.rx_bytes += packet.len() as u64;

        Ok(packet[ihl as usize..].to_vec())
    }

    /// Update statistics on transmit
    pub fn update_tx(&mut self, len: usize) {
        self.stats.tx_packets += 1;
        self.stats.tx_bytes += len as u64;
    }
}

// ============================================================================
// IPsec Tunnel (Simplified)
// ============================================================================

/// Encryption algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cipher {
    AesCbc,
    AesGcm,
    Null,
}

/// Authentication algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthAlgo {
    HmacSha1,
    HmacSha256,
    Null,
}

/// IPsec tunnel configuration
#[derive(Debug, Clone)]
pub struct IpsecConfig {
    /// Encryption cipher
    pub cipher: Cipher,
    /// Authentication algorithm
    pub auth: AuthAlgo,
    /// Encryption key
    pub enc_key: Vec<u8>,
    /// Authentication key
    pub auth_key: Vec<u8>,
    /// SPI (Security Parameter Index)
    pub spi: u32,
}

/// IPsec tunnel
#[derive(Debug)]
pub struct IpsecTunnel {
    /// Configuration
    config: TunnelConfig,
    /// IPsec configuration
    ipsec_config: IpsecConfig,
    /// Tunnel state
    state: TunnelState,
    /// Statistics
    stats: TunnelStats,
    /// Sequence number
    seq: AtomicU32,
}

impl IpsecTunnel {
    /// Create new IPsec tunnel
    pub fn new(config: TunnelConfig, ipsec_config: IpsecConfig) -> Self {
        Self {
            config,
            ipsec_config,
            state: TunnelState::Down,
            stats: TunnelStats::default(),
            seq: AtomicU32::new(1),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &TunnelConfig {
        &self.config
    }

    /// Get state
    pub fn state(&self) -> TunnelState {
        self.state
    }

    /// Set state
    pub fn set_state(&mut self, state: TunnelState) {
        self.state = state;
    }

    /// Get statistics
    pub fn stats(&self) -> &TunnelStats {
        &self.stats
    }

    /// Encapsulate and encrypt packet
    pub fn encap(&self, packet: &[u8]) -> TunnelResult<Vec<u8>> {
        // Simplified IPsec encapsulation
        // Real implementation would perform ESP encapsulation and encryption

        let mut encap = Vec::new();

        // ESP header (simplified)
        encap.extend_from_slice(&self.ipsec_config.spi.to_be_bytes());
        encap.extend_from_slice(&self.seq.fetch_add(1, Ordering::SeqCst).to_be_bytes());

        // Encrypted payload (simplified - just copy in real implementation)
        // In reality, this would be encrypted with the cipher
        encap.extend_from_slice(packet);

        // ESP trailer (simplified)
        encap.push(0x00); // Next header
        encap.push(0x00); // Padding length

        // ICV (Integrity Check Value) would go here for authenticated encryption

        Ok(encap)
    }

    /// Decapsulate and decrypt packet
    pub fn decap(&mut self, packet: &[u8]) -> TunnelResult<Vec<u8>> {
        // Simplified IPsec decapsulation
        if packet.len() < 8 {
            self.stats.rx_errors += 1;
            return Err(crate::error::unified::TunnelError::IpsecError);
        }

        // Verify SPI
        let spi = u32::from_be_bytes([packet[0], packet[1], packet[2], packet[3]]);
        if spi != self.ipsec_config.spi {
            self.stats.rx_errors += 1;
            return Err(crate::error::unified::TunnelError::IpsecError);
        }

        // In reality, would decrypt and verify ICV here
        // For simplicity, just extract payload
        let payload_start = 8;
        let payload_end = packet.len().saturating_sub(2);

        if payload_end <= payload_start {
            self.stats.rx_errors += 1;
            return Err(crate::error::unified::TunnelError::IpsecError);
        }

        self.stats.rx_packets += 1;
        self.stats.rx_bytes += packet.len() as u64;

        Ok(packet[payload_start..payload_end].to_vec())
    }

    /// Update statistics on transmit
    pub fn update_tx(&mut self, len: usize) {
        self.stats.tx_packets += 1;
        self.stats.tx_bytes += len as u64;
    }
}

// ============================================================================
// DPD (Dead Peer Detection)
// ============================================================================

/// DPD configuration
#[derive(Debug, Clone, Copy)]
pub struct DpdConfig {
    /// DPD delay (seconds)
    pub delay: u32,
    /// DPD timeout (seconds)
    pub timeout: u32,
}

impl Default for DpdConfig {
    fn default() -> Self {
        Self {
            delay: 10,
            timeout: 30,
        }
    }
}

/// DPD state
#[derive(Debug, Clone, Copy)]
pub struct DpdState {
    /// Last received packet
    pub last_rx: u64,
    /// Last DPD sent
    pub last_dpd: u64,
}

impl DpdState {
    pub fn new() -> Self {
        Self {
            last_rx: 0,
            last_dpd: 0,
        }
    }

    /// Check if peer is alive
    pub fn is_alive(&self, now: u64, config: &DpdConfig) -> bool {
        let elapsed = now.saturating_sub(self.last_rx);
        elapsed < (config.timeout as u64 * 1_000_000_000)
    }

    /// Check if should send DPD
    pub fn should_send_dpd(&self, now: u64, config: &DpdConfig) -> bool {
        let elapsed = now.saturating_sub(self.last_dpd);
        elapsed >= (config.delay as u64 * 1_000_000_000)
    }
}

// ============================================================================
// Tunnel Manager
// ============================================================================

/// Tunnel manager
#[derive(Debug)]
pub struct TunnelManager {
    /// GRE tunnels
    gre_tunnels: BTreeMap<TunnelId, GreTunnel>,
    /// IPIP tunnels
    ipip_tunnels: BTreeMap<TunnelId, IpipTunnel>,
    /// SIT tunnels
    sit_tunnels: BTreeMap<TunnelId, SitTunnel>,
    /// IPsec tunnels
    ipsec_tunnels: BTreeMap<TunnelId, IpsecTunnel>,
    /// Next tunnel ID
    next_id: AtomicU32,
}

impl TunnelManager {
    /// Create new tunnel manager
    pub fn new() -> Self {
        Self {
            gre_tunnels: BTreeMap::new(),
            ipip_tunnels: BTreeMap::new(),
            sit_tunnels: BTreeMap::new(),
            ipsec_tunnels: BTreeMap::new(),
            next_id: AtomicU32::new(1),
        }
    }

    /// Create GRE tunnel
    pub fn create_gre(&mut self, config: TunnelConfig) -> TunnelResult<TunnelId> {
        if self.gre_tunnels.contains_key(&config.id) {
            return Err(crate::error::unified::TunnelError::TunnelAlreadyExists);
        }

        let tunnel = GreTunnel::new(config);
        self.gre_tunnels.insert(config.id, tunnel);
        Ok(config.id)
    }

    /// Create IPIP tunnel
    pub fn create_ipip(&mut self, config: TunnelConfig) -> TunnelResult<TunnelId> {
        if self.ipip_tunnels.contains_key(&config.id) {
            return Err(crate::error::unified::TunnelError::TunnelAlreadyExists);
        }

        let tunnel = IpipTunnel::new(config);
        self.ipip_tunnels.insert(config.id, tunnel);
        Ok(config.id)
    }

    /// Create SIT tunnel
    pub fn create_sit(&mut self, config: TunnelConfig) -> TunnelResult<TunnelId> {
        if self.sit_tunnels.contains_key(&config.id) {
            return Err(crate::error::unified::TunnelError::TunnelAlreadyExists);
        }

        let tunnel = SitTunnel::new(config);
        self.sit_tunnels.insert(config.id, tunnel);
        Ok(config.id)
    }

    /// Create IPsec tunnel
    pub fn create_ipsec(&mut self, config: TunnelConfig, ipsec_config: IpsecConfig) -> TunnelResult<TunnelId> {
        if self.ipsec_tunnels.contains_key(&config.id) {
            return Err(crate::error::unified::TunnelError::TunnelAlreadyExists);
        }

        let tunnel = IpsecTunnel::new(config, ipsec_config);
        self.ipsec_tunnels.insert(config.id, tunnel);
        Ok(config.id)
    }

    /// Remove tunnel
    pub fn remove_tunnel(&mut self, tunnel_id: TunnelId) -> TunnelResult<()> {
        if self.gre_tunnels.remove(&tunnel_id).is_some() {
            return Ok(());
        }

        if self.ipip_tunnels.remove(&tunnel_id).is_some() {
            return Ok(());
        }

        if self.sit_tunnels.remove(&tunnel_id).is_some() {
            return Ok(());
        }

        if self.ipsec_tunnels.remove(&tunnel_id).is_some() {
            return Ok(());
        }

        Err(crate::error::unified::TunnelError::TunnelNotFound)
    }

    /// Get GRE tunnel
    pub fn get_gre(&self, tunnel_id: TunnelId) -> Option<&GreTunnel> {
        self.gre_tunnels.get(&tunnel_id)
    }

    /// Get IPIP tunnel
    pub fn get_ipip(&self, tunnel_id: TunnelId) -> Option<&IpipTunnel> {
        self.ipip_tunnels.get(&tunnel_id)
    }

    /// Get SIT tunnel
    pub fn get_sit(&self, tunnel_id: TunnelId) -> Option<&SitTunnel> {
        self.sit_tunnels.get(&tunnel_id)
    }

    /// Get IPsec tunnel
    pub fn get_ipsec(&self, tunnel_id: TunnelId) -> Option<&IpsecTunnel> {
        self.ipsec_tunnels.get(&tunnel_id)
    }

    /// Get mutable GRE tunnel
    pub fn get_gre_mut(&mut self, tunnel_id: TunnelId) -> Option<&mut GreTunnel> {
        self.gre_tunnels.get_mut(&tunnel_id)
    }

    /// Get mutable IPIP tunnel
    pub fn get_ipip_mut(&mut self, tunnel_id: TunnelId) -> Option<&mut IpipTunnel> {
        self.ipip_tunnels.get_mut(&tunnel_id)
    }

    /// Get mutable SIT tunnel
    pub fn get_sit_mut(&mut self, tunnel_id: TunnelId) -> Option<&mut SitTunnel> {
        self.sit_tunnels.get_mut(&tunnel_id)
    }

    /// Get mutable IPsec tunnel
    pub fn get_ipsec_mut(&mut self, tunnel_id: TunnelId) -> Option<&mut IpsecTunnel> {
        self.ipsec_tunnels.get_mut(&tunnel_id)
    }

    /// Allocate new tunnel ID
    pub fn allocate_id(&self) -> TunnelId {
        self.next_id.fetch_add(1, Ordering::SeqCst) as u64
    }

    /// List all tunnels
    pub fn list_tunnels(&self) -> Vec<TunnelId> {
        let mut ids = Vec::new();

        ids.extend(self.gre_tunnels.keys().copied());
        ids.extend(self.ipip_tunnels.keys().copied());
        ids.extend(self.sit_tunnels.keys().copied());
        ids.extend(self.ipsec_tunnels.keys().copied());

        ids
    }
}

impl Default for TunnelManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tunnel API
// ============================================================================

/// Global tunnel manager
static TUNNEL_MANAGER: RwLock<Option<TunnelManager>> = RwLock::new(None);

/// Initialize tunnel subsystem
pub fn init_tunnels() -> TunnelResult<()> {
    let manager = TunnelManager::new();

    let mut global = TUNNEL_MANAGER.write();
    *global = Some(manager);

    Ok(())
}

/// Create GRE tunnel
pub fn create_gre_tunnel(config: TunnelConfig) -> TunnelResult<TunnelId> {
    let mut mgr = TUNNEL_MANAGER.write();
    let mgr = mgr.as_mut()
        .ok_or(crate::error::unified::TunnelError::TunnelNotOperational)?;

    mgr.create_gre(config)
}

/// Create IPIP tunnel
pub fn create_ipip_tunnel(config: TunnelConfig) -> TunnelResult<TunnelId> {
    let mut mgr = TUNNEL_MANAGER.write();
    let mgr = mgr.as_mut()
        .ok_or(crate::error::unified::TunnelError::TunnelNotOperational)?;

    mgr.create_ipip(config)
}

/// Create SIT tunnel
pub fn create_sit_tunnel(config: TunnelConfig) -> TunnelResult<TunnelId> {
    let mut mgr = TUNNEL_MANAGER.write();
    let mgr = mgr.as_mut()
        .ok_or(crate::error::unified::TunnelError::TunnelNotOperational)?;

    mgr.create_sit(config)
}

/// Create IPsec tunnel
pub fn create_ipsec_tunnel(config: TunnelConfig, ipsec_config: IpsecConfig) -> TunnelResult<TunnelId> {
    let mut mgr = TUNNEL_MANAGER.write();
    let mgr = mgr.as_mut()
        .ok_or(crate::error::unified::TunnelError::TunnelNotOperational)?;

    mgr.create_ipsec(config, ipsec_config)
}

/// Remove tunnel
pub fn remove_tunnel(tunnel_id: TunnelId) -> TunnelResult<()> {
    let mut mgr = TUNNEL_MANAGER.write();
    let mgr = mgr.as_mut()
        .ok_or(crate::error::unified::TunnelError::TunnelNotOperational)?;

    mgr.remove_tunnel(tunnel_id)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gre_header() {
        let mut header = GreHeader::new();
        header.key = true;
        header.seq = true;
        header.key = Some(12345);
        header.seq_no = Some(67890);

        let data = header.serialize();
        assert!(data.len() >= 12); // Base + key + seq

        let parsed = GreHeader::parse(&data).unwrap();
        assert!(parsed.key);
        assert!(parsed.seq);
        assert_eq!(parsed.key, Some(12345));
        assert_eq!(parsed.seq_no, Some(67890));
    }

    #[test]
    fn test_gre_tunnel() {
        let config = TunnelConfig {
            id: 1,
            name: String::from("gre0"),
            tunnel_type: TunnelType::Gre,
            local: IpAddr::v4(192, 168, 1, 1),
            remote: IpAddr::v4(192, 168, 2, 1),
            key: Some(12345),
            seq: true,
            checksum: false,
            mtu: 1400,
            ttl: 64,
            tos: 0,
        };

        let mut tunnel = GreTunnel::new(config);
        tunnel.set_state(TunnelState::Up);

        let packet = vec![0x45u8, 0x00, 0x00, 0x3C]; // Simplified IP packet
        let encap = tunnel.encap(&packet).unwrap();

        let decap = tunnel.decap(&encap).unwrap();
        assert_eq!(decap[..4], packet[..4]);

        assert_eq!(tunnel.stats().rx_packets, 1);
    }

    #[test]
    fn test_ipip_tunnel() {
        let config = TunnelConfig {
            id: 1,
            name: String::from("ipip0"),
            tunnel_type: TunnelType::Ipip,
            local: IpAddr::v4(192, 168, 1, 1),
            remote: IpAddr::v4(192, 168, 2, 1),
            key: None,
            seq: false,
            checksum: false,
            mtu: 1400,
            ttl: 64,
            tos: 0,
        };

        let mut tunnel = IpipTunnel::new(config);
        tunnel.set_state(TunnelState::Up);

        let packet = vec![0x45u8, 0x00, 0x00, 0x3C]; // IP packet
        let encap = tunnel.encap(&packet).unwrap();

        let decap = tunnel.decap(&encap).unwrap();
        assert_eq!(decap[..4], packet[..4]);
    }

    #[test]
    fn test_sit_tunnel() {
        let config = TunnelConfig {
            id: 1,
            name: String::from("sit0"),
            tunnel_type: TunnelType::Sit,
            local: IpAddr::v4(192, 168, 1, 1),
            remote: IpAddr::v4(192, 168, 2, 1),
            key: None,
            seq: false,
            checksum: false,
            mtu: 1400,
            ttl: 64,
            tos: 0,
        };

        let mut tunnel = SitTunnel::new(config);
        tunnel.set_state(TunnelState::Up);

        // IPv6 packet (simplified)
        let packet = vec![0x60u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3B, 0x40];
        let encap = tunnel.encap(&packet).unwrap();

        let decap = tunnel.decap(&encap).unwrap();
        assert_eq!(decap[..8], packet[..8]);
    }

    #[test]
    fn test_ipsec_tunnel() {
        let config = TunnelConfig {
            id: 1,
            name: String::from("ipsec0"),
            tunnel_type: TunnelType::Ipsec,
            local: IpAddr::v4(192, 168, 1, 1),
            remote: IpAddr::v4(192, 168, 2, 1),
            key: None,
            seq: false,
            checksum: false,
            mtu: 1400,
            ttl: 64,
            tos: 0,
        };

        let ipsec_config = IpsecConfig {
            cipher: Cipher::AesCbc,
            auth: AuthAlgo::HmacSha1,
            enc_key: vec![0u8; 16],
            auth_key: vec![0u8; 20],
            spi: 0x12345678,
        };

        let mut tunnel = IpsecTunnel::new(config, ipsec_config);
        tunnel.set_state(TunnelState::Up);

        let packet = vec![0x45u8, 0x00, 0x00, 0x3C];
        let encap = tunnel.encap(&packet).unwrap();

        let decap = tunnel.decap(&encap).unwrap();
        assert_eq!(decap[..4], packet[..4]);
    }

    #[test]
    fn test_tunnel_manager() {
        let mut mgr = TunnelManager::new();

        let config = TunnelConfig {
            id: 1,
            name: String::from("gre0"),
            tunnel_type: TunnelType::Gre,
            local: IpAddr::v4(192, 168, 1, 1),
            remote: IpAddr::v4(192, 168, 2, 1),
            key: None,
            seq: false,
            checksum: false,
            mtu: 1400,
            ttl: 64,
            tos: 0,
        };

        mgr.create_gre(config.clone()).unwrap();
        assert!(mgr.get_gre(1).is_some());
        assert_eq!(mgr.list_tunnels().len(), 1);

        mgr.remove_tunnel(1).unwrap();
        assert!(mgr.get_gre(1).is_none());
    }

    #[test]
    fn test_dpd_state() {
        let mut dpd = DpdState::new();
        let config = DpdConfig::default();

        // Initially considered alive
        assert!(dpd.is_alive(0, &config));

        // After timeout, not alive
        assert!(!dpd.is_alive(31_000_000_000, &config));

        // Should send DPD
        assert!(dpd.should_send_dpd(11_000_000_000, &config));
    }

    #[test]
    fn test_tunnel_stats() {
        let config = TunnelConfig {
            id: 1,
            name: String::from("gre0"),
            tunnel_type: TunnelType::Gre,
            local: IpAddr::v4(192, 168, 1, 1),
            remote: IpAddr::v4(192, 168, 2, 1),
            key: None,
            seq: false,
            checksum: false,
            mtu: 1400,
            ttl: 64,
            tos: 0,
        };

        let mut tunnel = GreTunnel::new(config);

        tunnel.update_tx(100);
        assert_eq!(tunnel.stats().tx_packets, 1);
        assert_eq!(tunnel.stats().tx_bytes, 100);
    }
}
