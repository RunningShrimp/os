//! Device Enumeration - PCI device discovery and management
//!
//! Provides:
//! - PCI configuration space access
//! - PCI device enumeration and detection
//! - Device information management
//! - PCI bridge and function handling

/// PCI configuration space base address
pub const PCI_CONFIG_BASE: u64 = 0xCF8;
pub const PCI_CONFIG_DATA: u64 = 0xCFC;

/// PCI register offsets
pub const PCI_VENDOR_ID: u8 = 0x00;
pub const PCI_DEVICE_ID: u8 = 0x02;
pub const PCI_COMMAND: u8 = 0x04;
pub const PCI_STATUS: u8 = 0x06;
pub const PCI_CLASS_CODE: u8 = 0x08;
pub const PCI_SUBCLASS: u8 = 0x09;
pub const PCI_PROG_IF: u8 = 0x0A;
pub const PCI_HEADER_TYPE: u8 = 0x0E;
pub const PCI_BAR0: u8 = 0x10;

/// PCI device class codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciClass {
    /// Legacy device
    LegacyDevice = 0x00,
    /// Mass storage controller
    MassStorage = 0x01,
    /// Network controller
    NetworkController = 0x02,
    /// Display controller
    DisplayController = 0x03,
    /// Multimedia controller
    MultimediaController = 0x04,
    /// Memory controller
    MemoryController = 0x05,
    /// Bridge device
    Bridge = 0x06,
    /// Simple communication controller
    Communication = 0x07,
    /// Base system peripheral
    SystemPeripheral = 0x08,
    /// Input device
    InputDevice = 0x09,
    /// Docking station
    DockingStation = 0x0A,
    /// Processor
    Processor = 0x0B,
    /// Serial bus controller
    SerialBusController = 0x0C,
    /// Wireless controller
    WirelessController = 0x0D,
    /// Intelligent I/O controller
    IntelligentIO = 0x0E,
    /// Satellite communication
    SatelliteComm = 0x0F,
    /// Encryption/Decryption
    Encryption = 0x10,
    /// Signal processing
    SignalProcessing = 0x11,
}

/// PCI vendor ID (0xFFFF = invalid)
pub const INVALID_VENDOR_ID: u16 = 0xFFFF;

/// PCI device header type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderType {
    /// Standard device header
    Device = 0,
    /// PCI-to-PCI bridge header
    Bridge = 1,
    /// CardBus bridge header
    CardBus = 2,
}

/// PCI device information
#[derive(Debug, Clone, Copy)]
pub struct PciDeviceInfo {
    /// Bus number
    pub bus: u8,
    /// Device slot number
    pub slot: u8,
    /// Function number
    pub function: u8,
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Class code
    pub class_code: u8,
    /// Subclass code
    pub subclass: u8,
    /// Programming interface
    pub prog_if: u8,
    /// Header type
    pub header_type: HeaderType,
    /// Device enabled
    pub enabled: bool,
}

impl PciDeviceInfo {
    /// Create PCI device information
    pub fn new(
        bus: u8,
        slot: u8,
        function: u8,
        vendor_id: u16,
        device_id: u16,
    ) -> Self {
        PciDeviceInfo {
            bus,
            slot,
            function,
            vendor_id,
            device_id,
            class_code: 0,
            subclass: 0,
            prog_if: 0,
            header_type: HeaderType::Device,
            enabled: false,
        }
    }

    /// Get device address (bus:slot.function)
    pub fn get_address(&self) -> u32 {
        ((self.bus as u32) << 16) | ((self.slot as u32) << 11) | ((self.function as u32) << 8)
    }

    /// Is this a valid device
    pub fn is_valid(&self) -> bool {
        self.vendor_id != INVALID_VENDOR_ID
    }
}

/// PCI device enumerator
pub struct PciEnumerator {
    /// Discovered devices
    devices: [Option<PciDeviceInfo>; 256],
    /// Number of devices found
    device_count: u32,
    /// Enumeration complete
    enumeration_done: bool,
    /// Buses scanned
    buses_scanned: u32,
}

impl PciEnumerator {
    /// Create PCI enumerator
    pub fn new() -> Self {
        PciEnumerator {
            devices: [None; 256],
            device_count: 0,
            enumeration_done: false,
            buses_scanned: 0,
        }
    }

    /// Start device enumeration
    pub fn enumerate(&mut self) -> bool {
        self.device_count = 0;
        self.buses_scanned = 0;

        // Enumerate PCI bus 0 (primary)
        self.enumerate_bus(0);

        self.enumeration_done = true;
        true
    }

    /// Enumerate a single PCI bus
    fn enumerate_bus(&mut self, bus: u8) {
        for slot in 0..32 {
            for function in 0..8 {
                if let Some(device) = self.read_device(bus, slot, function) {
                    if device.is_valid() {
                        self.add_device(device);

                        // Check for PCI-to-PCI bridge
                        if device.header_type == HeaderType::Bridge && bus < 255 {
                            self.enumerate_bus(bus + 1);
                        }
                    }
                }
            }
        }
        self.buses_scanned += 1;
    }

    /// Read device information
    fn read_device(&self, bus: u8, slot: u8, function: u8) -> Option<PciDeviceInfo> {
        let vendor_id = self.read_config_word(bus, slot, function, PCI_VENDOR_ID);

        if vendor_id == INVALID_VENDOR_ID {
            return None;
        }

        let device_id = self.read_config_word(bus, slot, function, PCI_DEVICE_ID);
        let class_info = self.read_config_byte(bus, slot, function, PCI_CLASS_CODE);
        let subclass = self.read_config_byte(bus, slot, function, PCI_SUBCLASS);
        let prog_if = self.read_config_byte(bus, slot, function, PCI_PROG_IF);
        let header_type_byte = self.read_config_byte(bus, slot, function, PCI_HEADER_TYPE);

        let header_type = match header_type_byte & 0x7F {
            1 => HeaderType::Bridge,
            2 => HeaderType::CardBus,
            _ => HeaderType::Device,
        };

        let mut device = PciDeviceInfo::new(bus, slot, function, vendor_id, device_id);
        device.class_code = class_info;
        device.subclass = subclass;
        device.prog_if = prog_if;
        device.header_type = header_type;
        device.enabled = true;

        Some(device)
    }

    /// Add device to list
    fn add_device(&mut self, device: PciDeviceInfo) {
        if (self.device_count as usize) < self.devices.len() {
            self.devices[self.device_count as usize] = Some(device);
            self.device_count += 1;
        }
    }

    /// Get number of devices found
    pub fn device_count(&self) -> u32 {
        self.device_count
    }

    /// Get device by index
    pub fn get_device(&self, index: u32) -> Option<PciDeviceInfo> {
        if (index as usize) < self.devices.len() {
            self.devices[index as usize]
        } else {
            None
        }
    }

    /// Find device by vendor and device ID
    pub fn find_device(&self, vendor_id: u16, device_id: u16) -> Option<PciDeviceInfo> {
        for i in 0..self.device_count {
            if let Some(device) = self.get_device(i) {
                if device.vendor_id == vendor_id && device.device_id == device_id {
                    return Some(device);
                }
            }
        }
        None
    }

    /// Find devices by class
    pub fn find_by_class(&self, class_code: u8) -> u32 {
        let mut count = 0;
        for i in 0..self.device_count {
            if let Some(device) = self.get_device(i) {
                if device.class_code == class_code {
                    count += 1;
                }
            }
        }
        count
    }

    /// Count devices by type
    pub fn count_by_type(&self, class_code: u8) -> u32 {
        self.find_by_class(class_code)
    }

    /// Get enumeration report
    pub fn enumeration_report(&self) -> EnumerationReport {
        let mut storage_count = 0;
        let mut network_count = 0;
        let mut bridge_count = 0;

        for i in 0..self.device_count {
            if let Some(device) = self.get_device(i) {
                match device.class_code {
                    0x01 => storage_count += 1,
                    0x02 => network_count += 1,
                    0x06 => bridge_count += 1,
                    _ => {}
                }
            }
        }

        EnumerationReport {
            total_devices: self.device_count,
            buses_scanned: self.buses_scanned,
            storage_devices: storage_count,
            network_devices: network_count,
            bridge_devices: bridge_count,
            enumeration_complete: self.enumeration_done,
        }
    }

    /// Read configuration word (16-bit)
    fn read_config_word(&self, _bus: u8, _slot: u8, _func: u8, _offset: u8) -> u16 {
        // Real implementation would use PCI config space I/O
        0
    }

    /// Read configuration byte (8-bit)
    fn read_config_byte(&self, _bus: u8, _slot: u8, _func: u8, _offset: u8) -> u8 {
        // Real implementation would use PCI config space I/O
        0
    }
}

/// Device enumeration report
#[derive(Debug, Clone, Copy)]
pub struct EnumerationReport {
    /// Total devices found
    pub total_devices: u32,
    /// PCI buses scanned
    pub buses_scanned: u32,
    /// Storage devices
    pub storage_devices: u32,
    /// Network devices
    pub network_devices: u32,
    /// Bridge devices
    pub bridge_devices: u32,
    /// Enumeration complete
    pub enumeration_complete: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pci_class_codes() {
        assert_eq!(PciClass::MassStorage as u8, 0x01);
        assert_eq!(PciClass::NetworkController as u8, 0x02);
        assert_eq!(PciClass::Bridge as u8, 0x06);
    }

    #[test]
    fn test_invalid_vendor_id() {
        assert_eq!(INVALID_VENDOR_ID, 0xFFFF);
    }

    #[test]
    fn test_header_types() {
        assert_eq!(HeaderType::Device as u8, 0);
        assert_eq!(HeaderType::Bridge as u8, 1);
        assert_eq!(HeaderType::CardBus as u8, 2);
    }

    #[test]
    fn test_pci_device_info_creation() {
        let device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        assert_eq!(device.bus, 0);
        assert_eq!(device.slot, 5);
        assert_eq!(device.function, 0);
        assert_eq!(device.vendor_id, 0x8086);
        assert_eq!(device.device_id, 0x1234);
    }

    #[test]
    fn test_pci_device_info_valid() {
        let device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        assert!(device.is_valid());

        let invalid = PciDeviceInfo::new(0, 5, 0, INVALID_VENDOR_ID, 0);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_pci_device_address() {
        let device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        let addr = device.get_address();
        assert_eq!(addr, (5 << 11) | (0 << 8));
    }

    #[test]
    fn test_pci_device_address_with_bus() {
        let device = PciDeviceInfo::new(1, 5, 0, 0x8086, 0x1234);
        let addr = device.get_address();
        assert_eq!(addr, (1 << 16) | (5 << 11) | (0 << 8));
    }

    #[test]
    fn test_pci_enumerator_creation() {
        let enumerator = PciEnumerator::new();
        assert_eq!(enumerator.device_count(), 0);
        assert!(!enumerator.enumeration_done);
    }

    #[test]
    fn test_pci_enumerator_enumerate() {
        let mut enumerator = PciEnumerator::new();
        assert!(enumerator.enumerate());
        assert!(enumerator.enumeration_done);
    }

    #[test]
    fn test_pci_enumerator_device_count() {
        let mut enumerator = PciEnumerator::new();
        assert_eq!(enumerator.device_count(), 0);
        
        // Add device manually for testing
        let device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        enumerator.add_device(device);
        
        assert_eq!(enumerator.device_count(), 1);
    }

    #[test]
    fn test_pci_enumerator_get_device() {
        let mut enumerator = PciEnumerator::new();
        let device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        enumerator.add_device(device);

        let retrieved = enumerator.get_device(0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().vendor_id, 0x8086);
    }

    #[test]
    fn test_pci_enumerator_find_device() {
        let mut enumerator = PciEnumerator::new();
        let device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        enumerator.add_device(device);

        let found = enumerator.find_device(0x8086, 0x1234);
        assert!(found.is_some());
    }

    #[test]
    fn test_pci_enumerator_find_device_not_found() {
        let enumerator = PciEnumerator::new();
        let found = enumerator.find_device(0x9999, 0x9999);
        assert!(found.is_none());
    }

    #[test]
    fn test_pci_enumerator_find_by_class() {
        let mut enumerator = PciEnumerator::new();
        let mut device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        device.class_code = 0x01; // Storage
        enumerator.add_device(device);

        let count = enumerator.find_by_class(0x01);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_pci_enumerator_multiple_devices() {
        let mut enumerator = PciEnumerator::new();
        
        for i in 0..5 {
            let device = PciDeviceInfo::new(0, i, 0, 0x8086, 0x1000 + i as u16);
            enumerator.add_device(device);
        }

        assert_eq!(enumerator.device_count(), 5);
        
        for i in 0..5 {
            let device = enumerator.get_device(i);
            assert!(device.is_some());
        }
    }

    #[test]
    fn test_enumeration_report() {
        let mut enumerator = PciEnumerator::new();
        let mut device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        device.class_code = 0x01; // Storage
        enumerator.add_device(device);
        enumerator.enumeration_done = true;

        let report = enumerator.enumeration_report();
        assert!(report.enumeration_complete);
        assert_eq!(report.total_devices, 1);
        assert_eq!(report.storage_devices, 1);
    }

    #[test]
    fn test_pci_registers() {
        assert_eq!(PCI_VENDOR_ID, 0x00);
        assert_eq!(PCI_DEVICE_ID, 0x02);
        assert_eq!(PCI_COMMAND, 0x04);
        assert_eq!(PCI_CLASS_CODE, 0x08);
    }

    #[test]
    fn test_pci_device_slot_device_function() {
        let device = PciDeviceInfo::new(2, 15, 7, 0x8086, 0x5678);
        assert_eq!(device.bus, 2);
        assert_eq!(device.slot, 15);
        assert_eq!(device.function, 7);
    }

    #[test]
    fn test_pci_enumerator_count_by_type() {
        let mut enumerator = PciEnumerator::new();
        
        for i in 0..3 {
            let mut device = PciDeviceInfo::new(0, i, 0, 0x8086, 0x1000 + i as u16);
            device.class_code = 0x02; // Network
            enumerator.add_device(device);
        }

        assert_eq!(enumerator.count_by_type(0x02), 3);
    }

    #[test]
    fn test_pci_device_enabled_flag() {
        let mut device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        assert!(!device.enabled);
        device.enabled = true;
        assert!(device.enabled);
    }

    #[test]
    fn test_pci_header_type_filtering() {
        let mut device = PciDeviceInfo::new(0, 5, 0, 0x8086, 0x1234);
        device.header_type = HeaderType::Bridge;
        assert_eq!(device.header_type, HeaderType::Bridge);
    }

    #[test]
    fn test_pci_class_all_codes() {
        assert_eq!(PciClass::LegacyDevice as u8, 0x00);
        assert_eq!(PciClass::DisplayController as u8, 0x03);
        assert_eq!(PciClass::Processor as u8, 0x0B);
    }

    #[test]
    fn test_mixed_device_types_enumeration() {
        let mut enumerator = PciEnumerator::new();
        
        let mut storage = PciDeviceInfo::new(0, 0, 0, 0x8086, 0x1000);
        storage.class_code = 0x01;
        enumerator.add_device(storage);

        let mut network = PciDeviceInfo::new(0, 1, 0, 0x8086, 0x2000);
        network.class_code = 0x02;
        enumerator.add_device(network);

        let report = enumerator.enumeration_report();
        assert_eq!(report.total_devices, 2);
        assert_eq!(report.storage_devices, 1);
        assert_eq!(report.network_devices, 1);
    }
}
