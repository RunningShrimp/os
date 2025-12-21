//! UEFI application entry point
//!
//! This module provides the UEFI application entry point that initializes
//! the bootloader and runs the main boot logic.

use crate::error::{BootError, Result};
use crate::protocol::uefi::{UefiProtocol, set_active_protocol};
use crate::protocol::BootProtocol;
use core::ptr;

#[cfg(feature = "uefi_support")]
use uefi::{prelude::*, table::Boot};

/// UEFI application entry point
#[cfg(feature = "uefi_support")]
#[entry]
pub fn efi_main(
    image_handle: uefi::Handle,
    system_table: SystemTable<Boot>,
) -> uefi::Status {
    // Initialize panic handler early
    init_uefi_panic_handler();

    // Create and initialize UEFI protocol
    let mut uefi_protocol = UefiProtocol::new();

    // Initialize with the system table
    if let Err(e) = uefi_protocol.initialize_with_system_table(&system_table) {
        println!("Failed to initialize UEFI protocol: {:?}", e);
        return uefi::Status::ABORTED;
    }

    // Set the active protocol for global access
    set_active_protocol(uefi_protocol);

    // Run the main bootloader logic
    if let Err(e) = run_uefi_bootloader() {
        println!("Bootloader failed: {:?}", e);
        return uefi::Status::ABORTED;
    }

    // If we get here, something went wrong
    uefi::Status::ABORTED
}

/// Run the UEFI bootloader main logic
#[cfg(feature = "uefi_support")]
fn run_uefi_bootloader() -> Result<()> {
    println!("NOS UEFI Bootloader v0.1.0");
    println!("================================");

    // Initialize bootloader components
    initialize_bootloader_components()?;

    // Run main bootloader loop
    run_main_bootloader_loop()?;

    Ok(())
}

/// Initialize bootloader components for UEFI
#[cfg(feature = "uefi_support")]
fn initialize_bootloader_components() -> Result<()> {
    // Initialize architecture-specific components
    crate::arch::early_init();

    // Initialize memory management
    println!("[boot] Initializing memory management...");
    let memory_manager = create_uefi_memory_manager()?;

    // Initialize protocol manager
    println!("[boot] Initializing protocol manager...");
    let protocol_manager = create_uefi_protocol_manager()?;

    // Initialize secure boot if enabled
    if cfg!(feature = "secure_boot_support") {
        println!("[boot] Initializing secure boot...");
        initialize_uefi_secure_boot()?;
    }

    // Initialize graphics if available
    if cfg!(feature = "graphics_support") {
        println!("[boot] Initializing graphics...");
        initialize_uefi_graphics()?;
    }

    // Initialize file system for kernel loading
    println!("[boot] Initializing file system...");
    initialize_uefi_file_system()?;

    println!("[boot] UEFI bootloader initialization complete");
    Ok(())
}

/// Create memory manager for UEFI
#[cfg(feature = "uefi_support")]
fn create_uefi_memory_manager() -> Result<crate::memory::BootMemoryManager> {
    let arch = crate::arch::Architecture::current();
    crate::memory::BootMemoryManager::new(arch)
}

/// Create protocol manager for UEFI
#[cfg(feature = "uefi_support")]
fn create_uefi_protocol_manager() -> Result<crate::protocol::ProtocolManager> {
    let mut manager = crate::protocol::ProtocolManager::new();

    // The UEFI protocol should already be initialized
    // The manager will detect it during detection phase

    Ok(manager)
}

/// Initialize UEFI Secure Boot
#[cfg(feature = "uefi_support")]
fn initialize_uefi_secure_boot() -> Result<()> {
    use crate::uefi::secure_boot::SecureBootManager;

    if let Some(uefi_protocol) = crate::protocol::uefi::get_active_protocol() {
        if let Ok(system_table) = uefi_protocol.system_table() {
            let mut secure_boot_manager = SecureBootManager::new();
            secure_boot_manager.initialize(system_table)?;

            let status = secure_boot_manager.get_status_summary();
            println!("[secure_boot] Status summary: {:?}", status);

            return Ok(());
        }
    }

    Err(BootError::UefiNotFound)
}

/// Initialize UEFI graphics
#[cfg(feature = "uefi_support")]
fn initialize_uefi_graphics() -> Result<()> {
    if let Some(uefi_protocol) = crate::protocol::uefi::get_active_protocol() {
        if let Ok(_framebuffer) = uefi_protocol.get_framebuffer_info() {
            println!("[graphics] UEFI Graphics Output Protocol available");
            // Initialize graphics subsystem here
        } else {
            println!("[graphics] No UEFI Graphics Output Protocol available");
        }
    }

    Ok(())
}

/// Initialize UEFI file system
#[cfg(feature = "uefi_support")]
fn initialize_uefi_file_system() -> Result<()> {
    println!("[filesystem] UEFI file system support initialized");
    // File system operations will be available when needed
    Ok(())
}

/// Run main bootloader loop
#[cfg(feature = "uefi_support")]
fn run_main_bootloader_loop() -> Result<()> {
    println!("[boot] Starting main bootloader logic");

    // Create protocol manager
    let mut protocol_manager = create_uefi_protocol_manager()?;

    // Detect and initialize boot protocol
    let protocol_type = protocol_manager.detect_and_initialize()?;
    println!("[boot] Detected boot protocol: {:?}", protocol_type);

    // Display boot menu if enabled
    #[cfg(feature = "menu_support")]
    {
        return handle_uefi_boot_menu(&mut protocol_manager);
    }

    // Default boot logic
    load_and_boot_kernel_uefi(&mut protocol_manager)
}

/// Handle UEFI boot menu
#[cfg(all(feature = "uefi_support", feature = "menu_support"))]
fn handle_uefi_boot_menu(protocol_manager: &mut crate::protocol::ProtocolManager) -> Result<()> {
    println!("[menu] UEFI boot menu not yet implemented");
    // For now, proceed with default boot
    load_and_boot_kernel_uefi(protocol_manager)
}

/// Load and boot kernel in UEFI
#[cfg(feature = "uefi_support")]
fn load_and_boot_kernel_uefi(protocol_manager: &mut crate::protocol::ProtocolManager) -> Result<()> {
    println!("[boot] Loading NOS kernel...");

    // Try to load kernel from default locations
    let kernel_paths = [
        "EFI\\BOOT\\nos_kernel.efi",
        "nos_kernel.efi",
        "kernel.efi",
    ];

    for path in &kernel_paths {
        match protocol_manager.load_kernel(path) {
            Ok(kernel_image) => {
                println!("[boot] Kernel loaded successfully from: {}", path);
                println!("[boot] Entry point: {:#x}", kernel_image.entry_point);

                // Get boot information
                let boot_info = protocol_manager.get_boot_info()?;

                // Create boot parameters
                let boot_params = crate::kernel::KernelBootParameters::new(&boot_info, &kernel_image);

                // Exit boot services
                protocol_manager.exit_boot_services()?;

                // Jump to kernel
                println!("[boot] Jumping to kernel...");
                unsafe {
                    crate::arch::jump_to_kernel(kernel_image.entry_point, &boot_params.into_struct());
                }

                unreachable!();
            }
            Err(e) => {
                println!("[boot] Failed to load kernel from {}: {:?}", path, e);
                continue;
            }
        }
    }

    Err(BootError::KernelNotFound)
}

/// Initialize UEFI panic handler
#[cfg(feature = "uefi_support")]
fn init_uefi_panic_handler() {
    // Set up a panic handler that prints to UEFI console
    // This is a simplified version
}

// Non-UEFI stub implementations
#[cfg(not(feature = "uefi_support"))]
#[no_mangle]
pub extern "C" fn efi_main() -> u32 {
    0xDEADBEEF // Error code for UEFI not supported
}

extern crate alloc;
#[cfg(not(feature = "uefi_support"))]
fn run_uefi_bootloader() -> Result<()> {
    Err(BootError::FeatureNotEnabled("UEFI support"))
}