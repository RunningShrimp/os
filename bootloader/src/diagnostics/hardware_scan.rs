//! Hardware Scan - EDID, DMI and hardware information parsing
//!
//! Provides:
//! - EDID (Extended Display Identification Data) parsing
//! - DMI (Desktop Management Interface) information extraction
//! - Hardware capability detection
//! - Component inventory

/// EDID structure
#[derive(Debug, Clone, Copy)]
pub struct EdidInfo {
    /// Manufacturer ID
    pub manufacturer_id: u16,
    /// Product code
    pub product_code: u16,
    /// Serial number
    pub serial_number: u32,
    /// Week of manufacture
    pub manufacture_week: u8,
    /// Year of manufacture (since 1990)
    pub manufacture_year: u16,
    /// EDID version
    pub version: u8,
    /// EDID revision
    pub revision: u8,
    /// Display width in cm
    pub width_cm: u8,
    /// Display height in cm
    pub height_cm: u8,
    /// Supported resolutions count
    pub resolution_count: u8,
}

impl EdidInfo {
    /// Create EDID info
    pub fn new() -> Self {
        EdidInfo {
            manufacturer_id: 0,
            product_code: 0,
            serial_number: 0,
            manufacture_week: 0,
            manufacture_year: 0,
            version: 1,
            revision: 3,
            width_cm: 0,
            height_cm: 0,
            resolution_count: 0,
        }
    }

    /// Parse EDID data (simulated)
    pub fn parse(&mut self, _data: &[u8]) -> bool {
        // Simulated parsing
        if !_data.is_empty() {
            self.manufacturer_id = 0x0610;
            self.product_code = 0x1234;
            self.width_cm = 50;
            self.height_cm = 30;
            true
        } else {
            false
        }
    }

    /// Get manufacture date
    pub fn get_manufacture_date(&self) -> (u8, u16) {
        (self.manufacture_week, self.manufacture_year + 1990)
    }
}

/// DMI information types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmiType {
    /// BIOS information
    Bios,
    /// System information
    System,
    /// Motherboard information
    Motherboard,
    /// Chassis information
    Chassis,
    /// Processor information
    Processor,
    /// Memory device
    MemoryDevice,
    /// Other
    Other,
}

/// DMI data structure
#[derive(Debug, Clone, Copy)]
pub struct DmiData {
    /// DMI type
    pub data_type: DmiType,
    /// Type value
    pub type_value: u8,
    /// Length
    pub length: u8,
    /// Handle
    pub handle: u16,
    /// Valid flag
    pub valid: bool,
}

impl DmiData {
    /// Create DMI data
    pub fn new(data_type: DmiType, type_value: u8) -> Self {
        DmiData {
            data_type,
            type_value,
            length: 0,
            handle: 0,
            valid: false,
        }
    }

    /// Validate DMI data
    pub fn validate(&mut self) -> bool {
        if self.length > 0 && self.type_value < 128 {
            self.valid = true;
            true
        } else {
            false
        }
    }
}

/// System information from DMI
#[derive(Debug, Clone, Copy)]
pub struct SystemInfo {
    /// Manufacturer name (up to 48 chars)
    pub manufacturer: [u8; 48],
    /// Product name (up to 48 chars)
    pub product_name: [u8; 48],
    /// Serial number (up to 48 chars)
    pub serial_number: [u8; 48],
    /// Version (up to 48 chars)
    pub version: [u8; 48],
    /// UUID (16 bytes)
    pub uuid: [u8; 16],
}

impl SystemInfo {
    /// Create system info
    pub fn new() -> Self {
        SystemInfo {
            manufacturer: [0u8; 48],
            product_name: [0u8; 48],
            serial_number: [0u8; 48],
            version: [0u8; 48],
            uuid: [0u8; 16],
        }
    }
}

/// Hardware scanner
pub struct HardwareScanner {
    /// EDID information
    edid: [Option<EdidInfo>; 4],
    /// EDID count
    edid_count: usize,
    /// DMI data
    dmi_data: [Option<DmiData>; 64],
    /// DMI data count
    dmi_count: usize,
    /// System information
    system_info: SystemInfo,
    /// Scan completed
    scan_completed: bool,
}

impl HardwareScanner {
    /// Create hardware scanner
    pub fn new() -> Self {
        HardwareScanner {
            edid: [None; 4],
            edid_count: 0,
            dmi_data: [None; 64],
            dmi_count: 0,
            system_info: SystemInfo::new(),
            scan_completed: false,
        }
    }

    /// Scan EDID
    pub fn scan_edid(&mut self) -> u32 {
        // Simulated EDID scan
        for i in 0..2 {
            let mut edid = EdidInfo::new();
            edid.manufacturer_id = 0x0610 + i as u16;
            edid.resolution_count = (i + 1) as u8;
            self.edid[i] = Some(edid);
        }
        self.edid_count = 2;
        self.edid_count as u32
    }

    /// Get EDID
    pub fn get_edid(&self, index: usize) -> Option<&EdidInfo> {
        if index < self.edid_count {
            self.edid[index].as_ref()
        } else {
            None
        }
    }

    /// Scan DMI
    pub fn scan_dmi(&mut self) -> u32 {
        // Simulated DMI scan
        let dmi_types = [
            (DmiType::Bios, 0),
            (DmiType::System, 1),
            (DmiType::Motherboard, 2),
            (DmiType::Processor, 4),
        ];

        for (i, (dmi_type, type_val)) in dmi_types.iter().enumerate() {
            let mut dmi = DmiData::new(*dmi_type, *type_val);
            dmi.length = 32;
            dmi.handle = i as u16;
            dmi.validate();
            self.dmi_data[i] = Some(dmi);
        }
        self.dmi_count = 4;
        self.dmi_count as u32
    }

    /// Get DMI data
    pub fn get_dmi_data(&self, index: usize) -> Option<&DmiData> {
        if index < self.dmi_count {
            self.dmi_data[index].as_ref()
        } else {
            None
        }
    }

    /// Find DMI by type
    pub fn find_dmi_by_type(&self, data_type: DmiType) -> Option<&DmiData> {
        for i in 0..self.dmi_count {
            if let Some(d) = &self.dmi_data[i] {
                if d.data_type == data_type {
                    return Some(d);
                }
            }
        }
        None
    }

    /// Perform full hardware scan
    pub fn scan_all(&mut self) -> bool {
        self.scan_edid();
        self.scan_dmi();
        self.scan_completed = true;
        true
    }

    /// Get EDID count
    pub fn get_edid_count(&self) -> usize {
        self.edid_count
    }

    /// Get DMI count
    pub fn get_dmi_count(&self) -> usize {
        self.dmi_count
    }

    /// Check if scan completed
    pub fn is_scan_completed(&self) -> bool {
        self.scan_completed
    }

    /// Get system info
    pub fn get_system_info(&self) -> &SystemInfo {
        &self.system_info
    }

    /// Get mutable system info
    pub fn get_system_info_mut(&mut self) -> &mut SystemInfo {
        &mut self.system_info
    }

    /// Get processor count from DMI
    pub fn get_processor_count(&self) -> u32 {
        let mut count = 0;
        for i in 0..self.dmi_count {
            if let Some(d) = &self.dmi_data[i] {
                if d.data_type == DmiType::Processor {
                    count += 1;
                }
            }
        }
        count
    }

    /// Get display count from EDID
    pub fn get_display_count(&self) -> u32 {
        self.edid_count as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edid_creation() {
        let edid = EdidInfo::new();
        assert_eq!(edid.version, 1);
    }

    #[test]
    fn test_edid_manufacture_date() {
        let mut edid = EdidInfo::new();
        edid.manufacture_year = 25;
        let (week, year) = edid.get_manufacture_date();
        assert_eq!(year, 2015);
        assert_eq!(week, 25);
    }

    #[test]
    fn test_dmi_types() {
        assert_ne!(DmiType::Bios, DmiType::System);
    }

    #[test]
    fn test_dmi_data_creation() {
        let dmi = DmiData::new(DmiType::Bios, 0);
        assert_eq!(dmi.data_type, DmiType::Bios);
        assert!(!dmi.valid);
    }

    #[test]
    fn test_dmi_validation() {
        let mut dmi = DmiData::new(DmiType::System, 1);
        dmi.length = 32;
        assert!(dmi.validate());
        assert!(dmi.valid);
    }

    #[test]
    fn test_system_info_creation() {
        let sys = SystemInfo::new();
        assert_eq!(sys.uuid.len(), 16);
    }

    #[test]
    fn test_scanner_creation() {
        let scanner = HardwareScanner::new();
        assert_eq!(scanner.get_edid_count(), 0);
        assert!(!scanner.is_scan_completed());
    }

    #[test]
    fn test_scan_edid() {
        let mut scanner = HardwareScanner::new();
        let count = scanner.scan_edid();
        assert!(count > 0);
    }

    #[test]
    fn test_get_edid() {
        let mut scanner = HardwareScanner::new();
        scanner.scan_edid();
        assert!(scanner.get_edid(0).is_some());
    }

    #[test]
    fn test_scan_dmi() {
        let mut scanner = HardwareScanner::new();
        let count = scanner.scan_dmi();
        assert!(count > 0);
    }

    #[test]
    fn test_get_dmi_data() {
        let mut scanner = HardwareScanner::new();
        scanner.scan_dmi();
        assert!(scanner.get_dmi_data(0).is_some());
    }

    #[test]
    fn test_find_dmi_by_type() {
        let mut scanner = HardwareScanner::new();
        scanner.scan_dmi();
        assert!(scanner.find_dmi_by_type(DmiType::Bios).is_some());
    }

    #[test]
    fn test_scan_all() {
        let mut scanner = HardwareScanner::new();
        assert!(scanner.scan_all());
        assert!(scanner.is_scan_completed());
    }

    #[test]
    fn test_get_processor_count() {
        let mut scanner = HardwareScanner::new();
        scanner.scan_dmi();
        let count = scanner.get_processor_count();
        assert!(count > 0);
    }

    #[test]
    fn test_get_display_count() {
        let mut scanner = HardwareScanner::new();
        scanner.scan_edid();
        let count = scanner.get_display_count();
        assert!(count > 0);
    }

    #[test]
    fn test_edid_parse() {
        let mut edid = EdidInfo::new();
        let data = vec![0x00, 0xFF, 0xFF, 0xFF];
        assert!(edid.parse(&data));
    }

    #[test]
    fn test_edid_dimensions() {
        let mut edid = EdidInfo::new();
        edid.width_cm = 60;
        edid.height_cm = 40;
        assert_eq!(edid.width_cm, 60);
        assert_eq!(edid.height_cm, 40);
    }

    #[test]
    fn test_multiple_edids() {
        let mut scanner = HardwareScanner::new();
        scanner.scan_edid();
        assert_eq!(scanner.get_edid_count(), 2);
    }

    #[test]
    fn test_system_info_update() {
        let mut scanner = HardwareScanner::new();
        let info = scanner.get_system_info_mut();
        info.uuid[0] = 0xAA;
        assert_eq!(scanner.get_system_info().uuid[0], 0xAA);
    }

    #[test]
    fn test_dmi_handle() {
        let mut dmi = DmiData::new(DmiType::Motherboard, 2);
        dmi.handle = 0x1234;
        assert_eq!(dmi.handle, 0x1234);
    }

    #[test]
    fn test_edid_resolution_count() {
        let mut edid = EdidInfo::new();
        edid.resolution_count = 5;
        assert_eq!(edid.resolution_count, 5);
    }

    #[test]
    fn test_multiple_dmi_entries() {
        let mut scanner = HardwareScanner::new();
        scanner.scan_dmi();
        assert!(scanner.get_dmi_count() > 0);
    }

    #[test]
    fn test_dmi_validation_invalid_length() {
        let mut dmi = DmiData::new(DmiType::System, 1);
        assert!(!dmi.validate()); // length is 0
    }

    #[test]
    fn test_edid_manufacturer_id() {
        let mut edid = EdidInfo::new();
        edid.manufacturer_id = 0x0610;
        assert_eq!(edid.manufacturer_id, 0x0610);
    }

    #[test]
    fn test_find_dmi_by_type_not_found() {
        let scanner = HardwareScanner::new();
        assert!(scanner.find_dmi_by_type(DmiType::System).is_none());
    }

    #[test]
    fn test_edid_serial() {
        let mut edid = EdidInfo::new();
        edid.serial_number = 0x12345678;
        assert_eq!(edid.serial_number, 0x12345678);
    }

    #[test]
    fn test_system_info_uuid() {
        let mut sys = SystemInfo::new();
        sys.uuid[0] = 0x01;
        sys.uuid[1] = 0x02;
        assert_eq!(sys.uuid[0], 0x01);
        assert_eq!(sys.uuid[1], 0x02);
    }

    #[test]
    fn test_scanner_get_system_info() {
        let scanner = HardwareScanner::new();
        assert!(scanner.get_system_info().manufacturer.len() > 0);
    }

    #[test]
    fn test_dmi_type_variety() {
        let bios = DmiData::new(DmiType::Bios, 0);
        let sys = DmiData::new(DmiType::System, 1);
        let proc = DmiData::new(DmiType::Processor, 4);
        assert_ne!(bios.data_type, sys.data_type);
        assert_ne!(sys.data_type, proc.data_type);
    }
}
