//! USB Device Manager Implementation
//!
//! This module implements a comprehensive USB device manager for NOS,
//! providing USB bus enumeration, device configuration, resource management,
//! and driver binding. The implementation supports USB 1.1, 2.0, 3.x,
//! and advanced features like hot-plug and power management.

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering};
use crate::sync::{Mutex, Sleeplock};
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
// USB Constants and Structures
// ============================================================================

/// USB specification versions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbVersion {
    /// Major version
    pub major: u8,
    /// Minor version
    pub minor: u8,
    /// Revision
    pub revision: u8,
}

impl UsbVersion {
    /// USB 1.1
    pub const USB_1_1: Self = Self { major: 1, minor: 1, revision: 0 };
    /// USB 2.0
    pub const USB_2_0: Self = Self { major: 2, minor: 0, revision: 0 };
    /// USB 3.0
    pub const USB_3_0: Self = Self { major: 3, minor: 0, revision: 0 };
    /// USB 3.1
    pub const USB_3_1: Self = Self { major: 3, minor: 1, revision: 0 };
    /// USB 3.2
    pub const USB_3_2: Self = Self { major: 3, minor: 2, revision: 0 };
}

/// USB speeds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    /// Low speed (1.5 Mbps)
    Low,
    /// Full speed (12 Mbps)
    Full,
    /// High speed (480 Mbps)
    High,
    /// Super speed (5 Gbps)
    SuperSpeed,
    /// Super speed plus (10 Gbps)
    SuperSpeedPlus,
    /// Unknown speed
    Unknown,
}

/// USB device class codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UsbClassCode {
    /// Use class information in the Interface Descriptors
    UseInterfaceClass = 0x00,
    /// Audio class
    Audio = 0x01,
    /// Communications and CDC Control
    Communications = 0x02,
    /// HID (Human Interface Device)
    Hid = 0x03,
    /// Physical
    Physical = 0x05,
    /// Image
    Image = 0x06,
    /// Printer
    Printer = 0x07,
    /// Mass storage
    MassStorage = 0x08,
    /// Hub
    Hub = 0x09,
    /// CDC-Data
    CdcData = 0x0A,
    /// Smart Card
    SmartCard = 0x0B,
    /// Content Security
    ContentSecurity = 0x0D,
    /// Video
    Video = 0x0E,
    /// Personal Healthcare
    PersonalHealthcare = 0x0F,
    /// Audio/Video
    AudioVideo = 0x10,
    /// Billboard
    Billboard = 0x11,
    /// USB Type-C Bridge Class
    UsbTypeCBridge = 0x12,
    /// Diagnostic Device
    Diagnostic = 0xDC,
    /// Wireless
    Wireless = 0xE0,
    /// Miscellaneous
    Miscellaneous = 0xEF,
    /// Application Specific
    ApplicationSpecific = 0xFE,
    /// Vendor Specific
    VendorSpecific = 0xFF,
}

/// USB device descriptor
#[derive(Debug, Clone)]
pub struct UsbDeviceDescriptor {
    /// Descriptor length
    pub length: u8,
    /// Descriptor type
    pub descriptor_type: u8,
    /// USB specification version (BCD)
    pub usb_version: u16,
    /// Device class
    pub device_class: u8,
    /// Device subclass
    pub device_subclass: u8,
    /// Device protocol
    pub device_protocol: u8,
    /// Maximum packet size for endpoint 0
    pub max_packet_size: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Device version (BCD)
    pub device_version: u16,
    /// Manufacturer string index
    pub manufacturer_index: u8,
    /// Product string index
    pub product_index: u8,
    /// Serial number string index
    pub serial_number_index: u8,
    /// Number of configurations
    pub num_configurations: u8,
}

/// USB configuration descriptor
#[derive(Debug, Clone)]
pub struct UsbConfigurationDescriptor {
    /// Total length
    pub total_length: u16,
    /// Number of interfaces
    pub num_interfaces: u8,
    /// Configuration value
    pub configuration_value: u8,
    /// Configuration string index
    pub configuration_index: u8,
    /// Attributes
    pub attributes: u8,
    /// Maximum power (2mA units)
    pub max_power: u8,
}

/// USB interface descriptor
#[derive(Debug, Clone)]
pub struct UsbInterfaceDescriptor {
    /// Interface number
    pub interface_number: u8,
    /// Alternate setting
    pub alternate_setting: u8,
    /// Number of endpoints
    pub num_endpoints: u8,
    /// Interface class
    pub interface_class: u8,
    /// Interface subclass
    pub interface_subclass: u8,
    /// Interface protocol
    pub interface_protocol: u8,
    /// Interface string index
    pub interface_index: u8,
}

/// USB endpoint descriptor
#[derive(Debug, Clone)]
pub struct UsbEndpointDescriptor {
    /// Endpoint address
    pub endpoint_address: u8,
    /// Attributes
    pub attributes: u8,
    /// Maximum packet size
    pub max_packet_size: u16,
    /// Interval (ms)
    pub interval: u8,
}

/// USB device information
#[derive(Debug, Clone)]
pub struct UsbDeviceInfo {
    /// Device address
    pub address: u8,
    /// Device port number
    pub port_number: u8,
    /// Device speed
    pub speed: UsbSpeed,
    /// Device version
    pub version: UsbVersion,
    /// Device descriptor
    pub device_descriptor: UsbDeviceDescriptor,
    /// Configuration descriptors
    pub configurations: Vec<UsbConfigurationDescriptor>,
    /// Interface descriptors
    pub interfaces: Vec<UsbInterfaceDescriptor>,
    /// Endpoint descriptors
    pub endpoints: Vec<UsbEndpointDescriptor>,
    /// String descriptors
    pub string_descriptors: BTreeMap<u8, String>,
    /// Current configuration
    pub current_configuration: Option<u8>,
    /// Device class
    pub device_class: UsbClassCode,
    /// Parent hub address (if any)
    pub parent_address: Option<u8>,
    /// Connection time
    pub connection_time: u64,
}

/// USB host controller information
#[derive(Debug, Clone)]
pub struct UsbHostControllerInfo {
    /// Controller ID
    pub id: u32,
    /// Controller type
    pub controller_type: UsbControllerType,
    /// Register base address
    pub register_base: usize,
    /// Interrupt line
    pub interrupt_line: u8,
    /// Supported USB version
    pub usb_version: UsbVersion,
    /// Number of ports
    pub num_ports: u8,
    /// Controller capabilities
    pub capabilities: UsbControllerCapabilities,
}

/// USB host controller types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbControllerType {
    /// UHCI (Universal Host Controller Interface)
    Uhci,
    /// OHCI (Open Host Controller Interface)
    Ohci,
    /// EHCI (Enhanced Host Controller Interface)
    Ehci,
    /// xHCI (eXtensible Host Controller Interface)
    Xhci,
    /// Unknown type
    Unknown,
}

/// USB host controller capabilities
#[derive(Debug, Clone)]
pub struct UsbControllerCapabilities {
    /// Supports 64-bit addressing
    pub supports_64bit: bool,
    /// Supports multiple interrupters
    pub supports_multiple_interrupters: bool,
    /// Supports extended endpoints
    pub supports_extended_endpoints: bool,
    /// Supports USB 3.0
    pub supports_usb3: bool,
    /// Supports USB 2.0
    pub supports_usb2: bool,
    /// Supports USB 1.1
    pub supports_usb1: bool,
    /// Supports power management
    pub supports_power_management: bool,
    /// Supports remote wakeup
    pub supports_remote_wakeup: bool,
    /// Supports USB legacy support
    pub supports_legacy_support: bool,
}

/// USB device manager
pub struct UsbDeviceManager {
    /// USB devices by address
    devices: Mutex<BTreeMap<u8, UsbDeviceInfo>>,
    /// USB host controllers
    controllers: Mutex<BTreeMap<u32, UsbHostControllerInfo>>,
    /// Device model reference
    device_model: Arc<Mutex<dyn DeviceModel>>,
    /// Next device ID
    next_device_id: AtomicU32,
    /// Manager statistics
    stats: Mutex<UsbStats>,
    /// Manager initialized flag
    initialized: AtomicBool,
}

/// USB manager statistics
#[derive(Debug, Default, Clone)]
pub struct UsbStats {
    /// Total USB devices found
    pub total_devices: u32,
    /// Devices by class
    pub devices_by_class: BTreeMap<UsbClassCode, u32>,
    /// Devices by speed
    pub devices_by_speed: BTreeMap<UsbSpeed, u32>,
    /// Number of USB 3.x devices
    pub usb3_devices: u32,
    /// Number of USB 2.0 devices
    pub usb2_devices: u32,
    /// Number of USB 1.1 devices
    pub usb1_devices: u32,
    /// Number of host controllers
    pub host_controllers: u32,
    /// Number of hot-plug events
    pub hotplug_events: u64,
    /// Number of control transfers
    pub control_transfers: u64,
    /// Number of bulk transfers
    pub bulk_transfers: u64,
    /// Number of interrupt transfers
    pub interrupt_transfers: u64,
    /// Number of isochronous transfers
    pub isochronous_transfers: u64,
    /// Number of transfer errors
    pub transfer_errors: u64,
    /// Total bytes transferred
    pub bytes_transferred: u64,
}

impl UsbDeviceManager {
    /// Create a new USB device manager
    pub fn new(device_model: Arc<Mutex<dyn DeviceModel>>) -> Self {
        Self {
            devices: Mutex::new(BTreeMap::new()),
            controllers: Mutex::new(BTreeMap::new()),
            device_model,
            next_device_id: AtomicU32::new(1),
            stats: Mutex::new(UsbStats::default()),
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize the USB device manager
    pub fn init(&self) -> Result<(), KernelError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Clear device registry
        {
            let mut devices = self.devices.lock();
            devices.clear();
        }

        // Clear controller registry
        {
            let mut controllers = self.controllers.lock();
            controllers.clear();
        }

        // Reset statistics
        {
            let mut stats = self.stats.lock();
            *stats = UsbStats::default();
        }

        // Enumerate USB host controllers
        self.enumerate_host_controllers()?;

        // Enumerate USB devices
        self.enumerate_usb_devices()?;

        self.initialized.store(true, Ordering::SeqCst);
        crate::println!("usb: USB device manager initialized");
        Ok(())
    }

    /// Enumerate USB host controllers
    fn enumerate_host_controllers(&self) -> Result<(), KernelError> {
        // In a real implementation, this would scan PCI for USB host controllers
        // For now, we'll create a few example controllers
        
        // EHCI Controller
        let ehci_controller = UsbHostControllerInfo {
            id: 1,
            controller_type: UsbControllerType::Ehci,
            register_base: 0xFEBC0000, // Example address
            interrupt_line: 11,
            usb_version: UsbVersion::USB_2_0,
            num_ports: 6,
            capabilities: UsbControllerCapabilities {
                supports_64bit: false,
                supports_multiple_interrupters: false,
                supports_extended_endpoints: false,
                supports_usb3: false,
                supports_usb2: true,
                supports_usb1: true,
                supports_power_management: true,
                supports_remote_wakeup: true,
                supports_legacy_support: true,
            },
        };
        
        // xHCI Controller
        let xhci_controller = UsbHostControllerInfo {
            id: 2,
            controller_type: UsbControllerType::Xhci,
            register_base: 0xFEC00000, // Example address
            interrupt_line: 12,
            usb_version: UsbVersion::USB_3_0,
            num_ports: 8,
            capabilities: UsbControllerCapabilities {
                supports_64bit: true,
                supports_multiple_interrupters: true,
                supports_extended_endpoints: true,
                supports_usb3: true,
                supports_usb2: true,
                supports_usb1: true,
                supports_power_management: true,
                supports_remote_wakeup: true,
                supports_legacy_support: false,
            },
        };

        // Register controllers
        {
            let mut controllers = self.controllers.lock();
            controllers.insert(ehci_controller.id, ehci_controller.clone());
            controllers.insert(xhci_controller.id, xhci_controller.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.host_controllers = 2;
        }

        crate::println!("usb: found {} host controllers", 2);
        Ok(())
    }

    /// Enumerate USB devices on all controllers
    fn enumerate_usb_devices(&self) -> Result<(), KernelError> {
        let controllers = self.controllers.lock();
        
        for controller in controllers.values() {
            // Scan all ports on this controller
            for port in 0..controller.num_ports {
                // Check if a device is connected to this port
                if self.is_device_connected(controller.id, port)? {
                    // Enumerate the device
                    self.enumerate_device_on_port(controller.id, port)?;
                }
            }
        }

        Ok(())
    }

    /// Check if a device is connected to a specific port
    fn is_device_connected(&self, controller_id: u32, port: u8) -> Result<bool, KernelError> {
        // In a real implementation, this would read the port status register
        // For now, we'll simulate a few connected devices
        match (controller_id, port) {
            (1, 0) => Ok(true),  // EHCI port 0 has a device
            (1, 1) => Ok(true),  // EHCI port 1 has a device
            (2, 0) => Ok(true),  // xHCI port 0 has a device
            _ => Ok(false),      // No device on other ports
        }
    }

    /// Enumerate a device on a specific port
    fn enumerate_device_on_port(&self, controller_id: u32, port: u8) -> Result<(), KernelError> {
        // Allocate a device address
        let device_address = self.allocate_device_address()?;
        
        // Get device descriptor
        let device_descriptor = self.get_device_descriptor(controller_id, port, device_address)?;
        
        // Determine device class
        let device_class = self.determine_device_class(&device_descriptor);
        
        // Determine device speed
        let device_speed = self.determine_device_speed(controller_id, port)?;
        
        // Get device version
        let device_version = UsbVersion {
            major: ((device_descriptor.usb_version >> 8) & 0xFF) as u8,
            minor: ((device_descriptor.usb_version >> 4) & 0x0F) as u8,
            revision: (device_descriptor.usb_version & 0x0F) as u8,
        };
        
        // Get configuration descriptors
        let configurations = self.get_configurations(controller_id, port, device_address)?;
        
        // Get interface descriptors
        let interfaces = self.get_interfaces(controller_id, port, device_address)?;
        
        // Get endpoint descriptors
        let endpoints = self.get_endpoints(controller_id, port, device_address)?;
        
        // Get string descriptors
        let string_descriptors = self.get_string_descriptors(controller_id, port, device_address)?;
        
        // Create device info
        let device_info = UsbDeviceInfo {
            address: device_address,
            port_number: port,
            speed: device_speed,
            version: device_version,
            device_descriptor,
            configurations,
            interfaces,
            endpoints,
            string_descriptors,
            current_configuration: None,
            device_class,
            parent_address: None,
            connection_time: crate::time::get_timestamp(),
        };
        
        // Register device
        self.register_usb_device(controller_id, device_info)?;
        
        Ok(())
    }

    /// Allocate a device address
    fn allocate_device_address(&self) -> Result<u8, KernelError> {
        let devices = self.devices.lock();
        for addr in 1..128 {
            if !devices.contains_key(&addr) {
                return Ok(addr);
            }
        }
        Err(KernelError::ResourceExhausted("No available USB device addresses".to_string()))
    }

    /// Get device descriptor
    fn get_device_descriptor(&self, controller_id: u32, port: u8, device_address: u8) -> Result<UsbDeviceDescriptor, KernelError> {
        // In a real implementation, this would send a control transfer to get the descriptor
        // For now, we'll return example descriptors based on the port
        
        match (controller_id, port) {
            (1, 0) => {
                // EHCI port 0: Mass storage device
                Ok(UsbDeviceDescriptor {
                    length: 18,
                    descriptor_type: 1,
                    usb_version: 0x0200,
                    device_class: 0x08,  // Mass storage
                    device_subclass: 0x06,
                    device_protocol: 0x50,
                    max_packet_size: 64,
                    vendor_id: 0x0781,  // SanDisk
                    product_id: 0x5583, // Cruzer Blade
                    device_version: 0x0100,
                    manufacturer_index: 1,
                    product_index: 2,
                    serial_number_index: 3,
                    num_configurations: 1,
                })
            }
            (1, 1) => {
                // EHCI port 1: HID device (keyboard)
                Ok(UsbDeviceDescriptor {
                    length: 18,
                    descriptor_type: 1,
                    usb_version: 0x0110,
                    device_class: 0x00,  // Use interface class
                    device_subclass: 0x00,
                    device_protocol: 0x00,
                    max_packet_size: 8,
                    vendor_id: 0x046D,  // Logitech
                    product_id: 0xC31C, // Keyboard K120
                    device_version: 0x1200,
                    manufacturer_index: 1,
                    product_index: 2,
                    serial_number_index: 0,
                    num_configurations: 1,
                })
            }
            (2, 0) => {
                // xHCI port 0: USB 3.0 mass storage device
                Ok(UsbDeviceDescriptor {
                    length: 18,
                    descriptor_type: 1,
                    usb_version: 0x0300,
                    device_class: 0x00,  // Use interface class
                    device_subclass: 0x00,
                    device_protocol: 0x00,
                    max_packet_size: 9,
                    vendor_id: 0x0930,  // Toshiba
                    product_id: 0x6545, // USB 3.0 Flash Drive
                    device_version: 0x0100,
                    manufacturer_index: 1,
                    product_index: 2,
                    serial_number_index: 3,
                    num_configurations: 1,
                })
            }
            _ => {
                // Default device
                Ok(UsbDeviceDescriptor {
                    length: 18,
                    descriptor_type: 1,
                    usb_version: 0x0200,
                    device_class: 0xFF,  // Vendor specific
                    device_subclass: 0xFF,
                    device_protocol: 0xFF,
                    max_packet_size: 64,
                    vendor_id: 0x1234,
                    product_id: 0x5678,
                    device_version: 0x0100,
                    manufacturer_index: 1,
                    product_index: 2,
                    serial_number_index: 3,
                    num_configurations: 1,
                })
            }
        }
    }

    /// Determine device class
    fn determine_device_class(&self, descriptor: &UsbDeviceDescriptor) -> UsbClassCode {
        // If device class is 0, use interface class
        if descriptor.device_class == 0 {
            // In a real implementation, we would check the interface descriptors
            // For now, we'll use a placeholder
            return UsbClassCode::VendorSpecific;
        }
        
        match descriptor.device_class {
            0x01 => UsbClassCode::Audio,
            0x02 => UsbClassCode::Communications,
            0x03 => UsbClassCode::Hid,
            0x05 => UsbClassCode::Physical,
            0x06 => UsbClassCode::Image,
            0x07 => UsbClassCode::Printer,
            0x08 => UsbClassCode::MassStorage,
            0x09 => UsbClassCode::Hub,
            0x0A => UsbClassCode::CdcData,
            0x0B => UsbClassCode::SmartCard,
            0x0D => UsbClassCode::ContentSecurity,
            0x0E => UsbClassCode::Video,
            0x0F => UsbClassCode::PersonalHealthcare,
            0x10 => UsbClassCode::AudioVideo,
            0x11 => UsbClassCode::Billboard,
            0x12 => UsbClassCode::UsbTypeCBridge,
            0xDC => UsbClassCode::Diagnostic,
            0xE0 => UsbClassCode::Wireless,
            0xEF => UsbClassCode::Miscellaneous,
            0xFE => UsbClassCode::ApplicationSpecific,
            0xFF => UsbClassCode::VendorSpecific,
            _ => UsbClassCode::UseInterfaceClass,
        }
    }

    /// Determine device speed
    fn determine_device_speed(&self, controller_id: u32, port: u8) -> Result<UsbSpeed, KernelError> {
        // In a real implementation, this would read the port status register
        // For now, we'll return example speeds based on the controller and port
        match (controller_id, port) {
            (1, 0) => Ok(UsbSpeed::High),      // EHCI port 0: High speed
            (1, 1) => Ok(UsbSpeed::Full),     // EHCI port 1: Full speed
            (2, 0) => Ok(UsbSpeed::SuperSpeed), // xHCI port 0: Super speed
            _ => Ok(UsbSpeed::Full),           // Default: Full speed
        }
    }

    /// Get configuration descriptors
    fn get_configurations(&self, _controller_id: u32, _port: u8, _device_address: u8) -> Result<Vec<UsbConfigurationDescriptor>, KernelError> {
        // In a real implementation, this would send control transfers to get the descriptors
        // For now, we'll return a single configuration
        Ok(vec![
            UsbConfigurationDescriptor {
                total_length: 32,
                num_interfaces: 1,
                configuration_value: 1,
                configuration_index: 0,
                attributes: 0x80, // Bus powered
                max_power: 50,    // 100mA
            }
        ])
    }

    /// Get interface descriptors
    fn get_interfaces(&self, _controller_id: u32, _port: u8, _device_address: u8) -> Result<Vec<UsbInterfaceDescriptor>, KernelError> {
        // In a real implementation, this would send control transfers to get the descriptors
        // For now, we'll return a single interface
        Ok(vec![
            UsbInterfaceDescriptor {
                interface_number: 0,
                alternate_setting: 0,
                num_endpoints: 2,
                interface_class: 0x08, // Mass storage
                interface_subclass: 0x06,
                interface_protocol: 0x50,
                interface_index: 0,
            }
        ])
    }

    /// Get endpoint descriptors
    fn get_endpoints(&self, _controller_id: u32, _port: u8, _device_address: u8) -> Result<Vec<UsbEndpointDescriptor>, KernelError> {
        // In a real implementation, this would send control transfers to get the descriptors
        // For now, we'll return two endpoints (bulk in and bulk out)
        Ok(vec![
            UsbEndpointDescriptor {
                endpoint_address: 0x81, // Bulk IN
                attributes: 0x02,       // Bulk
                max_packet_size: 512,
                interval: 0,
            },
            UsbEndpointDescriptor {
                endpoint_address: 0x02, // Bulk OUT
                attributes: 0x02,       // Bulk
                max_packet_size: 512,
                interval: 0,
            }
        ])
    }

    /// Get string descriptors
    fn get_string_descriptors(&self, _controller_id: u32, _port: u8, _device_address: u8) -> Result<BTreeMap<u8, String>, KernelError> {
        // In a real implementation, this would send control transfers to get the descriptors
        // For now, we'll return a few example strings
        let mut strings = BTreeMap::new();
        strings.insert(1, "Example Manufacturer".to_string());
        strings.insert(2, "Example Product".to_string());
        strings.insert(3, "123456789".to_string());
        Ok(strings)
    }

    /// Register a USB device with the device model
    fn register_usb_device(&self, controller_id: u32, device_info: UsbDeviceInfo) -> Result<(), KernelError> {
        // Add to device registry
        {
            let mut devices = self.devices.lock();
            devices.insert(device_info.address, device_info.clone());
        }

        // Create device name
        let device_name = format!("usb-{}-{}", controller_id, device_info.address);
        
        let vendor_name = self.get_vendor_name(device_info.device_descriptor.vendor_id);
        let device_name_full = self.get_device_name(
            device_info.device_descriptor.vendor_id,
            device_info.device_descriptor.product_id
        );
        
        let class_name = match device_info.device_class {
            UsbClassCode::Audio => "audio",
            UsbClassCode::Communications => "communications",
            UsbClassCode::Hid => "hid",
            UsbClassCode::Physical => "physical",
            UsbClassCode::Image => "image",
            UsbClassCode::Printer => "printer",
            UsbClassCode::MassStorage => "mass_storage",
            UsbClassCode::Hub => "hub",
            UsbClassCode::CdcData => "cdc_data",
            UsbClassCode::SmartCard => "smart_card",
            UsbClassCode::ContentSecurity => "content_security",
            UsbClassCode::Video => "video",
            UsbClassCode::PersonalHealthcare => "personal_healthcare",
            UsbClassCode::AudioVideo => "audio_video",
            UsbClassCode::Billboard => "billboard",
            UsbClassCode::UsbTypeCBridge => "usb_type_c_bridge",
            UsbClassCode::Diagnostic => "diagnostic",
            UsbClassCode::Wireless => "wireless",
            UsbClassCode::Miscellaneous => "miscellaneous",
            UsbClassCode::ApplicationSpecific => "application_specific",
            UsbClassCode::VendorSpecific(_) => "vendor_specific",
            UsbClassCode::UseInterfaceClass => "use_interface_class",
        };

        let device_class = match device_info.device_class {
            UsbClassCode::Audio => DeviceClass::Audio,
            UsbClassCode::Communications => DeviceClass::Communication,
            UsbClassCode::Hid => DeviceClass::Input,
            UsbClassCode::Physical => DeviceClass::Sensor,
            UsbClassCode::Image => DeviceClass::Camera,
            UsbClassCode::Printer => DeviceClass::Printer,
            UsbClassCode::MassStorage => DeviceClass::Storage,
            UsbClassCode::Hub => DeviceClass::Bus,
            UsbClassCode::CdcData => DeviceClass::Communication,
            UsbClassCode::SmartCard => DeviceClass::SmartCard,
            UsbClassCode::ContentSecurity => DeviceClass::Security,
            UsbClassCode::Video => DeviceClass::Camera,
            UsbClassCode::PersonalHealthcare => DeviceClass::Medical,
            UsbClassCode::AudioVideo => DeviceClass::Multimedia,
            UsbClassCode::Billboard => DeviceClass::Display,
            UsbClassCode::UsbTypeCBridge => DeviceClass::Bus,
            UsbClassCode::Diagnostic => DeviceClass::Diagnostic,
            UsbClassCode::Wireless => DeviceClass::Network,
            UsbClassCode::Miscellaneous => DeviceClass::Custom,
            UsbClassCode::ApplicationSpecific => DeviceClass::Custom,
            UsbClassCode::VendorSpecific(_) => DeviceClass::Custom,
            UsbClassCode::UseInterfaceClass => DeviceClass::Custom,
        };

        // Create device resources
        let mut resources = DeviceResources::default();
        
        // Add interrupt resource
        resources.interrupts.push(InterruptInfo {
            irq: device_info.port_number as u32,
            trigger: "edge".to_string(),
            priority: 5,
        });

        // Create device capabilities
        let mut capabilities = DeviceCapabilities::default();
        capabilities.interrupts = true;
        capabilities.hotplug = true;
        capabilities.power_management = true;
        
        match device_info.speed {
            UsbSpeed::Low | UsbSpeed::Full => {
                capabilities.power_management = true;
            }
            UsbSpeed::High => {
                capabilities.power_management = true;
                capabilities.high_speed = true;
            }
            UsbSpeed::SuperSpeed | UsbSpeed::SuperSpeedPlus => {
                capabilities.power_management = true;
                capabilities.high_speed = true;
                capabilities.super_speed = true;
            }
            UsbSpeed::Unknown => {}
        }

        // Create enhanced device info
        let enhanced_info = EnhancedDeviceInfo {
            base_info: DeviceInfo {
                id: 0, // Will be set by device model
                name: device_name,
                device_type: DeviceType::Custom("usb".to_string()),
                status: DeviceStatus::Present,
                driver_id: 0, // Will be set when driver is bound
                path: format!("/sys/devices/usb{}/{}", controller_id, device_info.address),
                version: format!("{}.{}.{}", 
                                device_info.version.major,
                                device_info.version.minor,
                                device_info.version.revision),
                vendor: vendor_name,
                model: device_name_full,
                serial_number: device_info.string_descriptors.get(&3).cloned().unwrap_or_default(),
                resources,
                capabilities: Vec::new(),
                attributes: BTreeMap::new(),
            },
            device_class,
            parent_id: 0, // USB devices are typically root devices
            child_ids: Vec::new(),
            depth: 1,
            power_state: DevicePowerState::On,
            capabilities,
            performance_metrics: DevicePerformanceMetrics::default(),
            firmware_version: "".to_string(),
            hardware_revision: format!("{}.{}.{}", 
                                    device_info.version.major,
                                    device_info.version.minor,
                                    device_info.version.revision),
            serial_number: device_info.string_descriptors.get(&3).cloned().unwrap_or_default(),
            uuid: format!("usb-{}-{}-{}", 
                        controller_id,
                        device_info.address,
                        device_info.port_number),
            location: format!("USB Controller {} Port {} Address {}", 
                            controller_id,
                            device_info.port_number,
                            device_info.address),
            aliases: Vec::new(),
            tags: vec![class_name.to_string(), format!("usb{}", device_info.speed as u8)],
            creation_timestamp: device_info.connection_time,
            last_modified_timestamp: crate::time::get_timestamp(),
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
            *stats.devices_by_class.entry(device_info.device_class).or_insert(0) += 1;
            *stats.devices_by_speed.entry(device_info.speed).or_insert(0) += 1;
            
            match device_info.version.major {
                3 => stats.usb3_devices += 1,
                2 => stats.usb2_devices += 1,
                1 => stats.usb1_devices += 1,
                _ => {}
            }
        }

        crate::println!("usb: registered USB device {}:{} ({:04X}:{:04X}) as device {}", 
                      controller_id,
                      device_info.address,
                      device_info.device_descriptor.vendor_id,
                      device_info.device_descriptor.product_id,
                      device_id);

        Ok(())
    }

    /// Get vendor name from vendor ID
    fn get_vendor_name(&self, vendor_id: u16) -> String {
        // In a real implementation, this would use a comprehensive vendor database
        // For now, we'll return a few common vendors
        match vendor_id {
            0x046D => "Logitech".to_string(),
            0x0781 => "SanDisk".to_string(),
            0x0930 => "Toshiba".to_string(),
            0x1D6B => "Linux Foundation".to_string(),
            0x058F => "Alcor Micro".to_string(),
            0x090C => "Silicon Motion".to_string(),
            0x13FE => "Kingston".to_string(),
            0x0951 => "Kingston".to_string(),
            _ => format!("Unknown (0x{:04X})", vendor_id),
        }
    }

    /// Get device name from vendor and device ID
    fn get_device_name(&self, vendor_id: u16, device_id: u16) -> String {
        // In a real implementation, this would use a comprehensive device database
        // For now, we'll return a few common devices
        match (vendor_id, device_id) {
            (0x046D, 0xC31C) => "Keyboard K120".to_string(),
            (0x0781, 0x5583) => "Cruzer Blade".to_string(),
            (0x0930, 0x6545) => "USB 3.0 Flash Drive".to_string(),
            (0x1D6B, 0x0002) => "EHCI Host Controller".to_string(),
            (0x1D6B, 0x0003) => "xHCI Host Controller".to_string(),
            _ => format!("Unknown Device (0x{:04X}:{:04X})", vendor_id, device_id),
        }
    }

    /// Get USB device information
    pub fn get_device_info(&self, address: u8) -> Option<UsbDeviceInfo> {
        let devices = self.devices.lock();
        devices.get(&address).cloned()
    }

    /// Get all USB devices
    pub fn get_all_devices(&self) -> Vec<UsbDeviceInfo> {
        let devices = self.devices.lock();
        devices.values().cloned().collect()
    }

    /// Get USB devices by class
    pub fn get_devices_by_class(&self, class: UsbClassCode) -> Vec<UsbDeviceInfo> {
        let devices = self.devices.lock();
        devices.values()
            .filter(|device| device.device_class == class)
            .cloned()
            .collect()
    }

    /// Get USB devices by speed
    pub fn get_devices_by_speed(&self, speed: UsbSpeed) -> Vec<UsbDeviceInfo> {
        let devices = self.devices.lock();
        devices.values()
            .filter(|device| device.speed == speed)
            .cloned()
            .collect()
    }

    /// Get USB devices by vendor
    pub fn get_devices_by_vendor(&self, vendor_id: u16) -> Vec<UsbDeviceInfo> {
        let devices = self.devices.lock();
        devices.values()
            .filter(|device| device.device_descriptor.vendor_id == vendor_id)
            .cloned()
            .collect()
    }

    /// Get USB devices by vendor and device ID
    pub fn get_devices_by_vendor_device(&self, vendor_id: u16, device_id: u16) -> Vec<UsbDeviceInfo> {
        let devices = self.devices.lock();
        devices.values()
            .filter(|device| {
                device.device_descriptor.vendor_id == vendor_id &&
                device.device_descriptor.product_id == device_id
            })
            .cloned()
            .collect()
    }

    /// Get USB manager statistics
    pub fn get_stats(&self) -> UsbStats {
        self.stats.lock().clone()
    }

    /// Reset USB manager statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = UsbStats::default();
    }

    /// Handle hot-plug event
    pub fn handle_hotplug_event(&self, controller_id: u32, port: u8, event_type: HotplugEventType) {
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.hotplug_events += 1;
        }

        match event_type {
            HotplugEventType::DeviceAdded => {
                // Device was added, enumerate it
                if let Ok(()) = self.enumerate_device_on_port(controller_id, port) {
                    crate::println!("usb: device added on controller {} port {}", controller_id, port);
                }
            }
            HotplugEventType::DeviceRemoved => {
                // Device was removed, unregister it
                self.handle_device_removal(controller_id, port);
            }
        }
    }

    /// Handle device removal
    fn handle_device_removal(&self, controller_id: u32, port: u8) {
        let mut devices = self.devices.lock();
        let mut device_to_remove = None;
        
        // Find the device on this port
        for (address, device) in devices.iter() {
            if device.port_number == port {
                device_to_remove = Some(*address);
                break;
            }
        }
        
        if let Some(address) = device_to_remove {
            if let Some(device) = devices.remove(&address) {
                crate::println!("usb: removed USB device {}:{} ({:04X}:{:04X})", 
                              controller_id, address,
                              device.device_descriptor.vendor_id,
                              device.device_descriptor.product_id);
                
                // In a real implementation, we would also remove it from the device model
            }
        }
    }

    /// Configure a device
    pub fn configure_device(&self, address: u8, configuration_value: u8) -> Result<(), KernelError> {
        let mut devices = self.devices.lock();
        if let Some(device) = devices.get_mut(&address) {
            device.current_configuration = Some(configuration_value);
            
            // In a real implementation, this would send a SET_CONFIGURATION control transfer
            crate::println!("usb: configured device {} with configuration {}", address, configuration_value);
            return Ok(());
        }
        
        Err(KernelError::NotFound(format!("USB device {} not found", address)))
    }

    /// Get device configuration
    pub fn get_device_configuration(&self, address: u8) -> Option<u8> {
        let devices = self.devices.lock();
        devices.get(&address).and_then(|device| device.current_configuration)
    }

    /// Reset a device
    pub fn reset_device(&self, address: u8) -> Result<(), KernelError> {
        let devices = self.devices.lock();
        if devices.contains_key(&address) {
            // In a real implementation, this would send a reset command to the controller
            crate::println!("usb: reset device {}", address);
            return Ok(());
        }
        
        Err(KernelError::NotFound(format!("USB device {} not found", address)))
    }

    /// Suspend a device
    pub fn suspend_device(&self, address: u8) -> Result<(), KernelError> {
        let devices = self.devices.lock();
        if devices.contains_key(&address) {
            // In a real implementation, this would send a suspend command to the controller
            crate::println!("usb: suspend device {}", address);
            return Ok(());
        }
        
        Err(KernelError::NotFound(format!("USB device {} not found", address)))
    }

    /// Resume a device
    pub fn resume_device(&self, address: u8) -> Result<(), KernelError> {
        let devices = self.devices.lock();
        if devices.contains_key(&address) {
            // In a real implementation, this would send a resume command to the controller
            crate::println!("usb: resume device {}", address);
            return Ok(());
        }
        
        Err(KernelError::NotFound(format!("USB device {} not found", address)))
    }

    /// Get host controller information
    pub fn get_controller_info(&self, controller_id: u32) -> Option<UsbHostControllerInfo> {
        let controllers = self.controllers.lock();
        controllers.get(&controller_id).cloned()
    }

    /// Get all host controllers
    pub fn get_all_controllers(&self) -> Vec<UsbHostControllerInfo> {
        let controllers = self.controllers.lock();
        controllers.values().cloned().collect()
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

/// Global USB device manager instance
static mut USB_DEVICE_MANAGER: Option<UsbDeviceManager> = None;

/// Initialize USB device manager
pub fn init(device_model: Arc<Mutex<dyn DeviceModel>>) -> Result<(), KernelError> {
    unsafe {
        let manager = UsbDeviceManager::new(device_model);
        manager.init()?;
        USB_DEVICE_MANAGER = Some(manager);
    }
    crate::println!("usb: USB device manager initialized");
    Ok(())
}

/// Get USB device manager instance
pub fn get_usb_device_manager() -> Option<&'static UsbDeviceManager> {
    unsafe { USB_DEVICE_MANAGER.as_ref() }
}