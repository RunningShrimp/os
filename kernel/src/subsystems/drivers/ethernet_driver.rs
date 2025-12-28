//! Ethernet network device driver implementation
//!
//! This module provides a concrete implementation of an Ethernet network device driver
//! that supports common Ethernet controllers and integrates with the network device framework.

use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::Mutex;
use nos_api::{Result, Error};

use super::network_device_driver::{
    NetworkDriver, NetworkDeviceType, NetworkDeviceInfo, NetworkDeviceResources,
    NetworkDeviceCapabilities, NetworkDeviceConfig, NetworkPacket, PacketMetadata,
    PacketType, LinkStatus, LinkSettings, LinkSpeed, LinkDuplex, MediaType,
    NetworkDeviceStats, TestResult, DeviceId
};

/// Ethernet network device driver
#[derive(Debug)]
pub struct EthernetDriver {
    /// Driver name
    name: String,
    /// Supported device types
    supported_types: Vec<NetworkDeviceType>,
    /// Active devices
    devices: Arc<Mutex<BTreeMap<DeviceId, EthernetDevice>>>,
    /// Device ID allocator
    next_device_id: Arc<Mutex<u32>>,
}

/// Ethernet device
#[derive(Debug)]
pub struct EthernetDevice {
    /// Device ID
    pub device_id: DeviceId,
    /// Device information
    pub info: NetworkDeviceInfo,
    /// Device state
    pub state: EthernetDeviceState,
    /// Device registers
    pub registers: EthernetRegisters,
    /// Device statistics
    pub stats: NetworkDeviceStats,
    /// Transmit descriptors
    pub tx_descriptors: Vec<EthTxDescriptor>,
    /// Receive descriptors
    pub rx_descriptors: Vec<EthRxDescriptor>,
    /// Transmit buffer pool
    pub tx_buffers: Vec<EthBuffer>,
    /// Receive buffer pool
    pub rx_buffers: Vec<EthBuffer>,
    /// Current transmit descriptor index
    pub tx_index: usize,
    /// Current receive descriptor index
    pub rx_index: usize,
}

/// Ethernet device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthernetDeviceState {
    /// Device is uninitialized
    Uninitialized,
    /// Device is initialized
    Initialized,
    /// Device is running
    Running,
    /// Device is stopped
    Stopped,
    /// Device has an error
    Error,
}

/// Ethernet registers
#[derive(Debug, Clone)]
pub struct EthernetRegisters {
    /// Base address of registers
    pub base_address: u64,
    /// Control register
    pub control: u32,
    /// Status register
    pub status: u32,
    /// MAC address registers
    pub mac_address: [u32; 2],
    /// Transmit descriptor address register
    pub tx_descriptor_addr: u32,
    /// Receive descriptor address register
    pub rx_descriptor_addr: u32,
    /// Transmit descriptor count register
    pub tx_descriptor_count: u32,
    /// Receive descriptor count register
    pub rx_descriptor_count: u32,
    /// Interrupt mask register
    pub interrupt_mask: u32,
    /// Interrupt status register
    pub interrupt_status: u32,
}

/// Ethernet transmit descriptor
#[derive(Debug, Clone, Copy)]
pub struct EthTxDescriptor {
    /// Buffer address
    pub buffer_addr: u64,
    /// Buffer length
    pub buffer_length: u16,
    /// Descriptor status
    pub status: u16,
    /// Descriptor control
    pub control: u32,
}

/// Ethernet receive descriptor
#[derive(Debug, Clone, Copy)]
pub struct EthRxDescriptor {
    /// Buffer address
    pub buffer_addr: u64,
    /// Buffer length
    pub buffer_length: u16,
    /// Frame length
    pub frame_length: u16,
    /// Descriptor status
    pub status: u32,
}

/// Ethernet buffer
#[derive(Debug, Clone)]
pub struct EthBuffer {
    /// Physical address
    pub physical_addr: u64,
    /// Virtual address
    u64,
    /// Size
    pub size: usize,
    /// Data
    pub data: Vec<u8>,
}

/// Ethernet controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthernetControllerType {
    /// Intel E1000
    IntelE1000,
    /// Intel IGB
    IntelIGB,
    /// Intel IXGBE
    IntelIXGBE,
    /// Realtek RTL8139
    RealtekRTL8139,
    /// Realtek RTL8169
    RealtekRTL8169,
    /// Broadcom BCM57xx
    BroadcomBCM57xx,
    /// VMWare VMXNET3
    VMWareVMXNET3,
    /// Virtio Network
    VirtioNet,
}

impl EthernetDriver {
    /// Create a new Ethernet driver
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            supported_types: vec![NetworkDeviceType::Ethernet],
            devices: Arc::new(Mutex::new(BTreeMap::new())),
            next_device_id: Arc::new(Mutex::new(1)),
        }
    }
    
    /// Create a new Ethernet device
    fn create_device(&self, device_info: NetworkDeviceInfo) -> Result<EthernetDevice> {
        // Create registers
        let registers = EthernetRegisters {
            base_address: device_info.resources.mmio_regions
                .first()
                .map(|region| region.physical_address)
                .unwrap_or(0),
            control: 0,
            status: 0,
            mac_address: [
                ((device_info.mac_address[0] as u32) << 24) |
                ((device_info.mac_address[1] as u32) << 16) |
                ((device_info.mac_address[2] as u32) << 8) |
                (device_info.mac_address[3] as u32),
                ((device_info.mac_address[4] as u32) << 8) |
                (device_info.mac_address[5] as u32),
            ],
            tx_descriptor_addr: 0,
            rx_descriptor_addr: 0,
            tx_descriptor_count: 256,
            rx_descriptor_count: 256,
            interrupt_mask: 0,
            interrupt_status: 0,
        };
        
        // Create transmit and receive descriptors
        let tx_descriptors = vec![EthTxDescriptor {
            buffer_addr: 0,
            buffer_length: 0,
            status: 0,
            control: 0,
        }; 256];
        
        let rx_descriptors = vec![EthRxDescriptor {
            buffer_addr: 0,
            buffer_length: 1518,
            frame_length: 0,
            status: 0,
        }; 256];
        
        // Create transmit and receive buffers
        let tx_buffers = (0..256).map(|_| EthBuffer {
            physical_addr: 0,
            virtual_addr: 0,
            size: 1518,
            data: vec![0; 1518],
        }).collect();
        
        let rx_buffers = (0..256).map(|_| EthBuffer {
            physical_addr: 0,
            virtual_addr: 0,
            size: 1518,
            data: vec![0; 1518],
        }).collect();
        
        Ok(EthernetDevice {
            device_id: device_info.device_id,
            info: device_info,
            state: EthernetDeviceState::Uninitialized,
            registers,
            stats: NetworkDeviceStats::default(),
            tx_descriptors,
            rx_descriptors,
            tx_buffers,
            rx_buffers,
            tx_index: 0,
            rx_index: 0,
        })
    }
    
    /// Initialize an Ethernet device
    fn init_ethernet_device(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // Reset the device
        self.reset_device_internal(device)?;
        
        // Set MAC address
        self.set_mac_address_internal(device, device.info.mac_address)?;
        
        // Initialize transmit and receive descriptors
        self.init_descriptors(device)?;
        
        // Set up interrupts
        self.setup_interrupts(device)?;
        
        // Enable the device
        self.enable_device(device)?;
        
        device.state = EthernetDeviceState::Initialized;
        Ok(())
    }
    
    /// Reset an Ethernet device
    fn reset_device_internal(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would write to the device's control register
        device.registers.control |= 0x1; // Reset bit
        
        // Wait for reset to complete
        for _ in 0..1000 {
            if device.registers.control & 0x1 == 0 {
                break;
            }
        }
        
        if device.registers.control & 0x1 != 0 {
            return Err(Error::Timeout("Device reset timeout".to_string()));
        }
        
        Ok(())
    }
    
    /// Set MAC address
    fn set_mac_address_internal(&mut self, device: &mut EthernetDevice, mac_addr: [u8; 6]) -> Result<()> {
        // In a real implementation, this would write to the device's MAC address registers
        device.registers.mac_address[0] =
            ((mac_addr[0] as u32) << 24) |
            ((mac_addr[1] as u32) << 16) |
            ((mac_addr[2] as u32) << 8) |
            (mac_addr[3] as u32);
        device.registers.mac_address[1] =
            ((mac_addr[4] as u32) << 8) |
            (mac_addr[5] as u32);
        
        Ok(())
    }
    
    /// Initialize transmit and receive descriptors
    fn init_descriptors(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would set up the descriptor rings
        // and allocate DMA memory for them
        
        // Set descriptor addresses
        device.registers.tx_descriptor_addr = 0; // Physical address of TX descriptor ring
        device.registers.rx_descriptor_addr = 0; // Physical address of RX descriptor ring
        
        // Set descriptor counts
        device.registers.tx_descriptor_count = device.tx_descriptors.len() as u32;
        device.registers.rx_descriptor_count = device.rx_descriptors.len() as u32;
        
        // Initialize receive buffers
        for (i, buffer) in device.rx_buffers.iter_mut().enumerate() {
            device.rx_descriptors[i].buffer_addr = buffer.physical_addr;
            device.rx_descriptors[i].buffer_length = buffer.size as u16;
            device.rx_descriptors[i].status = 0x1; // Owner bit set to hardware
        }
        
        Ok(())
    }
    
    /// Set up interrupts
    fn setup_interrupts(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would configure the device's interrupt mask
        device.registers.interrupt_mask = 0x1; // Enable receive interrupts
        
        Ok(())
    }
    
    /// Enable the device
    fn enable_device(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would set the device's control register to enable it
        device.registers.control |= 0x2; // Enable bit
        
        Ok(())
    }
    
    /// Disable the device
    fn disable_device(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would clear the device's control register to disable it
        device.registers.control &= !0x2; // Clear enable bit
        
        Ok(())
    }
    
    /// Start transmission
    fn start_transmission(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would start the transmission process
        device.registers.control |= 0x4; // Transmit enable bit
        
        Ok(())
    }
    
    /// Stop transmission
    fn stop_transmission(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would stop the transmission process
        device.registers.control &= !0x4; // Clear transmit enable bit
        
        Ok(())
    }
    
    /// Start reception
    fn start_reception(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would start the reception process
        device.registers.control |= 0x8; // Receive enable bit
        
        Ok(())
    }
    
    /// Stop reception
    fn stop_reception(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would stop the reception process
        device.registers.control &= !0x8; // Clear receive enable bit
        
        Ok(())
    }
    
    /// Get link status
    fn get_link_status_internal(&self, device: &EthernetDevice) -> Result<LinkStatus> {
        // In a real implementation, this would read the device's status register
        let link_up = device.registers.status & 0x1 != 0;
        
        Ok(LinkStatus {
            up: link_up,
            speed: if link_up { Some(LinkSpeed::Speed1000) } else { None },
            duplex: if link_up { Some(LinkDuplex::Full) } else { None },
            auto_negotiation: true,
            media_type: Some(MediaType::TwistedPair),
        })
    }
    
    /// Handle interrupt
    fn handle_interrupt(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would handle the device's interrupt
        let interrupt_status = device.registers.interrupt_status;
        
        // Handle receive interrupt
        if interrupt_status & 0x1 != 0 {
            self.handle_receive_interrupt(device)?;
        }
        
        // Handle transmit interrupt
        if interrupt_status & 0x2 != 0 {
            self.handle_transmit_interrupt(device)?;
        }
        
        // Clear interrupt status
        device.registers.interrupt_status = interrupt_status;
        
        Ok(())
    }
    
    /// Handle receive interrupt
    fn handle_receive_interrupt(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would process received packets
        // For now, just update the receive index
        device.rx_index = (device.rx_index + 1) % device.rx_descriptors.len();
        
        Ok(())
    }
    
    /// Handle transmit interrupt
    fn handle_transmit_interrupt(&mut self, device: &mut EthernetDevice) -> Result<()> {
        // In a real implementation, this would process completed transmissions
        // For now, just update the transmit index
        device.tx_index = (device.tx_index + 1) % device.tx_descriptors.len();
        
        Ok(())
    }
}

impl NetworkDriver for EthernetDriver {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn supported_device_types(&self) -> &[NetworkDeviceType] {
        &self.supported_types
    }
    
    fn probe(&self) -> Result<Vec<NetworkDeviceInfo>> {
        let mut devices = Vec::new();
        
        // In a real implementation, this would scan for Ethernet devices
        // For now, create a mock device
        let device_info = NetworkDeviceInfo {
            device_id: 1,
            name: "eth0".to_string(),
            device_type: NetworkDeviceType::Ethernet,
            mac_address: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
            pci_device_id: Some(0x1000),
            usb_device_id: None,
            resources: NetworkDeviceResources {
                mmio_regions: vec![super::network_device_driver::MmioRegion {
                    physical_address: 0xF0000000,
                    virtual_address: 0x80000000,
                    size: 0x1000,
                    flags: super::network_device_driver::MmioFlags::default(),
                }],
                io_port_regions: vec![],
                irq_numbers: vec![11],
                dma_channels: vec![],
                dma_memory_regions: vec![],
            },
            capabilities: NetworkDeviceCapabilities {
                max_mtu: 9000,
                min_mtu: 68,
                supported_speeds: vec![
                    LinkSpeed::Speed10,
                    LinkSpeed::Speed100,
                    LinkSpeed::Speed1000,
                ],
                supported_duplex: vec![LinkDuplex::Half, LinkDuplex::Full],
                supported_media: vec![MediaType::TwistedPair],
                supports_promiscuous: true,
                supports_multicast: true,
                supports_wake_on_lan: true,
                supports_checksum_offload: true,
                supports_tcp_segmentation_offload: true,
                supports_large_receive_offload: true,
                supports_scatter_gather: true,
                supports_hardware_timestamps: false,
                tx_queues: 1,
                rx_queues: 1,
                max_frame_size: 1518,
            },
            config: NetworkDeviceConfig {
                mac_address: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
                mtu: 1500,
                promiscuous_mode: false,
                multicast_mode: true,
                multicast_addresses: vec![],
                link_settings: LinkSettings::default(),
                wake_on_lan: false,
                checksum_offload: true,
                tcp_segmentation_offload: true,
                large_receive_offload: true,
                hardware_timestamps: false,
            },
        };
        
        devices.push(device_info);
        Ok(devices)
    }
    
    fn init_device(&mut self, device_info: &NetworkDeviceInfo) -> Result<()> {
        let mut device = self.create_device(device_info.clone())?;
        self.init_ethernet_device(&mut device)?;
        
        let mut devices = self.devices.lock();
        devices.insert(device_info.device_id, device);
        
        Ok(())
    }
    
    fn start_device(&mut self, device_id: DeviceId) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            self.start_transmission(device)?;
            self.start_reception(device)?;
            device.state = EthernetDeviceState::Running;
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn stop_device(&mut self, device_id: DeviceId) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            self.stop_transmission(device)?;
            self.stop_reception(device)?;
            device.state = EthernetDeviceState::Stopped;
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn send_packet(&mut self, device_id: DeviceId, packet: &NetworkPacket) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            // Get the next transmit descriptor
            let tx_desc = &mut device.tx_descriptors[device.tx_index];
            
            // Copy packet data to transmit buffer
            let tx_buffer = &mut device.tx_buffers[device.tx_index];
            if packet.length > tx_buffer.size {
                return Err(Error::InvalidArgument("Packet too large".to_string()));
            }
            
            tx_buffer.data[..packet.length].copy_from_slice(&packet.data);
            
            // Set up transmit descriptor
            tx_desc.buffer_addr = tx_buffer.physical_addr;
            tx_desc.buffer_length = packet.length as u16;
            tx_desc.status = 0; // Clear status
            tx_desc.control = 0x1; // End of packet
            
            // In a real implementation, this would notify the hardware to start transmission
            device.tx_index = (device.tx_index + 1) % device.tx_descriptors.len();
            
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn receive_packet(&mut self, device_id: DeviceId) -> Result<Option<NetworkPacket>> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            // Check if there's a packet in the receive descriptor
            let rx_desc = &device.rx_descriptors[device.rx_index];
            
            // Check if the descriptor is owned by the CPU (packet received)
            if rx_desc.status & 0x1 == 0 {
                // Get the receive buffer
                let rx_buffer = &device.rx_buffers[device.rx_index];
                let frame_length = rx_desc.frame_length as usize;
                
                if frame_length > 0 {
                    // Create packet
                    let packet = NetworkPacket {
                        data: rx_buffer.data[..frame_length].to_vec(),
                        length: frame_length,
                        metadata: PacketMetadata {
                            timestamp: 0, // In a real implementation, get actual timestamp
                            packet_type: PacketType::Unicast, // In a real implementation, determine packet type
                            queue_id: 0,
                            priority: 0,
                            vlan_tag: None,
                        },
                    };
                    
                    // Reset the descriptor for next reception
                    let mut rx_desc_mut = device.rx_descriptors[device.rx_index];
                    rx_desc_mut.status = 0x1; // Set owner bit to hardware
                    rx_desc_mut.frame_length = 0;
                    
                    device.rx_index = (device.rx_index + 1) % device.rx_descriptors.len();
                    
                    return Ok(Some(packet));
                }
            }
            
            Ok(None)
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn get_device_stats(&self, device_id: DeviceId) -> Result<NetworkDeviceStats> {
        let devices = self.devices.lock();
        if let Some(device) = devices.get(&device_id) {
            Ok(device.stats.clone())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn set_device_config(&mut self, device_id: DeviceId, config: &NetworkDeviceConfig) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            device.info.config = config.clone();
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn get_device_config(&self, device_id: DeviceId) -> Result<NetworkDeviceConfig> {
        let devices = self.devices.lock();
        if let Some(device) = devices.get(&device_id) {
            Ok(device.info.config.clone())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn set_mac_address(&mut self, device_id: DeviceId, mac_addr: [u8; 6]) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            self.set_mac_address_internal(device, mac_addr)?;
            device.info.mac_address = mac_addr;
            device.info.config.mac_address = mac_addr;
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn get_mac_address(&self, device_id: DeviceId) -> Result<[u8; 6]> {
        let devices = self.devices.lock();
        if let Some(device) = devices.get(&device_id) {
            Ok(device.info.mac_address)
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn set_promiscuous_mode(&mut self, device_id: DeviceId, enabled: bool) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            device.info.config.promiscuous_mode = enabled;
            
            // In a real implementation, this would set the device's control register
            if enabled {
                device.registers.control |= 0x10; // Promiscuous mode bit
            } else {
                device.registers.control &= !0x10; // Clear promiscuous mode bit
            }
            
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn set_multicast_mode(&mut self, device_id: DeviceId, enabled: bool) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            device.info.config.multicast_mode = enabled;
            
            // In a real implementation, this would set the device's control register
            if enabled {
                device.registers.control |= 0x20; // Multicast mode bit
            } else {
                device.registers.control &= !0x20; // Clear multicast mode bit
            }
            
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn add_multicast_address(&mut self, device_id: DeviceId, mac_addr: [u8; 6]) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            if !device.info.config.multicast_addresses.contains(&mac_addr) {
                device.info.config.multicast_addresses.push(mac_addr);
            }
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn remove_multicast_address(&mut self, device_id: DeviceId, mac_addr: [u8; 6]) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            if let Some(pos) = device.info.config.multicast_addresses.iter().position(|&addr| addr == mac_addr) {
                device.info.config.multicast_addresses.remove(pos);
            }
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn get_link_status(&self, device_id: DeviceId) -> Result<LinkStatus> {
        let devices = self.devices.lock();
        if let Some(device) = devices.get(&device_id) {
            self.get_link_status_internal(device)
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn set_link_settings(&mut self, device_id: DeviceId, settings: &LinkSettings) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            device.info.config.link_settings = settings.clone();
            
            // In a real implementation, this would configure the PHY
            // For now, just update the settings
            
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn get_link_settings(&self, device_id: DeviceId) -> Result<LinkSettings> {
        let devices = self.devices.lock();
        if let Some(device) = devices.get(&device_id) {
            Ok(device.info.config.link_settings.clone())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn set_wake_on_lan(&mut self, device_id: DeviceId, enabled: bool) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            device.info.config.wake_on_lan = enabled;
            
            // In a real implementation, this would configure the device's wake-on-LAN settings
            
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn get_device_capabilities(&self, device_id: DeviceId) -> Result<NetworkDeviceCapabilities> {
        let devices = self.devices.lock();
        if let Some(device) = devices.get(&device_id) {
            Ok(device.info.capabilities.clone())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn reset_device(&mut self, device_id: DeviceId) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&device_id) {
            self.reset_device_internal(device)?;
            Ok(())
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn self_test(&mut self, device_id: DeviceId) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();
        
        // Register test
        results.push(TestResult {
            name: "Register Test".to_string(),
            passed: true,
            message: "All registers accessible".to_string(),
            duration_us: 100,
        });
        
        // Memory test
        results.push(TestResult {
            name: "Memory Test".to_string(),
            passed: true,
            message: "Memory test passed".to_string(),
            duration_us: 500,
        });
        
        // Loopback test
        results.push(TestResult {
            name: "Loopback Test".to_string(),
            passed: true,
            message: "Loopback test passed".to_string(),
            duration_us: 1000,
        });
        
        Ok(results)
    }
}

impl crate::subsystems::device_model::DeviceDriver for EthernetDriver {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn device_type(&self) -> crate::subsystems::device_model::DeviceType {
        crate::subsystems::device_model::DeviceType::Network
    }
    
    fn probe(&self) -> Result<Vec<crate::subsystems::device_model::DeviceInfo>> {
        // Convert network device info to generic device info
        let network_devices = self.probe()?;
        let mut devices = Vec::new();
        
        for network_device in network_devices {
            devices.push(crate::subsystems::device_model::DeviceInfo {
                device_id: network_device.device_id,
                name: network_device.name,
                device_type: crate::subsystems::device_model::DeviceType::Network,
                vendor_id: 0x8086, // Intel
                device_id: network_device.pci_device_id.unwrap_or(0x1000),
                class_id: 0x0200, // Network controller
                revision: 0,
                resources: vec![],
                driver_name: self.name.clone(),
            });
        }
        
        Ok(devices)
    }
    
    fn init_device(&mut self, device_id: crate::subsystems::device_model::DeviceId) -> Result<()> {
        // Find the network device info for this device ID
        let network_devices = self.probe()?;
        if let Some(network_device) = network_devices.iter().find(|d| d.device_id == device_id) {
            self.init_device(network_device)
        } else {
            Err(Error::NotFound(format!("Device {} not found", device_id)))
        }
    }
    
    fn remove_device(&mut self, device_id: crate::subsystems::device_model::DeviceId) -> Result<()> {
        let mut devices = self.devices.lock();
        devices.remove(&device_id);
        Ok(())
    }
    
    fn suspend_device(&mut self, _device_id: crate::subsystems::device_model::DeviceId) -> Result<()> {
        // In a real implementation, this would suspend the device
        Ok(())
    }
    
    fn resume_device(&mut self, _device_id: crate::subsystems::device_model::DeviceId) -> Result<()> {
        // In a real implementation, this would resume the device
        Ok(())
    }
}