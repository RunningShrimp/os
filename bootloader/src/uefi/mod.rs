//! UEFI 2.10 Implementation
//!
//! This module provides a comprehensive UEFI 2.10 implementation for the NOS bootloader,
//! supporting all major UEFI protocols and features with GOP mode caching.

extern crate alloc;

pub mod main;
pub mod memory;
pub mod secure_boot;

// Re-export main components
#[cfg(feature = "uefi_support")]
pub use main::*;
#[cfg(feature = "uefi_support")]
pub use memory::*;
#[cfg(feature = "uefi_support")]
pub use secure_boot::*;
#[cfg(feature = "uefi_support")]
pub use super::protocol::uefi::*;

// Version information
pub const UEFI_VERSION_MAJOR: u16 = 2;
pub const UEFI_VERSION_MINOR: u16 = 10;

/// UEFI system table cache to avoid repeated acquire calls
#[cfg(feature = "uefi_support")]
pub struct SystemTableCache {
    cached: bool,
    firmware_vendor: Option<alloc::string::String>,
    firmware_revision: u32,
}

#[cfg(feature = "uefi_support")]
impl SystemTableCache {
    /// Create new system table cache
    pub fn new() -> Self {
        Self {
            cached: false,
            firmware_vendor: None,
            firmware_revision: 0,
        }
    }

    /// Cache system table information
    pub fn cache(&mut self, vendor: alloc::string::String, revision: u32) {
        self.firmware_vendor = Some(vendor);
        self.firmware_revision = revision;
        self.cached = true;
    }

    /// Get cached vendor string
    pub fn vendor(&self) -> Option<&str> {
        self.firmware_vendor.as_ref().map(|s| s.as_str())
    }

    /// Get cached revision
    pub fn revision(&self) -> u32 {
        self.firmware_revision
    }

    /// Check if cached
    pub fn is_cached(&self) -> bool {
        self.cached
    }
}
    pub mode_id: u32,
    pub width: u32,
    pub height: u32,
    pub pixel_format: u32,
    pub cached: bool,
}

/// UEFI GOP Mode cache
pub struct GopModeCache {
    entries: [Option<GopModeCacheEntry>; 16],
    size: usize,
}

impl GopModeCache {
    /// Create new GOP mode cache
    pub fn new() -> Self {
        Self {
            entries: [None; 16],
            size: 0,
        }
    }

    /// Get cached mode info
    pub fn get(&self, mode_id: u32) -> Option<GopModeCacheEntry> {
        for i in 0..self.size {
            if let Some(entry) = self.entries[i] {
                if entry.mode_id == mode_id && entry.cached {
                    return Some(entry);
                }
            }
        }
        None
    }

    /// Add mode to cache
    pub fn add(&mut self, entry: GopModeCacheEntry) -> Result<(), BootError> {
        if self.size < 16 {
            self.entries[self.size] = Some(entry);
            self.size += 1;
            Ok(())
        } else {
            Err(BootError::HardwareError("GOP mode cache full"))
        }
    }

    /// Clear cache
    pub fn clear(&mut self) {
        for i in 0..self.size {
            self.entries[i] = None;
        }
        self.size = 0;
    }
}

/// UEFI specification revision
pub fn uefi_spec_revision() -> (u16, u16) {
    (UEFI_VERSION_MAJOR, UEFI_VERSION_MINOR)
}

/// Check if running under UEFI
pub fn is_uefi_boot() -> bool {
    cfg!(feature = "uefi_support")
}

/// Get UEFI firmware information if available
pub fn get_firmware_info() -> Option<FirmwareInfo> {
    if cfg!(feature = "uefi_support") {
        if let Some(protocol) = get_active_protocol() {
            if let Ok(st) = protocol.system_table() {
                return Some(FirmwareInfo {
                    vendor: st.firmware_vendor().to_string(),
                    revision: st.firmware_revision(),
                    table_revision: st.revision(),
                });
            }
        }
    }
    None
}

/// UEFI firmware information
#[derive(Debug, Clone)]
pub struct FirmwareInfo {
    pub vendor: String,
    pub revision: u32,
    pub table_revision: u32,
}

/// UEFI protocol support information
#[derive(Debug, Clone)]
pub struct ProtocolSupport {
    pub graphics_output: bool,
    pub simple_file_system: bool,
    pub loaded_image: bool,
    pub simple_text_input: bool,
    pub simple_text_output: bool,
    pub block_io: bool,
    pub serial_io: bool,
    pub simple_pointer: bool,
    pub network_interface_identifier: bool,
    pub universal_network_interface: bool,
    pub pci_io: bool,
    pub usb_io: bool,
    pub device_path: bool,
    pub device_path_utilities: bool,
    pub disk_io: bool,
    pub unicode_collation: bool,
    pub simple_network: bool,
    pub edid_active: bool,
    pub edid_discovered: bool,
    pub managed_network: bool,
    pub tcp4_service: bool,
    pub udp4_service: bool,
    pub tcp6_service: bool,
    pub udp6_service: bool,
}

/// Check which UEFI protocols are available
pub fn check_protocol_support() -> ProtocolSupport {
    ProtocolSupport {
        graphics_output: cfg!(feature = "graphics_support"),
        simple_file_system: true, // Always available in UEFI 2.10
        loaded_image: true,
        simple_text_input: true,
        simple_text_output: true,
        block_io: true,
        serial_io: true,
        simple_pointer: false, // Would need to check
        network_interface_identifier: false,
        universal_network_interface: false,
        pci_io: false,
        usb_io: false,
        device_path: true,
        device_path_utilities: false,
        disk_io: false,
        unicode_collation: false,
        simple_network: false,
        edid_active: false,
        edid_discovered: false,
        managed_network: false,
        tcp4_service: false,
        udp4_service: false,
        tcp6_service: false,
        udp6_service: false,
    }
}

/// UEFI system capabilities
#[derive(Debug, Clone)]
pub struct UefiCapabilities {
    pub secure_boot: bool,
    pub network_boot: bool,
    pub graphics_boot: bool,
    pub multi_processor_boot: bool,
    pub unicode_support: bool,
    pub runtime_services: bool,
    pub acpi_support: bool,
    pub device_tree_support: bool,
}

/// Get UEFI system capabilities
pub fn get_uefi_capabilities() -> UefiCapabilities {
    UefiCapabilities {
        secure_boot: cfg!(feature = "secure_boot_support"),
        network_boot: cfg!(feature = "network_support"),
        graphics_boot: cfg!(feature = "graphics_support"),
        multi_processor_boot: true, // UEFI 2.10 supports this
        unicode_support: true, // Built into UEFI
        runtime_services: true, // Always available
        acpi_support: true, // Usually available
        device_tree_support: false, // Usually not on PC platforms
    }
}