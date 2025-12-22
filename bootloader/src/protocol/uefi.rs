//! UEFI Boot Protocol
//! 
//! This module provides the UEFI protocol implementation for the bootloader.

use spin::Mutex;
use crate::utils::error::{BootError, Result};
use crate::protocol::BootInfo;

#[cfg(feature = "uefi_support")]
use uefi::{Handle};
#[cfg(feature = "uefi_support")]
use uefi_raw::table::system::SystemTable;

/// Active UEFI protocol instance (global singleton)
#[cfg(feature = "uefi_support")]
static ACTIVE_PROTOCOL: Mutex<Option<UefiProtocol>> = Mutex::new(None);

/// UEFI Boot Protocol
#[cfg(feature = "uefi_support")]
pub struct UefiProtocol {
    system_table: Option<*const SystemTable>,
    image_handle: Option<Handle>,
    boot_info: Option<BootInfo>,
}

#[cfg(feature = "uefi_support")]
unsafe impl Send for UefiProtocol {}

#[cfg(feature = "uefi_support")]
impl UefiProtocol {
    /// Create a new UEFI protocol instance
    pub fn new() -> Self {
        Self {
            system_table: None,
            image_handle: None,
            boot_info: None,
        }
    }

    /// Initialize the protocol with the UEFI system table
    pub fn initialize_with_system_table(
        &mut self, 
        system_table: *const SystemTable
    ) -> Result<()> {
        if system_table.is_null() {
            return Err(BootError::UefiNullSystemTable);
        }

        self.system_table = Some(system_table);
        Ok(())
    }

    /// Set the UEFI image handle
    pub fn set_image_handle(&mut self, handle: Handle) {
        self.image_handle = Some(handle);
    }

    /// Get the UEFI system table
    pub fn system_table(&self) -> Result<&'static SystemTable> {
        self.system_table
            .ok_or(BootError::UefiSystemTableNotInitialized)
            .map(|ptr| unsafe { &*ptr })
    }

    /// Get framebuffer information from UEFI
    pub fn get_framebuffer_info(&self) -> Result<crate::protocol::FramebufferInfo> {
        // TODO: Implement actual framebuffer info retrieval from UEFI GOP
        Err(BootError::NotImplemented)
    }

    /// Get the boot information
    pub fn boot_info(&self) -> Option<&BootInfo> {
        self.boot_info.as_ref()
    }

    /// Set the boot information
    pub fn set_boot_info(&mut self, info: BootInfo) {
        self.boot_info = Some(info);
    }
}

/// Set the active UEFI protocol instance
#[cfg(feature = "uefi_support")]
pub fn set_active_protocol(protocol: UefiProtocol) {
    let mut active = ACTIVE_PROTOCOL.lock();
    *active = Some(protocol);
}

/// Get the active UEFI protocol instance (if any)
#[cfg(feature = "uefi_support")]
pub fn get_active_protocol() -> Option<spin::MutexGuard<'static, Option<UefiProtocol>>> {
    let guard = ACTIVE_PROTOCOL.lock();
    if guard.is_some() {
        Some(guard)
    } else {
        None
    }
}

/// Initialize UEFI panic handler
pub fn init_uefi_panic_handler() {
    #[cfg(feature = "uefi_support")]
    {
        // Set up UEFI-specific panic handler
        use crate::utils::error_recovery::panic_handler;
        // In UEFI, we use the existing panic handler for now
        let _ = panic_handler;
    }
}

/// Run the UEFI bootloader main logic
pub fn run_uefi_bootloader() -> Result<()> {
    // TODO: Implement actual UEFI bootloader logic
    Err(BootError::NotImplemented)
}
