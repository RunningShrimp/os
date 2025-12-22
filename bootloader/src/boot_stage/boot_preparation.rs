/// Boot Preparation and Kernel Readiness Module
///
/// Validates kernel readiness, prepares boot parameters, and ensures system is
/// ready for transition to kernel execution.

use alloc::format;
use alloc::string::String;
use crate::kernel_if::kernel_handoff::BootInformation;
use crate::core::boot_sequence::BootMemoryLayout;

/// Kernel readiness status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelReadiness {
    Unknown,
    Valid,
    InvalidSignature,
    InvalidChecksum,
    AddressOutOfRange,
    CorruptedHeader,
}

impl KernelReadiness {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Valid)
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown status",
            Self::Valid => "Kernel valid and ready",
            Self::InvalidSignature => "Invalid kernel signature",
            Self::InvalidChecksum => "Invalid kernel checksum",
            Self::AddressOutOfRange => "Kernel address out of valid range",
            Self::CorruptedHeader => "Kernel header corrupted",
        }
    }
}

/// Boot parameter validation (internal to bootloader)
#[derive(Debug, Clone)]
pub struct BootPreparationParams {
    kernel_address: u64,
    entry_address: u64,
    memory_map_address: u64,
    boot_info_address: u64,
    module_count: u32,
    command_line: &'static str,
}

impl BootPreparationParams {
    /// Create boot parameters
    pub fn new(kernel_address: u64, entry_address: u64) -> Self {
        Self {
            kernel_address,
            entry_address,
            memory_map_address: 0x1000,
            boot_info_address: 0x2000,
            module_count: 0,
            command_line: "",
        }
    }

    /// Set memory map address
    pub fn with_memory_map(mut self, address: u64) -> Self {
        self.memory_map_address = address;
        self
    }

    /// Set boot info address
    pub fn with_boot_info(mut self, address: u64) -> Self {
        self.boot_info_address = address;
        self
    }

    /// Set module count
    pub fn with_module_count(mut self, count: u32) -> Self {
        self.module_count = count;
        self
    }

    /// Set command line
    pub fn with_command_line(mut self, line: &'static str) -> Self {
        self.command_line = line;
        self
    }

    /// Validate parameters
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.kernel_address == 0 {
            return Err("Kernel address is zero");
        }

        if self.entry_address == 0 {
            return Err("Entry address is zero");
        }

        if self.kernel_address >= 0x100000000 {
            return Err("Kernel address exceeds 4GB");
        }

        if self.memory_map_address >= self.boot_info_address {
            return Err("Memory map and boot info addresses overlap");
        }

        Ok(())
    }

    /// Get kernel address
    pub fn kernel_address(&self) -> u64 {
        self.kernel_address
    }

    /// Get entry address
    pub fn entry_address(&self) -> u64 {
        self.entry_address
    }
}

/// Boot preparation and validation
pub struct BootPreparation {
    parameters: Option<BootPreparationParams>,
    kernel_readiness: KernelReadiness,
    boot_info: Option<BootInformation>,
    memory_layout: Option<BootMemoryLayout>,
}

impl BootPreparation {
    /// Create new boot preparation
    pub fn new() -> Self {
        Self {
            parameters: None,
            kernel_readiness: KernelReadiness::Unknown,
            boot_info: None,
            memory_layout: None,
        }
    }

    /// Set boot parameters
    pub fn set_parameters(&mut self, params: BootPreparationParams) -> Result<(), &'static str> {
        params.validate()?;
        self.parameters = Some(params);
        Ok(())
    }

    /// Validate kernel
    pub fn validate_kernel(
        &mut self,
        signature: u32,
        checksum: u32,
    ) -> Result<(), &'static str> {
        // Multiboot2 signature: 0xE85250D6
        if signature != 0xE85250D6 {
            self.kernel_readiness = KernelReadiness::InvalidSignature;
            return Err("Invalid kernel signature");
        }

        // Validate checksum (checksum + signature should equal 0 mod 2^32)
        if signature.wrapping_add(checksum) != 0 {
            self.kernel_readiness = KernelReadiness::InvalidChecksum;
            return Err("Invalid kernel checksum");
        }

        self.kernel_readiness = KernelReadiness::Valid;
        Ok(())
    }

    /// Set boot information
    pub fn set_boot_info(&mut self, boot_info: BootInformation) {
        self.boot_info = Some(boot_info);
    }

    /// Set memory layout
    pub fn set_memory_layout(&mut self, layout: BootMemoryLayout) {
        self.memory_layout = Some(layout);
    }

    /// Check if system is ready
    pub fn is_ready(&self) -> bool {
        self.parameters.is_some()
            && self.kernel_readiness.is_ready()
            && self.boot_info.is_some()
            && self.memory_layout.is_some()
    }

    /// Get kernel readiness
    pub fn kernel_readiness(&self) -> KernelReadiness {
        self.kernel_readiness
    }

    /// Get boot parameters
    pub fn parameters(&self) -> Option<&BootPreparationParams> {
        self.parameters.as_ref()
    }

    /// Get boot information
    pub fn boot_info(&self) -> Option<&BootInformation> {
        self.boot_info.as_ref()
    }

    /// Get readiness status
    pub fn readiness_status(&self) -> String {
        let mut status = String::new();

        status.push_str("Boot Readiness Status:\n");
        status.push_str(&format!(
            "  Parameters: {}\n",
            if self.parameters.is_some() { "OK" } else { "MISSING" }
        ));
        status.push_str(&format!(
            "  Kernel: {}\n",
            self.kernel_readiness.description()
        ));
        status.push_str(&format!(
            "  Boot Info: {}\n",
            if self.boot_info.is_some() { "OK" } else { "MISSING" }
        ));
        status.push_str(&format!(
            "  Memory Layout: {}\n",
            if self.memory_layout.is_some() { "OK" } else { "MISSING" }
        ));

        status
    }
}

/// Boot handoff preparation and verification
pub struct BootHandoff {
    preparation: BootPreparation,
}

impl BootHandoff {
    /// Create new boot handoff
    pub fn new() -> Self {
        Self {
            preparation: BootPreparation::new(),
        }
    }

    /// Prepare for handoff
    pub fn prepare(
        &mut self,
        kernel_address: u64,
        signature: u32,
        checksum: u32,
    ) -> Result<(), &'static str> {
        // Set parameters
        let params = BootPreparationParams::new(kernel_address, kernel_address + 0x1000);
        self.preparation.set_parameters(params)?;

        // Validate kernel
        self.preparation.validate_kernel(signature, checksum)?;

        Ok(())
    }

    /// Get mutable preparation
    pub fn preparation_mut(&mut self) -> &mut BootPreparation {
        &mut self.preparation
    }

    /// Get preparation
    pub fn preparation(&self) -> &BootPreparation {
        &self.preparation
    }

    /// Check if ready to transfer control
    pub fn is_ready_for_transfer(&self) -> bool {
        self.preparation.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_readiness_is_ready() {
        assert!(KernelReadiness::Valid.is_ready());
        assert!(!KernelReadiness::InvalidSignature.is_ready());
        assert!(!KernelReadiness::Unknown.is_ready());
    }

    #[test]
    fn test_kernel_readiness_description() {
        assert_eq!(KernelReadiness::Valid.description(), "Kernel valid and ready");
        assert!(KernelReadiness::InvalidSignature
            .description()
            .contains("signature"));
    }

    #[test]
    fn test_boot_parameters_creation() {
        let params = BootPreparationParams::new(0x100000, 0x101000);
        assert_eq!(params.kernel_address(), 0x100000);
        assert_eq!(params.entry_address(), 0x101000);
    }

    #[test]
    fn test_boot_parameters_builder() {
        let params = BootPreparationParams::new(0x100000, 0x101000)
            .with_memory_map(0x2000)
            .with_module_count(1);

        assert_eq!(params.kernel_address(), 0x100000);
        assert_eq!(params.module_count, 1);
    }

    #[test]
    fn test_boot_parameters_validate() {
        let params = BootPreparationParams::new(0x100000, 0x101000);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_boot_parameters_validate_zero_kernel() {
        let params = BootPreparationParams::new(0, 0x101000);
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_boot_preparation_creation() {
        let prep = BootPreparation::new();
        assert!(!prep.is_ready());
    }

    #[test]
    fn test_boot_preparation_kernel_validation() {
        let mut prep = BootPreparation::new();
        // Valid signature and checksum
        let result = prep.validate_kernel(0xE85250D6, 0x17ACAF2A);
        assert!(result.is_ok());
        assert_eq!(prep.kernel_readiness(), KernelReadiness::Valid);
    }

    #[test]
    fn test_boot_handoff_creation() {
        let handoff = BootHandoff::new();
        assert!(!handoff.is_ready_for_transfer());
    }
}
