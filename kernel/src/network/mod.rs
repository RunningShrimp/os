//! Network subsystem
//!
//! This module provides network functionality including zero-copy I/O optimizations.

pub mod zero_copy_io;

use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;
use nos_api::{Result, Error};

use zero_copy_io::{ZeroCopyNetworkManager, ZeroCopyConfig, ZeroCopySocket, SocketType};

/// Network interface manager
#[derive(Debug)]
pub struct NetworkManager {
    /// Zero-copy network manager
    pub zero_copy_manager: Arc<ZeroCopyNetworkManager>,
    /// Network interfaces
    pub interfaces: Arc<Mutex<Vec<NetworkInterface>>>,
    /// Routing table
    pub routing_table: Arc<Mutex<Vec<Route>>>,
    /// Network statistics
    pub stats: Arc<Mutex<NetworkStats>>,
}

/// Network interface
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    /// Interface name
    pub name: alloc::string::String,
    /// Interface index
    pub index: u32,
    /// MAC address
    pub mac_addr: [u8; 6],
    /// IP address
    pub ip_addr: u32,
    /// Netmask
    pub netmask: u32,
    /// Interface state
    pub state: InterfaceState,
    /// Interface type
    pub interface_type: InterfaceType,
    /// MTU (Maximum Transmission Unit)
    pub mtu: u16,
    /// Zero-copy enabled
    pub zero_copy_enabled: bool,
}

/// Interface state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    /// Interface is down
    Down,
    /// Interface is up
    Up,
    /// Interface is testing
    Testing,
}

/// Interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// Ethernet interface
    Ethernet,
    /// Wireless interface
    Wireless,
    /// Loopback interface
    Loopback,
    /// Virtual interface
    Virtual,
}

/// Routing table entry
#[derive(Debug, Clone)]
pub struct Route {
    /// Destination network
    pub destination: u32,
    /// Netmask
    pub netmask: u32,
    /// Gateway
    pub gateway: u32,
    /// Interface to use
    pub interface: u32,
    /// Route metric
    pub metric: u32,
    /// Route type
    pub route_type: RouteType,
}

/// Route type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteType {
    /// Direct route
    Direct,
    /// Indirect route
    Indirect,
    /// Default route
    Default,
}

/// Network statistics
#[derive(Debug, Default, Clone)]
pub struct NetworkStats {
    /// Total bytes received
    pub bytes_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Total packets sent
    pub packets_sent: u64,
    /// Packets dropped due to errors
    pub packets_dropped: u64,
    /// Packets with errors
    pub packet_errors: u64,
    /// Collisions detected
    pub collisions: u64,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new() -> Result<Self> {
        let zero_copy_config = ZeroCopyConfig::default();
        let zero_copy_manager = Arc::new(ZeroCopyNetworkManager::new(zero_copy_config)?);
        
        Ok(Self {
            zero_copy_manager,
            interfaces: Arc::new(Mutex::new(Vec::new())),
            routing_table: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(NetworkStats::default())),
        })
    }
    
    /// Initialize the network subsystem
    pub fn initialize(&self) -> Result<()> {
        // Initialize network interfaces
        self.setup_interfaces()?;
        
        // Initialize routing table
        self.setup_routing_table()?;
        
        // Initialize zero-copy I/O
        self.zero_copy_manager.get_stats(); // Just to ensure it's initialized
        
        Ok(())
    }
    
    /// Setup network interfaces
    fn setup_interfaces(&self) -> Result<()> {
        let mut interfaces = self.interfaces.lock();
        
        // Add loopback interface
        interfaces.push(NetworkInterface {
            name: "lo".to_string(),
            index: 0,
            mac_addr: [0, 0, 0, 0, 0, 0],
            ip_addr: 0x7F000001, // 127.0.0.1
            netmask: 0xFF000000,  // 255.0.0.0
            state: InterfaceState::Up,
            interface_type: InterfaceType::Loopback,
            mtu: 65536,
            zero_copy_enabled: true,
        });
        
        // Add Ethernet interface (mock)
        interfaces.push(NetworkInterface {
            name: "eth0".to_string(),
            index: 1,
            mac_addr: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
            ip_addr: 0xC0A80101,   // 192.168.1.1
            netmask: 0xFFFFFF00,   // 255.255.255.0
            state: InterfaceState::Up,
            interface_type: InterfaceType::Ethernet,
            mtu: 1500,
            zero_copy_enabled: true,
        });
        
        Ok(())
    }
    
    /// Setup routing table
    fn setup_routing_table(&self) -> Result<()> {
        let mut routing_table = self.routing_table.lock();
        
        // Add loopback route
        routing_table.push(Route {
            destination: 0x7F000000,  // 127.0.0.0
            netmask: 0xFF000000,     // 255.0.0.0
            gateway: 0,
            interface: 0,
            metric: 0,
            route_type: RouteType::Direct,
        });
        
        // Add local network route
        routing_table.push(Route {
            destination: 0xC0A80100,  // 192.168.1.0
            netmask: 0xFFFFFF00,     // 255.255.255.0
            gateway: 0,
            interface: 1,
            metric: 0,
            route_type: RouteType::Direct,
        });
        
        // Add default route
        routing_table.push(Route {
            destination: 0,
            netmask: 0,
            gateway: 0xC0A80101,  // 192.168.1.1
            interface: 1,
            metric: 1,
            route_type: RouteType::Default,
        });
        
        Ok(())
    }
    
    /// Create a new socket
    pub fn create_socket(&self, socket_type: SocketType) -> Result<i32> {
        let config = ZeroCopyConfig::default();
        self.zero_copy_manager.create_socket(socket_type, config)
    }
    
    /// Send data using zero-copy if available
    pub fn send_zero_copy(&self, fd: i32, buffer: zero_copy_io::ZeroCopyBuffer) -> Result<usize> {
        self.zero_copy_manager.send_zero_copy(fd, buffer)
    }
    
    /// Receive data using zero-copy if available
    pub fn recv_zero_copy(&self, fd: i32) -> Result<zero_copy_io::ZeroCopyPacket> {
        self.zero_copy_manager.recv_zero_copy(fd)
    }
    
    /// Get network statistics
    pub fn get_stats(&self) -> NetworkStats {
        self.stats.lock().clone()
    }
    
    /// Get zero-copy statistics
    pub fn get_zero_copy_stats(&self) -> zero_copy_io::ZeroCopyStats {
        self.zero_copy_manager.get_stats()
    }
    
    /// Reset network statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = NetworkStats::default();
        self.zero_copy_manager.reset_stats();
    }
    
    /// Find the best route for a destination
    pub fn find_route(&self, destination: u32) -> Option<Route> {
        let routing_table = self.routing_table.lock();
        let mut best_route: Option<Route> = None;
        let mut best_metric = u32::MAX;
        
        for route in routing_table.iter() {
            // Check if destination matches route
            if (destination & route.netmask) == (route.destination & route.netmask) {
                if route.metric < best_metric {
                    best_metric = route.metric;
                    best_route = Some(route.clone());
                }
            }
        }
        
        best_route
    }
    
    /// Get interface by index
    pub fn get_interface_by_index(&self, index: u32) -> Option<NetworkInterface> {
        let interfaces = self.interfaces.lock();
        interfaces.iter().find(|iface| iface.index == index).cloned()
    }
    
    /// Get interface by name
    pub fn get_interface_by_name(&self, name: &str) -> Option<NetworkInterface> {
        let interfaces = self.interfaces.lock();
        interfaces.iter().find(|iface| iface.name == name).cloned()
    }
    
    /// Enable or disable zero-copy on an interface
    pub fn set_zero_copy_enabled(&self, index: u32, enabled: bool) -> Result<()> {
        let mut interfaces = self.interfaces.lock();
        if let Some(interface) = interfaces.iter_mut().find(|iface| iface.index == index) {
            interface.zero_copy_enabled = enabled;
            Ok(())
        } else {
            Err(Error::NotFound("Interface not found".to_string()))
        }
    }
}

/// Global network manager instance
static mut GLOBAL_NETWORK_MANAGER: Option<NetworkManager> = None;
static NETWORK_MANAGER_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Get the global network manager
pub fn get_network_manager() -> Result<&'static NetworkManager> {
    unsafe {
        if !NETWORK_MANAGER_INIT.load(core::sync::atomic::Ordering::Relaxed) {
            return Err(Error::InvalidState("Network manager not initialized".to_string()));
        }
        GLOBAL_NETWORK_MANAGER.as_ref().ok_or_else(|| {
            Error::InvalidState("Network manager not initialized".to_string())
        })
    }
}

/// Initialize the global network manager
pub fn init_network_manager() -> Result<()> {
    unsafe {
        if NETWORK_MANAGER_INIT.swap(true, core::sync::atomic::Ordering::Relaxed) {
            return Err(Error::InvalidState("Network manager already initialized".to_string()));
        }
        
        let manager = NetworkManager::new()?;
        manager.initialize()?;
        GLOBAL_NETWORK_MANAGER = Some(manager);
        
        Ok(())
    }
}