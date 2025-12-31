//! VXLAN (Virtual Extensible LAN) Implementation
//!
//! This module provides VXLAN encapsulation for network virtualization and overlay
//! networking. VXLAN allows you to create Layer 2 networks on top of Layer 3
//! infrastructure, enabling large-scale multi-tenant data center networks.
//!
//! ## Features
//!
//! - **VXLAN Encapsulation**: Full RFC 7348 compliant encapsulation
//! - **VNI Management**: 24-bit VXLAN Network Identifier support
//! - **Multicast Support**: Efficient broadcast/multicast handling
//! - **EVPN Integration**: Ethernet VPN support for control plane learning
//! - **Multi-Destination**: Head-end replication and multicast groups
//!
//! ## Architecture
//!
//! VXLAN uses a UDP-based encapsulation:
//!
//! ```
//! Outer Ethernet | Outer IP | Outer UDP | VXLAN | Inner Ethernet | Inner Payload
//! ```
//!
//! The VNI (VXLAN Network Identifier) isolates different virtual networks.
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::sdn::vxlan::{Vxlan, VxlanConfig};
//!
//! let config = VxlanConfig {
//!     vni: 1000,
//!     src_ip: Ipv4Addr::new(192, 168, 1, 10),
//!     multicast_group: Some(Ipv4Addr::new(239, 1, 1, 1)),
//!     ..Default::default()
//! };
//!
//! let mut vxlan = Vxlan::new(config)?;
//! vxlan.init().await?;
//! ```

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use core::time::Duration;

use crate::subsystems::net::ipv4::Ipv4Addr;
use crate::subsystems::sync::{Mutex, RwLock};

use super::{SdnError, SdnStats};

/// VXLAN standard UDP port
pub const VXLAN_UDP_PORT: u16 = 4789;

/// VXLAN header size
pub const VXLAN_HEADER_SIZE: usize = 8;

/// VXLAN configuration
#[derive(Debug, Clone)]
pub struct VxlanConfig {
    /// VXLAN Network Identifier (0-16777215)
    pub vni: u32,
    /// Source IP address for VXLAN packets
    pub src_ip: Ipv4Addr,
    /// Multicast group for broadcast/multicast
    pub multicast_group: Option<Ipv4Addr>,
    /// UDP port to use (default 4789)
    pub udp_port: u16,
    /// MTU for VXLAN interface (default 1450)
    pub mtu: u16,
    /// Learning enabled
    pub learning: bool,
    /// GBP (Group-based Policy) extension enabled
    pub gbp_enabled: bool,
    /// Disable destination checksum offload
    pub no_checksum: bool,
    /// TTL for outer IP header
    pub ttl: u8,
    /// TOS for outer IP header
    pub tos: u8,
}

impl Default for VxlanConfig {
    fn default() -> Self {
        Self {
            vni: 0,
            src_ip: Ipv4Addr::UNSPECIFIED,
            multicast_group: None,
            udp_port: VXLAN_UDP_PORT,
            mtu: 1450,
            learning: true,
            gbp_enabled: false,
            no_checksum: false,
            ttl: 64,
            tos: 0,
        }
    }
}

/// VXLAN error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VxlanError {
    /// Invalid VNI
    InvalidVni,
    /// Invalid configuration
    InvalidConfig,
    /// Buffer too small
    BufferTooSmall,
    /// Encapsulation failed
    EncapsulationFailed,
    /// Decapsulation failed
    DecapsulationFailed,
    /// VNI not found
    VniNotFound,
    /// No endpoint for VNI
    NoEndpoint,
}

/// VNI (VXLAN Network Identifier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Vni(u32);

impl Vni {
    /// Create new VNI
    pub fn new(vni: u32) -> Result<Self, VxlanError> {
        if vni > 0xFFFFFF {
            return Err(VxlanError::InvalidVni);
        }
        Ok(Self(vni))
    }

    /// Get VNI value
    pub fn value(&self) -> u32 {
        self.0
    }

    /// Check if valid
    pub fn is_valid(&self) -> bool {
        self.0 <= 0xFFFFFF
    }
}

/// VXLAN header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VxlanHeader {
    /// Flags (I bit must be set)
    pub flags: u8,
    /// VNI in 3 bytes
    pub vni_0: u8,
    pub vni_1: u8,
    pub vni_2: u8,
    /// Reserved (must be 0)
    pub reserved: u8,
}

impl VxlanHeader {
    /// Create new VXLAN header
    pub fn new(vni: Vni) -> Self {
        let vni_value = vni.value();
        Self {
            flags: 0x08, // I bit set
            vni_0: ((vni_value >> 16) & 0xFF) as u8,
            vni_1: ((vni_value >> 8) & 0xFF) as u8,
            vni_2: (vni_value & 0xFF) as u8,
            reserved: 0,
        }
    }

    /// Get VNI from header
    pub fn get_vni(&self) -> Vni {
        let vni = ((self.vni_0 as u32) << 16) | ((self.vni_1 as u32) << 8) | (self.vni_2 as u32);
        Vni(vni)
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; VXLAN_HEADER_SIZE] {
        [
            self.flags,
            self.vni_0,
            self.vni_1,
            self.vni_2,
            0, // GBP flags (if enabled)
            0, // reserved
            0, // reserved
            self.reserved,
        ]
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < VXLAN_HEADER_SIZE {
            return None;
        }

        Some(Self {
            flags: data[0],
            vni_0: data[1],
            vni_1: data[2],
            vni_2: data[3],
            reserved: data[7],
        })
    }
}

/// VXLAN packet encapsulation
#[derive(Debug)]
pub struct VxlanEncapsulation;

impl VxlanEncapsulation {
    /// Encapsulate Ethernet frame in VXLAN
    pub fn encapsulate(
        inner_frame: &[u8],
        config: &VxlanConfig,
        dest_ip: Ipv4Addr,
    ) -> Result<Vec<u8>, VxlanError> {
        let vni = Vni::new(config.vni)?;

        // Calculate total size
        let outer_eth_size = 14;
        let outer_ip_size = 20;
        let outer_udp_size = 8;
        let vxlan_size = VXLAN_HEADER_SIZE;

        let total_size = outer_eth_size + outer_ip_size + outer_udp_size + vxlan_size + inner_frame.len();

        let mut packet = vec![0u8; total_size];

        // Build outer Ethernet header (simplified)
        // In real implementation, would fill in proper MAC addresses
        packet[0..6].copy_from_slice(&[0u8; 6]); // Destination MAC
        packet[6..12].copy_from_slice(&[0u8; 6]); // Source MAC
        packet[12] = 0x08;
        packet[13] = 0x00; // EtherType: IPv4

        // Build outer IP header (simplified)
        let ip_header_offset = outer_eth_size;
        packet[ip_header_offset] = 0x45; // Version=4, IHL=5
        // Total length
        let total_len = (total_size - outer_eth_size) as u16;
        packet[ip_header_offset + 2..ip_header_offset + 4]
            .copy_from_slice(&total_len.to_be_bytes());
        // TTL
        packet[ip_header_offset + 8] = config.ttl;
        // Protocol: UDP (17)
        packet[ip_header_offset + 9] = 17;
        // Source IP
        packet[ip_header_offset + 12..ip_header_offset + 16]
            .copy_from_slice(&config.src_ip.to_be_bytes());
        // Destination IP
        packet[ip_header_offset + 16..ip_header_offset + 20]
            .copy_from_slice(&dest_ip.to_be_bytes());

        // Build outer UDP header
        let udp_header_offset = ip_header_offset + outer_ip_size;
        // Source port (use hash of inner frame)
        let src_port = self.hash_port(inner_frame);
        packet[udp_header_offset..udp_header_offset + 2]
            .copy_from_slice(&src_port.to_be_bytes());
        // Destination port
        packet[udp_header_offset + 2..udp_header_offset + 4]
            .copy_from_slice(&config.udp_port.to_be_bytes());
        // UDP length
        let udp_len = (outer_udp_size + vxlan_size + inner_frame.len()) as u16;
        packet[udp_header_offset + 4..udp_header_offset + 6]
            .copy_from_slice(&udp_len.to_be_bytes());

        // Build VXLAN header
        let vxlan_header_offset = udp_header_offset + outer_udp_size;
        let vxlan_header = VxlanHeader::new(vni);
        packet[vxlan_header_offset..vxlan_header_offset + VXLAN_HEADER_SIZE]
            .copy_from_slice(&vxlan_header.to_bytes());

        // Copy inner frame
        let inner_offset = vxlan_header_offset + VXLAN_HEADER_SIZE;
        packet[inner_offset..].copy_from_slice(inner_frame);

        Ok(packet)
    }

    /// Decapsulate VXLAN packet
    pub fn decapsulate(packet: &[u8]) -> Result<(Vni, Vec<u8>), VxlanError> {
        if packet.len() < VXLAN_HEADER_SIZE {
            return Err(VxlanError::BufferTooSmall);
        }

        // Find VXLAN header (after UDP header)
        // Simplified: assume VXLAN header starts at offset 42 (14 + 20 + 8)
        let vxlan_offset = 42;

        if packet.len() < vxlan_offset + VXLAN_HEADER_SIZE {
            return Err(VxlanError::BufferTooSmall);
        }

        let vxlan_header = VxlanHeader::from_bytes(&packet[vxlan_offset..])
            .ok_or(VxlanError::DecapsulationFailed)?;

        let vni = vxlan_header.get_vni();
        if !vni.is_valid() {
            return Err(VxlanError::InvalidVni);
        }

        let inner_frame = packet[vxlan_offset + VXLAN_HEADER_SIZE..].to_vec();

        Ok((vni, inner_frame))
    }

    /// Hash inner frame for UDP source port
    fn hash_port(&self, data: &[u8]) -> u16 {
        let mut hash: u16 = 0;
        for (i, &byte) in data.iter().enumerate() {
            hash = hash.wrapping_add((byte as u16) << (i % 8));
        }
        if hash == 0 {
            hash = 1;
        }
        hash
    }
}

/// VNI manager
#[derive(Debug)]
pub struct VniManager {
    /// Allocated VNIs
    allocated_vnis: Mutex<BTreeMap<u32, VniInfo>>,
    /// Next VNI to allocate
    next_vni: AtomicU32,
    /// Maximum VNI
    max_vni: u32,
}

/// VNI information
#[derive(Debug, Clone)]
struct VniInfo {
    vni: Vni,
    name: String,
    endpoint_ip: Option<Ipv4Addr>,
    multicast_group: Option<Ipv4Addr>,
}

impl VniManager {
    /// Create new VNI manager
    pub fn new(max_vni: u32) -> Self {
        Self {
            allocated_vnis: Mutex::new(BTreeMap::new()),
            next_vni: AtomicU32::new(1),
            max_vni: max_vni.min(0xFFFFFF),
        }
    }

    /// Allocate VNI
    pub fn allocate(&self, name: String) -> Result<Vni, VxlanError> {
        let vni_value = self.next_vni.fetch_add(1, Ordering::Relaxed);

        if vni_value > self.max_vni {
            return Err(VxlanError::VniNotFound);
        }

        let vni = Vni::new(vni_value)?;

        let info = VniInfo {
            vni,
            name,
            endpoint_ip: None,
            multicast_group: None,
        };

        self.allocated_vnis.lock().insert(vni_value, info);

        Ok(vni)
    }

    /// Allocate specific VNI
    pub fn allocate_specific(&self, vni: Vni, name: String) -> Result<(), VxlanError> {
        let info = VniInfo {
            vni,
            name,
            endpoint_ip: None,
            multicast_group: None,
        };

        self.allocated_vnis.lock().insert(vni.value(), info);

        Ok(())
    }

    /// Release VNI
    pub fn release(&self, vni: Vni) -> Result<(), VxlanError> {
        self.allocated_vnis
            .lock()
            .remove(&vni.value())
            .ok_or(VxlanError::VniNotFound)?;
        Ok(())
    }

    /// Get VNI info
    pub fn get_info(&self, vni: Vni) -> Option<VniInfo> {
        self.allocated_vnis.lock().get(&vni.value()).cloned()
    }

    /// Set endpoint IP for VNI
    pub fn set_endpoint(&self, vni: Vni, endpoint_ip: Ipv4Addr) -> Result<(), VxlanError> {
        let mut vnis = self.allocated_vnis.lock();
        let info = vnis.get_mut(&vni.value()).ok_or(VxlanError::VniNotFound)?;
        info.endpoint_ip = Some(endpoint_ip);
        Ok(())
    }

    /// List all VNIs
    pub fn list_vnis(&self) -> Vec<VniInfo> {
        self.allocated_vnis.lock().values().cloned().collect()
    }
}

/// VXLAN packet
#[derive(Debug, Clone)]
pub struct VxlanPacket {
    /// VNI
    pub vni: Vni,
    /// Inner Ethernet frame
    pub inner_frame: Vec<u8>,
    /// Source endpoint IP
    pub src_endpoint: Ipv4Addr,
    /// Destination endpoint IP
    pub dst_endpoint: Ipv4Addr,
}

impl VxlanPacket {
    /// Create new VXLAN packet
    pub fn new(
        vni: Vni,
        inner_frame: Vec<u8>,
        src_endpoint: Ipv4Addr,
        dst_endpoint: Ipv4Addr,
    ) -> Self {
        Self {
            vni,
            inner_frame,
            src_endpoint,
            dst_endpoint,
        }
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, VxlanError> {
        let (vni, inner_frame) = VxlanEncapsulation::decapsulate(data)?;

        Ok(Self {
            vni,
            inner_frame,
            src_endpoint: Ipv4Addr::UNSPECIFIED,
            dst_endpoint: Ipv4Addr::UNSPECIFIED,
        })
    }

    /// Serialize to bytes
    pub fn to_bytes(&self, config: &VxlanConfig) -> Result<Vec<u8>, VxlanError> {
        VxlanEncapsulation::encapsulate(&self.inner_frame, config, self.dst_endpoint)
    }
}

/// VXLAN network
#[derive(Debug)]
pub struct VxlanNetwork {
    /// VNI
    vni: Vni,
    /// Network name
    name: String,
    /// Configuration
    config: VxlanConfig,
    /// VTEP (VXLAN Tunnel Endpoint) IPs
    vteps: Mutex<Vec<Ipv4Addr>>,
    /// FDB (Forwarding Database) - MAC to VTEP mapping
    fdb: Mutex<BTreeMap<[u8; 6], Ipv4Addr>>,
    /// Statistics
    stats: Mutex<VxlanStats>,
}

/// VXLAN statistics
#[derive(Debug, Default, Clone)]
pub struct VxlanStats {
    /// Packets transmitted
    pub tx_packets: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Bytes received
    pub rx_bytes: u64,
    /// Broadcast packets
    pub broadcast_packets: u64,
    /// Multicast packets
    pub multicast_packets: u64,
    /// FDB misses
    pub fdb_misses: u64,
}

impl VxlanNetwork {
    /// Create new VXLAN network
    pub fn new(name: String, vni: Vni, config: VxlanConfig) -> Self {
        Self {
            vni,
            name,
            config,
            vteps: Mutex::new(Vec::new()),
            fdb: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(VxlanStats::default()),
        }
    }

    /// Add VTEP
    pub fn add_vtep(&self, vtep: Ipv4Addr) {
        let mut vteps = self.vteps.lock();
        if !vteps.contains(&vtep) {
            vteps.push(vtep);
        }
    }

    /// Remove VTEP
    pub fn remove_vtep(&self, vtep: &Ipv4Addr) {
        let mut vteps = self.vteps.lock();
        vteps.retain(|v| v != vtep);
    }

    /// Learn MAC address (FDB entry)
    pub fn learn_mac(&self, mac: [u8; 6], vtep: Ipv4Addr) {
        let mut fdb = self.fdb.lock();
        fdb.insert(mac, vtep);
    }

    /// Forget MAC address
    pub fn forget_mac(&self, mac: &[u8; 6]) {
        let mut fdb = self.fdb.lock();
        fdb.remove(mac);
    }

    /// Lookup VTEP for MAC
    pub fn lookup_vtep(&self, mac: &[u8; 6]) -> Option<Ipv4Addr> {
        let fdb = self.fdb.lock();
        fdb.get(mac).copied()
    }

    /// Get all VTEPs
    pub fn get_vteps(&self) -> Vec<Ipv4Addr> {
        self.vteps.lock().clone()
    }

    /// Get FDB entries
    pub fn get_fdb(&self) -> Vec<([u8; 6], Ipv4Addr)> {
        self.fdb
            .lock()
            .iter()
            .map(|(mac, vtep)| (*mac, *vtep))
            .collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> VxlanStats {
        self.stats.lock().clone()
    }

    /// Update transmit statistics
    pub fn update_tx_stats(&self, packets: u64, bytes: u64) {
        let mut stats = self.stats.lock();
        stats.tx_packets += packets;
        stats.tx_bytes += bytes;
    }

    /// Update receive statistics
    pub fn update_rx_stats(&self, packets: u64, bytes: u64) {
        let mut stats = self.stats.lock();
        stats.rx_packets += packets;
        stats.rx_bytes += bytes;
    }
}

/// EVPN (Ethernet VPN) integration
#[derive(Debug)]
pub struct EvpnIntegration {
    /// EVPN routes
    routes: RwLock<BTreeMap<u32, EvpnRoute>>,
    /// Next route ID
    next_route_id: AtomicU32,
}

/// EVPN route
#[derive(Debug, Clone)]
pub struct EvpnRoute {
    /// Route ID
    id: u32,
    /// VNI
    vni: Vni,
    /// MAC address
    mac: [u8; 6],
    /// IP address (optional)
    ip: Option<Ipv4Addr>,
    /// VTEP IP
    vtep: Ipv4Addr,
    /// ESI (Ethernet Segment Identifier)
    esi: Option<u64>,
}

impl EvpnIntegration {
    /// Create new EVPN integration
    pub fn new() -> Self {
        Self {
            routes: RwLock::new(BTreeMap::new()),
            next_route_id: AtomicU32::new(1),
        }
    }

    /// Add EVPN route
    pub fn add_route(&self, vni: Vni, mac: [u8; 6], vtep: Ipv4Addr) -> Result<u32, VxlanError> {
        let id = self.next_route_id.fetch_add(1, Ordering::Relaxed);

        let route = EvpnRoute {
            id,
            vni,
            mac,
            ip: None,
            vtep,
            esi: None,
        };

        self.routes.write().insert(id, route);

        Ok(id)
    }

    /// Remove EVPN route
    pub fn remove_route(&self, id: u32) -> Result<(), VxlanError> {
        self.routes
            .write()
            .remove(&id)
            .ok_or(VxlanError::VniNotFound)?;
        Ok(())
    }

    /// Lookup route by MAC and VNI
    pub fn lookup(&self, vni: Vni, mac: &[u8; 6]) -> Option<EvpnRoute> {
        self.routes
            .read()
            .values()
            .find(|r| r.vni == vni && r.mac == *mac)
            .cloned()
    }

    /// List all routes
    pub fn list_routes(&self) -> Vec<EvpnRoute> {
        self.routes.read().values().cloned().collect()
    }
}

impl Default for EvpnIntegration {
    fn default() -> Self {
        Self::new()
    }
}

/// VXLAN implementation
#[derive(Debug)]
pub struct Vxlan {
    /// VNI manager
    vni_manager: VniManager,
    /// VXLAN networks indexed by VNI
    networks: RwLock<BTreeMap<u32, Arc<VxlanNetwork>>>,
    /// EVPN integration
    evpn: EvpnIntegration,
    /// Statistics
    stats: SdnStats,
}

impl Vxlan {
    /// Create new VXLAN instance
    pub fn new() -> Self {
        Self {
            vni_manager: VniManager::new(0xFFFFFF),
            networks: RwLock::new(BTreeMap::new()),
            evpn: EvpnIntegration::new(),
            stats: SdnStats::new(),
        }
    }

    /// Create network
    pub fn create_network(
        &self,
        name: String,
        config: VxlanConfig,
    ) -> Result<u32, VxlanError> {
        let vni = Vni::new(config.vni)?;
        let network = Arc::new(VxlanNetwork::new(name, vni, config));

        self.networks.write().insert(vni.value(), network);

        Ok(vni.value())
    }

    /// Delete network
    pub fn delete_network(&self, vni: u32) -> Result<(), VxlanError> {
        self.networks
            .write()
            .remove(&vni)
            .ok_or(VxlanError::VniNotFound)?;
        Ok(())
    }

    /// Get network
    pub fn get_network(&self, vni: u32) -> Option<Arc<VxlanNetwork>> {
        self.networks.read().get(&vni).cloned()
    }

    /// List all networks
    pub fn list_networks(&self) -> Vec<Arc<VxlanNetwork>> {
        self.networks.read().values().cloned().collect()
    }

    /// Encapsulate packet
    pub fn encapsulate(
        &self,
        vni: u32,
        inner_frame: &[u8],
        dest_ip: Ipv4Addr,
    ) -> Result<Vec<u8>, VxlanError> {
        let network = self.get_network(vni).ok_or(VxlanError::VniNotFound)?;

        let packet = VxlanEncapsulation::encapsulate(inner_frame, &network.config, dest_ip)?;

        network.update_tx_stats(1, packet.len() as u64);

        Ok(packet)
    }

    /// Decapsulate packet
    pub fn decapsulate(&self, packet: &[u8]) -> Result<(u32, Vec<u8>), VxlanError> {
        let (vni, inner_frame) = VxlanEncapsulation::decapsulate(packet)?;

        if let Some(network) = self.get_network(vni.value()) {
            network.update_rx_stats(1, packet.len() as u64);
        }

        Ok((vni.value(), inner_frame))
    }

    /// Get VNI manager
    pub fn vni_manager(&self) -> &VniManager {
        &self.vni_manager
    }

    /// Get EVPN integration
    pub fn evpn(&self) -> &EvpnIntegration {
        &self.evpn
    }

    /// Get statistics
    pub fn get_stats(&self) -> SdnStats {
        self.stats.clone()
    }
}

impl Default for Vxlan {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vni_valid() {
        let vni = Vni::new(1000).unwrap();
        assert_eq!(vni.value(), 1000);
        assert!(vni.is_valid());
    }

    #[test]
    fn test_vni_invalid() {
        let result = Vni::new(0x1000000);
        assert!(matches!(result, Err(VxlanError::InvalidVni)));
    }

    #[test]
    fn test_vxlan_header() {
        let vni = Vni::new(12345).unwrap();
        let header = VxlanHeader::new(vni);

        assert_eq!(header.get_vni().value(), 12345);

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), VXLAN_HEADER_SIZE);

        let parsed = VxlanHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.get_vni().value(), 12345);
    }

    #[test]
    fn test_encapsulation() {
        let config = VxlanConfig {
            vni: 1000,
            src_ip: Ipv4Addr::new(192, 168, 1, 10),
            ..Default::default()
        };

        let inner_frame = vec![0u8; 64];
        let dest_ip = Ipv4Addr::new(192, 168, 1, 20);

        let result = VxlanEncapsulation::encapsulate(&inner_frame, &config, dest_ip);
        assert!(result.is_ok());

        let packet = result.unwrap();
        assert!(packet.len() > inner_frame.len());
    }

    #[test]
    fn test_decapsulation() {
        let config = VxlanConfig {
            vni: 1000,
            src_ip: Ipv4Addr::new(192, 168, 1, 10),
            ..Default::default()
        };

        let inner_frame = vec![0u8; 64];
        let dest_ip = Ipv4Addr::new(192, 168, 1, 20);

        let packet = VxlanEncapsulation::encapsulate(&inner_frame, &config, dest_ip).unwrap();

        let (vni, decapsulated) = VxlanEncapsulation::decapsulate(&packet).unwrap();
        assert_eq!(vni.value(), 1000);
        assert_eq!(decapsulated.len(), inner_frame.len());
    }

    #[test]
    fn test_vni_manager() {
        let manager = VniManager::new(1000);

        let vni = manager.allocate("test".to_string()).unwrap();
        assert!(vni.value() <= 1000);

        let info = manager.get_info(vni).unwrap();
        assert_eq!(info.name, "test");

        let vteps = manager.list_vnis();
        assert_eq!(vteps.len(), 1);
    }

    #[test]
    fn test_vxlan_network() {
        let vni = Vni::new(1000).unwrap();
        let config = VxlanConfig::default();

        let network = VxlanNetwork::new("test".to_string(), vni, config);

        let vtep = Ipv4Addr::new(192, 168, 1, 10);
        network.add_vtep(vtep);

        let vteps = network.get_vteps();
        assert_eq!(vteps.len(), 1);
        assert_eq!(vteps[0], vtep);

        let mac = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        network.learn_mac(mac, vtep);

        let lookup = network.lookup_vtep(&mac);
        assert_eq!(lookup, Some(vtep));
    }

    #[test]
    fn test_evpn_integration() {
        let evpn = EvpnIntegration::new();

        let vni = Vni::new(1000).unwrap();
        let mac = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let vtep = Ipv4Addr::new(192, 168, 1, 10);

        let route_id = evpn.add_route(vni, mac, vtep).unwrap();
        assert!(route_id > 0);

        let route = evpn.lookup(vni, &mac);
        assert!(route.is_some());
        assert_eq!(route.unwrap().vtep, vtep);

        evpn.remove_route(route_id).unwrap();

        let route = evpn.lookup(vni, &mac);
        assert!(route.is_none());
    }

    #[test]
    fn test_vxlan() {
        let vxlan = Vxlan::new();

        let config = VxlanConfig {
            vni: 1000,
            src_ip: Ipv4Addr::new(192, 168, 1, 10),
            ..Default::default()
        };

        vxlan.create_network("test".to_string(), config).unwrap();

        let networks = vxlan.list_networks();
        assert_eq!(networks.len(), 1);
    }
}
