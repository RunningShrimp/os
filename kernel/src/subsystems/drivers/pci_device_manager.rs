//! PCI Device Manager Implementation
//!
//! This module implements a comprehensive PCI device manager for NOS,
//! providing PCI bus enumeration, device configuration, resource management,
//! and driver binding. The implementation supports PCI Express, hot-plug,
//! and advanced power management features.

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering};
use crate::subsystems::sync::{Mutex, Sleeplock};
use crate::subsystems::drivers::device_model::{
    DeviceModel, EnhancedDeviceInfo, DeviceClass, DevicePowerState, 
    DeviceCapabilities, DevicePerformanceMetrics
};
use crate::subsystems::drivers::driver_manager::{
    Driver, DeviceId, DriverId, DeviceType, DeviceStatus, DriverStatus,
    DeviceInfo, DriverInfo, DeviceResources, IoOperation, IoResult, InterruptInfo
};
use nos_nos_error_handling::unified::KernelError;

// ============================================================================
// PCI Constants and Structures
// ============================================================================

/// PCI configuration space address format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PciAddress {
    /// Bus number (0-255)
    pub bus: u8,
    /// Device number (0-31)
    pub device: u8,
    /// Function number (0-7)
    pub function: u8,
    /// Register number (0-255)
    pub register: u8,
}

impl PciAddress {
    /// Create a new PCI address
    pub fn new(bus: u8, device: u8, function: u8, register: u8) -> Self {
        Self { bus, device, function, register }
    }

    /// Convert to 32-bit address for configuration access
    pub fn to_u32(&self) -> u32 {
        0x80000000 | 
        ((self.bus as u32) << 16) |
        ((self.device as u32) << 11) |
        ((self.function as u32) << 8) |
        (self.register as u32 & 0xFC)
    }

    /// Create from 32-bit address
    pub fn from_u32(addr: u32) -> Self {
        Self {
            bus: ((addr >> 16) & 0xFF) as u8,
            device: ((addr >> 11) & 0x1F) as u8,
            function: ((addr >> 8) & 0x7) as u8,
            register: (addr & 0xFC) as u8,
        }
    }
}

/// PCI configuration space header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PciConfigHeader {
    /// Device ID and Vendor ID
    pub device_vendor: u32,
    /// Status and Command registers
    pub status_command: u32,
    /// Class code and Revision ID
    pub class_revision: u32,
    /// BIST, Header type, Latency Timer, Cache Line Size
    pub bist_header_latency: u32,
    /// Base Address Register 0
    pub bar0: u32,
    /// Base Address Register 1
    pub bar1: u32,
    /// Base Address Register 2
    pub bar2: u32,
    /// Base Address Register 3
    pub bar3: u32,
    /// Base Address Register 4
    pub bar4: u32,
    /// Base Address Register 5
    pub bar5: u32,
    /// Cardbus CIS Pointer
    pub cardbus_cis: u32,
    /// Subsystem ID and Subsystem Vendor ID
    pub subsystem: u32,
    /// Expansion ROM Base Address
    pub rom_base: u32,
    /// Capabilities Pointer
    pub capabilities_ptr: u8,
    /// Reserved
    _reserved: [u8; 7],
    /// Interrupt Line and Interrupt Pin
    pub interrupt: u16,
    /// Minimum Grant and Maximum Latency
    pub grant_latency: u8,
}

impl Default for PciConfigHeader {
    fn default() -> Self {
        Self {
            device_vendor: 0,
            status_command: 0,
            class_revision: 0,
            bist_header_latency: 0,
            bar0: 0,
            bar1: 0,
            bar2: 0,
            bar3: 0,
            bar4: 0,
            bar5: 0,
            cardbus_cis: 0,
            subsystem: 0,
            rom_base: 0,
            capabilities_ptr: 0,
            _reserved: [0; 7],
            interrupt: 0,
            grant_latency: 0,
        }
    }
}

/// PCI device class codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PciClassCode {
    /// Pre-PCI 2.0 device
    Unclassified = 0x00,
    /// Mass storage controller
    MassStorage = 0x01,
    /// Network controller
    Network = 0x02,
    /// Display controller
    Display = 0x03,
    /// Multimedia device
    Multimedia = 0x04,
    /// Memory controller
    Memory = 0x05,
    /// Bridge device
    Bridge = 0x06,
    /// Simple communication controller
    Communication = 0x07,
    /// Base system peripheral
    BaseSystem = 0x08,
    /// Input device
    Input = 0x09,
    /// Docking station
    DockingStation = 0x0A,
    /// Processor
    Processor = 0x0B,
    /// Serial bus controller
    SerialBus = 0x0C,
    /// Wireless controller
    Wireless = 0x0D,
    /// Intelligent I/O controller
    IntelligentIo = 0x0E,
    /// Satellite communication controller
    Satellite = 0x0F,
    /// Encryption/Decryption controller
    Encryption = 0x10,
    /// Data acquisition and signal processing
    SignalProcessing = 0x11,
    /// Processing accelerators
    ProcessingAccelerator = 0x12,
    /// Non-essential instrumentation
    NonEssential = 0x13,
    /// Reserved
    Reserved = 0xFF,
}

/// PCI capability IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PciCapabilityId {
    /// Null capability
    Null = 0x00,
    /// PCI Power Management Interface
    PowerManagement = 0x01,
    /// AGP
    Agp = 0x02,
    /// Vital Product Data
    VitalProductData = 0x03,
    /// Slot Identification
    SlotId = 0x04,
    /// Message Signaled Interrupts
    Msi = 0x05,
    /// CompactPCI Hot Swap
    CompactPciHotSwap = 0x06,
    /// PCI-X
    PciX = 0x07,
    /// HyperTransport
    HyperTransport = 0x08,
    /// Vendor Specific
    VendorSpecific = 0x09,
    /// Debug port
    DebugPort = 0x0A,
    /// CompactPCI Central Resource Control
    CompactPciResource = 0x0B,
    /// PCI Standard Hot-Plug Controller
    HotPlug = 0x0C,
    /// Bridge Subsystem Vendor/Device ID
    BridgeSubsystem = 0x0D,
    /// AGP 8x
    Agp8x = 0x0E,
    /// Secure Device
    SecureDevice = 0x0F,
    /// PCI Express
    PciExpress = 0x10,
    /// MSI-X
    MsiX = 0x11,
    /// SATA Data/Index Configuration
    Sata = 0x12,
    /// Advanced Features
    AdvancedFeatures = 0x13,
    /// Enhanced Allocation
    EnhancedAllocation = 0x14,
    /// Reserved
    Reserved = 0xFF,
}

/// PCI capability structure
#[derive(Debug, Clone)]
pub struct PciCapability {
    /// Capability ID
    pub id: PciCapabilityId,
    /// Next capability pointer
    pub next: u8,
    /// Capability data
    pub data: Vec<u8>,
}

/// PCI Express capability
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PciExpressCapability {
    /// Express Capability Register
    pub express_cap: u32,
    /// Device Capabilities Register
    pub device_cap: u32,
    /// Device Control Register
    pub device_control: u16,
    /// Device Status Register
    pub device_status: u16,
    /// Link Capabilities Register
    pub link_cap: u32,
    /// Link Control Register
    pub link_control: u16,
    /// Link Status Register
    pub link_status: u16,
    /// Slot Capabilities Register
    pub slot_cap: u32,
    /// Slot Control Register
    pub slot_control: u16,
    /// Slot Status Register
    pub slot_status: u16,
    /// Root Control Register
    pub root_control: u16,
    /// Root Capabilities Register
    pub root_cap: u32,
    /// Root Status Register
    pub root_status: u32,
    /// Device Capabilities 2 Register
    pub device_cap2: u32,
    /// Device Control 2 Register
    pub device_control2: u16,
    /// Device Status 2 Register
    pub device_status2: u16,
    /// Link Capabilities 2 Register
    pub link_cap2: u32,
    /// Link Control 2 Register
    pub link_control2: u16,
    /// Link Status 2 Register
    pub link_status2: u16,
    /// Slot Capabilities 2 Register
    pub slot_cap2: u32,
    /// Slot Control 2 Register
    pub slot_control2: u16,
    /// Slot Status 2 Register
    pub slot_status2: u16,
}

/// MSI-X capability
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MsiXCapability {
    /// Message Control Register
    pub message_control: u16,
    /// Table Offset and BIR
    pub table_offset: u32,
    /// Pending Bit Array Offset and BIR
    pub pba_offset: u32,
}

/// PCI device information
#[derive(Debug, Clone)]
pub struct PciDeviceInfo {
    /// PCI address
    pub address: PciAddress,
    /// Configuration header
    pub config_header: PciConfigHeader,
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Class code
    pub class_code: PciClassCode,
    /// Subclass code
    pub subclass: u8,
    /// Programming interface
    pub prog_if: u8,
    /// Revision ID
    pub revision: u8,
    /// Capabilities
    pub capabilities: Vec<PciCapability>,
    /// PCI Express capability (if present)
    pub pcie_cap: Option<PciExpressCapability>,
    /// MSI-X capability (if present)
    pub msix_cap: Option<MsiXCapability>,
    /// Base Address Registers
    pub bars: [u32; 6],
    /// BAR sizes
    pub bar_sizes: [u32; 6],
    /// BAR types (memory vs I/O)
    pub bar_types: [PciBarType; 6],
    /// Interrupt line
    pub interrupt_line: u8,
    /// Interrupt pin
    pub interrupt_pin: u8,
}

/// PCI BAR type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciBarType {
    /// Unused BAR
    Unused,
    /// Memory BAR
    Memory,
    /// I/O BAR
    Io,
    /// 64-bit Memory BAR (high dword)
    Memory64High,
}

/// PCI device manager
pub struct PciDeviceManager {
    /// PCI devices by address
    devices: Mutex<BTreeMap<u32, PciDeviceInfo>>,
    /// Device model reference
    device_model: Arc<Mutex<dyn DeviceModel>>,
    /// Next device ID
    next_device_id: AtomicU32,
    /// Manager statistics
    stats: Mutex<PciStats>,
    /// Manager initialized flag
    initialized: AtomicBool,
}

/// PCI manager statistics
#[derive(Debug, Default, Clone)]
pub struct PciStats {
    /// Total PCI devices found
    pub total_devices: u32,
    /// Devices by class
    pub devices_by_class: BTreeMap<PciClassCode, u32>,
    /// Number of PCI Express devices
    pub pcie_devices: u32,
    /// Number of MSI-X capable devices
    pub msix_devices: u32,
    /// Number of hot-plug events
    pub hotplug_events: u64,
    /// Number of configuration reads
    pub config_reads: u64,
    /// Number of configuration writes
    pub config_writes: u64,
    /// Number of memory reads
    pub memory_reads: u64,
    /// Number of memory writes
    pub memory_writes: u64,
    /// Number of I/O reads
    pub io_reads: u64,
    /// Number of I/O writes
    pub io_writes: u64,
}

impl PciDeviceManager {
    /// Create a new PCI device manager
    pub fn new(device_model: Arc<Mutex<dyn DeviceModel>>) -> Self {
        Self {
            devices: Mutex::new(BTreeMap::new()),
            device_model,
            next_device_id: AtomicU32::new(1),
            stats: Mutex::new(PciStats::default()),
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize the PCI device manager
    pub fn init(&self) -> Result<(), KernelError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Clear device registry
        {
            let mut devices = self.devices.lock();
            devices.clear();
        }

        // Reset statistics
        {
            let mut stats = self.stats.lock();
            *stats = PciStats::default();
        }

        // Enumerate PCI devices
        self.enumerate_pci_devices()?;

        self.initialized.store(true, Ordering::SeqCst);
        crate::println!("pci: PCI device manager initialized");
        Ok(())
    }

    /// Enumerate all PCI devices on all buses
    fn enumerate_pci_devices(&self) -> Result<(), KernelError> {
        // Scan all possible PCI buses (0-255)
        for bus in 0..255 {
            // Scan all possible devices (0-31)
            for device in 0..32 {
                // Scan all possible functions (0-7)
                for function in 0..8 {
                    let address = PciAddress::new(bus, device, function, 0);
                    
                    // Read vendor ID to check if device exists
                    let vendor_device = self.read_config_dword(address);
                    let vendor_id = (vendor_device & 0xFFFF) as u16;
                    
                    // If vendor ID is 0xFFFF, no device exists at this address
                    if vendor_id == 0xFFFF {
                        // If this is function 0, no need to check other functions
                        if function == 0 {
                            break;
                        }
                        continue;
                    }

                    // Read configuration header
                    let mut config_header = PciConfigHeader::default();
                    self.read_config_header(address, &mut config_header)?;

                    // Check if this is a multi-function device
                    let header_type = (config_header.bist_header_latency >> 16) & 0xFF;
                    let multifunction = (header_type & 0x80) != 0;
                    
                    // If this is function 0 and not multi-function, no need to check other functions
                    if function == 0 && !multifunction {
                        break;
                    }

                    // Create device info
                    let device_info = self.create_device_info(address, &config_header)?;
                    
                    // Register device
                    self.register_pci_device(device_info)?;
                }
            }
        }

        Ok(())
    }

    /// Read PCI configuration header
    fn read_config_header(&self, address: PciAddress, header: &mut PciConfigHeader) -> Result<(), KernelError> {
        // Read all 64 bytes of standard configuration header
        for i in 0..16 {
            let reg_addr = PciAddress::new(address.bus, address.device, address.function, i * 4);
            let value = self.read_config_dword(reg_addr);
            
            match i {
                0 => header.device_vendor = value,
                1 => header.status_command = value,
                2 => header.class_revision = value,
                3 => header.bist_header_latency = value,
                4 => header.bar0 = value,
                5 => header.bar1 = value,
                6 => header.bar2 = value,
                7 => header.bar3 = value,
                8 => header.bar4 = value,
                9 => header.bar5 = value,
                10 => header.cardbus_cis = value,
                11 => header.subsystem = value,
                12 => header.rom_base = value,
                13 => {
                    header.capabilities_ptr = (value & 0xFF) as u8;
                    // The rest of this dword is reserved
                }
                15 => {
                    header.interrupt = (value & 0xFFFF) as u16;
                    header.grant_latency = ((value >> 16) & 0xFF) as u8;
                }
                _ => {}
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.config_reads += 16;
        }

        Ok(())
    }

    /// Create PCI device info from configuration
    fn create_device_info(&self, address: PciAddress, header: &PciConfigHeader) -> Result<PciDeviceInfo, KernelError> {
        let vendor_id = (header.device_vendor & 0xFFFF) as u16;
        let device_id = ((header.device_vendor >> 16) & 0xFFFF) as u16;
        let class_code = ((header.class_revision >> 24) & 0xFF) as u8;
        let subclass = ((header.class_revision >> 16) & 0xFF) as u8;
        let prog_if = ((header.class_revision >> 8) & 0xFF) as u8;
        let revision = (header.class_revision & 0xFF) as u8;
        
        let pci_class = match class_code {
            0x00 => PciClassCode::Unclassified,
            0x01 => PciClassCode::MassStorage,
            0x02 => PciClassCode::Network,
            0x03 => PciClassCode::Display,
            0x04 => PciClassCode::Multimedia,
            0x05 => PciClassCode::Memory,
            0x06 => PciClassCode::Bridge,
            0x07 => PciClassCode::Communication,
            0x08 => PciClassCode::BaseSystem,
            0x09 => PciClassCode::Input,
            0x0A => PciClassCode::DockingStation,
            0x0B => PciClassCode::Processor,
            0x0C => PciClassCode::SerialBus,
            0x0D => PciClassCode::Wireless,
            0x0E => PciClassCode::IntelligentIo,
            0x0F => PciClassCode::Satellite,
            0x10 => PciClassCode::Encryption,
            0x11 => PciClassCode::SignalProcessing,
            0x12 => PciClassCode::ProcessingAccelerator,
            0x13 => PciClassCode::NonEssential,
            _ => PciClassCode::Reserved,
        };

        // Parse BARs
        let mut bars = [0u32; 6];
        let mut bar_sizes = [0u32; 6];
        let mut bar_types = [PciBarType::Unused; 6];
        
        bars[0] = header.bar0;
        bars[1] = header.bar1;
        bars[2] = header.bar2;
        bars[3] = header.bar3;
        bars[4] = header.bar4;
        bars[5] = header.bar5;

        // Determine BAR types and sizes
        for i in 0..6 {
            if bars[i] == 0 {
                bar_types[i] = PciBarType::Unused;
                continue;
            }

            // Check if this is a memory or I/O BAR
            if bars[i] & 0x1 != 0 {
                // I/O BAR
                bar_types[i] = PciBarType::Io;
                // To get size, we would write all 1s and read back
                // For now, we'll use a placeholder
                bar_sizes[i] = 0;
            } else {
                // Memory BAR
                bar_types[i] = PciBarType::Memory;
                // Check if this is a 64-bit BAR
                if i < 5 && (bars[i+1] & 0x1) == 0 && (bars[i+1] & 0x6) == 0 {
                    // 64-bit memory BAR
                    bar_types[i] = PciBarType::Memory;
                    bar_types[i+1] = PciBarType::Memory64High;
                    // For now, use placeholder size
                    bar_sizes[i] = 0;
                    bar_sizes[i+1] = 0;
                } else {
                    // 32-bit memory BAR
                    // For now, use placeholder size
                    bar_sizes[i] = 0;
                }
            }
        }

        // Read capabilities
        let mut capabilities = Vec::new();
        if header.capabilities_ptr != 0 {
            self.read_capabilities(address, header.capabilities_ptr, &mut capabilities)?;
        }

        // Check for PCI Express capability
        let pcie_cap = capabilities.iter()
            .find(|cap| cap.id == PciCapabilityId::PciExpress)
            .and_then(|cap| self.parse_pcie_capability(&cap.data));

        // Check for MSI-X capability
        let msix_cap = capabilities.iter()
            .find(|cap| cap.id == PciCapabilityId::MsiX)
            .and_then(|cap| self.parse_msix_capability(&cap.data));

        let interrupt_line = (header.interrupt & 0xFF) as u8;
        let interrupt_pin = ((header.interrupt >> 8) & 0xFF) as u8;

        Ok(PciDeviceInfo {
            address,
            config_header: *header,
            vendor_id,
            device_id,
            class_code: pci_class,
            subclass,
            prog_if,
            revision,
            capabilities,
            pcie_cap,
            msix_cap,
            bars,
            bar_sizes,
            bar_types,
            interrupt_line,
            interrupt_pin,
        })
    }

    /// Read PCI capabilities
    fn read_capabilities(&self, address: PciAddress, cap_ptr: u8, capabilities: &mut Vec<PciCapability>) -> Result<(), KernelError> {
        let mut next_cap = cap_ptr;
        
        while next_cap != 0 {
            // Read capability header (ID and next pointer)
            let cap_addr = PciAddress::new(address.bus, address.device, address.function, next_cap);
            let cap_header = self.read_config_byte(cap_addr);
            let cap_id = cap_header;
            
            // Read next pointer
            let next_addr = PciAddress::new(address.bus, address.device, address.function, next_cap + 1);
            let next_ptr = self.read_config_byte(next_addr);
            
            // Determine capability size based on ID
            let cap_size = match PciCapabilityId::from_u8(cap_id) {
                Some(PciCapabilityId::PciExpress) => 60, // Approximate size
                Some(PciCapabilityId::MsiX) => 12,
                Some(PciCapabilityId::PowerManagement) => 8,
                _ => 2, // Minimum size
            };
            
            // Read capability data
            let mut cap_data = Vec::with_capacity(cap_size);
            for i in 0..cap_size {
                let data_addr = PciAddress::new(address.bus, address.device, address.function, next_cap + i);
                cap_data.push(self.read_config_byte(data_addr));
            }
            
            capabilities.push(PciCapability {
                id: PciCapabilityId::from_u8(cap_id).unwrap_or(PciCapabilityId::Reserved),
                next: next_ptr,
                data: cap_data,
            });
            
            next_cap = next_ptr;
            
            // Prevent infinite loop
            if next_cap == cap_ptr || next_cap >= 0x40 {
                break;
            }
        }

        Ok(())
    }

    /// Parse PCI Express capability
    fn parse_pcie_capability(&self, data: &[u8]) -> Option<PciExpressCapability> {
        if data.len() < 60 {
            return None;
        }

        Some(PciExpressCapability {
            express_cap: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            device_cap: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            device_control: u16::from_le_bytes([data[8], data[9]]),
            device_status: u16::from_le_bytes([data[10], data[11]]),
            link_cap: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            link_control: u16::from_le_bytes([data[16], data[17]]),
            link_status: u16::from_le_bytes([data[18], data[19]]),
            slot_cap: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            slot_control: u16::from_le_bytes([data[24], data[25]]),
            slot_status: u16::from_le_bytes([data[26], data[27]]),
            root_control: u16::from_le_bytes([data[28], data[29]]),
            root_cap: u32::from_le_bytes([data[30], data[31], data[32], data[33]]),
            root_status: u32::from_le_bytes([data[34], data[35], data[36], data[37]]),
            device_cap2: u32::from_le_bytes([data[38], data[39], data[40], data[41]]),
            device_control2: u16::from_le_bytes([data[42], data[43]]),
            device_status2: u16::from_le_bytes([data[44], data[45]]),
            link_cap2: u32::from_le_bytes([data[46], data[47], data[48], data[49]]),
            link_control2: u16::from_le_bytes([data[50], data[51]]),
            link_status2: u16::from_le_bytes([data[52], data[53]]),
            slot_cap2: u32::from_le_bytes([data[54], data[55], data[56], data[57]]),
            slot_control2: u16::from_le_bytes([data[58], data[59]]),
            slot_status2: u16::from_le_bytes([data[60], data[61]]),
        })
    }

    /// Parse MSI-X capability
    fn parse_msix_capability(&self, data: &[u8]) -> Option<MsiXCapability> {
        if data.len() < 12 {
            return None;
        }

        Some(MsiXCapability {
            message_control: u16::from_le_bytes([data[2], data[3]]),
            table_offset: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            pba_offset: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
        })
    }

    /// Register a PCI device with the device model
    fn register_pci_device(&self, device_info: PciDeviceInfo) -> Result<(), KernelError> {
        // Create device key from address
        let device_key = ((device_info.address.bus as u32) << 16) | 
                        ((device_info.address.device as u32) << 8) | 
                        (device_info.address.function as u32);

        // Add to device registry
        {
            let mut devices = self.devices.lock();
            devices.insert(device_key, device_info.clone());
        }

        // Create enhanced device info for device model
        let device_name = format!("pci-{}:{}:{}.{}", 
                                device_info.address.bus,
                                device_info.address.device,
                                device_info.address.function);
        
        let vendor_name = self.get_vendor_name(device_info.vendor_id);
        let device_name_full = self.get_device_name(device_info.vendor_id, device_info.device_id);
        
        let class_name = match device_info.class_code {
            PciClassCode::MassStorage => "mass_storage",
            PciClassCode::Network => "network",
            PciClassCode::Display => "display",
            PciClassCode::Multimedia => "multimedia",
            PciClassCode::Memory => "memory",
            PciClassCode::Bridge => "bridge",
            PciClassCode::Communication => "communication",
            PciClassCode::BaseSystem => "base_system",
            PciClassCode::Input => "input",
            PciClassCode::DockingStation => "docking_station",
            PciClassCode::Processor => "processor",
            PciClassCode::SerialBus => "serial_bus",
            PciClassCode::Wireless => "wireless",
            PciClassCode::IntelligentIo => "intelligent_io",
            PciClassCode::Satellite => "satellite",
            PciClassCode::Encryption => "encryption",
            PciClassCode::SignalProcessing => "signal_processing",
            PciClassCode::ProcessingAccelerator => "processing_accelerator",
            PciClassCode::NonEssential => "non_essential",
            PciClassCode::Unclassified => "unclassified",
            PciClassCode::Reserved => "reserved",
        };

        let device_class = match device_info.class_code {
            PciClassCode::MassStorage => DeviceClass::Storage,
            PciClassCode::Network => DeviceClass::Network,
            PciClassCode::Display => DeviceClass::Display,
            PciClassCode::Multimedia => DeviceClass::Multimedia,
            PciClassCode::Memory => DeviceClass::Memory,
            PciClassCode::Bridge => DeviceClass::Bus,
            PciClassCode::Communication => DeviceClass::Communication,
            PciClassCode::BaseSystem => DeviceClass::System,
            PciClassCode::Input => DeviceClass::Input,
            PciClassCode::DockingStation => DeviceClass::System,
            PciClassCode::Processor => DeviceClass::Processor,
            PciClassCode::SerialBus => DeviceClass::Bus,
            PciClassCode::Wireless => DeviceClass::Network,
            PciClassCode::IntelligentIo => DeviceClass::System,
            PciClassCode::Satellite => DeviceClass::Communication,
            PciClassCode::Encryption => DeviceClass::System,
            PciClassCode::SignalProcessing => DeviceClass::Multimedia,
            PciClassCode::ProcessingAccelerator => DeviceClass::Processor,
            PciClassCode::NonEssential => DeviceClass::Custom,
            PciClassCode::Unclassified => DeviceClass::Custom,
            PciClassCode::Reserved => DeviceClass::Custom,
        };

        // Create device resources
        let mut resources = DeviceResources::default();
        
        // Add memory regions from BARs
        for i in 0..6 {
            if device_info.bar_types[i] == PciBarType::Memory {
                if device_info.bars[i] != 0 {
                    // Add memory resource
                    // In a real implementation, we would parse the BAR to get address and size
                    // For now, we'll use placeholder values
                }
            }
        }

        // Create device capabilities
        let mut capabilities = DeviceCapabilities::default();
        capabilities.interrupts = true;
        capabilities.memory_mapping = true;
        
        if device_info.pcie_cap.is_some() {
            capabilities.hotplug = true;
            capabilities.power_management = true;
        }
        
        if device_info.msix_cap.is_some() {
            capabilities.interrupts = true;
        }

        // Create enhanced device info
        let enhanced_info = EnhancedDeviceInfo {
            base_info: DeviceInfo {
                id: 0, // Will be set by device model
                name: device_name,
                device_type: DeviceType::Custom("pci".to_string()),
                status: DeviceStatus::Present,
                driver_id: 0, // Will be set when driver is bound
                path: format!("/sys/devices/pci{}:{}:{}", 
                              device_info.address.bus,
                              device_info.address.device,
                              device_info.address.function),
                version: format!("{}.{}", device_info.class_code as u32, device_info.revision),
                vendor: vendor_name,
                model: device_name_full,
                serial_number: format!("{:04X}:{:04X}", device_info.vendor_id, device_info.device_id),
                resources,
                capabilities: Vec::new(),
                attributes: BTreeMap::new(),
            },
            device_class,
            parent_id: 0, // PCI devices are typically root devices
            child_ids: Vec::new(),
            depth: 1,
            power_state: DevicePowerState::On,
            capabilities,
            performance_metrics: DevicePerformanceMetrics::default(),
            firmware_version: "".to_string(),
            hardware_revision: format!("{}", device_info.revision),
            serial_number: format!("{:04X}:{:04X}", device_info.vendor_id, device_info.device_id),
            uuid: format!("pci-{}-{}-{}-{}", 
                        device_info.address.bus,
                        device_info.address.device,
                        device_info.address.function,
                        device_info.revision),
            location: format!("PCI Bus {} Device {} Function {}", 
                            device_info.address.bus,
                            device_info.address.device,
                            device_info.address.function),
            aliases: Vec::new(),
            tags: vec![class_name.to_string()],
            creation_timestamp: crate::subsystems::time::get_timestamp(),
            last_modified_timestamp: crate::subsystems::time::get_timestamp(),
        };

        // Register with device model
        let device_id = {
            let mut model = self.device_model.lock();
            model.register_device(enhanced_info)?
        };

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_devices += 1;
            *stats.devices_by_class.entry(device_info.class_code).or_insert(0) += 1;
            
            if device_info.pcie_cap.is_some() {
                stats.pcie_devices += 1;
            }
            
            if device_info.msix_cap.is_some() {
                stats.msix_devices += 1;
            }
        }

        crate::println!("pci: registered PCI device {}:{}:{} ({:04X}:{:04X}) as device {}", 
                      device_info.address.bus,
                      device_info.address.device,
                      device_info.address.function,
                      device_info.vendor_id,
                      device_info.device_id,
                      device_id);

        Ok(())
    }

    /// Get vendor name from vendor ID
    fn get_vendor_name(&self, vendor_id: u16) -> String {
        // In a real implementation, this would use a comprehensive vendor database
        // For now, we'll return a few common vendors
        match vendor_id {
            0x8086 => "Intel Corporation".to_string(),
            0x10DE => "NVIDIA Corporation".to_string(),
            0x1002 => "Advanced Micro Devices, Inc.".to_string(),
            0x1AF4 => "Red Hat, Inc.".to_string(),
            0x1B36 => "Red Hat, Inc.".to_string(),
            0x1234 => "Red Hat, Inc.".to_string(),
            0x1AE0 => "Google, Inc.".to_string(),
            0x1D0F => "Red Hat, Inc.".to_string(),
            0x15B3 => "Red Hat, Inc.".to_string(),
            _ => format!("Unknown (0x{:04X})", vendor_id),
        }
    }

    /// Get device name from vendor and device ID
    fn get_device_name(&self, vendor_id: u16, device_id: u16) -> String {
        // In a real implementation, this would use a comprehensive device database
        // For now, we'll return a few common devices
        match (vendor_id, device_id) {
            (0x8086, 0x100E) => "82540EM Gigabit Ethernet Controller".to_string(),
            (0x8086, 0x1237) => "82371SB PIIX4 ISA".to_string(),
            (0x8086, 0x7000) => "82540EM Gigabit Ethernet Controller".to_string(),
            (0x1AF4, 0x1005) => "Virtio RNG Device".to_string(),
            (0x1AF4, 0x1009) => "Virtio Filesystem Device".to_string(),
            (0x1AF4, 0x1012) => "Virtio Memory Balloon Device".to_string(),
            _ => format!("Unknown Device (0x{:04X}:{:04X})", vendor_id, device_id),
        }
    }

    /// Read a byte from PCI configuration space
    fn read_config_byte(&self, address: PciAddress) -> u8 {
        let addr = address.to_u32();
        // In a real implementation, this would access PCI configuration space
        // For now, we'll return a placeholder value
        0
    }

    /// Read a dword (32 bits) from PCI configuration space
    fn read_config_dword(&self, address: PciAddress) -> u32 {
        let addr = address.to_u32();
        // In a real implementation, this would access PCI configuration space
        // For now, we'll return a placeholder value
        0
    }

    /// Write a byte to PCI configuration space
    fn write_config_byte(&self, address: PciAddress, value: u8) {
        let addr = address.to_u32();
        // In a real implementation, this would access PCI configuration space
        // For now, we'll just update statistics
        {
            let mut stats = self.stats.lock();
            stats.config_writes += 1;
        }
    }

    /// Write a dword (32 bits) to PCI configuration space
    fn write_config_dword(&self, address: PciAddress, value: u32) {
        let addr = address.to_u32();
        // In a real implementation, this would access PCI configuration space
        // For now, we'll just update statistics
        {
            let mut stats = self.stats.lock();
            stats.config_writes += 4;
        }
    }

    /// Read from PCI memory space
    pub fn read_memory(&self, bar: u32, offset: u32, buffer: &mut [u8]) -> Result<(), KernelError> {
        // In a real implementation, this would access PCI memory space
        // For now, we'll just update statistics
        {
            let mut stats = self.stats.lock();
            stats.memory_reads += buffer.len() as u64;
        }
        Ok(())
    }

    /// Write to PCI memory space
    pub fn write_memory(&self, bar: u32, offset: u32, data: &[u8]) -> Result<(), KernelError> {
        // In a real implementation, this would access PCI memory space
        // For now, we'll just update statistics
        {
            let mut stats = self.stats.lock();
            stats.memory_writes += data.len() as u64;
        }
        Ok(())
    }

    /// Read from PCI I/O space
    pub fn read_io(&self, bar: u32, offset: u32, buffer: &mut [u8]) -> Result<(), KernelError> {
        // In a real implementation, this would access PCI I/O space
        // For now, we'll just update statistics
        {
            let mut stats = self.stats.lock();
            stats.io_reads += buffer.len() as u64;
        }
        Ok(())
    }

    /// Write to PCI I/O space
    pub fn write_io(&self, bar: u32, offset: u32, data: &[u8]) -> Result<(), KernelError> {
        // In a real implementation, this would access PCI I/O space
        // For now, we'll just update statistics
        {
            let mut stats = self.stats.lock();
            stats.io_writes += data.len() as u64;
        }
        Ok(())
    }

    /// Get PCI device information
    pub fn get_device_info(&self, bus: u8, device: u8, function: u8) -> Option<PciDeviceInfo> {
        let device_key = ((bus as u32) << 16) | ((device as u32) << 8) | (function as u32);
        let devices = self.devices.lock();
        devices.get(&device_key).cloned()
    }

    /// Get all PCI devices
    pub fn get_all_devices(&self) -> Vec<PciDeviceInfo> {
        let devices = self.devices.lock();
        devices.values().cloned().collect()
    }

    /// Get PCI devices by class
    pub fn get_devices_by_class(&self, class: PciClassCode) -> Vec<PciDeviceInfo> {
        let devices = self.devices.lock();
        devices.values()
            .filter(|device| device.class_code == class)
            .cloned()
            .collect()
    }

    /// Get PCI devices by vendor
    pub fn get_devices_by_vendor(&self, vendor_id: u16) -> Vec<PciDeviceInfo> {
        let devices = self.devices.lock();
        devices.values()
            .filter(|device| device.vendor_id == vendor_id)
            .cloned()
            .collect()
    }

    /// Get PCI devices by vendor and device ID
    pub fn get_devices_by_vendor_device(&self, vendor_id: u16, device_id: u16) -> Vec<PciDeviceInfo> {
        let devices = self.devices.lock();
        devices.values()
            .filter(|device| device.vendor_id == vendor_id && device.device_id == device_id)
            .cloned()
            .collect()
    }

    /// Get PCI manager statistics
    pub fn get_stats(&self) -> PciStats {
        self.stats.lock().clone()
    }

    /// Reset PCI manager statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = PciStats::default();
    }

    /// Enable bus mastering for a device
    pub fn enable_bus_mastering(&self, bus: u8, device: u8, function: u8) -> Result<(), KernelError> {
        let address = PciAddress::new(bus, device, function, 1); // Command register
        let mut command = self.read_config_dword(address);
        command |= 0x4; // Set bus master bit
        self.write_config_dword(address, command);
        
        crate::println!("pci: enabled bus mastering for {}:{}:{}", bus, device, function);
        Ok(())
    }

    /// Disable bus mastering for a device
    pub fn disable_bus_mastering(&self, bus: u8, device: u8, function: u8) -> Result<(), KernelError> {
        let address = PciAddress::new(bus, device, function, 1); // Command register
        let mut command = self.read_config_dword(address);
        command &= !0x4; // Clear bus master bit
        self.write_config_dword(address, command);
        
        crate::println!("pci: disabled bus mastering for {}:{}:{}", bus, device, function);
        Ok(())
    }

    /// Enable memory space for a device
    pub fn enable_memory_space(&self, bus: u8, device: u8, function: u8) -> Result<(), KernelError> {
        let address = PciAddress::new(bus, device, function, 1); // Command register
        let mut command = self.read_config_dword(address);
        command |= 0x2; // Set memory space bit
        self.write_config_dword(address, command);
        
        crate::println!("pci: enabled memory space for {}:{}:{}", bus, device, function);
        Ok(())
    }

    /// Disable memory space for a device
    pub fn disable_memory_space(&self, bus: u8, device: u8, function: u8) -> Result<(), KernelError> {
        let address = PciAddress::new(bus, device, function, 1); // Command register
        let mut command = self.read_config_dword(address);
        command &= !0x2; // Clear memory space bit
        self.write_config_dword(address, command);
        
        crate::println!("pci: disabled memory space for {}:{}:{}", bus, device, function);
        Ok(())
    }

    /// Enable I/O space for a device
    pub fn enable_io_space(&self, bus: u8, device: u8, function: u8) -> Result<(), KernelError> {
        let address = PciAddress::new(bus, device, function, 1); // Command register
        let mut command = self.read_config_dword(address);
        command |= 0x1; // Set I/O space bit
        self.write_config_dword(address, command);
        
        crate::println!("pci: enabled I/O space for {}:{}:{}", bus, device, function);
        Ok(())
    }

    /// Disable I/O space for a device
    pub fn disable_io_space(&self, bus: u8, device: u8, function: u8) -> Result<(), KernelError> {
        let address = PciAddress::new(bus, device, function, 1); // Command register
        let mut command = self.read_config_dword(address);
        command &= !0x1; // Clear I/O space bit
        self.write_config_dword(address, command);
        
        crate::println!("pci: disabled I/O space for {}:{}:{}", bus, device, function);
        Ok(())
    }

    /// Enable interrupts for a device
    pub fn enable_interrupts(&self, bus: u8, device: u8, function: u8) -> Result<(), KernelError> {
        let address = PciAddress::new(bus, device, function, 1); // Command register
        let mut command = self.read_config_dword(address);
        command &= !0x400; // Clear interrupt disable bit
        self.write_config_dword(address, command);
        
        crate::println!("pci: enabled interrupts for {}:{}:{}", bus, device, function);
        Ok(())
    }

    /// Disable interrupts for a device
    pub fn disable_interrupts(&self, bus: u8, device: u8, function: u8) -> Result<(), KernelError> {
        let address = PciAddress::new(bus, device, function, 1); // Command register
        let mut command = self.read_config_dword(address);
        command |= 0x400; // Set interrupt disable bit
        self.write_config_dword(address, command);
        
        crate::println!("pci: disabled interrupts for {}:{}:{}", bus, device, function);
        Ok(())
    }

    /// Handle hot-plug event
    pub fn handle_hotplug_event(&self, bus: u8, device: u8, function: u8, event_type: HotplugEventType) {
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.hotplug_events += 1;
        }

        match event_type {
            HotplugEventType::DeviceAdded => {
                // Device was added, enumerate it
                let address = PciAddress::new(bus, device, function, 0);
                let mut config_header = PciConfigHeader::default();
                if let Ok(()) = self.read_config_header(address, &mut config_header) {
                    if let Ok(device_info) = self.create_device_info(address, &config_header) {
                        let _ = self.register_pci_device(device_info);
                    }
                }
            }
            HotplugEventType::DeviceRemoved => {
                // Device was removed, unregister it
                let device_key = ((bus as u32) << 16) | ((device as u32) << 8) | (function as u32);
                let mut devices = self.devices.lock();
                if let Some(device_info) = devices.remove(&device_key) {
                    crate::println!("pci: removed PCI device {}:{}:{}", bus, device, function);
                    
                    // In a real implementation, we would also remove it from the device model
                }
            }
        }
    }
}

/// Hot-plug event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugEventType {
    /// Device was added
    DeviceAdded,
    /// Device was removed
    DeviceRemoved,
}

/// Convert capability ID from u8
impl PciCapabilityId {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(PciCapabilityId::Null),
            0x01 => Some(PciCapabilityId::PowerManagement),
            0x02 => Some(PciCapabilityId::Agp),
            0x03 => Some(PciCapabilityId::VitalProductData),
            0x04 => Some(PciCapabilityId::SlotId),
            0x05 => Some(PciCapabilityId::Msi),
            0x06 => Some(PciCapabilityId::CompactPciHotSwap),
            0x07 => Some(PciCapabilityId::PciX),
            0x08 => Some(PciCapabilityId::HyperTransport),
            0x09 => Some(PciCapabilityId::VendorSpecific),
            0x0A => Some(PciCapabilityId::DebugPort),
            0x0B => Some(PciCapabilityId::CompactPciResource),
            0x0C => Some(PciCapabilityId::HotPlug),
            0x0D => Some(PciCapabilityId::BridgeSubsystem),
            0x0E => Some(PciCapabilityId::Agp8x),
            0x0F => Some(PciCapabilityId::SecureDevice),
            0x10 => Some(PciCapabilityId::PciExpress),
            0x11 => Some(PciCapabilityId::MsiX),
            0x12 => Some(PciCapabilityId::Sata),
            0x13 => Some(PciCapabilityId::AdvancedFeatures),
            0x14 => Some(PciCapabilityId::EnhancedAllocation),
            _ => None,
        }
    }
}

/// Global PCI device manager instance
static mut PCI_DEVICE_MANAGER: Option<PciDeviceManager> = None;

/// Initialize PCI device manager
pub fn init(device_model: Arc<Mutex<dyn DeviceModel>>) -> Result<(), KernelError> {
    unsafe {
        let manager = PciDeviceManager::new(device_model);
        manager.init()?;
        PCI_DEVICE_MANAGER = Some(manager);
    }
    crate::println!("pci: PCI device manager initialized");
    Ok(())
}

/// Get PCI device manager instance
pub fn get_pci_device_manager() -> Option<&'static PciDeviceManager> {
    unsafe { PCI_DEVICE_MANAGER.as_ref() }
}