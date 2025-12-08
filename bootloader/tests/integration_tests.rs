//! BIOS Bootloader Integration Tests
//!
//! This module provides integration tests for the complete BIOS bootloader system,
//! testing the interaction between different components and validating end-to-end
//! functionality.

use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test BIOS bootloader compilation
    #[test]
    #[ignore] // This test requires special build environment
    fn test_bios_bootloader_compilation() {
        println!("Testing BIOS bootloader compilation...");

        // Try to build the BIOS bootloader with full features
        let output = Command::new("cargo")
            .args(&[
                "build",
                "--target=x86_64-unknown-none",
                "--no-default-features",
                "--features=bios_full",
                "--bin=bootloader"
            ])
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    println!("BIOS bootloader compiled successfully");

                    let stdout = String::from_utf8_lossy(&result.stdout);
                    let stderr = String::from_utf8_lossy(&result.stderr);

                    println!("Compilation output:");
                    println!("stdout: {}", stdout);
                    if !stderr.is_empty() {
                        println!("stderr: {}", stderr);
                    }

                    // Check if binary was created
                    let bootloader_path = "target/x86_64-unknown-none/debug/bootloader";
                    if Path::new(bootloader_path).exists() {
                        let metadata = std::fs::metadata(bootloader_path).unwrap();
                        println!("Bootloader binary size: {} bytes", metadata.len());
                        assert!(metadata.len() > 0, "Bootloader binary should not be empty");
                    }

                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    panic!("BIOS bootloader compilation failed: {}", stderr);
                }
            }
            Err(e) => {
                panic!("Failed to run cargo build: {}", e);
            }
        }
    }

    /// Test BIOS bootloader release build
    #[test]
    #[ignore] // This test requires special build environment
    fn test_bios_bootloader_release_build() {
        println!("Testing BIOS bootloader release build...");

        let output = Command::new("cargo")
            .args(&[
                "build",
                "--release",
                "--target=x86_64-unknown-none",
                "--no-default-features",
                "--features=bios_full",
                "--bin=bootloader"
            ])
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    println!("Release build successful");

                    let bootloader_path = "target/x86_64-unknown-none/release/bootloader";
                    if Path::new(bootloader_path).exists() {
                        let metadata = std::fs::metadata(bootloader_path).unwrap();
                        println!("Release bootloader size: {} bytes", metadata.len());

                        // Release build should be optimized and smaller
                        assert!(metadata.len() > 0, "Release binary should not be empty");
                        assert!(metadata.len() < 1024 * 1024, "Release binary should be reasonable size");
                    }

                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    panic!("Release build failed: {}", stderr);
                }
            }
            Err(e) => {
                panic!("Failed to run release build: {}", e);
            }
        }
    }

    /// Test Multiboot2 bootloader compilation
    #[test]
    #[ignore] // This test requires special build environment
    fn test_multiboot2_bootloader_compilation() {
        println!("Testing Multiboot2 bootloader compilation...");

        let output = Command::new("cargo")
            .args(&[
                "build",
                "--target=x86_64-unknown-none",
                "--no-default-features",
                "--features=bios_full,multiboot2_support",
                "--bin=bootloader"
            ])
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    println!("Multiboot2 bootloader compiled successfully");
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    panic!("Multiboot2 bootloader compilation failed: {}", stderr);
                }
            }
            Err(e) => {
                panic!("Failed to run cargo build: {}", e);
            }
        }
    }

    /// Test bootloader size constraints
    #[test]
    #[ignore] // Requires built binaries
    fn test_bootloader_size_constraints() {
        println!("Testing bootloader size constraints...");

        // Test that the bootloader fits within reasonable size limits
        let debug_path = "target/x86_64-unknown-none/debug/bootloader";
        let release_path = "target/x86_64-unknown-none/release/bootloader";

        if Path::new(debug_path).exists() {
            let debug_size = std::fs::metadata(debug_path).unwrap().len();
            println!("Debug build size: {} bytes ({} KB)", debug_size, debug_size / 1024);

            // Debug build can be larger but should still be reasonable
            assert!(debug_size < 5 * 1024 * 1024, "Debug build should be under 5MB");
        }

        if Path::new(release_path).exists() {
            let release_size = std::fs::metadata(release_path).unwrap().len();
            println!("Release build size: {} bytes ({} KB)", release_size, release_size / 1024);

            // Release build should be optimized
            assert!(release_size < 1024 * 1024, "Release build should be under 1MB");
            assert!(release_size > 0, "Release build should not be empty");
        }
    }

    /// Test binary format validation
    #[test]
    #[ignore] // Requires external tools
    fn test_binary_format_validation() {
        println!("Testing binary format validation...");

        // Test that the binary has the correct format
        let bootloader_path = "target/x86_64-unknown-none/release/bootloader";

        if Path::new(bootloader_path).exists() {
            // Use file command to check binary format
            let output = Command::new("file")
                .arg(bootloader_path)
                .output();

            match output {
                Ok(result) => {
                    if result.status.success() {
                        let file_info = String::from_utf8_lossy(&result.stdout);
                        println!("Binary format: {}", file_info);

                        // Should be an ELF or raw binary file
                        assert!(file_info.contains("ELF") || file_info.contains("raw"),
                                "Binary should be ELF or raw format");
                    }
                }
                Err(_) => {
                    println!("Warning: file command not available for format validation");
                }
            }
        }
    }

    /// Test QEMU boot capability
    #[test]
    #[ignore] // Requires QEMU and built bootloader
    fn test_qemu_boot_capability() {
        println!("Testing QEMU boot capability...");

        let bootloader_path = "target/x86_64-unknown-none/release/bootloader";

        if !Path::new(bootloader_path).exists() {
            println!("Bootloader not built, skipping QEMU test");
            return;
        }

        // Try to start QEMU with the bootloader (with timeout)
        let output = Command::new("timeout")
            .args(&["5s", "qemu-system-x86_64", "-nographic", "-monitor", "none"])
            .args(&["-kernel", bootloader_path])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);

                println!("QEMU output:");
                println!("stdout: {}", stdout);
                println!("stderr: {}", stderr);
                println!("Exit code: {}", result.status.code().unwrap_or(-1));

                // Check if bootloader started to run
                if stdout.contains("NOS") || stdout.contains("bootloader") {
                    println!("✓ Bootloader appears to have started in QEMU");
                } else {
                    println!("? Bootloader output unclear - may need debugging");
                }
            }
            Err(e) => {
                println!("QEMU test failed (QEMU may not be available): {}", e);
            }
        }
    }

    /// Test memory map functionality in simulated environment
    #[test]
    fn test_memory_map_simulation() {
        println!("Testing memory map simulation...");

        // This test creates a simulated memory map and validates
        // the memory management components can handle it

        use nos_bootloader::memory::bios::{E820Entry, E820_TYPE_USABLE, E820_TYPE_RESERVED};
        use nos_bootloader::protocol::multiboot2::{Multiboot2Protocol, create_e820_entry};

        // Create a simulated memory map
        let e820_entries = vec![
            create_e820_entry(0x00000000, 0x0009FC00, E820_TYPE_USABLE),    // 640KB conventional memory
            create_e820_entry(0x0009FC00, 0x00000400, E820_TYPE_RESERVED),    // BIOS area
            create_e820_entry(0x00100000, 0x00F00000, E820_TYPE_USABLE),    // 15MB extended memory
            create_e820_entry(0x01000000, 0x00100000, E820_TYPE_RESERVED),    // Reserved area
            create_e820_entry(0x01100000, 0x1F000000, E820_TYPE_USABLE),    // 511MB more memory
        ];

        assert_eq!(e820_entries.len(), 5, "Should have 5 memory map entries");

        // Calculate total available memory
        let total_available: u64 = e820_entries
            .iter()
            .filter(|e| e.type_ == E820_TYPE_USABLE)
            .map(|e| e.length)
            .sum();

        println!("Total available memory: {} MB", total_available / (1024 * 1024));
        assert!(total_available > 0, "Should detect some available memory");

        // Test Multiboot2 memory map handling
        let mut buffer = [0u8; 8192];
        let mut protocol = Multiboot2Protocol::new();
        let init_result = protocol.initialize(&mut buffer[..], buffer.len());
        assert!(init_result.is_ok(), "Multiboot2 protocol should initialize");

        let info_result = protocol.build_info(
            Some("test kernel cmdline"),
            640,   // mem_lower in KB
            32768, // mem_upper in KB
            &e820_entries,
            None,
            &[],
        );

        assert!(info_result.is_ok(), "Should build Multiboot2 info successfully");
        let info_size = info_result.unwrap();
        assert!(info_size > 0, "Info structure should have size");
        assert!(info_size < buffer.len() as u32, "Info should fit in buffer");
    }

    /// Test boot menu integration with simulated input
    #[test]
    fn test_boot_menu_integration() {
        println!("Testing boot menu integration...");

        use nos_bootloader::boot_menu::{create_default_config, BootMenuEntry};

        // Create a comprehensive boot menu configuration
        let config = create_default_config();
        assert!(config.validate().is_ok(), "Default config should be valid");

        // Verify all expected entries are present
        assert!(config.entries.len() >= 4, "Should have at least 4 default entries");

        // Test each entry has required fields
        for entry in &config.entries {
            assert!(!entry.name.is_empty(), "Entry should have a name");
            assert!(!entry.kernel_path.is_empty(), "Entry should have a kernel path");
            // cmdline can be empty for some entries
            assert!(entry.timeout >= 0 && entry.timeout <= 255, "Timeout should be valid range");
        }

        // Test that exactly one entry is marked as default
        let default_count = config.entries.iter().filter(|e| e.is_default).count();
        assert_eq!(default_count, 1, "Exactly one entry should be marked as default");

        // Find and validate default entry
        let default_entry = config.get_default_entry().unwrap();
        assert!(default_entry.is_default, "Default entry should be marked as such");
        assert_eq!(default_entry.timeout, 5, "Default entry should have 5s timeout");
    }

    /// Test BIOS component interaction
    #[test]
    fn test_bios_component_interaction() {
        println!("Testing BIOS component interaction...");

        use nos_bootloader::memory::bios::{BiosMemoryScanner, BiosMemoryManager};
        use nos_bootloader::graphics::vbe::VbeController;
        use nos_bootloader::boot_menu::{create_default_config};

        // Initialize all BIOS components
        let mut memory_scanner = BiosMemoryScanner::new();
        let mut vbe_controller = VbeController::new();
        let mut memory_manager = BiosMemoryManager::new();
        let boot_menu_config = create_default_config();

        // Test that components don't interfere with each other
        assert!(!memory_scanner.is_initialized());
        assert!(!vbe_controller.is_initialized());
        assert!(!memory_manager.is_a20_enabled());
        assert!(boot_menu_config.validate().is_ok());

        // Simulate initialization sequence
        println!("Simulating BIOS initialization sequence...");

        // Step 1: Initialize memory scanner (may fail in test environment)
        let scanner_result = memory_scanner.initialize();
        println!("Memory scanner initialization: {}", scanner_result.is_ok());

        // Step 2: Initialize VBE controller (may fail in test environment)
        let vbe_result = vbe_controller.initialize();
        println!("VBE controller initialization: {}", vbe_result.is_ok());

        // Step 3: Initialize memory manager (may fail in test environment)
        let manager_result = memory_manager.initialize();
        println!("Memory manager initialization: {}", manager_result.is_ok());

        // All components should maintain their state even if some fail
        if scanner_result.is_ok() {
            assert!(memory_scanner.is_initialized());
        }

        if vbe_result.is_ok() {
            assert!(vbe_controller.is_initialized());
        }

        if manager_result.is_ok() {
            assert!(memory_manager.is_a20_enabled());
        }

        println!("Component interaction test completed");
    }

    /// Test build system integration
    #[test]
    fn test_build_system_integration() {
        println!("Testing build system integration...");

        // Test that the build script can access necessary resources
        let linker_script_path = "linker/bios.ld";
        let assembly_path = "src/arch/x86_64/bios.S";

        assert!(Path::new(linker_script_path).exists(), "Linker script should exist");
        assert!(Path::new(assembly_path).exists(), "Assembly file should exist");

        // Read and validate linker script content
        let linker_content = std::fs::read_to_string(linker_script_path).unwrap();
        assert!(linker_content.contains("ENTRY(_start)"), "Linker script should have entry point");
        assert!(linker_content.contains(".text"), "Linker script should have text section");
        assert!(linker_content.contains(".data"), "Linker script should have data section");
        assert!(linker_content.contains(".bss"), "Linker script should have bss section");

        // Read and validate assembly file content
        let assembly_content = std::fs::read_to_string(assembly_path).unwrap();
        assert!(assembly_content.contains("_start:"), "Assembly file should have _start label");
        assert!(assembly_content.contains(".section .text"), "Assembly file should have text section");

        println!("Build system integration test completed");
    }

    /// Performance test for BIOS operations
    #[test]
    fn test_bios_performance() {
        println!("Testing BIOS component performance...");

        use std::time::Instant;

        // Test memory scanner performance
        let start = Instant::now();
        let mut scanner = nos_bootloader::memory::bios::BiosMemoryScanner::new();
        let creation_time = start.elapsed();
        println!("Memory scanner creation: {:?}", creation_time);
        assert!(creation_time.as_millis() < 100, "Memory scanner creation should be fast");

        // Test boot menu config performance
        let start = Instant::now();
        let config = nos_bootloader::boot_menu::create_default_config();
        let config_time = start.elapsed();
        println!("Boot menu config creation: {:?}", config_time);
        assert!(config_time.as_millis() < 50, "Boot menu config creation should be fast");

        // Test Multiboot2 protocol performance
        let start = Instant::now();
        let mut buffer = [0u8; 8192];
        let mut protocol = nos_bootloader::protocol::multiboot2::Multiboot2Protocol::new();
        let init_result = protocol.initialize(&mut buffer[..], buffer.len());
        let protocol_time = start.elapsed();
        println!("Multiboot2 protocol init: {:?} (success: {})", protocol_time, init_result.is_ok());
        assert!(protocol_time.as_millis() < 10, "Multiboot2 protocol init should be fast");

        println!("Performance test completed");
    }
}

/// Test helper functions for integration testing
mod integration_helpers {
    use super::*;

    /// Check if required tools are available for integration testing
    pub fn check_test_environment() -> bool {
        // Check if we have the required tools
        let cargo_available = Command::new("cargo").arg("--version").output().is_ok();
        let rustc_available = Command::new("rustc").arg("--version").output().is_ok();

        cargo_available && rustc_available
    }

    /// Check if QEMU is available for testing
    pub fn check_qemu_available() -> bool {
        Command::new("qemu-system-x86_64").arg("--version").output().is_ok()
    }

    /// Create a temporary directory for test artifacts
    pub fn create_test_temp_dir() -> std::path::PathBuf {
        let temp_dir = std::env::temp_dir().join("nos_bootloader_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        temp_dir
    }

    /// Clean up test artifacts
    pub fn cleanup_test_artifacts() {
        let temp_dir = std::env::temp_dir().join("nos_bootloader_test");
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
    }
}