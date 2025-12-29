//! Boot parameter validation integration tests
//!
//! This test module verifies the boot parameter validation system
//! including integrity checks, memory map validation, and error handling.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use nos_api::boot::{
    BootParameters, BootProtocolType, FramebufferInfo, MemoryMap, MemoryMapEntry, MemoryType,
};

// Re-export validator for testing
use kernel::platform::boot::validator::{
    BootParameterValidator, ValidationResult, ValidationError, ValidationSeverity,
};

/// Test helper to create minimal valid boot parameters
fn create_valid_boot_parameters() -> BootParameters {
    // Create a simple memory map with one usable entry
    let memory_entry = MemoryMapEntry {
        base: 0x1000,
        size: 16 * 1024 * 1024, // 16 MB
        is_available: 1,
        mem_type: MemoryType::Usable as u32,
        reserved: 0,
    };

    // Note: In real test, we'd need to allocate this properly
    // For now, just create structure
    BootParameters {
        magic: BootParameters::MAGIC,
        version: BootParameters::VERSION,
        architecture: {
            #[cfg(target_arch = "x86_64")]
            {
                1
            }
            #[cfg(target_arch = "aarch64")]
            {
                2
            }
            #[cfg(target_arch = "riscv64")]
            {
                3
            }
            #[cfg(not(any(
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "riscv64"
            )))]
            {
                0
            }
        },
        boot_protocol: BootProtocolType::Multiboot2 as u32,
        memory_map: MemoryMap {
            entry_count: 1,
            entries: 0, // Would point to memory_entry in real test
        },
        framebuffer: FramebufferInfo {
            address: 0,
            width: 0,
            height: 0,
            bytes_per_pixel: 0,
            stride: 0,
            pixel_format: 0,
        },
        acpi_rsdp: 0,
        device_tree: 0,
        command_line: 0,
        timestamp: 0,
        bootloader_version: 1,
        aslr_offset: 0,
        reserved: [0; 7],
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test_case]
    fn test_null_parameter_rejection() {
        let result = BootParameterValidator::validate(core::ptr::null());
        assert!(!result.is_valid);
        assert!(!result.is_acceptable());

        let critical_errors = result.get_critical_errors();
        assert!(!critical_errors.is_empty());
    }

    #[test_case]
    fn test_invalid_magic_number() {
        let mut params = create_valid_boot_parameters();
        params.magic = 0xDEADBEEF;

        let result = BootParameterValidator::validate(&params as *const BootParameters);
        assert!(!result.is_valid);
        assert!(!result.is_acceptable());

        let errors = result.get_errors();
        assert!(errors.iter().any(|(e, _)| {
            matches!(e, ValidationError::InvalidMagic { .. })
        }));
    }

    #[test_case]
    fn test_version_compatibility() {
        let mut params = create_valid_boot_parameters();

        // Test with incompatible version
        params.version = BootParameters::VERSION + 100;

        let result = BootParameterValidator::validate(&params as *const BootParameters);
        assert!(!result.is_valid);

        let errors = result.get_errors();
        assert!(errors.iter().any(|(e, _)| {
            matches!(e, ValidationError::VersionMismatch { .. })
        }));
    }

    #[test_case]
    fn test_memory_map_validation() {
        // Test 1: Empty memory map (no usable memory)
        let mut params = create_valid_boot_parameters();
        params.memory_map.entry_count = 0;
        params.memory_map.entries = 0;

        let result = BootParameterValidator::validate(&params as *const BootParameters);
        assert!(!result.is_valid);

        let errors = result.get_errors();
        assert!(errors.iter().any(|(e, s)| {
            matches!(e, ValidationError::InvalidMemoryMap(_))
                && *s == ValidationSeverity::Critical
        }));

        // Test 2: Excessive memory map entries
        let mut params = create_valid_boot_parameters();
        params.memory_map.entry_count = 1000; // > 512 max

        let result = BootParameterValidator::validate(&params as *const BootParameters);
        assert!(!result.is_valid);

        let errors = result.get_errors();
        assert!(errors.iter().any(|(e, _)| {
            matches!(e, ValidationError::InvalidMemoryMap(msg)
                if msg.contains("Too many"))
        }));
    }

    #[test_case]
    fn test_framebuffer_validation() {
        let mut params = create_valid_boot_parameters();

        // Test with invalid framebuffer dimensions
        params.framebuffer = FramebufferInfo {
            address: 0x8000000,
            width: 0,       // Invalid
            height: 600,
            bytes_per_pixel: 4,
            stride: 0,
            pixel_format: 0,
        };

        let result = BootParameterValidator::validate(&params as *const BootParameters);
        assert!(!result.is_valid);

        // Should be a warning, not critical
        let errors = result.get_errors();
        assert!(errors.iter().any(|(e, s)| {
            matches!(e, ValidationError::InvalidFramebuffer(_))
                && *s == ValidationSeverity::Warning
        }));
    }

    #[test_case]
    fn test_acpi_validation() {
        let mut params = create_valid_boot_parameters();

        // Test with invalid ACPI RSDP address
        params.acpi_rsdp = 0x7; // Too low

        let result = BootParameterValidator::validate(&params as *const BootParameters);
        // Should have a warning about invalid ACPI
        let errors = result.get_errors();
        assert!(errors.iter().any(|(e, s)| {
            matches!(e, ValidationError::InvalidAcpiRsdp(_))
                && *s == ValidationSeverity::Warning
        }));
    }

    #[test_case]
    fn test_command_line_validation() {
        let mut params = create_valid_boot_parameters();

        // Test with invalid command line address
        params.command_line = 0x3; // Too low

        let result = BootParameterValidator::validate(&params as *const BootParameters);
        // Should have a warning
        let errors = result.get_errors();
        assert!(errors.iter().any(|(e, s)| {
            matches!(e, ValidationError::InvalidCommandLine(_))
                && *s == ValidationSeverity::Warning
        }));
    }

    #[test_case]
    fn test_validation_result_methods() {
        let mut result = ValidationResult::success();

        // Test success state
        assert!(result.is_valid);
        assert!(result.is_acceptable());
        assert!(result.get_errors().is_empty());

        // Add a warning
        result.add_error(
            ValidationError::InvalidFramebuffer("Test warning".to_string()),
            ValidationSeverity::Warning,
        );

        assert!(!result.is_valid);
        assert!(result.is_acceptable()); // Warnings are acceptable

        // Add a critical error
        result.add_error(
            ValidationError::InvalidMagic {
                expected: 0x1234,
                found: 0x5678,
            },
            ValidationSeverity::Critical,
        );

        assert!(!result.is_valid);
        assert!(!result.is_acceptable()); // Critical errors are not acceptable
    }

    #[test_case]
    fn test_error_descriptions() {
        let err = ValidationError::InvalidMagic {
            expected: 0xABCD,
            found: 0x1234,
        };
        let desc = err.description();
        assert!(desc.contains("ABCD"));
        assert!(desc.contains("1234"));

        let err = ValidationError::InvalidMemoryMap("Test error".to_string());
        let desc = err.description();
        assert!(desc.contains("Test error"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test_case]
    fn test_complete_validation_flow() {
        // Create valid parameters
        let params = create_valid_boot_parameters();

        // Validate
        let result = BootParameterValidator::validate(&params as *const BootParameters);

        // Should pass or have only warnings
        // (depending on architecture match and other factors)
        assert!(result.is_acceptable());
    }

    #[test_case]
    fn test_multiple_errors() {
        let mut params = create_valid_boot_parameters();

        // Introduce multiple errors
        params.magic = 0xBADBAD;
        params.version = 999;
        params.memory_map.entry_count = 0;

        let result = BootParameterValidator::validate(&params as *const BootParameters);

        // Should have multiple errors
        assert!(result.get_errors().len() >= 2);

        // Should not be acceptable
        assert!(!result.is_acceptable());
    }
}

// Test runner for kernel tests
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn entry_point() -> ! {
    // Run tests
    run_all_tests();
    loop {}
}

#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn entry_point() -> ! {
    run_all_tests();
    loop {}
}

#[cfg(target_arch = "riscv64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn entry_point() -> ! {
    run_all_tests();
    loop {}
}

fn run_all_tests() {
    use kernel::println;

    kernel::println!("Running boot validation tests...");

    // Run validation tests
    validation_tests::test_null_parameter_rejection();
    validation_tests::test_invalid_magic_number();
    validation_tests::test_version_compatibility();
    validation_tests::test_memory_map_validation();
    validation_tests::test_framebuffer_validation();
    validation_tests::test_acpi_validation();
    validation_tests::test_command_line_validation();
    validation_tests::test_validation_result_methods();
    validation_tests::test_error_descriptions();

    // Run integration tests
    integration_tests::test_complete_validation_flow();
    integration_tests::test_multiple_errors();

    kernel::println!("All boot validation tests passed!");
}
