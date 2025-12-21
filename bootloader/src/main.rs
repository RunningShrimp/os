//! NOS Bootloader - Modern UEFI and BIOS bootloader
//!
//! This is the main entry point for the NOS operating system bootloader.
//! It provides support for both UEFI and traditional BIOS boot methods
//! across multiple architectures (x86_64, AArch64, RISC-V).

#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// Core modules
mod arch;
mod error;
mod memory;
mod protocol;

// Architecture-specific modules
#[cfg(target_arch = "x86_64")]
use arch::x86_64;
#[cfg(target_arch = "aarch64")]
use arch::aarch64;
#[cfg(target_arch = "riscv64")]
use arch::riscv64;

// Protocol-specific modules
#[cfg(feature = "uefi_support")]
mod uefi;
#[cfg(feature = "bios_support")]
use protocol::bios;

// Feature-specific modules
#[cfg(feature = "graphics_support")]
mod graphics;
#[cfg(feature = "menu_support")]
mod boot_menu;
#[cfg(feature = "network_support")]
mod network;
#[cfg(feature = "recovery_support")]
mod recovery;

// Boot protocol modules
mod kernel;

use core::panic::PanicInfo;
use error::{BootError, Result};

/// Global bootloader state
static mut BOOTLOADER_STATE: Option<BootloaderState> = None;

/// Bootloader runtime state
struct BootloaderState {
    protocol_type: ProtocolType,
    architecture: Architecture,
    memory_manager: memory::BootMemoryManager,
    protocol_manager: protocol::ProtocolManager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProtocolType {
    Unknown,
    UEFI,
    BIOS,
    Multiboot2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Architecture {
    X86_64,
    AArch64,
    RiscV64,
}

/// Bootloader entry point - called from architecture-specific startup code
#[no_mangle]
pub extern "C" fn boot_main() -> ! {
    // Initialize early architecture-specific setup
    arch::early_init();

    // Initialize the bootloader state
    if let Err(error) = initialize_bootloader() {
        panic!("Failed to initialize bootloader: {:?}", error);
    }

    // Main bootloader loop
    if let Err(error) = run_bootloader() {
        // Try to enter recovery mode on error
        if cfg!(feature = "recovery_support") {
            recovery::enter_recovery_mode(&error);
        }
        panic!("Bootloader failed: {:?}", error);
    }

    // Should never reach here
    panic!("Bootloader unexpectedly returned");
}

/// Initialize bootloader components
fn initialize_bootloader() -> Result<()> {
    // Detect current architecture
    let architecture = detect_architecture();

    // Initialize memory manager
    let memory_manager = memory::BootMemoryManager::new(architecture)?;

    // Initialize protocol manager
    let mut protocol_manager = protocol::ProtocolManager::new();

    // Detect and initialize boot protocol
    let protocol_type = protocol_manager.detect_and_initialize()?;

    // Set up global state
    unsafe {
        BOOTLOADER_STATE = Some(BootloaderState {
            protocol_type,
            architecture,
            memory_manager,
            protocol_manager,
        });
    }

    println!("NOS Bootloader v0.1.0 initialized");
    println!("Architecture: {:?}", architecture);
    println!("Protocol: {:?}", protocol_type);

    Ok(())
}

/// Run the main bootloader logic
fn run_bootloader() -> Result<()> {
    let state = unsafe { BOOTLOADER_STATE.as_mut().ok_or(BootError::NotInitialized)? };

    // Display boot menu if enabled
    #[cfg(feature = "menu_support")]
    {
        let boot_action = menu::display_boot_menu(&mut state.memory_manager)?;
        match boot_action {
            menu::BootAction::BootKernel(kernel_path) => {
                return load_and_boot_kernel(&kernel_path);
            }
            menu::BootAction::EnterRecovery => {
                #[cfg(feature = "recovery_support")]
                return recovery::enter_recovery_mode(&BootError::UserRequestedRecovery);
                #[cfg(not(feature = "recovery_support"))]
                return Err(BootError::RecoveryNotSupported);
            }
            menu::BootAction::Reboot => {
                arch::reboot();
            }
            menu::BootAction::Shutdown => {
                arch::shutdown();
            }
        }
    }

    // Default: try to load kernel from default locations
    load_and_boot_kernel("kernel.bin")
}

/// Load and boot the NOS kernel
fn load_and_boot_kernel(kernel_path: &str) -> Result<()> {
    let state = unsafe { BOOTLOADER_STATE.as_mut().ok_or(BootError::NotInitialized)? };

    println!("Loading kernel from: {}", kernel_path);

    // Load kernel image
    let kernel_image = state.protocol_manager.load_kernel(kernel_path)?;

    // Get boot information from protocol
    let boot_info = state.protocol_manager.get_boot_info()?;

    // Create boot parameters for the kernel
    let boot_params = kernel::KernelBootParameters::new(&boot_info, &kernel_image);

    println!("Kernel loaded successfully");
    println!("Entry point: {:#x}", kernel_image.entry_point);
    println!("Memory layout configured");

    // Boot the kernel - this should never return
    unsafe {
        arch::jump_to_kernel(kernel_image.entry_point, &boot_params);
    }

    // Should never reach here
    Err(BootError::KernelReturned)
}

/// Detect the current architecture
fn detect_architecture() -> Architecture {
    #[cfg(target_arch = "x86_64")]
    return Architecture::X86_64;

    #[cfg(target_arch = "aarch64")]
    return Architecture::AArch64;

    #[cfg(target_arch = "riscv64")]
    return Architecture::RiscV64;

    #[allow(unreachable_code)]
    {
        panic!("Unsupported target architecture");
    }
}

/// Bootloader panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("!! BOOTLOADER PANIC !!");

    if let Some(location) = info.location() {
        println!("Location: {}:{}:{}", location.file(), location.line(), location.column());
    }

    if let Some(message) = info.message() {
        println!("Message: {}", message);
    }

    // Try to display a simple error screen
    #[cfg(feature = "graphics_support")]
    {
        let _ = graphics::display_error_screen("Bootloader Panic", message);
    }

    // Halt the system
    arch::halt();
}

/// Simple println macro for bootloader debugging
macro_rules! println {
    ($($arg:tt)*) => ({
        #[cfg(feature = "uefi_support")]
        {
            if let Some(uefi_proto) = crate::protocol::uefi::get_active_protocol() {
                let _ = uefi_proto.println(format_args!($($arg)*));
            }
        }

        #[cfg(feature = "bios_support")]
        {
            // BIOS VGA text mode output
            crate::bios::console::print(format_args!($($arg)*));
            crate::bios::console::print("\n");
        }

        #[cfg(not(any(feature = "uefi_support", feature = "bios_support")))]
        {
            // No output available - just compile-time ignore
            let _ = format_args!($($arg)*);
        }
    });
}