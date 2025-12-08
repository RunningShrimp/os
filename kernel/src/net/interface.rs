//! Network interface management
//!
//! This module provides network interface abstraction, managing network devices
//! and handling packet routing between interfaces.

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use crate::sync::Mutex;

use super::device::{NetworkDevice, NetworkDeviceType, MacAddr, DeviceError};
use super::packet::{Packet, PacketBuffer, PacketType, PacketError};
use super::arp::ArpCache;
use super::ipv4::Ipv4Addr;

/// Network interface configuration
#[derive(Debug, Clone)]
pub struct InterfaceConfig {
    /// Interface name
    pub name: String,
    /// IPv4 address
    pub ipv4_addr: Option<Ipv4Addr>,
    /// IPv4 netmask
    pub ipv4_netmask: Option<Ipv4Addr>,
    /// IPv4 gateway
    pub ipv4_gateway: Option<Ipv4Addr>,
    /// Interface is up
    pub is_up: bool,
    /// MTU override (None to use device MTU)
    pub mtu: Option<usize>,
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        Self {
            name: "eth0".to_string(),
            ipv4_addr: None,
            ipv4_netmask: None,
            ipv4_gateway: None,
            is_up: false,
            mtu: None,
        }
    }
}

/// Network interface
pub struct Interface {
    /// Interface ID
    id: u32,
    /// Interface name
    name: String,
    /// Network device
    device: Arc<dyn NetworkDevice>,
    /// Interface configuration
    config: Mutex<InterfaceConfig>,
    /// Interface state
    is_up: AtomicBool,
    /// ARP cache
    arp_cache: Mutex<ArpCache>,
    /// Interface statistics
    stats: Mutex<InterfaceStats>,
    /// Packet receive queue
    rx_queue: Mutex<Vec<Packet>>,
    /// Maximum queue size
    max_queue_size: usize,
}

impl Interface {
    /// Create a new network interface
    pub fn new(id: u32, device: Arc<dyn NetworkDevice>) -> Self {
        let config = InterfaceConfig {
            name: device.name().to_string(),
            ..Default::default()
        };

        Self {
            id,
            name: device.name().to_string(),
            device,
            config: Mutex::new(config),
            is_up: AtomicBool::new(false),
            arp_cache: Mutex::new(ArpCache::new()),
            stats: Mutex::new(InterfaceStats::new()),
            rx_queue: Mutex::new(Vec::new()),
            max_queue_size: 1000,
        }
    }

    /// Get interface ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get interface name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the network device
    pub fn device(&self) -> &Arc<dyn NetworkDevice> {
        &self.device
    }

    /// Get device type
    pub fn device_type(&self) -> NetworkDeviceType {
        self.device.device_type()
    }

    /// Get MAC address
    pub fn mac_address(&self) -> MacAddr {
        self.device.mac_address()
    }

    /// Get MTU
    pub fn mtu(&self) -> usize {
        let config = self.config.lock();
        config.mtu.unwrap_or_else(|| self.device.mtu())
    }

    /// Check if interface is up
    pub fn is_up(&self) -> bool {
        self.is_up.load(Ordering::Relaxed) && self.device.is_up()
    }

    /// Bring interface up
    pub fn up(&self) -> Result<(), InterfaceError> {
        // Bring device up first
        self.device.up().map_err(|e| InterfaceError::DeviceError(e))?;

        // Update interface state
        self.is_up.store(true, Ordering::Relaxed);

        // Update config
        {
            let mut config = self.config.lock();
            config.is_up = true;
        }

        crate::log_info!("Network interface '{}' (ID: {}) is up", self.name, self.id);
        Ok(())
    }

    /// Bring interface down
    pub fn down(&self) -> Result<(), InterfaceError> {
        // Update interface state
        self.is_up.store(false, Ordering::Relaxed);

        // Update config
        {
            let mut config = self.config.lock();
            config.is_up = false;
        }

        // Bring device down
        self.device.down().map_err(|e| InterfaceError::DeviceError(e))?;

        // Clear receive queue
        {
            let mut queue = self.rx_queue.lock();
            queue.clear();
        }

        crate::log_info!("Network interface '{}' (ID: {}) is down", self.name, self.id);
        Ok(())
    }

    /// Configure the interface
    pub fn configure(&mut self, config: &InterfaceConfig) -> Result<(), InterfaceError> {
        let mut current_config = self.config.lock();

        // Validate configuration
        if let (Some(addr), Some(netmask)) = (config.ipv4_addr, config.ipv4_netmask) {
            if !Self::is_valid_ipv4_config(addr, netmask) {
                return Err(InterfaceError::InvalidConfig);
            }
        }

        // Update device MTU if specified
        if let Some(mtu) = config.mtu {
            if mtu < 68 || mtu > self.device.mtu() {
                return Err(InterfaceError::InvalidMtu);
            }
        }

        // Update configuration
        current_config.name = config.name.clone();
        current_config.ipv4_addr = config.ipv4_addr;
        current_config.ipv4_netmask = config.ipv4_netmask;
        current_config.ipv4_gateway = config.ipv4_gateway;
        current_config.mtu = config.mtu;

        // Update interface name
        self.name = config.name.clone();

        crate::log_info!("Network interface '{}' configured", self.name);
        Ok(())
    }

    /// Get current configuration
    pub fn config(&self) -> InterfaceConfig {
        self.config.lock().clone()
    }

    /// Get IPv4 address
    pub fn ipv4_addr(&self) -> Option<Ipv4Addr> {
        self.config.lock().ipv4_addr
    }

    /// Get IPv4 netmask
    pub fn ipv4_netmask(&self) -> Option<Ipv4Addr> {
        self.config.lock().ipv4_netmask
    }

    /// Get IPv4 gateway
    pub fn ipv4_gateway(&self) -> Option<Ipv4Addr> {
        self.config.lock().ipv4_gateway
    }

    /// Check if an IP address is in this interface's network
    pub fn is_in_network(&self, ip: Ipv4Addr) -> bool {
        let config = self.config.lock();
        if let (Some(iface_ip), Some(netmask)) = (config.ipv4_addr, config.ipv4_netmask) {
            (iface_ip.to_u32() & netmask.to_u32()) == (ip.to_u32() & netmask.to_u32())
        } else {
            false
        }
    }

    /// Check if an IP address is this interface's address
    pub fn is_my_address(&self, ip: Ipv4Addr) -> bool {
        let config = self.config.lock();
        config.ipv4_addr == Some(ip)
    }

    /// Send a packet through the interface
    pub fn send_packet(&self, packet: Packet) -> Result<(), InterfaceError> {
        if !self.is_up() {
            return Err(InterfaceError::InterfaceDown);
        }

        let packet_data = packet.data();
        if packet_data.len() > self.mtu() {
            return Err(InterfaceError::PacketTooLarge);
        }

        // Send through device
        self.device.send_packet(packet_data)
            .map_err(|e| InterfaceError::DeviceError(e))?;

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.tx_packets += 1;
            stats.tx_bytes += packet_data.len() as u64;
        }

        Ok(())
    }

    /// Receive a packet from the interface
    pub fn receive_packet(&self) -> Result<Option<Packet>, InterfaceError> {
        if !self.is_up() {
            return Err(InterfaceError::InterfaceDown);
        }

        // Check receive queue first
        {
            let mut queue = self.rx_queue.lock();
            if let Some(packet) = queue.pop() {
                let mut stats = self.stats.lock();
                stats.rx_packets += 1;
                stats.rx_bytes += packet.len() as u64;
                return Ok(Some(packet));
            }
        }

        // Try to receive from device
        match self.device.receive_packet() {
            Ok(Some(data)) => {
                let packet = Packet::from_bytes(&data, PacketType::Ethernet)
                    .map_err(|e| InterfaceError::PacketError(e))?;

                let mut stats = self.stats.lock();
                stats.rx_packets += 1;
                stats.rx_bytes += data.len() as u64;

                Ok(Some(packet))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(InterfaceError::DeviceError(e)),
        }
    }

    /// Queue a packet for reception
    pub fn queue_packet(&self, packet: Packet) -> Result<(), InterfaceError> {
        let mut queue = self.rx_queue.lock();
        if queue.len() >= self.max_queue_size {
            return Err(InterfaceError::QueueFull);
        }
        queue.push(packet);
        Ok(())
    }

    /// Get interface statistics
    pub fn stats(&self) -> InterfaceStats {
        self.stats.lock().clone()
    }

    /// Get ARP cache reference
    pub fn arp_cache(&self) -> &Mutex<ArpCache> {
        &self.arp_cache
    }

    
    /// Reset interface statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = InterfaceStats::new();
    }

    /// Validate IPv4 configuration
    fn is_valid_ipv4_config(addr: Ipv4Addr, netmask: Ipv4Addr) -> bool {
        // Basic validation - could be enhanced
        addr != Ipv4Addr::UNSPECIFIED &&
        netmask != Ipv4Addr::UNSPECIFIED &&
        Self::is_valid_netmask(netmask)
    }

    /// Check if netmask is valid (contiguous ones)
    fn is_valid_netmask(netmask: Ipv4Addr) -> bool {
        let mask = netmask.to_u32();
        let inverted = !mask;
        // Valid if inverted + 1 is a power of 2
        inverted.wrapping_add(1) & inverted == 0
    }
}

/// Interface statistics
#[derive(Debug, Clone, Default)]
pub struct InterfaceStats {
    /// Packets transmitted
    pub tx_packets: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Transmit errors
    pub tx_errors: u64,
    /// Transmit dropped packets
    pub tx_dropped: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Bytes received
    pub rx_bytes: u64,
    /// Receive errors
    pub rx_errors: u64,
    /// Receive dropped packets
    pub rx_dropped: u64,
    /// Multicast packets received
    pub rx_multicast: u64,
}

impl InterfaceStats {
    /// Create new interface statistics
    pub fn new() -> Self {
        Self::default()
    }
}

/// Interface errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceError {
    /// Interface is down
    InterfaceDown,
    /// Device error
    DeviceError(DeviceError),
    /// Invalid configuration
    InvalidConfig,
    /// Invalid MTU
    InvalidMtu,
    /// Packet too large
    PacketTooLarge,
    /// Packet error
    PacketError(PacketError),
    /// Receive queue full
    QueueFull,
    /// No such interface
    NoSuchInterface,
    /// Operation not supported
    NotSupported,
}

/// Interface manager for handling multiple interfaces
pub struct InterfaceManager {
    /// List of interfaces
    interfaces: Vec<Arc<Interface>>,
    /// Next interface ID
    next_id: AtomicU32,
}

impl InterfaceManager {
    /// Create a new interface manager
    pub fn new() -> Self {
        Self {
            interfaces: Vec::new(),
            next_id: AtomicU32::new(1),
        }
    }

    /// Add a new interface
    pub fn add_interface(&mut self, device: Arc<dyn NetworkDevice>) -> Arc<Interface> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let interface = Arc::new(Interface::new(id, device));
        self.interfaces.push(interface.clone());
        crate::log_info!("Added network interface with ID: {}", id);
        interface
    }

    /// Get interface by ID
    pub fn get_interface(&self, id: u32) -> Option<Arc<Interface>> {
        self.interfaces.iter().find(|iface| iface.id() == id).cloned()
    }

    /// Get interface by name
    pub fn get_interface_by_name(&self, name: &str) -> Option<Arc<Interface>> {
        self.interfaces.iter().find(|iface| iface.name() == name).cloned()
    }

    /// Get all interfaces
    pub fn interfaces(&self) -> &[Arc<Interface>] {
        &self.interfaces
    }

    /// Find interface for a given IP address
    pub fn find_interface_for_ip(&self, ip: Ipv4Addr) -> Option<Arc<Interface>> {
        self.interfaces.iter()
            .find(|iface| iface.is_in_network(ip))
            .cloned()
    }

    /// Get interface count
    pub fn count(&self) -> usize {
        self.interfaces.len()
    }
}
