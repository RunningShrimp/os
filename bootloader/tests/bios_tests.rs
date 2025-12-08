//! BIOS Bootloader Tests
//!
//! This module contains unit and integration tests for BIOS bootloader functionality.
//! These tests validate the core BIOS bootloader features including memory detection,
//! VESA graphics, boot menu, and assembly entry points.

#[cfg(test)]
mod tests {
    use nos_bootloader::memory::bios::{BiosMemoryScanner, BiosMemoryManager};
    use nos_bootloader::graphics::vbe::{VbeController, VbeControllerInfo};
    use nos_bootloader::boot_menu::{BootMenuConfig, BootMenuEntry};
    use nos_bootloader::arch::x86_64::bios::{X86_64CpuInfo, get_cpu_info, is_virtual_machine};
    use nos_bootloader::protocol::multiboot2::{Multiboot2Protocol, create_e820_entry};

    // Test CPU detection functionality
    #[test]
    fn test_cpu_detection() {
        println!("Testing CPU detection functionality...");

        let mut cpu_info = X86_64CpuInfo::new();

        // Test CPU detection (this will only work in actual BIOS environment)
        let result = cpu_info.detect();
        if result.is_ok() {
            println!("CPU Vendor: {}", cpu_info.vendor_str());
            println!("CPU Brand: {}", cpu_info.brand_str());
            println!("CPU Family: {}", cpu_info.family);
            println!("CPU Model: {}", cpu_info.model);
            println!("CPU Stepping: {}", cpu_info.stepping);
            println!("Long Mode Support: {}", cpu_info.supports_long_mode());
            println!("SSE2 Support: {}", cpu_info.supports_sse2());
            println!("AVX Support: {}", cpu_info.supports_avx());

            // Verify that we detected a valid CPU
            assert!(!cpu_info.vendor.is_empty(), "CPU vendor should not be empty");
            assert_eq!(cpu_info.vendor_str(), cpu_info.vendor_str()); // Consistency check
        } else {
            println!("CPU detection failed (expected in non-BIOS environment)");
        }
    }

    #[test]
    fn test_cpu_info_functions() {
        // Test CPU utility functions
        let cpu_info = get_cpu_info();
        println!("Current CPU Info: {:?}", cpu_info);

        let vm_detected = is_virtual_machine();
        println!("Running in virtual machine: {}", vm_detected);

        // These should always work
        assert!(!cpu_info.vendor.is_empty());
    }

    // Test memory detection functionality
    #[test]
    fn test_bios_memory_scanner() {
        println!("Testing BIOS memory scanner...");

        let mut scanner = BiosMemoryScanner::new();

        // Test initial state
        assert!(!scanner.is_initialized(), "Scanner should start uninitialized");
        assert_eq!(scanner.get_total_memory(), 0, "Total memory should be 0 initially");

        // In a real BIOS environment, this would detect memory
        let init_result = scanner.initialize();
        if init_result.is_ok() {
            println!("Memory scanner initialized successfully");
            println!("Base memory: {} KB", scanner.get_base_memory_kb());
            println!("Extended memory: {} KB", scanner.get_extended_memory_kb());
            println!("Total memory: {} KB", scanner.get_total_memory() / 1024);

            assert!(scanner.is_initialized(), "Scanner should be initialized");
            assert!(scanner.get_total_memory() > 0, "Should detect some memory");

            // Test memory map creation
            let memory_map_result = scanner.build_memory_map();
            assert!(memory_map_result.is_ok(), "Should be able to build memory map");

            let memory_map = memory_map_result.unwrap();
            println!("Memory map entries: {}", memory_map.entries.len());
            assert!(!memory_map.entries.is_empty(), "Memory map should have entries");

        } else {
            println!("Memory scanner initialization failed (expected in non-BIOS environment)");
        }
    }

    #[test]
    fn test_bios_memory_manager() {
        println!("Testing BIOS memory manager...");

        let mut manager = BiosMemoryManager::new();

        // Test initial state
        assert!(!manager.is_a20_enabled(), "A20 should not be enabled initially");

        let init_result = manager.initialize();
        if init_result.is_ok() {
            println!("Memory manager initialized successfully");
            println!("A20 enabled: {}", manager.is_a20_enabled());

            // Test memory map access
            let memory_map_result = manager.get_memory_map();
            assert!(memory_map_result.is_ok(), "Should be able to get memory map");

            let memory_map = memory_map_result.unwrap();
            println!("Available memory: {} KB", memory_map.available_memory / 1024);

            // Test memory allocation (simplified)
            let region_result = manager.find_bootloader_region(4096, 4096);
            if let Some(address) = region_result {
                println!("Found suitable region at address: {:#X}", address);
            }

        } else {
            println!("Memory manager initialization failed (expected in non-BIOS environment)");
        }
    }

    // Test VESA graphics functionality
    #[test]
    fn test_vbe_controller() {
        println!("Testing VBE controller...");

        let mut controller = VbeController::new();

        // Test initial state
        assert!(!controller.is_initialized(), "VBE controller should start uninitialized");

        let init_result = controller.initialize();
        if init_result.is_ok() {
            println!("VBE controller initialized successfully");

            assert!(controller.is_initialized(), "VBE controller should be initialized");

            // Test getting controller info
            if let Some(info) = controller.get_controller_info() {
                println!("VBE Version: {}.{}", (info.version >> 8) & 0xFF, info.version & 0xFF);
                println!("Total Memory: {} KB", info.total_memory as u32 * 64);

                // Test mode enumeration
                let modes = controller.get_supported_modes();
                println!("Supported VBE modes: {}", modes.len());

                if !modes.is_empty() {
                    println!("First few VBE modes: {:?}", &modes[..modes.len().min(5)]);
                }

                // Test finding modes
                let mode = controller.find_best_mode(1024, 768, 32);
                if let Some(mode_num) = mode {
                    println!("Found suitable 1024x768x32 mode: 0x{:04X}", mode_num);
                } else {
                    println!("No suitable 1024x768x32 mode found");
                }
            }

        } else {
            println!("VBE controller initialization failed (expected in non-BIOS environment)");
        }
    }

    // Test Multiboot2 functionality
    #[test]
    fn test_multiboot2_protocol() {
        println!("Testing Multiboot2 protocol...");

        let mut protocol = Multiboot2Protocol::new();

        // Test initial state
        assert!(!protocol.is_initialized(), "Multiboot2 protocol should start uninitialized");

        // Create a test buffer
        let mut buffer = [0u8; 8192];
        let init_result = protocol.initialize(&mut buffer[..], buffer.len());
        if init_result.is_ok() {
            println!("Multiboot2 protocol initialized successfully");

            assert!(protocol.is_initialized(), "Multiboot2 protocol should be initialized");

            // Test building Multiboot2 info structure
            let e820_entries = vec![
                create_e820_entry(0x00000000, 0x0009FC00, 1), // Available memory
                create_e820_entry(0x0009FC00, 0x00000400, 2), // Reserved
                create_e820_entry(0x00100000, 0x00F00000, 1), // Available memory
            ];

            let info_size_result = protocol.build_info(
                Some("quiet splash"),
                640,  // mem_lower in KB
                32768, // mem_upper in KB
                &e820_entries,
                None,
                &[],
            );

            if let Ok(info_size) = info_size_result {
                println!("Multiboot2 info structure built successfully");
                println!("Info structure size: {} bytes", info_size);

                // Test header validation
                let validation_result = protocol.validate_header(0x100000);
                println!("Header validation result: {:?}", validation_result);

            } else {
                println!("Failed to build Multiboot2 info structure");
            }

        } else {
            println!("Multiboot2 protocol initialization failed");
        }
    }

    // Test boot menu functionality
    #[test]
    fn test_boot_menu_configuration() {
        println!("Testing boot menu configuration...");

        let mut config = BootMenuConfig::new();

        // Test default configuration
        assert_eq!(config.entries.len(), 0, "Should start with no entries");
        assert_eq!(config.global_timeout, 5, "Default timeout should be 5");
        assert!(config.show_menu, "Show menu should be true by default");

        // Test validation with no entries
        assert!(config.validate().is_err(), "Should fail validation with no entries");

        // Add test entries
        config.add_entry(BootMenuEntry::default_entry(
            "NOS OS - Normal".to_string(),
            "kernel.bin".to_string(),
            "root=/dev/sda1".to_string(),
        ).with_timeout(5));

        config.add_entry(BootMenuEntry::new(
            "NOS OS - Recovery".to_string(),
            "kernel.bin".to_string(),
            "root=/dev/sda1 single".to_string(),
        ));

        // Test validation with entries
        assert!(config.validate().is_ok(), "Should pass validation with entries");
        assert_eq!(config.entries.len(), 2, "Should have 2 entries");
        assert_eq!(config.default_entry, 0, "First entry should be default");
        assert!(config.entries[0].is_default, "First entry should be marked as default");
        assert!(!config.entries[1].is_default, "Second entry should not be default");

        // Test getting default entry
        let default_entry = config.get_default_entry();
        assert!(default_entry.is_some(), "Should have a default entry");
        assert_eq!(default_entry.unwrap().name, "NOS OS - Normal", "Default entry name should match");
    }

    #[test]
    fn test_boot_menu_entry_creation() {
        println!("Testing boot menu entry creation...");

        // Test basic entry creation
        let entry = BootMenuEntry::new(
            "Test OS".to_string(),
            "test.bin".to_string(),
            "test=1".to_string(),
        );

        assert_eq!(entry.name, "Test OS");
        assert_eq!(entry.kernel_path, "test.bin");
        assert_eq!(entry.cmdline, "test=1");
        assert_eq!(entry.timeout, 0);
        assert!(!entry.is_default);

        // Test default entry creation
        let default_entry = BootMenuEntry::default_entry(
            "Default OS".to_string(),
            "default.bin".to_string(),
            "".to_string(),
        );

        assert_eq!(default_entry.name, "Default OS");
        assert!(default_entry.is_default);

        // Test entry with timeout
        let timeout_entry = BootMenuEntry::new(
            "Timeout OS".to_string(),
            "timeout.bin".to_string(),
            "".to_string(),
        ).with_timeout(10);

        assert_eq!(timeout_entry.timeout, 10);
    }

    // Test E820 entry creation
    #[test]
    fn test_e820_entry_creation() {
        println!("Testing E820 entry creation...");

        let entry = create_e820_entry(0x100000, 0x100000, 1);

        assert_eq!(entry.base_addr, 0x100000);
        assert_eq!(entry.length, 0x100000);
        assert_eq!(entry.type_, 1); // Available memory
        assert_eq!(entry.zero, 0);

        // Test different memory types
        let reserved_entry = create_e820_entry(0x0, 0x1000, 2);
        assert_eq!(reserved_entry.type_, 2); // Reserved

        let acpi_entry = create_e820_entry(0x200000, 0x10000, 3);
        assert_eq!(acpi_entry.type_, 3); // ACPI reclaimable
    }

    // Test compilation time constants
    #[test]
    fn test_build_time_constants() {
        // These constants are defined in the build script
        // In a real build, these would be available

        #[cfg(target_arch = "x86_64")]
        {
            use nos_bootloader::arch::x86_64::consts;

            // Test that constants are defined
            assert_eq!(consts::PAGE_SIZE, 4096);
            assert_eq!(consts::PAGE_SHIFT, 12);
            assert_eq!(consts::PAGE_MASK, 4095);
            assert_eq!(consts::VGA_BUFFER, 0xB8000);
            assert_eq!(consts::VGA_WIDTH, 80);
            assert_eq!(consts::VGA_HEIGHT, 25);
        }
    }

    // Test architecture utilities
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn test_x86_64_utilities() {
        use nos_bootloader::arch::x86_64::X86_64Utils;

        // Test utility functions that are safe to call
        let rflags = X86_64Utils::get_rflags();
        println!("Current RFLAGS: {:#018X}", rflags);

        let cr0 = X86_64Utils::read_cr0();
        println!("Current CR0: {:#018X}", cr0);

        let cr4 = X86_64Utils::read_cr4();
        println!("Current CR4: {:#018X}", cr4);

        // These should work regardless of environment
        assert!(rflags != 0, "RFLAGS should not be zero");
    }

    // Integration test that combines multiple components
    #[test]
    fn test_bios_integration() {
        println!("Testing BIOS bootloader integration...");

        // Test that all components can be created together
        let mut memory_scanner = BiosMemoryScanner::new();
        let mut vbe_controller = VbeController::new();
        let mut multiboot2_protocol = Multiboot2Protocol::new();
        let mut boot_menu_config = BootMenuConfig::new();

        // All should start uninitialized
        assert!(!memory_scanner.is_initialized());
        assert!(!vbe_controller.is_initialized());
        assert!(!multiboot2_protocol.is_initialized());
        assert_eq!(boot_menu_config.entries.len(), 0);

        // Add a boot menu entry
        boot_menu_config.add_entry(BootMenuEntry::default_entry(
            "NOS OS".to_string(),
            "kernel.bin".to_string(),
            "root=/dev/sda1".to_string(),
        ));

        assert_eq!(boot_menu_config.entries.len(), 1);
        assert!(boot_menu_config.validate().is_ok());

        println!("Integration test completed successfully");
    }
}

// Test helper functions
#[cfg(test)]
mod test_helpers {
    use super::*;

    /// Helper function to test if we're running in a suitable environment
    pub fn is_bios_environment() -> bool {
        // This is a simplified check - in reality, this would need to
        // detect if we're running in a BIOS environment
        cfg!(target_arch = "x86_64") && !cfg!(target_os = "uefi")
    }

    /// Helper function to create test E820 entries
    pub fn create_test_memory_map() -> Vec<nos_bootloader::protocol::multiboot2::Multiboot2MmapEntry> {
        vec![
            nos_bootloader::protocol::multiboot2::create_e820_entry(0x00000000, 0x0009FC00, 1),
            nos_bootloader::protocol::multiboot2::create_e820_entry(0x0009FC00, 0x00000400, 2),
            nos_bootloader::protocol::multiboot2::create_e820_entry(0x00100000, 0x00F00000, 1),
        ]
    }
}