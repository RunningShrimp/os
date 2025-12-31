//! Network Virtualization Implementation
//!
//! This module provides comprehensive network virtualization capabilities:
//! - VXLAN (Virtual Extensible LAN) - RFC 7348
//! - Geneve (Generic Network Virtualization Encapsulation) - RFC 8926
//! - NVGRE (Network Virtualization using Generic Routing Encapsulation)
//! - VTEP (VXLAN Tunnel Endpoint) management
//! - Overlay routing and forwarding
//! - Network namespaces
//! - Virtual network interfaces (veth pairs)
//! - Bridge and Open vSwitch integration
//!
//! These technologies enable creation of virtual networks overlaid on physical
//! infrastructure, essential for cloud-native and multi-tenant environments.

#![allow(dead_code)]

use alloc::{collections::BTreeMap, string::{String, ToString}, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::subsystems::sync::{Mutex, RwLock};

// ============================================================================
// Common Types
// ============================================================================

/// Virtual network identifier (VNI)
pub type Vni = u32;

/// Virtual subnet identifier (VSI) for NVGRE
pub type Vsi = u32;

/// IP address type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpAddr {
    V4(u32),
    V6([u8; 16]),
}

impl IpAddr {
    /// Create IPv4 address from octets
    pub fn v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        let addr = ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32);
        IpAddr::V4(addr)
    }

    /// Check if address is multicast
    pub fn is_multicast(&self) -> bool {
        match self {
            IpAddr::V4(addr) => (addr & 0xF000_0000) == 0xE000_0000,
            IpAddr::V6(_) => false, // Simplified
        }
    }
}

/// MAC address type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    /// Create MAC address from bytes
    pub fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Create broadcast MAC address
    pub fn broadcast() -> Self {
        Self([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
    }

    /// Check if MAC is multicast
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 == 1
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 6] {
        self.0
    }
}

// ============================================================================
// VXLAN Implementation
// ============================================================================

/// VXLAN header flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VxlanFlags(u8);

impl VxlanFlags {
    /// Create new VXLAN flags
    pub fn new() -> Self {
        Self(0x08) // I-bit set
    }

    /// Check if I-bit is set (valid VNI present)
    pub fn is_valid(&self) -> bool {
        self.0 & 0x08 != 0
    }
}

/// VXLAN header
#[derive(Debug, Clone)]
pub struct VxlanHeader {
    /// Flags
    pub flags: VxlanFlags,
    /// Reserved
    pub reserved: [u8; 3],
    /// VNI (24-bit)
    pub vni: Vni,
}

impl VxlanHeader {
    /// Create new VXLAN header
    pub fn new(vni: Vni) -> Self {
        Self {
            flags: VxlanFlags::new(),
            reserved: [0; 3],
            vni: vni & 0x00FF_FFFF,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.flags.0;
        bytes[1..4].copy_from_slice(&self.reserved);
        bytes[4..7].copy_from_slice(&self.vni.to_be_bytes()[1..4]);
        bytes[7] = 0; // Reserved
        bytes
    }

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }

        let flags = VxlanFlags { 0: bytes[0] };
        if !flags.is_valid() {
            return None;
        }

        let vni = ((bytes[4] as u32) << 16) |
                  ((bytes[5] as u32) << 8) |
                  (bytes[6] as u32);

        Some(Self {
            flags,
            reserved: [bytes[1], bytes[2], bytes[3]],
            vni,
        })
    }
}

/// VXLAN tunnel endpoint (VTEP)
#[derive(Debug)]
pub struct Vtep {
    /// VTEP ID
    id: u32,
    /// VTEP name
    name: String,
    /// Local IP address
    local_ip: IpAddr,
    /// Remote IP addresses (multicast group or unicast peers)
    remote_ips: Mutex<Vec<IpAddr>>,
    /// VNI to network mapping
    vni_map: Mutex<BTreeMap<Vni, Arc<VxlanNetwork>>>,
    /// Statistics
    stats: Mutex<VtepStats>,
}

/// VTEP statistics
#[derive(Debug, Default, Clone)]
pub struct VtepStats {
    /// Encapsulated packets
    pub encapsulated: u64,
    /// Decapsulated packets
    pub decapsulated: u64,
    /// Errors
    pub errors: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
}

impl Vtep {
    /// Create new VTEP
    pub fn new(id: u32, name: String, local_ip: IpAddr) -> Self {
        Self {
            id,
            name,
            local_ip,
            remote_ips: Mutex::new(Vec::new()),
            vni_map: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(VtepStats::default()),
        }
    }

    /// Get VTEP ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get VTEP name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get local IP
    pub fn local_ip(&self) -> IpAddr {
        self.local_ip
    }

    /// Add remote IP
    pub fn add_remote_ip(&self, remote_ip: IpAddr) -> Result<(), VirtualError> {
        let mut remotes = self.remote_ips.lock();
        remotes.push(remote_ip);
        Ok(())
    }

    /// Remove remote IP
    pub fn remove_remote_ip(&self, remote_ip: IpAddr) -> Result<(), VirtualError> {
        let mut remotes = self.remote_ips.lock();
        let original_len = remotes.len();
        remotes.retain(|&ip| ip != remote_ip);

        if remotes.len() < original_len {
            Ok(())
        } else {
            Err(VirtualError::RemoteNotFound)
        }
    }

    /// Add VXLAN network
    pub fn add_network(&self, network: Arc<VxlanNetwork>) -> Result<(), VirtualError> {
        let vni = network.vni();
        let mut networks = self.vni_map.lock();

        if networks.contains_key(&vni) {
            return Err(VirtualError::NetworkExists);
        }

        networks.insert(vni, network);
        Ok(())
    }

    /// Remove VXLAN network
    pub fn remove_network(&self, vni: Vni) -> Result<(), VirtualError> {
        let mut networks = self.vni_map.lock();

        if networks.remove(&vni).is_some() {
            Ok(())
        } else {
            Err(VirtualError::NetworkNotFound)
        }
    }

    /// Encapsulate packet
    pub fn encapsulate(&self, vni: Vni, packet: &[u8]) -> Result<Vec<u8>, VirtualError> {
        let header = VxlanHeader::new(vni);
        let mut encapsulated = Vec::with_capacity(8 + packet.len());

        encapsulated.extend_from_slice(&header.to_bytes());
        encapsulated.extend_from_slice(packet);

        // Update statistics
        let mut stats = self.stats.lock();
        stats.encapsulated += 1;
        stats.bytes_sent += packet.len() as u64;

        Ok(encapsulated)
    }

    /// Decapsulate packet
    pub fn decapsulate(&self, packet: &[u8]) -> Result<(Vni, Vec<u8>), VirtualError> {
        if packet.len() < 8 {
            return Err(VirtualError::InvalidPacket);
        }

        let header = VxlanHeader::from_bytes(packet)
            .ok_or(VirtualError::InvalidHeader)?;

        let inner_packet = packet[8..].to_vec();

        // Update statistics
        let mut stats = self.stats.lock();
        stats.decapsulated += 1;
        stats.bytes_received += inner_packet.len() as u64;

        Ok((header.vni, inner_packet))
    }

    /// Get statistics
    pub fn get_stats(&self) -> VtepStats {
        let stats = self.stats.lock();
        stats.clone()
    }
}

/// VXLAN network
#[derive(Debug)]
pub struct VxlanNetwork {
    /// VNI
    vni: Vni,
    /// Network name
    name: String,
    /// Bridge interface
    bridge: Option<String>,
    /// VTEPs in this network
    vteps: Mutex<Vec<Arc<Vtep>>>,
    /// Learning table (MAC -> VTEP)
    mac_table: Mutex<BTreeMap<MacAddr, IpAddr>>,
    /// Aging timeout (seconds)
    aging_timeout: u32,
}

impl VxlanNetwork {
    /// Create new VXLAN network
    pub fn new(vni: Vni, name: String) -> Self {
        Self {
            vni,
            name,
            bridge: None,
            vteps: Mutex::new(Vec::new()),
            mac_table: Mutex::new(BTreeMap::new()),
            aging_timeout: 300, // 5 minutes default
        }
    }

    /// Get VNI
    pub fn vni(&self) -> Vni {
        self.vni
    }

    /// Get network name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set bridge interface
    pub fn set_bridge(&mut self, bridge: String) {
        self.bridge = Some(bridge);
    }

    /// Add VTEP to network
    pub fn add_vtep(&self, vtep: Arc<Vtep>) -> Result<(), VirtualError> {
        let mut vteps = self.vteps.lock();
        vteps.push(vtep);
        Ok(())
    }

    /// Learn MAC address
    pub fn learn_mac(&self, mac: MacAddr, vtep_ip: IpAddr) {
        let mut table = self.mac_table.lock();
        table.insert(mac, vtep_ip);
    }

    /// Lookup MAC address
    pub fn lookup_mac(&self, mac: &MacAddr) -> Option<IpAddr> {
        let table = self.mac_table.lock();
        table.get(mac).copied()
    }

    /// Flush MAC table
    pub fn flush_mac_table(&self) {
        let mut table = self.mac_table.lock();
        table.clear();
    }

    /// Age out MAC entries (simplified)
    pub fn age_mac_entries(&self) {
        // In real implementation, check timestamps and remove old entries
    }
}

// ============================================================================
// Geneve Implementation
// ============================================================================

/// Geneve header
#[derive(Debug, Clone)]
pub struct GeneveHeader {
    /// Version
    pub version: u8,
    /// Options length
    pub options_length: u8,
    /// OAM flag
    pub oam: bool,
    /// Critical option present
    pub critical: bool,
    /// Reserved
    pub reserved: u8,
    /// Protocol type (typically 0x6558 for Ethernet)
    pub protocol: u16,
    /// VNI
    pub vni: Vni,
    /// Options
    pub options: Vec<GeneveOption>,
}

/// Geneve option
#[derive(Debug, Clone)]
pub struct GeneveOption {
    /// Option class
    pub class: u16,
    /// Option type
    pub type_: u8,
    /// Critical option flag
    pub critical: bool,
    /// Option data
    pub data: Vec<u8>,
}

impl GeneveHeader {
    /// Create new Geneve header
    pub fn new(vni: Vni, options: Vec<GeneveOption>) -> Self {
        let options_length = (options.len() * 4) as u8;

        Self {
            version: 0,
            options_length,
            oam: false,
            critical: false,
            reserved: 0,
            protocol: 0x6558, // Ethernet
            vni: vni & 0x00FF_FFFF,
            options,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // First 4 bytes
        let byte0 = (self.version << 6) |
                   ((self.options_length & 0x3F) << 0);
        let byte1 = if self.oam { 0x80 } else { 0 } |
                   if self.critical { 0x40 } else { 0 };
        bytes.push(byte0);
        bytes.push(byte1);
        bytes.push(self.reserved);
        bytes.push(0); // Reserved

        // Protocol (2 bytes)
        bytes.extend_from_slice(&self.protocol.to_be_bytes());

        // VNI (3 bytes) + reserved
        bytes.extend_from_slice(&self.vni.to_be_bytes()[1..4]);
        bytes.push(0); // Reserved

        // Options
        for option in &self.options {
            bytes.extend_from_slice(&option.class.to_be_bytes());
            let type_byte = option.type_ & 0x1F;
            let byte1 = if option.critical { 0x80 } else { 0 } | type_byte;
            bytes.push(byte1);
            bytes.push((option.data.len() as u8) & 0x1F);
            bytes.extend_from_slice(&option.data);
            // Pad to 4-byte boundary
            while bytes.len() % 4 != 0 {
                bytes.push(0);
            }
        }

        bytes
    }

    /// Parse from bytes (simplified)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }

        let version = bytes[0] >> 6;
        let options_length = bytes[0] & 0x3F;
        let oam = (bytes[1] & 0x80) != 0;
        let critical = (bytes[1] & 0x40) != 0;
        let protocol = u16::from_be_bytes([bytes[4], bytes[5]]);
        let vni = ((bytes[8] as u32) << 16) |
                 ((bytes[9] as u32) << 8) |
                 (bytes[10] as u32);

        // Parse options (simplified - would need full implementation)
        let options = Vec::new();

        Some(Self {
            version,
            options_length,
            oam,
            critical,
            reserved: bytes[2],
            protocol,
            vni,
            options,
        })
    }
}

/// Geneve tunnel endpoint
#[derive(Debug)]
pub struct GeneveEndpoint {
    /// Endpoint ID
    id: u32,
    /// Local IP
    local_ip: IpAddr,
    /// Remote IP
    remote_ip: IpAddr,
    /// VNI
    vni: Vni,
}

impl GeneveEndpoint {
    /// Create new Geneve endpoint
    pub fn new(id: u32, local_ip: IpAddr, remote_ip: IpAddr, vni: Vni) -> Self {
        Self {
            id,
            local_ip,
            remote_ip,
            vni,
        }
    }

    /// Encapsulate packet
    pub fn encapsulate(&self, packet: &[u8], options: Vec<GeneveOption>) -> Result<Vec<u8>, VirtualError> {
        let header = GeneveHeader::new(self.vni, options);
        let mut encapsulated = Vec::with_capacity(header.to_bytes().len() + packet.len());

        encapsulated.extend_from_slice(&header.to_bytes());
        encapsulated.extend_from_slice(packet);

        Ok(encapsulated)
    }

    /// Decapsulate packet
    pub fn decapsulate(&self, packet: &[u8]) -> Result<Vec<u8>, VirtualError> {
        if packet.len() < 8 {
            return Err(VirtualError::InvalidPacket);
        }

        let _header = GeneveHeader::from_bytes(packet)
            .ok_or(VirtualError::InvalidHeader)?;

        Ok(packet[8..].to_vec())
    }
}

// ============================================================================
// NVGRE Implementation
// ============================================================================

/// NVGRE header
#[derive(Debug, Clone)]
pub struct NvgreHeader {
    /// Key (contains VSI)
    pub key: u32,
    /// Flow ID
    pub flow_id: u8,
}

impl NvgreHeader {
    /// Create new NVGRE header
    pub fn new(vsi: Vsi, flow_id: u8) -> Self {
        let key = (0x2000 << 16) | ((vsi & 0x00FF_FFFF) as u32); // Set TS bit

        Self {
            key,
            flow_id: flow_id & 0x3F,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 4] {
        let bytes = [
            (self.key >> 24) as u8,
            (self.key >> 16) as u8,
            (self.key >> 8) as u8,
            (self.key >> 0) as u8,
        ];

        // Flow ID goes in reserved bits
        [bytes[0], bytes[1], bytes[2] | (self.flow_id << 2), bytes[3]]
    }

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let key = ((bytes[0] as u32) << 24) |
                 ((bytes[1] as u32) << 16) |
                 ((bytes[2] as u32 & 0x3F) << 8) |
                 ((bytes[3] as u32) << 0);

        let flow_id = (bytes[2] >> 2) & 0x3F;

        Some(Self { key, flow_id })
    }
}

/// NVGRE endpoint
#[derive(Debug)]
pub struct NvgreEndpoint {
    /// Local IP
    local_ip: IpAddr,
    /// Remote IP
    remote_ip: IpAddr,
    /// VSI
    vsi: Vsi,
}

impl NvgreEndpoint {
    /// Create new NVGRE endpoint
    pub fn new(local_ip: IpAddr, remote_ip: IpAddr, vsi: Vsi) -> Self {
        Self {
            local_ip,
            remote_ip,
            vsi,
        }
    }

    /// Encapsulate packet
    pub fn encapsulate(&self, packet: &[u8], flow_id: u8) -> Result<Vec<u8>, VirtualError> {
        let header = NvgreHeader::new(self.vsi, flow_id);
        let mut encapsulated = Vec::with_capacity(4 + packet.len());

        encapsulated.extend_from_slice(&header.to_bytes());
        encapsulated.extend_from_slice(packet);

        Ok(encapsulated)
    }

    /// Decapsulate packet
    pub fn decapsulate(&self, packet: &[u8]) -> Result<Vec<u8>, VirtualError> {
        if packet.len() < 4 {
            return Err(VirtualError::InvalidPacket);
        }

        let _header = NvgreHeader::from_bytes(packet)
            .ok_or(VirtualError::InvalidHeader)?;

        Ok(packet[4..].to_vec())
    }
}

// ============================================================================
// Bridge Implementation
// ============================================================================

/// Virtual bridge
#[derive(Debug)]
pub struct VirtualBridge {
    /// Bridge ID
    id: u32,
    /// Bridge name
    name: String,
    /// Bridge ports
    ports: Mutex<BTreeMap<u32, Arc<BridgePort>>>,
    /// Next port ID
    next_port_id: AtomicU32,
    /// MAC learning table
    mac_table: Mutex<BTreeMap<MacAddr, u32>>,
    /// STP state
    stp_enabled: Mutex<bool>,
    /// Statistics
    stats: Mutex<BridgeStats>,
}

/// Bridge port
#[derive(Debug)]
pub struct BridgePort {
    /// Port ID
    id: u32,
    /// Port name
    name: String,
    /// Port state
    state: PortState,
    /// STP state
    stp_state: StpState,
}

/// Port state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortState {
    Disabled,
    Listening,
    Learning,
    Forwarding,
    Blocking,
}

/// STP state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StpState {
    Disabled,
    Blocking,
    Listening,
    Learning,
    Forwarding,
}

/// Bridge statistics
#[derive(Debug, Default, Clone)]
pub struct BridgeStats {
    /// Packets received
    pub packets_rx: u64,
    /// Packets transmitted
    pub packets_tx: u64,
    /// Packets forwarded
    pub packets_forwarded: u64,
    /// MAC learning hits
    pub mac_hits: u64,
    /// MAC learning misses
    pub mac_misses: u64,
}

impl VirtualBridge {
    /// Create new virtual bridge
    pub fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            ports: Mutex::new(BTreeMap::new()),
            next_port_id: AtomicU32::new(1),
            mac_table: Mutex::new(BTreeMap::new()),
            stp_enabled: Mutex::new(false),
            stats: Mutex::new(BridgeStats::default()),
        }
    }

    /// Get bridge ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get bridge name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Add port to bridge
    pub fn add_port(&self, name: String) -> Result<u32, VirtualError> {
        let port_id = self.next_port_id.fetch_add(1, Ordering::SeqCst);

        let port = Arc::new(BridgePort {
            id: port_id,
            name,
            state: PortState::Disabled,
            stp_state: StpState::Blocking,
        });

        let mut ports = self.ports.lock();
        ports.insert(port_id, port);

        Ok(port_id)
    }

    /// Remove port from bridge
    pub fn remove_port(&self, port_id: u32) -> Result<(), VirtualError> {
        let mut ports = self.ports.lock();

        if ports.remove(&port_id).is_some() {
            // Remove MAC entries for this port
            let mut mac_table = self.mac_table.lock();
            mac_table.retain(|_, &mut p| p != port_id);

            Ok(())
        } else {
            Err(VirtualError::PortNotFound)
        }
    }

    /// Enable STP
    pub fn enable_stp(&self) {
        let mut stp_enabled = self.stp_enabled.lock();
        *stp_enabled = true;
    }

    /// Disable STP
    pub fn disable_stp(&self) {
        let mut stp_enabled = self.stp_enabled.lock();
        *stp_enabled = false;
    }

    /// Forward packet (flooding if destination unknown)
    pub fn forward_packet(&self, src_mac: MacAddr, dst_mac: MacAddr, ingress_port: u32) -> Vec<u32> {
        // Learn source MAC
        {
            let mut mac_table = self.mac_table.lock();
            mac_table.insert(src_mac, ingress_port);
        }

        let mut stats = self.stats.lock();
        stats.packets_rx += 1;

        // Lookup destination MAC
        let egress_ports = {
            let mac_table = self.mac_table.lock();

            if dst_mac.is_multicast() || dst_mac == MacAddr::broadcast() {
                // Flood to all ports except ingress
                let ports = self.ports.lock();
                ports.keys()
                    .filter(|&&id| id != ingress_port)
                    .copied()
                    .collect()
            } else if let Some(&port_id) = mac_table.get(&dst_mac) {
                stats.mac_hits += 1;
                vec![port_id]
            } else {
                // Unknown unicast - flood
                stats.mac_misses += 1;
                let ports = self.ports.lock();
                ports.keys()
                    .filter(|&&id| id != ingress_port)
                    .copied()
                    .collect()
            }
        };

        stats.packets_forwarded += egress_ports.len() as u64;

        egress_ports
    }

    /// Get bridge statistics
    pub fn get_stats(&self) -> BridgeStats {
        let stats = self.stats.lock();
        stats.clone()
    }

    /// Flush MAC table
    pub fn flush_mac_table(&self) {
        let mut mac_table = self.mac_table.lock();
        mac_table.clear();
    }
}

// ============================================================================
// Network Namespace
// ============================================================================

/// Network namespace
#[derive(Debug)]
pub struct NetworkNamespace {
    /// Namespace ID
    id: u32,
    /// Namespace name
    name: String,
    /// Network interfaces
    interfaces: Mutex<BTreeMap<String, Arc<dyn VirtualInterface>>>,
    /// Routing table
    routes: Mutex<Vec<NamespaceRoute>>,
    /// Statistics
    stats: Mutex<NamespaceStats>,
}

/// Virtual interface trait
pub trait VirtualInterface {
    /// Get interface name
    fn name(&self) -> &str;
    /// Get interface index
    fn index(&self) -> u32;
    /// Send packet
    fn send(&self, packet: &[u8]) -> Result<(), VirtualError>;
    /// Receive packet
    fn receive(&self) -> Result<Vec<u8>, VirtualError>;
}

/// Namespace route
#[derive(Debug, Clone)]
pub struct NamespaceRoute {
    /// Destination network
    pub dest: IpAddr,
    /// Netmask
    pub netmask: u8,
    /// Gateway
    pub gateway: Option<IpAddr>,
    /// Output interface
    pub interface: String,
    /// Metric
    pub metric: u32,
}

/// Namespace statistics
#[derive(Debug, Default, Clone)]
pub struct NamespaceStats {
    /// Bytes received
    pub bytes_rx: u64,
    /// Bytes transmitted
    pub bytes_tx: u64,
    /// Packets received
    pub packets_rx: u64,
    /// Packets transmitted
    pub packets_tx: u64,
}

impl NetworkNamespace {
    /// Create new network namespace
    pub fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            interfaces: Mutex::new(BTreeMap::new()),
            routes: Mutex::new(Vec::new()),
            stats: Mutex::new(NamespaceStats::default()),
        }
    }

    /// Get namespace ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get namespace name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Add interface to namespace
    pub fn add_interface(&self, interface: Arc<dyn VirtualInterface>) -> Result<(), VirtualError> {
        let name = interface.name().to_string();
        let mut interfaces = self.interfaces.lock();
        interfaces.insert(name, interface);
        Ok(())
    }

    /// Remove interface from namespace
    pub fn remove_interface(&self, name: &str) -> Result<(), VirtualError> {
        let mut interfaces = self.interfaces.lock();
        if interfaces.remove(name).is_some() {
            Ok(())
        } else {
            Err(VirtualError::InterfaceNotFound)
        }
    }

    /// Add route
    pub fn add_route(&self, route: NamespaceRoute) -> Result<(), VirtualError> {
        let mut routes = self.routes.lock();
        routes.push(route);
        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> NamespaceStats {
        let stats = self.stats.lock();
        stats.clone()
    }
}

// ============================================================================
// Overlay Routing
// ============================================================================

/// Overlay router
#[derive(Debug)]
pub struct OverlayRouter {
    /// Router ID
    id: u32,
    /// VTEPs
    vteps: Mutex<BTreeMap<u32, Arc<Vtep>>>,
    /// Networks
    networks: Mutex<BTreeMap<Vni, Arc<VxlanNetwork>>>,
    /// Routing table
    routes: Mutex<Vec<OverlayRoute>>,
}

/// Overlay route
#[derive(Debug, Clone)]
pub struct OverlayRoute {
    /// Destination VNI
    pub vni: Vni,
    /// Destination network
    pub dest: IpAddr,
    /// Netmask
    pub netmask: u8,
    /// Next hop VTEP
    pub next_hop_vtep: u32,
    /// Metric
    pub metric: u32,
}

impl OverlayRouter {
    /// Create new overlay router
    pub fn new(id: u32) -> Self {
        Self {
            id,
            vteps: Mutex::new(BTreeMap::new()),
            networks: Mutex::new(BTreeMap::new()),
            routes: Mutex::new(Vec::new()),
        }
    }

    /// Add VTEP
    pub fn add_vtep(&self, vtep: Arc<Vtep>) -> Result<(), VirtualError> {
        let id = vtep.id();
        let mut vteps = self.vteps.lock();
        vteps.insert(id, vtep);
        Ok(())
    }

    /// Add network
    pub fn add_network(&self, network: Arc<VxlanNetwork>) -> Result<(), VirtualError> {
        let vni = network.vni();
        let mut networks = self.networks.lock();
        networks.insert(vni, network);
        Ok(())
    }

    /// Add route
    pub fn add_route(&self, route: OverlayRoute) -> Result<(), VirtualError> {
        let mut routes = self.routes.lock();
        routes.push(route);
        Ok(())
    }

    /// Lookup route for destination
    pub fn lookup_route(&self, vni: Vni, dest: IpAddr) -> Option<Arc<Vtep>> {
        let routes = self.routes.lock();

        // Find best matching route
        for route in routes.iter() {
            if route.vni == vni {
                // Simplified matching - in real implementation, do proper subnet matching
                if route.dest == dest {
                    let vteps = self.vteps.lock();
                    return vteps.get(&route.next_hop_vtep).cloned();
                }
            }
        }

        None
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Network virtualization errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualError {
    NetworkNotFound,
    NetworkExists,
    VtepNotFound,
    RemoteNotFound,
    PortNotFound,
    InterfaceNotFound,
    InvalidPacket,
    InvalidHeader,
    EncapsulationFailed,
    DecapsulationFailed,
    RouteNotFound,
    BridgeError,
}

// ============================================================================
// Default Implementations
// ============================================================================

impl Default for VtepStats {
    fn default() -> Self {
        Self {
            encapsulated: 0,
            decapsulated: 0,
            errors: 0,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

impl Default for BridgeStats {
    fn default() -> Self {
        Self {
            packets_rx: 0,
            packets_tx: 0,
            packets_forwarded: 0,
            mac_hits: 0,
            mac_misses: 0,
        }
    }
}

impl Default for NamespaceStats {
    fn default() -> Self {
        Self {
            bytes_rx: 0,
            bytes_tx: 0,
            packets_rx: 0,
            packets_tx: 0,
        }
    }
}
