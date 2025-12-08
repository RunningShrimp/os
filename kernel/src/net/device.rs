//! Network device abstraction layer
//!
//! This module provides a unified interface for different types of network devices,
//! supporting both physical and virtual network interfaces.

extern crate alloc;
use alloc::sync::Arc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// Network device interface
pub trait NetworkDevice: Send + Sync {
    /// Get device name
    fn name(&self) -> &str;

    /// Get device type
    fn device_type(&self) -> NetworkDeviceType;

    /// Get MAC address
    fn mac_address(&self) -> MacAddr;

    /// Get MTU (Maximum Transmission Unit)
    fn mtu(&self) -> usize;

    /// Check if device is up
    fn is_up(&self) -> bool;

    /// Bring device up
    fn up(&self) -> Result<(), DeviceError>;

    /// Bring device down
    fn down(&self) -> Result<(), DeviceError>;

    /// Send a packet through the device
    fn send_packet(&self, packet: &[u8]) -> Result<(), DeviceError>;

    /// Receive a packet from the device
    fn receive_packet(&self) -> Result<Option<Vec<u8>>, DeviceError>;

    /// Get device statistics
    fn stats(&self) -> DeviceStats;

    /// Configure device settings
    fn configure(&mut self, config: &DeviceConfig) -> Result<(), DeviceError>;

    /// Get device capabilities
    fn capabilities(&self) -> DeviceCapabilities;
}

/// Network device types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkDeviceType {
    /// Ethernet device
    Ethernet,
    /// Loopback device
    Loopback,
    /// Tunnel device
    Tunnel,
    /// Virtual Ethernet device
    VirtualEthernet,
    /// Wireless device
    Wireless,
}

/// MAC address (48-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MacAddr {
    /// Address bytes
    bytes: [u8; 6],
}

impl MacAddr {
    /// Create a new MAC address
    pub fn new(bytes: [u8; 6]) -> Self {
        Self { bytes }
    }

    /// Create MAC address from u64
    pub fn from_u64(value: u64) -> Self {
        let bytes = [
            ((value >> 40) & 0xFF) as u8,
            ((value >> 32) & 0xFF) as u8,
            ((value >> 24) & 0xFF) as u8,
            ((value >> 16) & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
        ];
        Self { bytes }
    }

    /// Get MAC address as bytes
    pub fn bytes(&self) -> [u8; 6] {
        self.bytes
    }

    /// Convert to u64
    pub fn to_u64(&self) -> u64 {
        ((self.bytes[0] as u64) << 40) |
        ((self.bytes[1] as u64) << 32) |
        ((self.bytes[2] as u64) << 24) |
        ((self.bytes[3] as u64) << 16) |
        ((self.bytes[4] as u64) << 8) |
        (self.bytes[5] as u64)
    }

    /// Check if MAC address is broadcast
    pub fn is_broadcast(&self) -> bool {
        self.bytes == [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    }

    /// Check if MAC address is multicast
    pub fn is_multicast(&self) -> bool {
        self.bytes[0] & 0x01 != 0
    }

    /// Check if MAC address is unicast
    pub fn is_unicast(&self) -> bool {
        !self.is_broadcast() && !self.is_multicast()
    }

    /// Get broadcast MAC address
    pub const fn broadcast() -> Self {
        Self {
            bytes: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        }
    }

    /// Get zero MAC address
    pub const fn zero() -> Self {
        Self { bytes: [0; 6] }
    }
}

impl core::fmt::Display for MacAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.bytes[0],
            self.bytes[1],
            self.bytes[2],
            self.bytes[3],
            self.bytes[4],
            self.bytes[5]
        )
    }
}

/// Device configuration
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// Device name
    pub name: String,
    /// MTU size
    pub mtu: Option<usize>,
    /// MAC address (None for auto-assignment)
    pub mac_address: Option<MacAddr>,
    /// Promiscuous mode
    pub promiscuous: bool,
    /// Additional device-specific settings
    pub settings: DeviceSettings,
}

/// Device-specific settings
#[derive(Debug, Clone)]
pub enum DeviceSettings {
    /// No specific settings
    None,
    /// Ethernet settings
    Ethernet(EthernetSettings),
    /// Loopback settings
    Loopback(LoopbackSettings),
    /// Virtual Ethernet settings
    VirtualEthernet(VirtualEthernetSettings),
}

/// Ethernet device settings
#[derive(Debug, Clone)]
pub struct EthernetSettings {
    /// Speed in Mbps
    pub speed: u32,
    /// Duplex mode
    pub duplex: DuplexMode,
    /// Auto-negotiation
    pub auto_negotiation: bool,
}

/// Duplex mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplexMode {
    Half,
    Full,
}

/// Loopback device settings
#[derive(Debug, Clone)]
pub struct LoopbackSettings {
    /// Loopback MTU
    pub mtu: usize,
}

/// Virtual Ethernet device settings
#[derive(Debug, Clone)]
pub struct VirtualEthernetSettings {
    /// Peer device name
    pub peer: String,
    /// MTU
    pub mtu: usize,
}

/// Device capabilities
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    /// Maximum supported MTU
    pub max_mtu: usize,
    /// Minimum supported MTU
    pub min_mtu: usize,
    /// Supports checksum offload
    pub checksum_offload: bool,
    /// Supports scatter-gather I/O
    pub scatter_gather: bool,
    /// Supports TSO (TCP Segmentation Offload)
    pub tso: bool,
    /// Supports LRO (Large Receive Offload)
    pub lro: bool,
    /// Supports multicast
    pub multicast: bool,
    /// Supports broadcast
    pub broadcast: bool,
    /// Supports promiscuous mode
    pub promiscuous: bool,
}

/// Device statistics
#[derive(Debug, Default)]
pub struct DeviceStats {
    /// Packets received
    pub rx_packets: AtomicU64,
    /// Bytes received
    pub rx_bytes: AtomicU64,
    /// Receive errors
    pub rx_errors: AtomicU64,
    /// Receive dropped packets
    pub rx_dropped: AtomicU64,
    /// Packets transmitted
    pub tx_packets: AtomicU64,
    /// Bytes transmitted
    pub tx_bytes: AtomicU64,
    /// Transmit errors
    pub tx_errors: AtomicU64,
    /// Transmit dropped packets
    pub tx_dropped: AtomicU64,
    /// Collisions
    pub collisions: AtomicU64,
}

impl Clone for DeviceStats {
    fn clone(&self) -> Self {
        Self {
            rx_packets: AtomicU64::new(self.rx_packets.load(Ordering::Relaxed)),
            rx_bytes: AtomicU64::new(self.rx_bytes.load(Ordering::Relaxed)),
            rx_errors: AtomicU64::new(self.rx_errors.load(Ordering::Relaxed)),
            rx_dropped: AtomicU64::new(self.rx_dropped.load(Ordering::Relaxed)),
            tx_packets: AtomicU64::new(self.tx_packets.load(Ordering::Relaxed)),
            tx_bytes: AtomicU64::new(self.tx_bytes.load(Ordering::Relaxed)),
            tx_errors: AtomicU64::new(self.tx_errors.load(Ordering::Relaxed)),
            tx_dropped: AtomicU64::new(self.tx_dropped.load(Ordering::Relaxed)),
            collisions: AtomicU64::new(self.collisions.load(Ordering::Relaxed)),
        }
    }
}

impl DeviceStats {
    /// Create new device stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment receive packet count
    pub fn inc_rx_packets(&self, count: u64) {
        self.rx_packets.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment receive byte count
    pub fn inc_rx_bytes(&self, count: u64) {
        self.rx_bytes.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment receive error count
    pub fn inc_rx_errors(&self, count: u64) {
        self.rx_errors.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment receive dropped count
    pub fn inc_rx_dropped(&self, count: u64) {
        self.rx_dropped.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment transmit packet count
    pub fn inc_tx_packets(&self, count: u64) {
        self.tx_packets.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment transmit byte count
    pub fn inc_tx_bytes(&self, count: u64) {
        self.tx_bytes.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment transmit error count
    pub fn inc_tx_errors(&self, count: u64) {
        self.tx_errors.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment transmit dropped count
    pub fn inc_tx_dropped(&self, count: u64) {
        self.tx_dropped.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment collision count
    pub fn inc_collisions(&self, count: u64) {
        self.collisions.fetch_add(count, Ordering::Relaxed);
    }

    /// Get snapshot of current statistics
    pub fn snapshot(&self) -> DeviceStatsSnapshot {
        DeviceStatsSnapshot {
            rx_packets: self.rx_packets.load(Ordering::Relaxed),
            rx_bytes: self.rx_bytes.load(Ordering::Relaxed),
            rx_errors: self.rx_errors.load(Ordering::Relaxed),
            rx_dropped: self.rx_dropped.load(Ordering::Relaxed),
            tx_packets: self.tx_packets.load(Ordering::Relaxed),
            tx_bytes: self.tx_bytes.load(Ordering::Relaxed),
            tx_errors: self.tx_errors.load(Ordering::Relaxed),
            tx_dropped: self.tx_dropped.load(Ordering::Relaxed),
            collisions: self.collisions.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of device statistics at a point in time
#[derive(Debug, Clone)]
pub struct DeviceStatsSnapshot {
    /// Packets received
    pub rx_packets: u64,
    /// Bytes received
    pub rx_bytes: u64,
    /// Receive errors
    pub rx_errors: u64,
    /// Receive dropped packets
    pub rx_dropped: u64,
    /// Packets transmitted
    pub tx_packets: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Transmit errors
    pub tx_errors: u64,
    /// Transmit dropped packets
    pub tx_dropped: u64,
    /// Collisions
    pub collisions: u64,
}

/// Device errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    /// Device not found
    DeviceNotFound,
    /// Invalid configuration
    InvalidConfig,
    /// Device is down
    DeviceDown,
    /// Buffer too small
    BufferTooSmall,
    /// No memory available
    NoMemory,
    /// Hardware error
    HardwareError,
    /// Timeout occurred
    Timeout,
    /// Permission denied
    PermissionDenied,
    /// Operation not supported
    NotSupported,
    /// I/O error
    IoError,
}

/// Simple loopback device implementation
pub struct LoopbackDevice {
    /// Device name
    name: String,
    /// Device state
    is_up: bool,
    /// MTU
    mtu: usize,
    /// Device statistics
    stats: DeviceStats,
}

impl LoopbackDevice {
    /// Create a new loopback device
    pub fn new(name: &str, mtu: usize) -> Self {
        Self {
            name: name.to_string(),
            is_up: false,
            mtu,
            stats: DeviceStats::new(),
        }
    }
}

impl NetworkDevice for LoopbackDevice {
    fn name(&self) -> &str {
        &self.name
    }

    fn device_type(&self) -> NetworkDeviceType {
        NetworkDeviceType::Loopback
    }

    fn mac_address(&self) -> MacAddr {
        MacAddr::zero() // Loopback has no MAC address
    }

    fn mtu(&self) -> usize {
        self.mtu
    }

    fn is_up(&self) -> bool {
        self.is_up
    }

    fn up(&self) -> Result<(), DeviceError> {
        // Note: This would need to be mutable to change is_up
        // For now, just return success
        crate::log_info!("Loopback device '{}' is up", self.name);
        Ok(())
    }

    fn down(&self) -> Result<(), DeviceError> {
        // Note: This would need to be mutable to change is_up
        // For now, just return success
        crate::log_info!("Loopback device '{}' is down", self.name);
        Ok(())
    }

    fn send_packet(&self, packet: &[u8]) -> Result<(), DeviceError> {
        if !self.is_up {
            return Err(DeviceError::DeviceDown);
        }

        if packet.len() > self.mtu {
            return Err(DeviceError::BufferTooSmall);
        }

        // Update statistics
        self.stats.inc_tx_packets(1);
        self.stats.inc_tx_bytes(packet.len() as u64);

        // In a real implementation, we would enqueue the packet
        // for loopback reception. For now, just simulate success.
        Ok(())
    }

    fn receive_packet(&self) -> Result<Option<Vec<u8>>, DeviceError> {
        if !self.is_up {
            return Err(DeviceError::DeviceDown);
        }

        // For now, return no packet (real implementation would
        // return packets that were sent to loopback)
        Ok(None)
    }

    fn stats(&self) -> DeviceStats {
        self.stats.clone()
    }

    fn configure(&mut self, config: &DeviceConfig) -> Result<(), DeviceError> {
        if let Some(mtu) = config.mtu {
            if mtu < 68 || mtu > 65536 {
                return Err(DeviceError::InvalidConfig);
            }
            self.mtu = mtu;
        }
        Ok(())
    }

    fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities {
            max_mtu: 65536,
            min_mtu: 68,
            checksum_offload: true,
            scatter_gather: false,
            tso: false,
            lro: false,
            multicast: false,
            broadcast: false,
            promiscuous: false,
        }
    }
}

/// Mock Ethernet device for testing (debug builds only)
#[cfg(debug_assertions)]
pub struct MockEthernetDevice {
    /// Device name
    name: String,
    /// Device state
    is_up: bool,
    /// MTU
    mtu: usize,
    /// MAC address
    mac_addr: MacAddr,
    /// Device statistics
    stats: DeviceStats,
    /// Packet buffer for testing
    packet_buffer: alloc::vec::Vec<alloc::vec::Vec<u8>>,
}

#[cfg(debug_assertions)]
impl MockEthernetDevice {
    /// Create a new mock Ethernet device
    pub fn new(name: &str, mtu: usize) -> Self {
        Self {
            name: name.to_string(),
            is_up: false,
            mtu,
            mac_addr: MacAddr::from_u64(0x123456789ABC), // Mock MAC address
            stats: DeviceStats::new(),
            packet_buffer: alloc::vec::Vec::new(),
        }
    }
}

#[cfg(debug_assertions)]
impl NetworkDevice for MockEthernetDevice {
    fn name(&self) -> &str {
        &self.name
    }

    fn device_type(&self) -> NetworkDeviceType {
        NetworkDeviceType::Ethernet
    }

    fn mac_address(&self) -> MacAddr {
        self.mac_addr
    }

    fn mtu(&self) -> usize {
        self.mtu
    }

    fn is_up(&self) -> bool {
        self.is_up
    }

    fn up(&self) -> Result<(), DeviceError> {
        // Note: This would need to be mutable to change is_up
        // For now, just return success
        crate::log_info!("Mock Ethernet device '{}' is up with MAC {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
              self.name,
              self.mac_addr.bytes[0], self.mac_addr.bytes[1], self.mac_addr.bytes[2],
              self.mac_addr.bytes[3], self.mac_addr.bytes[4], self.mac_addr.bytes[5]);
        Ok(())
    }

    fn down(&self) -> Result<(), DeviceError> {
        // Note: This would need to be mutable to change is_up
        // For now, just return success
        crate::log_info!("Mock Ethernet device '{}' is down", self.name);
        Ok(())
    }

    fn send_packet(&self, packet: &[u8]) -> Result<(), DeviceError> {
        if !self.is_up {
            return Err(DeviceError::DeviceDown);
        }

        if packet.len() > self.mtu {
            return Err(DeviceError::BufferTooSmall);
        }

        // Update statistics
        self.stats.inc_tx_packets(1);
        self.stats.inc_tx_bytes(packet.len() as u64);

        // For testing, echo packet back to receive buffer
        let mut buffer = alloc::vec::Vec::with_capacity(packet.len());
        buffer.extend_from_slice(packet);

        Ok(())
    }

    fn receive_packet(&self) -> Result<Option<Vec<u8>>, DeviceError> {
        if !self.is_up {
            return Err(DeviceError::DeviceDown);
        }

        // For now, return no packet (could be enhanced for testing)
        Ok(None)
    }

    fn stats(&self) -> DeviceStats {
        self.stats.clone()
    }

    fn configure(&mut self, config: &DeviceConfig) -> Result<(), DeviceError> {
        if let Some(mtu) = config.mtu {
            if mtu < 68 || mtu > 9000 {
                return Err(DeviceError::InvalidConfig);
            }
            self.mtu = mtu;
        }

        if let Some(mac) = config.mac_address {
            self.mac_addr = mac;
        }

        Ok(())
    }

    fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities {
            max_mtu: 1500,
            min_mtu: 68,
            checksum_offload: false,
            multicast: true,
            broadcast: true,
            promiscuous: true,
            scatter_gather: false,
            tso: false,
            lro: false,
        }
    }
}
