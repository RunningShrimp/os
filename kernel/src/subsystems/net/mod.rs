//! Network stack implementation
//!
//! This module implements a complete TCP/IP network stack for NOS,
//! including Ethernet, ARP, IPv4, ICMP, UDP, and TCP protocols.

extern crate alloc;

use alloc::string::ToString;

pub mod packet;
pub mod interface;
pub mod device;
pub mod arp;
pub mod ipv4;
pub mod icmp;
pub mod icmp_enhanced; // Enhanced ICMP features (optional: extended ICMP types, traceroute, ping)
pub mod udp;
pub mod tcp;
pub mod tcp_enhanced; // Enhanced TCP features (optional: advanced options, SACK, window scaling)
pub mod route;
pub mod buffer_management;
pub mod fragment;
pub mod processor;
pub mod socket;
pub mod zero_copy;
pub mod enhanced_network; // POSIX-compatible network API (required for socket syscalls)

// 只在需要的地方使用日志系统
// use crate::{log_info, log_error};

// Import packet pool and other essential types
use packet::{PacketPool, PacketBuffer, PacketType};

use core::sync::atomic::{AtomicU32, Ordering};

/// Global network configuration and state
pub struct NetworkStack {
    /// Network interfaces
    interfaces: Vec<Interface>,
    /// Routing table
    routes: Vec<Route>,
    /// Packet buffer pool
    packet_pool: PacketPool,
    /// Next interface ID
    next_interface_id: AtomicU32,
    /// Enhanced network manager for POSIX compatibility
    enhanced_manager: enhanced_network::EnhancedNetworkManager,
}

impl NetworkStack {
    /// Create a new network stack instance
    pub fn new() -> Self {
        Self {
            interfaces: Vec::new(),
            routes: Vec::new(),
            packet_pool: PacketPool::new(),
            next_interface_id: AtomicU32::new(1),
            enhanced_manager: enhanced_network::EnhancedNetworkManager::new(),
        }
    }

    /// Initialize the network stack
    pub fn init(&mut self) {
        // Initialize packet buffer pool
        self.packet_pool.init();

        // Add default routes (will be configured later)
        crate::log_info!("Network stack initialized");
    }

    /// Add a network interface
    pub fn add_interface(&mut self, device: Arc<dyn NetworkDevice>) -> Result<u32, NetworkError> {
        let id = self.next_interface_id.fetch_add(1, Ordering::SeqCst);
        let interface = Interface::new(id, device);

        self.interfaces.push(interface);
        crate::log_info!("Added network interface with ID: {}", id);

        Ok(id)
    }

    /// Get interface by ID
    pub fn get_interface(&self, id: u32) -> Option<&Interface> {
        self.interfaces.iter().find(|iface| iface.id() == id)
    }

    /// Get interface by ID (mutable)
    pub fn get_interface_mut(&mut self, id: u32) -> Option<&mut Interface> {
        self.interfaces.iter_mut().find(|iface| iface.id() == id)
    }

    /// Get interface by name
    pub fn get_interface_by_name(&self, name: &str) -> Option<&Interface> {
        self.interfaces.iter().find(|iface| iface.name() == name)
    }

    /// Get all interfaces
    pub fn interfaces(&self) -> &[Interface] {
        &self.interfaces
    }

    /// Send a packet through the appropriate interface
    pub fn send_packet(&mut self, packet: Packet, dest_ip: Ipv4Addr) -> Result<(), NetworkError> {
        // Find the best interface for this destination
        let interface = self.find_route(dest_ip)?;

        // Send the packet
        interface.send_packet(packet).map_err(|e| NetworkError::from(e))
    }

    /// Send a batch of packets; stops on first error and returns number successfully queued.
    pub fn send_packets_batch(
        &mut self,
        packets: &mut [Packet],
        dest_ip: Ipv4Addr,
    ) -> Result<usize, NetworkError> {
        let mut sent = 0usize;
        for pkt in packets.iter_mut() {
            // move out packet to avoid double free on error
            let mut to_send = Packet::new(PacketType::Raw).unwrap_or_else(|_| pkt.clone());
            core::mem::swap(pkt, &mut to_send);
            match self.send_packet(to_send, dest_ip) {
                Ok(_) => sent += 1,
                Err(e) => return Ok(sent), // caller can retry remaining
            }
        }
        Ok(sent)
    }

    /// Allocate a DMA-capable TX buffer for zero-copy style fill.
    pub fn allocate_tx_buffer(&mut self) -> Result<(PacketBuffer, *mut u8, usize), NetworkError> {
        self.packet_pool
            .allocate_dma()
            .map_err(|_| NetworkError::BufferExhausted)
    }

    /// Send a DMA-prepared buffer as a packet without extra copies.
    pub fn send_tx_buffer(
        &mut self,
        mut buffer: PacketBuffer,
        packet_type: PacketType,
        len: usize,
        dest_ip: Ipv4Addr,
    ) -> Result<(), NetworkError> {
        if len > buffer.capacity() {
            return Err(NetworkError::PacketTooLarge);
        }
        unsafe { buffer.set_length(len); }
        let mut packet = Packet::from_buffer(buffer, packet_type);
        packet.size = len;
        self.send_packet(packet, dest_ip)
    }

    /// Find the best interface for a destination IP
    fn find_route(&self, dest: Ipv4Addr) -> Result<&Interface, NetworkError> {
        // Simple routing: find interface with matching network
        for interface in &self.interfaces {
            if interface.is_in_network(dest) {
                return Ok(interface);
            }
        }

        // Default route (first interface)
        self.interfaces.first()
            .ok_or(NetworkError::NoRouteToHost)
    }

    /// Get the enhanced network manager
    pub fn enhanced_manager(&self) -> &enhanced_network::EnhancedNetworkManager {
        &self.enhanced_manager
    }

    /// Get the enhanced network manager (mutable)
    pub fn enhanced_manager_mut(&mut self) -> &mut enhanced_network::EnhancedNetworkManager {
        &mut self.enhanced_manager
    }
}

/// Global network stack instance
static mut NETWORK_STACK: Option<NetworkStack> = None;
static NETWORK_STACK_INIT: Once = Once::new();

/// Get the global network stack instance
pub fn network_stack() -> &'static mut NetworkStack {
    unsafe {
        NETWORK_STACK_INIT.call_once(|| {
            NETWORK_STACK = Some(NetworkStack::new());
            if let Some(ref mut stack) = NETWORK_STACK {
                stack.init();
            }
        });
        NETWORK_STACK.as_mut().unwrap()
    }
}

/// Initialize the network stack
pub fn init() {
    // This will initialize the global network stack on first access
    let _stack = network_stack();

    // Initialize enhanced network manager
    network_stack().enhanced_manager_mut().init();

    // Initialize loopback device
    use alloc::sync::Arc;
    let loopback = Arc::new(crate::net::device::LoopbackDevice::new("lo", 65536));
    let lo_id = network_stack().add_interface(loopback).unwrap();

    // Configure loopback interface
    if let Some(lo_interface) = network_stack().get_interface_mut(lo_id) {
        let lo_config = crate::net::interface::InterfaceConfig {
            name: "lo".to_string(),
            ipv4_addr: Some(Ipv4Addr::new(127, 0, 0, 1)),
            ipv4_netmask: Some(Ipv4Addr::new(255, 0, 0, 0)),
            ipv4_gateway: None,
            is_up: true,
            mtu: Some(65536),
        };

        if let Err(_) = lo_interface.configure(&lo_config) {
            log_error!("Failed to configure loopback interface");
        } else {
            let _ = lo_interface.up();
            crate::log_info!("Loopback interface configured: 127.0.0.1/8");
        }
    }

    // Initialize other network interfaces (if available)
    init_network_interfaces();

    crate::log_info!("Network stack initialized with interfaces");
}

/// Initialize additional network interfaces
fn init_network_interfaces() {

    // Try to detect and initialize available network devices
    // For now, we'll create a mock Ethernet interface for testing
    #[cfg(debug_assertions)]
    {
        create_mock_ethernet_interface();
    }
}

/// Create a mock Ethernet interface for testing (debug builds only)
#[cfg(debug_assertions)]
fn create_mock_ethernet_interface() {
    use alloc::sync::Arc;
    use crate::net::interface::InterfaceConfig;

    // Create a mock network device (would normally be detected from hardware)
    let mock_device = Arc::new(crate::net::device::MockEthernetDevice::new("eth0", 1500));

    if let Ok(eth_id) = network_stack().add_interface(mock_device) {
        if let Some(eth_interface) = network_stack().get_interface_mut(eth_id) {
            let eth_config = InterfaceConfig {
                name: "eth0".to_string(),
                ipv4_addr: Some(Ipv4Addr::new(192, 168, 1, 100)),
                ipv4_netmask: Some(Ipv4Addr::new(255, 255, 255, 0)),
                ipv4_gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
                is_up: false, // Keep down by default
                mtu: Some(1500),
            };

            if let Err(_) = eth_interface.configure(&eth_config) {
                log_error!("Failed to configure mock Ethernet interface");
            } else {
                crate::log_info!("Mock Ethernet interface configured: 192.168.1.100/24 (down)");
            }
        }
    }
}

/// Configure a network interface
pub fn configure_interface(name: &str, config: &InterfaceConfig) -> Result<u32, NetworkError> {
    let stack = network_stack();

    // Find existing interface by name and get mutable reference
    let interface_id = stack.interfaces()
        .iter()
        .find(|interface| interface.name() == name)
        .map(|interface| interface.id());

    if let Some(id) = interface_id {
        if let Some(interface) = stack.get_interface_mut(id) {
            if let Err(_) = interface.configure(config) {
                return Err(NetworkError::InterfaceNotFound);
            }

            if config.is_up {
                if let Err(_) = interface.up() {
                    return Err(NetworkError::DeviceError);
                }
            } else {
                if let Err(_) = interface.down() {
                    return Err(NetworkError::DeviceError);
                }
            }

            return Ok(id);
        }
    }

    Err(NetworkError::InterfaceNotFound)
}

/// Get interface configuration
pub fn get_interface_config(name: &str) -> Option<InterfaceConfig> {
    let stack = network_stack();

    for interface in stack.interfaces() {
        if interface.name() == name {
            return Some(interface.config());
        }
    }

    None
}

/// List all network interfaces
pub fn list_interfaces() -> Vec<(u32, String, InterfaceConfig)> {
    let stack = network_stack();
    let mut result = Vec::new();

    for interface in stack.interfaces() {
        result.push((interface.id(), interface.name().to_string(), interface.config()));
    }

    result
}

/// Get the enhanced network manager
pub fn enhanced_network_manager() -> &'static enhanced_network::EnhancedNetworkManager {
    network_stack().enhanced_manager()
}

/// Get the enhanced network manager (mutable)
pub fn enhanced_network_manager_mut() -> &'static mut enhanced_network::EnhancedNetworkManager {
    network_stack().enhanced_manager_mut()
}

/// Re-export for use in other modules
pub use self::packet::{Packet, PacketBuffer, PacketType};
pub use self::interface::{Interface, InterfaceConfig};
pub use self::device::{NetworkDevice, NetworkDeviceType};
pub use self::arp::{ArpCache, ArpEntry};
pub use self::ipv4::{Ipv4Addr, Ipv4Header, Ipv4Packet};
pub use self::icmp::{IcmpPacket, IcmpType, IcmpCode, IcmpError};
pub use self::icmp_enhanced::{
    EnhancedIcmpProcessor, EnhancedIcmpPacket, ExtendedIcmpType, IcmpMessageData,
    IcmpConfig, IcmpComprehensiveStats, IcmpSendOptions, TracerouteHop, PingReply, PingResult
};
pub use self::udp::{UdpHeader, UdpPacket, UdpSocket};
pub use self::tcp::{
    TcpHeader, TcpPacket, TcpState, TcpSocket
};
pub use self::tcp::state::TcpStateMachine;
pub use self::tcp::manager::{TcpConnection, TcpConnectionManager};
pub use self::tcp_enhanced::{
    EnhancedTcp, TcpConfig, EnhancedTcpStats, TcpOption, TcpOptionKind,
    MssOption, WindowScaleOption, TimestampOption, SackOption, SackBlock
};
pub use self::route::{RouteEntry, RoutingTable, RouteManager, RouteLookupResult, RoutingTableStats};
pub use self::fragment::{FragmentReassembler, Fragmenter, ReassemblyEntry};
pub use self::processor::{NetworkProcessor, PacketResult};
pub use self::socket::{
    Socket, SocketType, ProtocolFamily, SocketAddr,
    SocketOptions, SocketEntry, SocketState
};
pub use self::buffer_management::{
    NetworkBuffer, NetworkBufferManager, BufferHandle, BufferType, BufferFlags,
    BufferMetadata, BufferOwner, ProtocolData, BufferStats, BufferPool,
    PoolStats, GlobalBufferStats, BufferError
};
pub use self::enhanced_network::{
    EnhancedNetworkManager, EnhancedSocket, SocketType as EnhancedSocketType,
    AddressFamily, SocketAddress, NetworkStats, NetworkError as EnhancedNetworkError
};

// Module imports
use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use crate::subsystems::sync::Once;

// Forward declarations (will be implemented in submodules)
pub struct Route;
#[derive(Debug)]
pub enum NetworkError {
    NoRouteToHost,
    InterfaceNotFound,
    InvalidPacket,
    BufferExhausted,
    DeviceError,
    PacketTooLarge,
}

// Conversion from InterfaceError to NetworkError
impl From<crate::net::interface::InterfaceError> for NetworkError {
    fn from(_error: crate::net::interface::InterfaceError) -> Self {
        NetworkError::DeviceError
    }
}