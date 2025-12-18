//! Network device driver framework
//!
//! This module provides a comprehensive network device driver framework that supports
//! multiple network interface types (Ethernet, Wi-Fi, etc.), integrates with the device model,
//! and provides network hardware abstraction for the network protocol stack.

use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::Mutex;
use nos_api::{Result, Error};

use crate::subsystems::device_model::{Device, DeviceId, DeviceType, DeviceDriver, DeviceState};
use crate::network::{NetworkInterface, InterfaceType, InterfaceState, NetworkStats};

/// Network device driver framework
#[derive(Debug)]
pub struct NetworkDeviceFramework {
    /// Registered network device drivers
    drivers: Arc<Mutex<BTreeMap<String, Arc<dyn NetworkDriver>>>>,
    /// Active network devices
    devices: Arc<Mutex<BTreeMap<DeviceId, Arc<NetworkDevice>>>>,
    /// Device ID allocator
    next_device_id: Arc<Mutex<u32>>,
    /// Network device statistics
    stats: Arc<Mutex<NetworkDeviceStats>>,
}

/// Network device driver trait
pub trait NetworkDriver: DeviceDriver + Send + Sync {
    /// Get the driver name
    fn name(&self) -> &str;
    
    /// Get the supported device types
    fn supported_device_types(&self) -> &[NetworkDeviceType];
    
    /// Probe for compatible devices
    fn probe(&self) -> Result<Vec<NetworkDeviceInfo>>;
    
    /// Initialize a network device
    fn init_device(&mut self, device_info: &NetworkDeviceInfo) -> Result<()>;
    
    /// Start a network device
    fn start_device(&mut self, device_id: DeviceId) -> Result<()>;
    
    /// Stop a network device
    fn stop_device(&mut self, device_id: DeviceId) -> Result<()>;
    
    /// Send a network packet
    fn send_packet(&mut self, device_id: DeviceId, packet: &NetworkPacket) -> Result<()>;
    
    /// Receive a network packet
    fn receive_packet(&mut self, device_id: DeviceId) -> Result<Option<NetworkPacket>>;
    
    /// Get device statistics
    fn get_device_stats(&self, device_id: DeviceId) -> Result<NetworkDeviceStats>;
    
    /// Set device configuration
    fn set_device_config(&mut self, device_id: DeviceId, config: &NetworkDeviceConfig) -> Result<()>;
    
    /// Get device configuration
    fn get_device_config(&self, device_id: DeviceId) -> Result<NetworkDeviceConfig>;
    
    /// Set MAC address
    fn set_mac_address(&mut self, device_id: DeviceId, mac_addr: [u8; 6]) -> Result<()>;
    
    /// Get MAC address
    fn get_mac_address(&self, device_id: DeviceId) -> Result<[u8; 6]>;
    
    /// Set promiscuous mode
    fn set_promiscuous_mode(&mut self, device_id: DeviceId, enabled: bool) -> Result<()>;
    
    /// Set multicast mode
    fn set_multicast_mode(&mut self, device_id: DeviceId, enabled: bool) -> Result<()>;
    
    /// Add multicast address
    fn add_multicast_address(&mut self, device_id: DeviceId, mac_addr: [u8; 6]) -> Result<()>;
    
    /// Remove multicast address
    fn remove_multicast_address(&mut self, device_id: DeviceId, mac_addr: [u8; 6]) -> Result<()>;
    
    /// Get link status
    fn get_link_status(&self, device_id: DeviceId) -> Result<LinkStatus>;
    
    /// Set link speed/duplex
    fn set_link_settings(&mut self, device_id: DeviceId, settings: &LinkSettings) -> Result<()>;
    
    /// Get link settings
    fn get_link_settings(&self, device_id: DeviceId) -> Result<LinkSettings>;
    
    /// Enable/disable wake-on-LAN
    fn set_wake_on_lan(&mut self, device_id: DeviceId, enabled: bool) -> Result<()>;
    
    /// Get device capabilities
    fn get_device_capabilities(&self, device_id: DeviceId) -> Result<NetworkDeviceCapabilities>;
    
    /// Perform device reset
    fn reset_device(&mut self, device_id: DeviceId) -> Result<()>;
    
    /// Perform device self-test
    fn self_test(&mut self, device_id: DeviceId) -> Result<Vec<TestResult>>;
}

/// Network device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkDeviceType {
    /// Ethernet device
    Ethernet,
    /// Wireless (Wi-Fi) device
    Wireless,
    /// Loopback device
    Loopback,
    /// Virtual device
    Virtual,
    /// PPP device
    PPP,
    /// SLIP device
    SLIP,
    /// Tunnel device
    Tunnel,
    /// VLAN device
    VLAN,
    /// Bridge device
    Bridge,
    /// Bond device
    Bond,
}

/// Network device information
#[derive(Debug, Clone)]
pub struct NetworkDeviceInfo {
    /// Device ID
    pub device_id: DeviceId,
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: NetworkDeviceType,
    /// MAC address
    pub mac_address: [u8; 6],
    /// PCI device ID (if applicable)
    pub pci_device_id: Option<u32>,
    /// USB device ID (if applicable)
    pub usb_device_id: Option<u32>,
    /// Device resources
    pub resources: NetworkDeviceResources,
    /// Device capabilities
    pub capabilities: NetworkDeviceCapabilities,
    /// Device configuration
    pub config: NetworkDeviceConfig,
}

/// Network device resources
#[derive(Debug, Clone)]
pub struct NetworkDeviceResources {
    /// Memory-mapped I/O regions
    pub mmio_regions: Vec<MmioRegion>,
    /// I/O port regions
    pub io_port_regions: Vec<IoPortRegion>,
    /// IRQ numbers
    pub irq_numbers: Vec<u32>,
    /// DMA channels
    pub dma_channels: Vec<u32>,
    /// Memory regions for DMA
    pub dma_memory_regions: Vec<DmaMemoryRegion>,
}

/// Memory-mapped I/O region
#[derive(Debug, Clone)]
pub struct MmioRegion {
    /// Physical address
    pub physical_address: u64,
    /// Virtual address
    pub virtual_address: u64,
    /// Size
    pub size: u64,
    /// Region flags
    pub flags: MmioFlags,
}

/// Memory-mapped I/O flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MmioFlags {
    /// Region is readable
    pub readable: bool,
    /// Region is writable
    pub writable: bool,
    /// Region is cacheable
    pub cacheable: bool,
    /// Region is prefetchable
    pub prefetchable: bool,
}

impl Default for MmioFlags {
    fn default() -> Self {
        Self {
            readable: true,
            writable: true,
            cacheable: false,
            prefetchable: false,
        }
    }
}

/// I/O port region
#[derive(Debug, Clone)]
pub struct IoPortRegion {
    /// Start port
    pub start_port: u16,
    /// End port
    pub end_port: u16,
    /// Region flags
    pub flags: IoPortFlags,
}

/// I/O port flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoPortFlags {
    /// Region is readable
    pub readable: bool,
    /// Region is writable
    pub writable: bool,
}

impl Default for IoPortFlags {
    fn default() -> Self {
        Self {
            readable: true,
            writable: true,
        }
    }
}

/// DMA memory region
#[derive(Debug, Clone)]
pub struct DmaMemoryRegion {
    /// Physical address
    pub physical_address: u64,
    /// Virtual address
    pub virtual_address: u64,
    /// Size
    pub size: u64,
    /// DMA coherent flag
    pub coherent: bool,
}

/// Network device capabilities
#[derive(Debug, Clone)]
pub struct NetworkDeviceCapabilities {
    /// Maximum transmission unit
    pub max_mtu: u32,
    /// Minimum transmission unit
    pub min_mtu: u32,
    /// Supported speeds
    pub supported_speeds: Vec<LinkSpeed>,
    /// Supported duplex modes
    pub supported_duplex: Vec<LinkDuplex>,
    /// Supported media types
    pub supported_media: Vec<MediaType>,
    /// Supports promiscuous mode
    pub supports_promiscuous: bool,
    /// Supports multicast
    pub supports_multicast: bool,
    /// Supports wake-on-LAN
    pub supports_wake_on_lan: bool,
    /// Supports checksum offload
    pub supports_checksum_offload: bool,
    /// Supports TCP segmentation offload
    pub supports_tcp_segmentation_offload: bool,
    /// Supports large receive offload
    pub supports_large_receive_offload: bool,
    /// Supports scatter-gather DMA
    pub supports_scatter_gather: bool,
    /// Supports hardware timestamps
    pub supports_hardware_timestamps: bool,
    /// Number of transmit queues
    pub tx_queues: u32,
    /// Number of receive queues
    pub rx_queues: u32,
    /// Maximum frame size
    pub max_frame_size: u32,
}

/// Network device configuration
#[derive(Debug, Clone)]
pub struct NetworkDeviceConfig {
    /// MAC address
    pub mac_address: [u8; 6],
    /// MTU
    pub mtu: u32,
    /// Promiscuous mode enabled
    pub promiscuous_mode: bool,
    /// Multicast mode enabled
    pub multicast_mode: bool,
    /// Multicast addresses
    pub multicast_addresses: Vec<[u8; 6]>,
    /// Link settings
    pub link_settings: LinkSettings,
    /// Wake-on-LAN enabled
    pub wake_on_lan: bool,
    /// Checksum offload enabled
    pub checksum_offload: bool,
    /// TCP segmentation offload enabled
    pub tcp_segmentation_offload: bool,
    /// Large receive offload enabled
    pub large_receive_offload: bool,
    /// Hardware timestamps enabled
    pub hardware_timestamps: bool,
}

/// Link speed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkSpeed {
    /// 10 Mbps
    Speed10,
    /// 100 Mbps
    Speed100,
    /// 1 Gbps
    Speed1000,
    /// 2.5 Gbps
    Speed2500,
    /// 5 Gbps
    Speed5000,
    /// 10 Gbps
    Speed10000,
    /// 25 Gbps
    Speed25000,
    /// 40 Gbps
    Speed40000,
    /// 100 Gbps
    Speed100000,
}

impl LinkSpeed {
    /// Get speed in Mbps
    pub fn mbps(&self) -> u32 {
        match self {
            LinkSpeed::Speed10 => 10,
            LinkSpeed::Speed100 => 100,
            LinkSpeed::Speed1000 => 1000,
            LinkSpeed::Speed2500 => 2500,
            LinkSpeed::Speed5000 => 5000,
            LinkSpeed::Speed10000 => 10000,
            LinkSpeed::Speed25000 => 25000,
            LinkSpeed::Speed40000 => 40000,
            LinkSpeed::Speed100000 => 100000,
        }
    }
}

/// Link duplex mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkDuplex {
    /// Half duplex
    Half,
    /// Full duplex
    Full,
}

/// Media type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    /// Twisted pair
    TwistedPair,
    /// Fiber optic
    FiberOptic,
    /// Coaxial
    Coaxial,
    /// Wireless
    Wireless,
    /// Virtual
    Virtual,
}

/// Link settings
#[derive(Debug, Clone)]
pub struct LinkSettings {
    /// Link speed
    pub speed: LinkSpeed,
    /// Link duplex
    pub duplex: LinkDuplex,
    /// Auto-negotiation enabled
    pub auto_negotiation: bool,
    /// Media type
    pub media_type: MediaType,
}

impl Default for LinkSettings {
    fn default() -> Self {
        Self {
            speed: LinkSpeed::Speed1000,
            duplex: LinkDuplex::Full,
            auto_negotiation: true,
            media_type: MediaType::TwistedPair,
        }
    }
}

/// Link status
#[derive(Debug, Clone)]
pub struct LinkStatus {
    /// Link is up
    pub up: bool,
    /// Link speed
    pub speed: Option<LinkSpeed>,
    /// Link duplex
    pub duplex: Option<LinkDuplex>,
    /// Auto-negotiation enabled
    pub auto_negotiation: bool,
    /// Media type
    pub media_type: Option<MediaType>,
}

/// Network packet
#[derive(Debug, Clone)]
pub struct NetworkPacket {
    /// Packet data
    pub data: Vec<u8>,
    /// Packet length
    pub length: usize,
    /// Packet metadata
    pub metadata: PacketMetadata,
}

/// Packet metadata
#[derive(Debug, Clone)]
pub struct PacketMetadata {
    /// Timestamp
    pub timestamp: u64,
    /// Packet type
    pub packet_type: PacketType,
    /// Queue ID
    pub queue_id: u32,
    /// Priority
    pub priority: u8,
    /// VLAN tag
    pub vlan_tag: Option<u16>,
}

/// Packet type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// Unicast packet
    Unicast,
    /// Multicast packet
    Multicast,
    /// Broadcast packet
    Broadcast,
}

/// Network device statistics
#[derive(Debug, Default, Clone)]
pub struct NetworkDeviceStats {
    /// Total bytes received
    pub bytes_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Total packets sent
    pub packets_sent: u64,
    /// Receive errors
    pub receive_errors: u64,
    /// Transmit errors
    pub transmit_errors: u64,
    /// Receive dropped
    pub receive_dropped: u64,
    /// Transmit dropped
    pub transmit_dropped: u64,
    /// Multicast packets received
    pub multicast_received: u64,
    /// Multicast packets sent
    pub multicast_sent: u64,
    /// Broadcast packets received
    pub broadcast_received: u64,
    /// Broadcast packets sent
    pub broadcast_sent: u64,
    /// Collisions
    pub collisions: u64,
    /// Receive length errors
    pub receive_length_errors: u64,
    /// Receive over errors
    pub receive_over_errors: u64,
    /// Receive CRC errors
    pub receive_crc_errors: u64,
    /// Receive frame errors
    pub receive_frame_errors: u64,
    /// Receive fifo errors
    pub receive_fifo_errors: u64,
    /// Receive missed errors
    pub receive_missed_errors: u64,
    /// Transmit aborted errors
    pub transmit_aborted_errors: u64,
    /// Transmit carrier errors
    pub transmit_carrier_errors: u64,
    /// Transmit fifo errors
    pub transmit_fifo_errors: u64,
    /// Transmit heartbeat errors
    pub transmit_heartbeat_errors: u64,
    /// Transmit window errors
    pub transmit_window_errors: u64,
}

/// Test result
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test name
    pub name: String,
    /// Test passed
    pub passed: bool,
    /// Test message
    pub message: String,
    /// Test duration in microseconds
    pub duration_us: u64,
}

/// Network device
#[derive(Debug)]
pub struct NetworkDevice {
    /// Device ID
    pub device_id: DeviceId,
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: NetworkDeviceType,
    /// Device information
    pub info: NetworkDeviceInfo,
    /// Device state
    pub state: DeviceState,
    /// Network interface
    pub network_interface: NetworkInterface,
    /// Device statistics
    pub stats: NetworkDeviceStats,
}

impl NetworkDeviceFramework {
    /// Create a new network device framework
    pub fn new() -> Self {
        Self {
            drivers: Arc::new(Mutex::new(BTreeMap::new())),
            devices: Arc::new(Mutex::new(BTreeMap::new())),
            next_device_id: Arc::new(Mutex::new(1)),
            stats: Arc::new(Mutex::new(NetworkDeviceStats::default())),
        }
    }
    
    /// Register a network device driver
    pub fn register_driver(&self, driver: Arc<dyn NetworkDriver>) -> Result<()> {
        let mut drivers = self.drivers.lock();
        let name = driver.name().to_string();
        
        if drivers.contains_key(&name) {
            return Err(Error::AlreadyExists(format!("Driver {} already registered", name)));
        }
        
        drivers.insert(name, driver);
        Ok(())
    }
    
    /// Unregister a network device driver
    pub fn unregister_driver(&self, name: &str) -> Result<()> {
        let mut drivers = self.drivers.lock();
        
        if !drivers.contains_key(name) {
            return Err(Error::NotFound(format!("Driver {} not found", name)));
        }
        
        drivers.remove(name);
        Ok(())
    }
    
    /// Get a registered driver by name
    pub fn get_driver(&self, name: &str) -> Result<Arc<dyn NetworkDriver>> {
        let drivers = self.drivers.lock();
        
        drivers.get(name).cloned()
            .ok_or_else(|| Error::NotFound(format!("Driver {} not found", name)))
    }
    
    /// Get all registered drivers
    pub fn get_all_drivers(&self) -> Vec<Arc<dyn NetworkDriver>> {
        let drivers = self.drivers.lock();
        drivers.values().cloned().collect()
    }
    
    /// Probe for network devices using all registered drivers
    pub fn probe_devices(&self) -> Result<Vec<NetworkDeviceInfo>> {
        let drivers = self.drivers.lock();
        let mut all_devices = Vec::new();
        
        for driver in drivers.values() {
            match driver.probe() {
                Ok(mut devices) => all_devices.append(&mut devices),
                Err(_) => continue, // Ignore probe errors
            }
        }
        
        Ok(all_devices)
    }
    
    /// Add a network device
    pub fn add_device(&self, device_info: NetworkDeviceInfo, driver_name: &str) -> Result<DeviceId> {
        let driver = self.get_driver(driver_name)?;
        let device_id = {
            let mut next_id = self.next_device_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        // Initialize the device
        let mut driver_mut = driver.as_ref() as *const dyn NetworkDriver as *mut dyn NetworkDriver;
        unsafe {
            (*driver_mut).init_device(&device_info)?;
        }
        
        // Create network interface
        let network_interface = NetworkInterface {
            name: device_info.name.clone(),
            index: device_id,
            mac_addr: device_info.mac_address,
            ip_addr: 0, // Will be set later
            netmask: 0, // Will be set later
            state: InterfaceState::Down,
            interface_type: match device_info.device_type {
                NetworkDeviceType::Ethernet => InterfaceType::Ethernet,
                NetworkDeviceType::Wireless => InterfaceType::Wireless,
                NetworkDeviceType::Loopback => InterfaceType::Loopback,
                _ => InterfaceType::Virtual,
            },
            mtu: device_info.config.mtu,
            zero_copy_enabled: true,
        };
        
        // Create device
        let device = Arc::new(NetworkDevice {
            device_id,
            name: device_info.name.clone(),
            device_type: device_info.device_type,
            info: device_info.clone(),
            state: DeviceState::Initialized,
            network_interface,
            stats: NetworkDeviceStats::default(),
        });
        
        // Add to devices
        let mut devices = self.devices.lock();
        devices.insert(device_id, device);
        
        Ok(device_id)
    }
    
    /// Remove a network device
    pub fn remove_device(&self, device_id: DeviceId) -> Result<()> {
        let mut devices = self.devices.lock();
        
        if !devices.contains_key(&device_id) {
            return Err(Error::NotFound(format!("Device {} not found", device_id)));
        }
        
        devices.remove(&device_id);
        Ok(())
    }
    
    /// Get a network device by ID
    pub fn get_device(&self, device_id: DeviceId) -> Result<Arc<NetworkDevice>> {
        let devices = self.devices.lock();
        
        devices.get(&device_id).cloned()
            .ok_or_else(|| Error::NotFound(format!("Device {} not found", device_id)))
    }
    
    /// Get all network devices
    pub fn get_all_devices(&self) -> Vec<Arc<NetworkDevice>> {
        let devices = self.devices.lock();
        devices.values().cloned().collect()
    }
    
    /// Get devices by type
    pub fn get_devices_by_type(&self, device_type: NetworkDeviceType) -> Vec<Arc<NetworkDevice>> {
        let devices = self.devices.lock();
        devices.values()
            .filter(|device| device.device_type == device_type)
            .cloned()
            .collect()
    }
    
    /// Start a network device
    pub fn start_device(&self, device_id: DeviceId) -> Result<()> {
        let device = self.get_device(device_id)?;
        
        // Find the driver for this device
        let drivers = self.drivers.lock();
        for driver in drivers.values() {
            if driver.supported_device_types().contains(&device.device_type) {
                let mut driver_mut = driver.as_ref() as *const dyn NetworkDriver as *mut dyn NetworkDriver;
                unsafe {
                    (*driver_mut).start_device(device_id)?;
                }
                break;
            }
        }
        
        // Update device state
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            device.state = DeviceState::Running;
            device.network_interface.state = InterfaceState::Up;
        }
        
        Ok(())
    }
    
    /// Stop a network device
    pub fn stop_device(&self, device_id: DeviceId) -> Result<()> {
        let device = self.get_device(device_id)?;
        
        // Find the driver for this device
        let drivers = self.drivers.lock();
        for driver in drivers.values() {
            if driver.supported_device_types().contains(&device.device_type) {
                let mut driver_mut = driver.as_ref() as *const dyn NetworkDriver as *mut dyn NetworkDriver;
                unsafe {
                    (*driver_mut).stop_device(device_id)?;
                }
                break;
            }
        }
        
        // Update device state
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            device.state = DeviceState::Stopped;
            device.network_interface.state = InterfaceState::Down;
        }
        
        Ok(())
    }
    
    /// Send a packet through a network device
    pub fn send_packet(&self, device_id: DeviceId, packet: NetworkPacket) -> Result<()> {
        let device = self.get_device(device_id)?;
        
        // Find the driver for this device
        let drivers = self.drivers.lock();
        for driver in drivers.values() {
            if driver.supported_device_types().contains(&device.device_type) {
                let mut driver_mut = driver.as_ref() as *const dyn NetworkDriver as *mut dyn NetworkDriver;
                unsafe {
                    (*driver_mut).send_packet(device_id, &packet)?;
                }
                break;
            }
        }
        
        // Update statistics
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            device.stats.bytes_sent += packet.length as u64;
            device.stats.packets_sent += 1;
            
            match packet.metadata.packet_type {
                PacketType::Multicast => device.stats.multicast_sent += 1,
                PacketType::Broadcast => device.stats.broadcast_sent += 1,
                _ => {}
            }
        }
        
        // Update global statistics
        let mut stats = self.stats.lock();
        stats.bytes_sent += packet.length as u64;
        stats.packets_sent += 1;
        
        Ok(())
    }
    
    /// Receive a packet from a network device
    pub fn receive_packet(&self, device_id: DeviceId) -> Result<Option<NetworkPacket>> {
        let device = self.get_device(device_id)?;
        
        // Find the driver for this device
        let drivers = self.drivers.lock();
        let mut packet = None;
        
        for driver in drivers.values() {
            if driver.supported_device_types().contains(&device.device_type) {
                let mut driver_mut = driver.as_ref() as *const dyn NetworkDriver as *mut dyn NetworkDriver;
                unsafe {
                    packet = (*driver_mut).receive_packet(device_id)?;
                }
                break;
            }
        }
        
        // Update statistics if we received a packet
        if let Some(ref p) = packet {
            let mut devices = self.devices.lock();
            if let Some(device) = devices.get_mut(&device_id) {
                device.stats.bytes_received += p.length as u64;
                device.stats.packets_received += 1;
                
                match p.metadata.packet_type {
                    PacketType::Multicast => device.stats.multicast_received += 1,
                    PacketType::Broadcast => device.stats.broadcast_received += 1,
                    _ => {}
                }
            }
            
            // Update global statistics
            let mut stats = self.stats.lock();
            stats.bytes_received += p.length as u64;
            stats.packets_received += 1;
        }
        
        Ok(packet)
    }
    
    /// Get device statistics
    pub fn get_device_stats(&self, device_id: DeviceId) -> Result<NetworkDeviceStats> {
        let device = self.get_device(device_id)?;
        Ok(device.stats.clone())
    }
    
    /// Get global statistics
    pub fn get_global_stats(&self) -> NetworkDeviceStats {
        self.stats.lock().clone()
    }
    
    /// Reset global statistics
    pub fn reset_global_stats(&self) {
        *self.stats.lock() = NetworkDeviceStats::default();
    }
}

/// Global network device framework instance
static mut GLOBAL_NETWORK_DEVICE_FRAMEWORK: Option<NetworkDeviceFramework> = None;
static NETWORK_DEVICE_FRAMEWORK_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Get the global network device framework
pub fn get_network_device_framework() -> Result<&'static NetworkDeviceFramework> {
    unsafe {
        if !NETWORK_DEVICE_FRAMEWORK_INIT.load(core::sync::atomic::Ordering::Relaxed) {
            return Err(Error::InvalidState("Network device framework not initialized".to_string()));
        }
        GLOBAL_NETWORK_DEVICE_FRAMEWORK.as_ref().ok_or_else(|| {
            Error::InvalidState("Network device framework not initialized".to_string())
        })
    }
}

/// Initialize the global network device framework
pub fn init_network_device_framework() -> Result<()> {
    unsafe {
        if NETWORK_DEVICE_FRAMEWORK_INIT.swap(true, core::sync::atomic::Ordering::Relaxed) {
            return Err(Error::InvalidState("Network device framework already initialized".to_string()));
        }
        
        let framework = NetworkDeviceFramework::new();
        GLOBAL_NETWORK_DEVICE_FRAMEWORK = Some(framework);
        
        Ok(())
    }
}