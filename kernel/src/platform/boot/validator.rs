//! Boot parameter validation and integrity checking
//!
//! This module provides comprehensive validation of boot parameters
//! passed from the bootloader to ensure system security and stability.

#![allow(dead_code)]

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

use nos_api::boot::{
    BootParameters, MemoryMapEntry, MemoryType,
};

/// Validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Invalid magic number
    InvalidMagic { expected: u64, found: u64 },
    /// Version mismatch
    VersionMismatch { expected: u32, found: u32 },
    /// Architecture mismatch
    ArchitectureMismatch,
    /// Invalid memory map
    InvalidMemoryMap(String),
    /// Invalid framebuffer info
    InvalidFramebuffer(String),
    /// Invalid pointer
    InvalidPointer { field: String, address: u64 },
    /// Invalid ACPI RSDP
    InvalidAcpiRsdp(u64),
    /// Invalid device tree
    InvalidDeviceTree(u64),
    /// Invalid command line
    InvalidCommandLine(u64),
    /// Checksum mismatch
    ChecksumMismatch { expected: u32, found: u32 },
    /// Missing required field
    MissingRequiredField(&'static str),
}

impl ValidationError {
    /// Get error description
    pub fn description(&self) -> String {
        match self {
            ValidationError::InvalidMagic { expected, found } => {
                format!("Invalid magic number: expected {:#x}, found {:#x}", expected, found)
            }
            ValidationError::VersionMismatch { expected, found } => {
                format!("Version mismatch: expected {}, found {}", expected, found)
            }
            ValidationError::ArchitectureMismatch => {
                "Architecture mismatch between boot params and kernel".to_string()
            }
            ValidationError::InvalidMemoryMap(msg) => {
                format!("Invalid memory map: {}", msg)
            }
            ValidationError::InvalidFramebuffer(msg) => {
                format!("Invalid framebuffer: {}", msg)
            }
            ValidationError::InvalidPointer { field, address } => {
                format!("Invalid pointer for field '{}' at address {:#x}", field, address)
            }
            ValidationError::InvalidAcpiRsdp(addr) => {
                format!("Invalid ACPI RSDP address: {:#x}", addr)
            }
            ValidationError::InvalidDeviceTree(addr) => {
                format!("Invalid device tree address: {:#x}", addr)
            }
            ValidationError::InvalidCommandLine(addr) => {
                format!("Invalid command line address: {:#x}", addr)
            }
            ValidationError::ChecksumMismatch { expected, found } => {
                format!("Checksum mismatch: expected {:#x}, found {:#x}", expected, found)
            }
            ValidationError::MissingRequiredField(field) => {
                format!("Missing required field: {}", field)
            }
        }
    }
}

/// Validation severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValidationSeverity {
    /// Warning - can continue with reduced functionality
    Warning,
    /// Error - should halt or enter safe mode
    Error,
    /// Critical - must halt
    Critical,
}

/// Validation result with severity
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// List of validation errors
    pub errors: Vec<(ValidationError, ValidationSeverity)>,
    /// Whether validation passed (no errors or warnings)
    pub is_valid: bool,
    /// Whether validation passed with acceptable warnings
    pub is_acceptable: bool,
}

impl ValidationResult {
    /// Create a new successful validation result
    pub fn success() -> Self {
        Self {
            errors: Vec::new(),
            is_valid: true,
            is_acceptable: true,
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: ValidationError, severity: ValidationSeverity) {
        self.errors.push((error, severity));
        self.is_valid = false;
        self.is_acceptable = severity != ValidationSeverity::Critical;
    }

    /// Check if validation is acceptable (no critical errors)
    pub fn is_acceptable(&self) -> bool {
        !self.errors.iter().any(|(_, severity)| *severity == ValidationSeverity::Critical)
    }

    /// Get all errors
    pub fn get_errors(&self) -> &[(ValidationError, ValidationSeverity)] {
        &self.errors
    }

    /// Get critical errors only
    pub fn get_critical_errors(&self) -> Vec<&ValidationError> {
        self.errors
            .iter()
            .filter(|(_, severity)| *severity == ValidationSeverity::Critical)
            .map(|(error, _)| error)
            .collect()
    }
}

/// Boot parameter validator
pub struct BootParameterValidator;

impl BootParameterValidator {
    /// Validate boot parameters comprehensively
    pub fn validate(params: *const BootParameters) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Check null pointer
        if params.is_null() {
            result.add_error(
                ValidationError::MissingRequiredField("boot_parameters"),
                ValidationSeverity::Critical,
            );
            return result;
        }

        // Safe to dereference - we checked for null and will validate contents
        let params_ref = unsafe { &*params };

        // Validate magic number
        Self::validate_magic(params_ref, &mut result);

        // Validate version
        Self::validate_version(params_ref, &mut result);

        // Validate architecture
        Self::validate_architecture(params_ref, &mut result);

        // Validate memory map
        Self::validate_memory_map(params_ref, &mut result);

        // Validate framebuffer
        Self::validate_framebuffer(params_ref, &mut result);

        // Validate ACPI RSDP
        Self::validate_acpi(params_ref, &mut result);

        // Validate device tree
        Self::validate_device_tree(params_ref, &mut result);

        // Validate command line
        Self::validate_command_line(params_ref, &mut result);

        result
    }

    /// Validate magic number
    fn validate_magic(params: &BootParameters, result: &mut ValidationResult) {
        if params.magic != BootParameters::MAGIC {
            result.add_error(
                ValidationError::InvalidMagic {
                    expected: BootParameters::MAGIC,
                    found: params.magic,
                },
                ValidationSeverity::Critical,
            );
        }
    }

    /// Validate version compatibility
    fn validate_version(params: &BootParameters, result: &mut ValidationResult) {
        if !params.is_version_compatible() {
            result.add_error(
                ValidationError::VersionMismatch {
                    expected: BootParameters::VERSION,
                    found: params.version,
                },
                ValidationSeverity::Critical,
            );
        }
    }

    /// Validate architecture match
    fn validate_architecture(params: &BootParameters, result: &mut ValidationResult) {
        if !params.validate_architecture() {
            result.add_error(
                ValidationError::ArchitectureMismatch,
                ValidationSeverity::Critical,
            );
        }
    }

    /// Validate memory map structure and contents
    fn validate_memory_map(params: &BootParameters, result: &mut ValidationResult) {
        let memory_map = &params.memory_map;

        // Check entry count is reasonable
        if memory_map.entry_count > 512 {
            result.add_error(
                ValidationError::InvalidMemoryMap(format!(
                    "Too many memory map entries: {}",
                    memory_map.entry_count
                )),
                ValidationSeverity::Error,
            );
            return;
        }

        // Check entries pointer is not null if we have entries
        if memory_map.entry_count > 0 && memory_map.entries == 0 {
            result.add_error(
                ValidationError::InvalidMemoryMap(
                    "Non-zero entry count but null entries pointer".to_string(),
                ),
                ValidationSeverity::Critical,
            );
            return;
        }

        // Validate individual entries if present
        if memory_map.entry_count > 0 && memory_map.entries != 0 {
            let entries = unsafe {
                core::slice::from_raw_parts(
                    memory_map.entries as *const MemoryMapEntry,
                    memory_map.entry_count as usize,
                )
            };

            let mut has_usable = false;
            let mut total_usable = 0u64;

            for (i, entry) in entries.iter().enumerate() {
                // Check for reasonable physical addresses
                if entry.base >= 0x1000000000u64 {
                    // > 64GB
                    result.add_error(
                        ValidationError::InvalidMemoryMap(format!(
                            "Entry {} has suspiciously high base address: {:#x}",
                            i, entry.base
                        )),
                        ValidationSeverity::Warning,
                    );
                }

                // Check for reasonable sizes
                if entry.size > 0x10000000000u64 {
                    // > 1TB
                    result.add_error(
                        ValidationError::InvalidMemoryMap(format!(
                            "Entry {} has suspiciously large size: {:#x}",
                            i, entry.size
                        )),
                        ValidationSeverity::Warning,
                    );
                }

                // Check memory type is valid
                if entry.mem_type > MemoryType::Reserved as u32 {
                    result.add_error(
                        ValidationError::InvalidMemoryMap(format!(
                            "Entry {} has invalid memory type: {}",
                            i, entry.mem_type
                        )),
                        ValidationSeverity::Error,
                    );
                }

                // Track usable memory
                if entry.is_available != 0 && entry.mem_type == MemoryType::Usable as u32 {
                    has_usable = true;
                    total_usable += entry.size;
                }
            }

            // Ensure we have some usable memory
            if !has_usable {
                result.add_error(
                    ValidationError::InvalidMemoryMap(
                        "No usable memory regions found".to_string(),
                    ),
                    ValidationSeverity::Critical,
                );
            }

            // Check for minimum usable memory (16 MB)
            if total_usable < 16 * 1024 * 1024 {
                result.add_error(
                    ValidationError::InvalidMemoryMap(format!(
                        "Insufficient usable memory: {} MB (minimum 16 MB required)",
                        total_usable / (1024 * 1024)
                    )),
                    ValidationSeverity::Critical,
                );
            }
        }
    }

    /// Validate framebuffer information
    fn validate_framebuffer(params: &BootParameters, result: &mut ValidationResult) {
        if !params.has_framebuffer() {
            return; // Framebuffer is optional
        }

        let fb = &params.framebuffer;

        // Validate address
        if fb.address == 0 {
            result.add_error(
                ValidationError::InvalidFramebuffer(
                    "Framebuffer address is null".to_string(),
                ),
                ValidationSeverity::Warning,
            );
            return;
        }

        // Check for reasonable dimensions
        if fb.width == 0 || fb.height == 0 {
            result.add_error(
                ValidationError::InvalidFramebuffer(
                    "Invalid framebuffer dimensions".to_string(),
                ),
                ValidationSeverity::Warning,
            );
        }

        // Check for reasonable maximum dimensions
        if fb.width > 16384 || fb.height > 16384 {
            result.add_error(
                ValidationError::InvalidFramebuffer(format!(
                    "Framebuffer dimensions too large: {}x{}",
                    fb.width, fb.height
                )),
                ValidationSeverity::Warning,
            );
        }

        // Validate bytes per pixel
        if fb.bytes_per_pixel == 0 || fb.bytes_per_pixel > 8 {
            result.add_error(
                ValidationError::InvalidFramebuffer(format!(
                    "Invalid bytes per pixel: {}",
                    fb.bytes_per_pixel
                )),
                ValidationSeverity::Warning,
            );
        }

        // Validate stride
        let expected_stride = fb.width * fb.bytes_per_pixel as u32;
        if fb.stride < expected_stride {
            result.add_error(
                ValidationError::InvalidFramebuffer(format!(
                    "Stride {} is less than expected {}",
                    fb.stride, expected_stride
                )),
                ValidationSeverity::Warning,
            );
        }

        // Validate pixel format (0=RGB, 1=BGR, etc.)
        if fb.pixel_format > 4 {
            result.add_error(
                ValidationError::InvalidFramebuffer(format!(
                    "Unknown pixel format: {}",
                    fb.pixel_format
                )),
                ValidationSeverity::Warning,
            );
        }
    }

    /// Validate ACPI RSDP address
    fn validate_acpi(params: &BootParameters, result: &mut ValidationResult) {
        if !params.has_acpi() {
            return; // ACPI is optional
        }

        let rsdp = params.acpi_rsdp;

        // RSDP should be in reasonable address range
        // On x86_64, typically in first 1MB or in EFI tables
        if rsdp < 0x800 || rsdp >= 0x100000000 {
            result.add_error(
                ValidationError::InvalidAcpiRsdp(rsdp),
                ValidationSeverity::Warning,
            );
        }
    }

    /// Validate device tree address
    fn validate_device_tree(params: &BootParameters, result: &mut ValidationResult) {
        if !params.has_device_tree() {
            return; // Device tree is optional
        }

        let dtb = params.device_tree;

        // Device tree should be in reasonable address range
        if dtb < 0x4000 || dtb >= 0x10000000000 {
            result.add_error(
                ValidationError::InvalidDeviceTree(dtb),
                ValidationSeverity::Warning,
            );
        }
    }

    /// Validate command line address
    fn validate_command_line(params: &BootParameters, result: &mut ValidationResult) {
        if !params.has_command_line() {
            return; // Command line is optional
        }

        let cmd = params.command_line;

        // Command line should be in reasonable address range
        if cmd < 0x1000 || cmd >= 0x100000000 {
            result.add_error(
                ValidationError::InvalidCommandLine(cmd),
                ValidationSeverity::Warning,
            );
        }

        // TODO: Could validate the actual string content if non-null
        // This would require finding the null terminator safely
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success();
        assert!(result.is_valid);
        assert!(result.is_acceptable());
        assert!(result.get_errors().is_empty());
    }

    #[test]
    fn test_validation_result_with_errors() {
        let mut result = ValidationResult::success();
        result.add_error(
            ValidationError::InvalidMagic {
                expected: 0x1234,
                found: 0x5678,
            },
            ValidationSeverity::Critical,
        );

        assert!(!result.is_valid);
        assert!(!result.is_acceptable());
        assert_eq!(result.get_errors().len(), 1);
    }

    #[test]
    fn test_validation_error_description() {
        let err = ValidationError::InvalidMagic {
            expected: 0xABCD,
            found: 0x1234,
        };
        assert!(err.description().contains("ABCD"));
        assert!(err.description().contains("1234"));
    }
}
