//! PCI Device Support for VFIO
//!
//! Provides PCI-specific functionality including:
//!
//! - PCI device enumeration
//! - SR-IOV support (Physical Functions / Virtual Functions)
//! - PCI config space access
//! - VGA arbitration for GPUs
//! - BAR mapping
//!
//! # SR-IOV
//!
//! SR-IOV allows a single physical device (PF) to present multiple
//! virtual devices (VFs) to the system:
//!
//! ```text
//! Physical Function (PF)
//!     |
//!     |-- VF 0 (0000:01:00.0)
//!     |-- VF 1 (0000:01:00.1)
//!     |-- VF 2 (0000:01:00.2)
//!     |-- ...
//!     |-- VF N (0000:01:00.N)
//! ```

use crate::drivers::vfio::{VfioError, VfioResult};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU16, Ordering};

/// PCI device identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PciDeviceId {
    /// Segment (PCI domain)
    pub segment: u16,

    /// Bus number
    pub bus: u8,

    /// Device number
    pub device: u8,

    /// Function number
    pub function: u8,
}

impl PciDeviceId {
    /// Create new PCI device ID
    pub fn new(segment: u16, bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment,
            bus,
            device,
            function,
        }
    }

    /// Convert to string (e.g., "0000:01:00.0")
    pub fn to_string(&self) -> alloc::string::String {
        alloc::format!(
            "{:04x}:{:02x}:{:02x}.{}",
            self.segment, self.bus, self.device, self.function
        )
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        // Parse "0000:01:00.0" format
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            return None;
        }

        let segment = u16::from_str_radix(parts[0], 16).ok()?;
        let bus = u8::from_str_radix(parts[1], 16).ok()?;

        let device_function: Vec<&str> = parts[2].split('.').collect();
        if device_function.len() != 2 {
            return None;
        }

        let device = u8::from_str_radix(device_function[0], 16).ok()?;
        let function = u8::from_str_radix(device_function[1], 10).ok()?;

        Some(Self {
            segment,
            bus,
            device,
            function,
        })
    }
}

/// PCI device
pub struct PciDevice {
    id: PciDeviceId,
    vendor_id: u16,
    device_id: u16,
    class_code: u32,
    revision: u8,
    bars: [PciBar; 6],
    rom: Option<PciRom>,
    is_vf: bool,
    pf_id: Option<PciDeviceId>,
    enabled: AtomicU16,
}

/// PCI BAR (Base Address Register)
#[derive(Debug, Clone, Copy)]
pub struct PciBar {
    /// BAR index
    pub index: u8,

    /// BAR type
    pub bar_type: BarType,

    /// Base address
    pub base_addr: u64,

    /// Size (bytes)
    pub size: u64,

    /// Flags
    pub flags: u32,
}

/// BAR type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarType {
    /// Memory-mapped I/O (32-bit)
    Io32,

    /// Memory-mapped I/O (64-bit)
    Io64,

    /// I/O port
    PortIo,
}

/// PCI ROM
#[derive(Debug, Clone, Copy)]
pub struct PciRom {
    pub base_addr: u64,
    pub size: u64,
}

/// PCI config space
#[derive(Debug, Clone)]
pub struct PciConfigSpace {
    /// Device ID
    pub device_id: u16,

    /// Vendor ID
    pub vendor_id: u16,

    /// Status register
    pub status: u16,

    /// Command register
    pub command: u16,

    /// Class code
    pub class_code: u8,

    /// Revision ID
    pub revision: u8,

    /// Header type
    pub header_type: u8,

    /// Cache line size
    pub cache_line_size: u8,

    /// Latency timer
    pub latency_timer: u8,

    /// BIST
    pub bist: u8,

    /// BARs
    pub bars: [u32; 6],

    /// Cardbus CIS pointer
    pub cardbus_cis: u32,

    /// Subsystem vendor ID
    pub sub_vendor_id: u16,

    /// Subsystem ID
    pub sub_device_id: u16,

    /// Expansion ROM base address
    pub rom_base: u32,

    /// Capabilities pointer
    pub capabilities_ptr: u8,

    /// Reserved
    pub reserved: [u8; 7],

    /// Interrupt line
    pub interrupt_line: u8,

    /// Interrupt pin
    pub interrupt_pin: u8,

    /// Minimum grant
    pub min_gnt: u8,

    /// Maximum latency
    pub max_lat: u8,
}

impl PciConfigSpace {
    /// Create default config space
    pub fn new() -> Self {
        Self {
            device_id: 0,
            vendor_id: 0,
            status: 0,
            command: 0,
            class_code: 0,
            revision: 0,
            header_type: 0,
            cache_line_size: 0,
            latency_timer: 0,
            bist: 0,
            bars: [0; 6],
            cardbus_cis: 0,
            sub_vendor_id: 0,
            sub_device_id: 0,
            rom_base: 0,
            capabilities_ptr: 0,
            reserved: [0; 7],
            interrupt_line: 0,
            interrupt_pin: 0,
            min_gnt: 0,
            max_lat: 0,
        }
    }

    /// Read config space register
    pub fn read(&self, offset: u16, size: u8) -> u32 {
        // GH-#928: Implement proper config space read
        // See: https://github.com/npos/kernel/issues/928
        0
    }

    /// Write config space register
    pub fn write(&mut self, offset: u16, value: u32, size: u8) {
        // GH-#929: Implement proper config space write
        // See: https://github.com/npos/kernel/issues/929
    }
}

impl Default for PciConfigSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl PciDevice {
    /// Create new PCI device
    pub fn new(
        id: PciDeviceId,
        vendor_id: u16,
        device_id: u16,
        class_code: u32,
    ) -> Self {
        Self {
            id,
            vendor_id,
            device_id,
            class_code,
            revision: 0,
            bars: [PciBar {
                index: 0,
                bar_type: BarType::Io32,
                base_addr: 0,
                size: 0,
                flags: 0,
            }; 6],
            rom: None,
            is_vf: false,
            pf_id: None,
            enabled: AtomicU16::new(0),
        }
    }

    /// Get device ID
    pub fn id(&self) -> PciDeviceId {
        self.id
    }

    /// Get vendor ID
    pub fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    /// Get device ID
    pub fn device_id(&self) -> u16 {
        self.device_id
    }

    /// Get class code
    pub fn class_code(&self) -> u32 {
        self.class_code
    }

    /// Check if this is a Virtual Function
    pub fn is_vf(&self) -> bool {
        self.is_vf
    }

    /// Set as Virtual Function
    pub fn set_vf(&mut self, is_vf: bool) {
        self.is_vf = is_vf;
    }

    /// Get PF device ID (if VF)
    pub fn pf_id(&self) -> Option<PciDeviceId> {
        self.pf_id
    }

    /// Set PF device ID
    pub fn set_pf_id(&mut self, pf_id: PciDeviceId) {
        self.pf_id = Some(pf_id);
    }

    /// Enable device
    pub fn enable(&self) -> VfioResult<()> {
        // GH-#930: Enable PCI device (bus master, etc.)
        // See: https://github.com/npos/kernel/issues/930
        self.enabled.store(1, Ordering::Relaxed);
        Ok(())
    }

    /// Disable device
    pub fn disable(&self) -> VfioResult<()> {
        // GH-#931: Disable PCI device
        // See: https://github.com/npos/kernel/issues/931
        self.enabled.store(0, Ordering::Relaxed);
        Ok(())
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed) != 0
    }

    /// Get BAR
    pub fn get_bar(&self, index: u8) -> Option<PciBar> {
        self.bars.get(index as usize).copied()
    }

    /// Set BAR
    pub fn set_bar(&mut self, bar: PciBar) {
        if (bar.index as usize) < self.bars.len() {
            self.bars[bar.index as usize] = bar;
        }
    }

    /// Get ROM
    pub fn get_rom(&self) -> Option<PciRom> {
        self.rom
    }

    /// Set ROM
    pub fn set_rom(&mut self, rom: PciRom) {
        self.rom = Some(rom);
    }

    /// Read config space
    pub fn read_config(&self, offset: u16, size: u8) -> u32 {
        // GH-#932: Implement config space read
        // See: https://github.com/npos/kernel/issues/932
        0
    }

    /// Write config space
    pub fn write_config(&mut self, offset: u16, value: u32, size: u8) {
        // GH-#933: Implement config space write
        // See: https://github.com/npos/kernel/issues/933
    }

    /// Reset device
    pub fn reset(&self) -> VfioResult<()> {
        // GH-#934: Send function level reset
        // See: https://github.com/npos/kernel/issues/934
        Ok(())
    }
}

/// SR-IOV Virtual Function
pub struct SrioVf {
    vf_id: PciDeviceId,
    pf_device: Arc<PciDevice>,
    vf_index: u16,
    enabled: bool,
}

impl SrioVf {
    /// Create new VF
    pub fn new(pf_device: Arc<PciDevice>, vf_index: u16) -> Self {
        let pf_id = pf_device.id();

        // Calculate VF device ID
        // VFs typically have function numbers starting from 1
        let vf_id = PciDeviceId {
            segment: pf_id.segment,
            bus: pf_id.bus,
            device: pf_id.device,
            function: pf_id.function + 1 + (vf_index as u8),
        };

        Self {
            vf_id,
            pf_device,
            vf_index,
            enabled: false,
        }
    }

    /// Get VF ID
    pub fn id(&self) -> PciDeviceId {
        self.vf_id
    }

    /// Get VF index
    pub fn index(&self) -> u16 {
        self.vf_index
    }

    /// Get PF device
    pub fn pf_device(&self) -> &Arc<PciDevice> {
        &self.pf_device
    }

    /// Enable VF
    pub fn enable(&mut self) -> VfioResult<()> {
        // GH-#935: Enable VF
        // See: https://github.com/npos/kernel/issues/935
        self.enabled = true;
        Ok(())
    }

    /// Disable VF
    pub fn disable(&mut self) -> VfioResult<()> {
        // GH-#936: Disable VF
        // See: https://github.com/npos/kernel/issues/936
        self.enabled = false;
        Ok(())
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// VGA arbiter for GPU devices
pub struct VgaArbiter {
    /// Current VGA owner
    owner: Mutex<Option<PciDeviceId>>,

    /// VGA devices
    vga_devices: Mutex<Vec<PciDeviceId>>,
}

impl VgaArbiter {
    /// Create new VGA arbiter
    pub fn new() -> Self {
        Self {
            owner: Mutex::new(None),
            vga_devices: Mutex::new(Vec::new()),
        }
    }

    /// Register VGA device
    pub fn register_device(&self, device_id: PciDeviceId) -> VfioResult<()> {
        self.vga_devices.lock().push(device_id);
        Ok(())
    }

    /// Try to acquire VGA
    pub fn try_acquire(&self, device_id: PciDeviceId) -> VfioResult<bool> {
        let mut owner = self.owner.lock();

        if owner.is_some() {
            return Ok(false);
        }

        *owner = Some(device_id);
        Ok(true)
    }

    /// Release VGA
    pub fn release(&self, device_id: PciDeviceId) -> VfioResult<()> {
        let mut owner = self.owner.lock();

        if owner.as_ref() != Some(&device_id) {
            return Err(VfioError::InvalidArgument);
        }

        *owner = None;
        Ok(())
    }

    /// Get current owner
    pub fn get_owner(&self) -> Option<PciDeviceId> {
        *self.owner.lock()
    }
}

/// PCI error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciError {
    DeviceNotFound,
    InvalidDeviceId,
    ConfigError,
    BarError,
    EnableFailed,
    ResetFailed,
    VfError,
}

impl core::fmt::Display for PciError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DeviceNotFound => write!(f, "PCI device not found"),
            Self::InvalidDeviceId => write!(f, "Invalid PCI device ID"),
            Self::ConfigError => write!(f, "PCI config error"),
            Self::BarError => write!(f, "PCI BAR error"),
            Self::EnableFailed => write!(f, "Failed to enable PCI device"),
            Self::ResetFailed => write!(f, "Failed to reset PCI device"),
            Self::VfError => write!(f, "SR-IOV VF error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pci_device_id_to_string() {
        let id = PciDeviceId::new(0, 1, 0, 0);
        assert_eq!(id.to_string(), "0000:01:00.0");
    }

    #[test]
    fn test_pci_device_id_from_str() {
        let id = PciDeviceId::from_str("0000:01:00.0").unwrap();
        assert_eq!(id.segment, 0);
        assert_eq!(id.bus, 1);
        assert_eq!(id.device, 0);
        assert_eq!(id.function, 0);
    }

    #[test]
    fn test_pci_device_id_roundtrip() {
        let id = PciDeviceId::new(0x1234, 0x56, 0x78, 9);
        let s = id.to_string();
        let id2 = PciDeviceId::from_str(&s).unwrap();
        assert_eq!(id, id2);
    }

    #[test]
    fn test_pci_device_create() {
        let id = PciDeviceId::new(0, 0, 1, 0);
        let device = PciDevice::new(id, 0x1234, 0x5678, 0x020000);

        assert_eq!(device.vendor_id(), 0x1234);
        assert_eq!(device.device_id(), 0x5678);
        assert!(!device.is_vf());
    }

    #[test]
    fn test_pci_device_enable_disable() {
        let id = PciDeviceId::new(0, 0, 1, 0);
        let device = PciDevice::new(id, 0x1234, 0x5678, 0x020000);

        device.enable().unwrap();
        assert!(device.is_enabled());

        device.disable().unwrap();
        assert!(!device.is_enabled());
    }

    #[test]
    fn test_pci_bar() {
        let bar = PciBar {
            index: 0,
            bar_type: BarType::Io32,
            base_addr: 0x10000000,
            size: 0x1000,
            flags: 0,
        };

        assert_eq!(bar.index, 0);
        assert_eq!(bar.bar_type, BarType::Io32);
        assert_eq!(bar.size, 0x1000);
    }

    #[test]
    fn test_vga_arbiter() {
        let arbiter = VgaArbiter::new();

        let device1 = PciDeviceId::new(0, 0, 1, 0);
        let device2 = PciDeviceId::new(0, 0, 2, 0);

        arbiter.register_device(device1).unwrap();

        // First acquire should succeed
        assert_eq!(arbiter.try_acquire(device1).unwrap(), true);
        assert_eq!(arbiter.get_owner(), Some(device1));

        // Second acquire should fail
        assert_eq!(arbiter.try_acquire(device2).unwrap(), false);

        // Release
        arbiter.release(device1).unwrap();
        assert_eq!(arbiter.get_owner(), None);
    }

    #[test]
    fn test_sr_iov_vf() {
        let pf_id = PciDeviceId::new(0, 0, 1, 0);
        let pf = Arc::new(PciDevice::new(pf_id, 0x1234, 0x5678, 0x020000));

        let vf = SrioVf::new(pf.clone(), 0);
        assert_eq!(vf.index(), 0);

        let vf_id = vf.id();
        assert_eq!(vf_id.segment, 0);
        assert_eq!(vf_id.bus, 0);
        assert_eq!(vf_id.device, 0);
        assert_eq!(vf_id.function, 2); // PF function (0) + 1 + VF index (0)
    }

    #[test]
    fn test_config_space() {
        let config = PciConfigSpace::new();
        assert_eq!(config.device_id, 0);
        assert_eq!(config.vendor_id, 0);
    }
}
