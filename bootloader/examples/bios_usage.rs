//! BIOS Bootloader Usage Example
//!
//! This example demonstrates how to use the NOS BIOS bootloader
//! for loading and booting an operating system kernel.

#![no_std]
#![no_main]

use nos_bootloader::{
    arch::x86_64::{X86_64CpuInfo, X86_64Utils},
    memory::bios::{BiosMemoryManager, BiosMemoryScanner},
    graphics::vbe::{VbeController, VbeGraphicsManager},
    boot_menu::{create_default_config, BootMenuConfig, BootMenuEntry},
    protocol::multiboot2::{Multiboot2Protocol, create_e820_entry},
    error::BootError,
    result::Result,
};

/// Main BIOS bootloader entry point
#[no_mangle]
pub extern "C" fn bios_main() -> ! {
    println!("=== NOS BIOS Bootloader Starting ===");

    // Step 1: Hardware initialization
    if let Err(e) = initialize_hardware() {
        println!("Hardware initialization failed: {:?}", e);
        bootloader_halt();
    }

    // Step 2: Memory detection and management
    let memory_manager = match setup_memory_management() {
        Ok(manager) => manager,
        Err(e) => {
            println!("Memory management setup failed: {:?}", e);
            bootloader_halt();
        }
    };

    // Step 3: Graphics initialization
    let framebuffer_info = setup_graphics();

    // Step 4: Display boot menu
    let boot_entry = match display_boot_menu(framebuffer_info.as_ref()) {
        Ok(entry) => entry,
        Err(e) => {
            println!("Boot menu failed: {:?}", e);
            bootloader_halt();
        }
    };

    // Step 5: Load kernel
    let kernel_image = match load_kernel(&boot_entry.kernel_path, &memory_manager) {
        Ok(image) => image,
        Err(e) => {
            println!("Kernel loading failed: {:?}", e);
            bootloader_halt();
        }
    };

    // Step 6: Prepare Multiboot2 information
    let multiboot_info = prepare_multiboot2_info(&boot_entry, &memory_manager);

    // Step 7: Boot the kernel
    println!("Booting kernel: {}", boot_entry.name);
    println!("Kernel path: {}", boot_entry.kernel_path);
    println!("Command line: {}", boot_entry.cmdline);
    println!("Kernel entry point: {:#X}", kernel_image.entry_point);

    // Jump to kernel
    unsafe {
        jump_to_kernel(kernel_image.entry_point, multiboot_info);
    }
}

/// Initialize hardware components
fn initialize_hardware() -> Result<()> {
    println!("Initializing hardware...");

    // Disable interrupts during initialization
    X86_64Utils::disable_interrupts();

    // Detect CPU capabilities
    let mut cpu_info = X86_64CpuInfo::new();
    cpu_info.detect()?;

    println!("CPU: {}", cpu_info.vendor_str());
    println!("Brand: {}", cpu_info.brand_str());
    println!("Long Mode Support: {}", cpu_info.supports_long_mode());
    println!("SSE2 Support: {}", cpu_info.supports_sse2());

    if !cpu_info.supports_long_mode() {
        return Err(BootError::HardwareError("Long mode not supported"));
    }

    // Enable A20 gate for extended memory access
    println!("Enabling A20 gate...");
    // In a real implementation, this would call BIOS interrupts

    println!("Hardware initialization complete");
    Ok(())
}

/// Setup memory management
fn setup_memory_management() -> Result<BiosMemoryManager> {
    println!("Setting up memory management...");

    let mut memory_manager = BiosMemoryManager::new();
    memory_manager.initialize()?;

    let scanner = memory_manager.get_scanner();
    println!("Base memory: {} KB", scanner.get_base_memory_kb());
    println!("Extended memory: {} KB", scanner.get_extended_memory_kb());
    println!("Total memory: {} MB", scanner.get_total_memory() / (1024 * 1024));

    // Print memory map summary
    let memory_map = memory_manager.get_memory_map()?;
    println!("Memory map entries: {}", memory_map.entries.len());

    for (i, entry) in memory_map.entries.iter().enumerate() {
        let mem_type = match entry.mem_type {
            nos_bootloader::protocol::MemoryType::Usable => "Usable",
            nos_bootloader::protocol::MemoryType::Reserved => "Reserved",
            nos_bootloader::protocol::MemoryType::ACPIReclaimable => "ACPI Reclaimable",
            _ => "Other",
        };

        println!("  {}: {:#018X}-{:#018X} {} ({}) MB",
                i,
                entry.base,
                entry.base + entry.size,
                mem_type,
                entry.size / (1024 * 1024));
    }

    println!("Memory management setup complete");
    Ok(memory_manager)
}

/// Setup graphics and display
fn setup_graphics() -> Option<nos_bootloader::protocol::FramebufferInfo> {
    println!("Setting up graphics...");

    let mut vbe_controller = VbeController::new();

    match vbe_controller.initialize() {
        Ok(()) => {
            println!("VBE controller initialized");

            // Try to set a common graphics mode
            match vbe_controller.set_graphics_mode(1024, 768, 32) {
                Ok(fb_info) => {
                    println!("Graphics mode set: {}x{}x{}",
                            fb_info.width,
                            fb_info.height,
                            fb_info.bytes_per_pixel * 8);
                    println!("Framebuffer address: {:#X}", fb_info.address);
                    return Some(fb_info);
                }
                Err(e) => {
                    println!("Failed to set graphics mode: {:?}, using text mode", e);
                    None
                }
            }
        }
        Err(e) => {
            println!("VBE initialization failed: {:?}, using text mode", e);
            None
        }
    }
}

/// Display boot menu and get user selection
fn display_boot_menu(framebuffer_info: Option<&nos_bootloader::protocol::FramebufferInfo>) -> Result<BootMenuEntry> {
    println!("Displaying boot menu...");

    // Create custom boot menu configuration
    let mut config = BootMenuConfig::new();
    config.title = "NOS Operating System - BIOS Bootloader".to_string();
    config.global_timeout = 10; // 10 seconds timeout
    config.graphical = framebuffer_info.is_some();

    // Add boot entries
    config.add_entry(BootMenuEntry::default_entry(
        "NOS OS - Normal Boot".to_string(),
        "boot/kernel.bin".to_string(),
        "root=/dev/sda1 quiet splash".to_string(),
    ).with_timeout(10));

    config.add_entry(BootMenuEntry::new(
        "NOS OS - Recovery Mode".to_string(),
        "boot/kernel.bin".to_string(),
        "root=/dev/sda1 single recovery".to_string(),
    ).with_timeout(5));

    config.add_entry(BootMenuEntry::new(
        "NOS OS - Debug Mode".to_string(),
        "boot/kernel.bin".to_string(),
        "root=/dev/sda1 debug=on console=ttyS0".to_string(),
    ));

    config.add_entry(BootMenuEntry::new(
        "NOS OS - Safe Mode".to_string(),
        "boot/kernel.bin".to_string(),
        "root=/dev/sda1 nomodeset acpi=off".to_string(),
    ));

    config.add_entry(BootMenuEntry::new(
        "Memory Test".to_string(),
        "boot/memtest.bin".to_string(),
        "".to_string(),
    ));

    // Validate configuration
    config.validate()?;

    // Create boot menu instance
    let mut boot_menu = nos_bootloader::boot_menu::create_boot_menu(config)?;

    // Initialize and display the menu
    boot_menu.initialize()?;

    // Get user selection
    let selected_entry = boot_menu.display_menu()?;
    println!("Selected: {}", selected_entry.name);

    Ok(selected_entry.clone())
}

/// Load kernel from specified path
fn load_kernel(path: &str, memory_manager: &BiosMemoryManager) -> Result<nos_bootloader::protocol::KernelImage> {
    println!("Loading kernel from: {}", path);

    // In a real implementation, this would load the kernel from disk
    // For this example, we'll simulate kernel loading

    let kernel_size = 1024 * 1024; // 1MB kernel
    let kernel_base = 0x100000; // 1MB mark

    // Allocate memory for kernel
    let kernel_addr = memory_manager.allocate_kernel_space(kernel_size)?;
    println!("Kernel loaded at: {:#X}, size: {} KB", kernel_addr, kernel_size / 1024);

    // Create kernel image structure
    let kernel_image = nos_bootloader::protocol::KernelImage::new(
        kernel_addr,
        kernel_addr, // Entry point same as load address for simplicity
        vec![0u8; kernel_size], // Placeholder kernel data
    );

    // Validate kernel image
    kernel_image.validate()?;

    println!("Kernel loaded successfully");
    Ok(kernel_image)
}

/// Prepare Multiboot2 information structure
fn prepare_multiboot2_info(
    boot_entry: &BootMenuEntry,
    memory_manager: &BiosMemoryManager,
) -> u64 {
    println!("Preparing Multiboot2 information...");

    // Create Multiboot2 protocol instance
    let mut buffer = [0u8; 8192];
    let mut multiboot2_protocol = Multiboot2Protocol::new();
    multiboot2_protocol.initialize(&mut buffer[..], buffer.len())?;

    // Build E820 memory map entries
    let memory_map = memory_manager.get_memory_map()?;
    let e820_entries: Vec<_> = memory_map.entries
        .iter()
        .map(|entry| {
            let mem_type = match entry.mem_type {
                nos_bootloader::protocol::MemoryType::Usable => 1,
                nos_bootloader::protocol::MemoryType::Reserved => 2,
                nos_bootloader::protocol::MemoryType::ACPIReclaimable => 3,
                nos_bootloader::protocol::MemoryType::ACPIONVS => 4,
                nos_bootloader::protocol::MemoryType::BadMemory => 5,
                _ => 2,
            };
            create_e820_entry(entry.base as u64, entry.size as u64, mem_type)
        })
        .collect();

    // Build Multiboot2 info structure
    let info_size = multiboot2_protocol.build_info(
        Some(&boot_entry.cmdline),
        640,   // Base memory in KB
        memory_manager.get_scanner().get_extended_memory_kb() as u32,
        &e820_entries,
        None,  // No framebuffer info for now
        &[],
    )?;

    println!("Multiboot2 info size: {} bytes", info_size);
    println!("Memory map entries: {}", e820_entries.len());

    multiboot2_protocol.get_info_address()
}

/// Jump to kernel entry point
unsafe fn jump_to_kernel(entry_point: usize, multiboot_info: u64) -> ! {
    println!("Jumping to kernel at {:#X} with Multiboot2 info at {:#X}", entry_point, multiboot_info);

    // Set up for long mode kernel entry
    let kernel_fn: extern "C" fn(u64) -> ! = core::mem::transmute(entry_point);

    // Jump to kernel
    kernel_fn(multiboot_info);
}

/// Halt the bootloader on error
fn bootloader_halt() -> ! {
    println!("Bootloader halted due to error");

    loop {
        X86_64Utils::halt();
    }
}

/// Simple print function for debugging
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        // In a real BIOS environment, this would use BIOS calls to print to VGA
        // For this example, we'll use a no-op implementation
    };
}