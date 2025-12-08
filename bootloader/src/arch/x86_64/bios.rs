//! x86_64 BIOS Bootloader Assembly Bindings
//!
//! This module contains Rust functions that are called from the assembly
//! entry points and provides the interface between low-level assembly
//! and high-level Rust bootloader code.

use crate::error::{BootError, Result};
use crate::memory::bios::BiosMemoryManager;
use crate::protocol::{ProtocolManager, ProtocolType};
use crate::boot_menu;
use core::panic::PanicInfo;
use core::ptr;

/// External assembly symbols
extern "C" {
    static bss_start: u8;
    static bss_end: u8;
    static saved_sp: u32;
    static saved_ss: u32;
    static boot_cmdline: [u8; 256];
    static mut boot_cmdline_len: u32;
    static kernel_image_path: [u8; 256];
    static mut kernel_image_path_len: u32;
    static mut boot_timeout: u32;
    static mut boot_flags: u32;
    static mut memory_map_buffer: [u8; 4096];
    static mut memory_map_entries: u32;
    static mut memory_map_size: u32;
    static mut framebuffer_info: [u64; 5]; // address, width, height, bpp, pitch
    static mut acpi_rsdp: u64;
    static mut bios_version: u32;
    static mut cpu_vendor: [u8; 16];
    static mut cpu_features: u64;
    static mut system_memory: u64;
    static mut available_memory: u64;
    static mut debug_enabled: u32;
    static mut debug_buffer: [u8; 4096];
    static mut debug_buffer_pos: u32;
}

/// Global state for the bootloader
static mut BOOTLOADER_STATE: Option<BootloaderState> = None;

/// Bootloader state structure
struct BootloaderState {
    memory_manager: BiosMemoryManager,
    protocol_manager: ProtocolManager,
    initialized: bool,
    in_protected_mode: bool,
    in_long_mode: bool,
}

impl BootloaderState {
    fn new() -> Self {
        Self {
            memory_manager: BiosMemoryManager::new(),
            protocol_manager: ProtocolManager::new(),
            initialized: false,
            in_protected_mode: false,
            in_long_mode: false,
        }
    }
}

/// Rust memory detection function called from assembly
#[no_mangle]
pub extern "C" fn rust_detect_memory() -> u32 {
    println!("[rust] Starting memory detection...");

    unsafe {
        if BOOTLOADER_STATE.is_none() {
            BOOTLOADER_STATE = Some(BootloaderState::new());
        }

        if let Some(state) = &mut BOOTLOADER_STATE {
            match state.memory_manager.initialize() {
                Ok(_) => {
                    let total_memory = state.memory_manager.get_scanner().get_total_memory();
                    let base_memory = state.memory_manager.get_scanner().get_base_memory_kb();
                    let extended_memory = state.memory_manager.get_scanner().get_extended_memory_kb();

                    // Update global variables
                    system_memory = total_memory as u64;
                    available_memory = (total_memory / 1024) as u64; // in KB

                    println!("[rust] Memory detection successful:");
                    println!("[rust]   Total memory: {} MB", total_memory / (1024 * 1024));
                    println!("[rust]   Base memory: {} KB", base_memory);
                    println!("[rust]   Extended memory: {} KB", extended_memory);

                    1 // Success
                }
                Err(e) => {
                    println!("[rust] Memory detection failed: {:?}", e);
                    0 // Failure
                }
            }
        } else {
            0 // State not initialized
        }
    }
}

/// Protected mode entry function called from assembly
#[no_mangle]
pub extern "C" fn rust_protected_mode_entry() {
    println!("[rust] Entering protected mode...");

    unsafe {
        if let Some(state) = &mut BOOTLOADER_STATE {
            state.in_protected_mode = true;

            // Continue with bootloader initialization
            match bootloader_main_protected(state) {
                Ok(_) => {
                    println!("[rust] Protected mode initialization successful");
                }
                Err(e) => {
                    println!("[rust] Protected mode initialization failed: {:?}", e);
                    bootloader_halt();
                }
            }
        }
    }
}

/// Main bootloader function in protected mode
fn bootloader_main_protected(state: &mut BootloaderState) -> Result<()> {
    println!("[rust] Starting protected mode bootloader main...");

    // Initialize protocol manager
    state.protocol_manager.detect_and_initialize()?;

    // Get boot information
    let boot_info = state.protocol_manager.get_boot_info()?;

    // Setup framebuffer information for assembly
    if let Some(fb_info) = &boot_info.framebuffer {
        unsafe {
            framebuffer_info[0] = fb_info.address as u64;
            framebuffer_info[1] = fb_info.width as u64;
            framebuffer_info[2] = fb_info.height as u64;
            framebuffer_info[3] = fb_info.bytes_per_pixel as u64;
            framebuffer_info[4] = fb_info.stride as u64;
        }
    }

    // Setup ACPI information
    if let Some(rsdp) = &boot_info.acpi_rsdp {
        unsafe {
            acpi_rsdp = *rsdp as u64;
        }
    }

    // Create and display boot menu
    let boot_menu_config = boot_menu::create_default_config();
    let mut boot_menu = boot_menu::bios::BiosBootMenu::new(boot_menu_config);

    // Initialize boot menu with framebuffer info
    let fb_info = if unsafe { framebuffer_info[0] != 0 } {
        Some(crate::protocol::FramebufferInfo {
            address: unsafe { framebuffer_info[0] as usize },
            width: unsafe { framebuffer_info[1] as u32 },
            height: unsafe { framebuffer_info[2] as u32 },
            bytes_per_pixel: unsafe { framebuffer_info[3] as u32 },
            stride: unsafe { framebuffer_info[4] as u32 },
            pixel_format: crate::protocol::PixelFormat::RGB,
        })
    } else {
        None
    };

    match boot_menu.initialize(fb_info) {
        Ok(_) => {
            // Display boot menu and get selected entry
            match boot_menu.display_menu() {
                Ok(selected_entry) => {
                    println!("[rust] Selected boot entry: {}", selected_entry.name);

                    // Update boot parameters
                    unsafe {
                        // Copy kernel path
                        let kernel_path_bytes = selected_entry.kernel_path.as_bytes();
                        kernel_image_path[..kernel_path_bytes.len().min(255)].copy_from_slice(&kernel_path_bytes[..kernel_path_bytes.len().min(255)]);
                        kernel_image_path_len = kernel_path_bytes.len() as u32;

                        // Copy command line
                        let cmdline_bytes = selected_entry.cmdline.as_bytes();
                        boot_cmdline[..cmdline_bytes.len().min(255)].copy_from_slice(&cmdline_bytes[..cmdline_bytes.len().min(255)]);
                        boot_cmdline_len = cmdline_bytes.len() as u32;

                        // Set timeout
                        boot_timeout = selected_entry.timeout as u32;
                    }

                    // Prepare to boot the selected entry
                    return prepare_kernel_boot(state, selected_entry);
                }
                Err(e) => {
                    println!("[rust] Boot menu failed: {:?}", e);
                    return Err(e);
                }
            }
        }
        Err(e) => {
            println!("[rust] Boot menu initialization failed: {:?}", e);
            return Err(e);
        }
    }
}

/// Prepare to boot the selected kernel entry
fn prepare_kernel_boot(_state: &BootloaderState, entry: &boot_menu::BootMenuEntry) -> Result<()> {
    println!("[rust] Preparing to boot kernel: {}", entry.name);
    println!("[rust] Kernel path: {}", entry.kernel_path);
    println!("[rust] Command line: {}", entry.cmdline);

    // In a real implementation, this would:
    // 1. Load the kernel from the specified path
    // 2. Validate the kernel image
    // 3. Set up Multiboot2 information structure
    // 4. Transition to long mode if needed
    // 5. Jump to kernel entry point

    println!("[rust] Kernel loading not implemented - halting");
    bootloader_halt();

    // Ok(())
}

/// Main Rust bootloader entry point (for UEFI or direct Rust execution)
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("[rust] NOS Bootloader v0.1.0");
    println!("[rust] Architecture: x86_64");

    // Initialize bootloader state
    unsafe {
        BOOTLOADER_STATE = Some(BootloaderState::new());

        if let Some(state) = &mut BOOTLOADER_STATE {
            state.in_long_mode = true;

            // Continue with long mode initialization
            match bootloader_main_long(state) {
                Ok(_) => {
                    println!("[rust] Long mode bootloader main completed");
                }
                Err(e) => {
                    println!("[rust] Long mode bootloader main failed: {:?}", e);
                }
            }
        }
    }

    bootloader_halt();
}

/// Main bootloader function in long mode
fn bootloader_main_long(_state: &mut BootloaderState) -> Result<()> {
    println!("[rust] Running in long mode");

    // Long mode specific initialization would go here
    // For now, we'll just halt

    Ok(())
}

/// Halt the bootloader
pub fn bootloader_halt() -> ! {
    println!("[rust] Halting system");

    loop {
        unsafe {
            // Disable interrupts and halt
            core::arch::asm!("cli; hlt; jmp .", options(nomem, nostack));
        }
    }
}

/// Reboot the system
pub fn bootloader_reboot() -> ! {
    println!("[rust] Rebooting system");

    unsafe {
        // Use keyboard controller reset
        let mut value: u8;
        core::arch::asm!(
            "in al, dx",
            out("al") value,
            in("dx") 0x64u16,
            options(nomem, nostack, preserves_flags)
        );

        // Wait for keyboard controller ready
        for _ in 0..100000 {
            core::arch::asm!(
                "in al, dx",
                out("al") value,
                in("dx") 0x64u16,
                options(nomem, nostack, preserves_flags)
            );
            if value & 0x02 == 0 {
                break;
            }
        }

        // Write reset command
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x64u16,
            in("al") 0xFEu8,
            options(nomem, nostack, preserves_flags)
        );

        // Should not reach here
        loop {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

/// Custom panic handler for the bootloader
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[rust] PANIC: {}", info);
    bootloader_halt();
}

/// Debug output function
#[no_mangle]
pub extern "C" fn rust_debug_print(msg: *const u8, len: usize) {
    unsafe {
        if debug_enabled != 0 {
            let msg_slice = core::slice::from_raw_parts(msg, len);
            let msg_str = core::str::from_utf8_unchecked(msg_slice);

            // Add to debug buffer
            let pos = debug_buffer_pos as usize;
            if pos + len < debug_buffer.len() {
                debug_buffer[pos..pos + len].copy_from_slice(msg_slice);
                debug_buffer_pos += len as u32;
            }

            // Print to console
            println!("[debug] {}", msg_str);
        }
    }
}

/// Get BIOS information
pub fn get_bios_info() -> BiosInfo {
    unsafe {
        BiosInfo {
            version: bios_version,
            memory_total: system_memory,
            memory_available: available_memory,
            cpu_vendor: core::str::from_utf8_unchecked(&cpu_vendor[..15]),
            cpu_features: cpu_features,
        }
    }
}

/// BIOS information structure
#[derive(Debug)]
pub struct BiosInfo {
    pub version: u32,
    pub memory_total: u64,
    pub memory_available: u64,
    pub cpu_vendor: &'static str,
    pub cpu_features: u64,
}

/// CPU information structure
#[derive(Debug)]
pub struct CpuInfo {
    pub vendor: [u8; 12],
    pub family: u8,
    pub model: u8,
    pub stepping: u8,
    pub features: u64,
}

/// Get CPU information
pub fn get_cpu_info() -> CpuInfo {
    let mut cpu_info = CpuInfo {
        vendor: [0; 12],
        family: 0,
        model: 0,
        stepping: 0,
        features: 0,
    };

    unsafe {
        if let Some(0x80000001..=u32::MAX) = cpuid_max_extended() {
            let mut ebx: u32;
            let mut ecx: u32;
            let mut edx: u32;

            core::arch::asm!(
                "cpuid",
                in("eax") 0x80000002u32,
                out("ebx") ebx,
                out("ecx") ecx,
                out("edx") edx,
            );

            let vendor_bytes = [
                (ebx & 0xFF) as u8, ((ebx >> 8) & 0xFF) as u8, ((ebx >> 16) & 0xFF) as u8, ((ebx >> 24) & 0xFF) as u8,
                (edx & 0xFF) as u8, ((edx >> 8) & 0xFF) as u8, ((edx >> 16) & 0xFF) as u8, ((edx >> 24) & 0xFF) as u8,
                (ecx & 0xFF) as u8, ((ecx >> 8) & 0xFF) as u8, ((ecx >> 16) & 0xFF) as u8, ((ecx >> 24) & 0xFF) as u8,
            ];

            cpu_info.vendor.copy_from_slice(&vendor_bytes);

            core::arch::asm!(
                "cpuid",
                in("eax") 0x80000001u32,
                out("edx") edx,
                out("ecx") ecx,
            );

            cpu_info.family = ((edx >> 8) & 0xF) as u8;
            cpu_info.model = ((edx >> 4) & 0xF) as u8;
            cpu_info.stepping = (edx & 0xF) as u8;
            cpu_info.features = ((edx as u64) << 32) | (ecx as u64);
        }
    }

    cpu_info
}

/// Get maximum extended CPUID function
unsafe fn cpuid_max_extended() -> Option<u32> {
    let mut eax: u32;
    let mut ebx: u32;
    let mut ecx: u32;
    let mut edx: u32;

    core::arch::asm!(
        "cpuid",
        in("eax") 0x80000000u32,
        out("eax") eax,
        out("ebx") ebx,
        out("ecx") ecx,
        out("edx") edx,
    );

    if eax >= 0x80000000 {
        Some(eax)
    } else {
        None
    }
}

/// Check if we're running in a virtual machine
pub fn is_virtual_machine() -> bool {
    let mut hypervisor_present = false;

    unsafe {
        if let Some(0x1..=u32::MAX) = cpuid_max_extended() {
            let mut ecx: u32;
            let mut edx: u32;

            core::arch::asm!(
                "cpuid",
                in("eax") 0x1u32,
                out("ecx") ecx,
                out("edx") edx,
            );

            // Check hypervisor bit
            if ecx & (1 << 31) != 0 {
                hypervisor_present = true;
            }
        }

        if !hypervisor_present && cpuid_max_extended().map_or(false, |max| max >= 0x40000000) {
            let mut ebx: u32;
            let mut ecx: u32;
            let mut edx: u32;

            core::arch::asm!(
                "cpuid",
                in("eax") 0x40000000u32,
                out("ebx") ebx,
                out("ecx") ecx,
                out("edx") edx,
            );

            // Check hypervisor leaf
            let vendor = [
                (ebx & 0xFF) as u8, ((ebx >> 8) & 0xFF) as u8, ((ebx >> 16) & 0xFF) as u8, ((ebx >> 24) & 0xFF) as u8,
                (ecx & 0xFF) as u8, ((ecx >> 8) & 0xFF) as u8, ((ecx >> 16) & 0xFF) as u8, ((ecx >> 24) & 0xFF) as u8,
                (edx & 0xFF) as u8, ((edx >> 8) & 0xFF) as u8, ((edx >> 16) & 0xFF) as u8, ((edx >> 24) & 0xFF) as u8,
            ];

            let vendor_str = core::str::from_utf8_unchecked(&vendor);
            hypervisor_present = vendor_str.trim_matches('\0') != "";
        }
    }

    hypervisor_present
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_info() {
        let cpu_info = get_cpu_info();
        // CPU vendor should be filled
        assert_ne!(cpu_info.vendor, [0; 12]);
    }

    #[test]
    fn test_bios_info() {
        let bios_info = get_bios_info();
        // BIOS version should be accessible
        println!("BIOS Version: 0x{:08X}", bios_info.version);
    }

    #[test]
    fn test_virtual_machine_detection() {
        let is_vm = is_virtual_machine();
        println!("Running in virtual machine: {}", is_vm);
    }
}