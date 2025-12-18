// UEFI bootloader support - entry point and minimal initialization

use super::uefi_boot_services::UefiSystemTable;

const UEFI_SYSTEM_TABLE_SIGNATURE: u64 = 0x5453_5953_2049_4249; // "IBI\x20SYS"

pub struct UefiBootInfo {
    pub image_handle: u64,
    pub system_table: u64,
}

impl UefiBootInfo {
    pub fn new(image_handle: u64, system_table: u64) -> Self {
        Self {
            image_handle,
            system_table,
        }
    }

    pub fn system_table_ptr(&self) -> *const UefiSystemTable {
        self.system_table as *const UefiSystemTable
    }

    pub fn is_valid(&self) -> bool {
        if self.image_handle == 0 || self.system_table == 0 {
            return false;
        }

        // Best-effort signature validation. If the pointer is invalid, firmware
        // execution would already be undefined, so we keep this minimal.
        unsafe { (*self.system_table_ptr()).header.signature == UEFI_SYSTEM_TABLE_SIGNATURE }
    }
}

/// UEFI entry point handler (stub)
pub extern "C" fn uefi_entry(image_handle: u64, system_table: u64) -> u32 {
    let boot_info = UefiBootInfo::new(image_handle, system_table);
    if !boot_info.is_valid() {
        return 1;
    }

    crate::drivers::console::write_str("UEFI bootloader initialized\n");
    0
}

/// Detect UEFI boot environment and return minimal boot info.
///
/// NOTE: In a real UEFI build, `image_handle` and `system_table` are provided
/// by firmware via the entry point. This function remains a stub for now.
pub fn detect_uefi_boot_info() -> Option<UefiBootInfo> {
    crate::drivers::console::write_str("Detecting UEFI boot environment\n");
    None
}

/// Minimal UEFI initialization hook (currently stubbed).
pub fn initialize_uefi_loader(boot_info: &UefiBootInfo) -> Result<(), &'static str> {
    if !boot_info.is_valid() {
        return Err("Invalid UEFI boot info");
    }

    crate::drivers::console::write_str("Initializing UEFI loader\n");
    Ok(())
}

/// Unified UEFI kernel load entry (stubbed).
pub fn uefi_load_kernel() -> Result<u64, &'static str> {
    match detect_uefi_boot_info() {
        Some(boot_info) => {
            initialize_uefi_loader(&boot_info)?;
            Ok(0x100000)
        }
        None => {
            crate::drivers::console::write_str(
                "UEFI not detected, falling back to multiboot2\n",
            );
            Err("No UEFI boot info")
        }
    }
}
