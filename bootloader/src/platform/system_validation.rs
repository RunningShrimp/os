//! System Validation Module
//!
//! Provides comprehensive system validation including:
//! - Hardware capability detection
//! - CPU feature verification
//! - Memory configuration validation
//! - Firmware integrity checks
//! - System readiness assessment

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;


/// Hardware capability flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareCapability {
    PAE,              // Physical Address Extension
    PSE,              // Page Size Extension
    NX,               // No-Execute bit
    SMEP,             // Supervisor Mode Execution Prevention
    SMAP,             // Supervisor Mode Access Prevention
    TSC,              // Time Stamp Counter
    MSR,              // Model Specific Registers
    APIC,             // Advanced Programmable Interrupt Controller
    MMX,              // MMX Instructions
    SSE,              // SSE Instructions
    AVX,              // AVX Instructions
    AES,              // AES Instructions
    RDRAND,           // Random number generator
    RDSEED,           // Seed for RNG
    FSGS,             // Fast SYSENTER/SYSEXIT
    PCID,             // Process Context Identifiers
}

impl fmt::Display for HardwareCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HardwareCapability::PAE => write!(f, "Physical Address Extension"),
            HardwareCapability::PSE => write!(f, "Page Size Extension"),
            HardwareCapability::NX => write!(f, "No-Execute Bit"),
            HardwareCapability::SMEP => write!(f, "Supervisor Mode Execution Prevention"),
            HardwareCapability::SMAP => write!(f, "Supervisor Mode Access Prevention"),
            HardwareCapability::TSC => write!(f, "Time Stamp Counter"),
            HardwareCapability::MSR => write!(f, "Model Specific Registers"),
            HardwareCapability::APIC => write!(f, "APIC"),
            HardwareCapability::MMX => write!(f, "MMX"),
            HardwareCapability::SSE => write!(f, "SSE"),
            HardwareCapability::AVX => write!(f, "AVX"),
            HardwareCapability::AES => write!(f, "AES"),
            HardwareCapability::RDRAND => write!(f, "RDRAND"),
            HardwareCapability::RDSEED => write!(f, "RDSEED"),
            HardwareCapability::FSGS => write!(f, "FSGS Base"),
            HardwareCapability::PCID => write!(f, "PCID"),
        }
    }
}

/// Memory configuration information
#[derive(Debug, Clone)]
pub struct MemoryConfiguration {
    pub total_pages: u32,
    pub available_pages: u32,
    pub reserved_pages: u32,
    pub page_size: u32,
    pub max_physical_address: u64,
}

impl MemoryConfiguration {
    /// Create new memory configuration
    pub fn new() -> Self {
        MemoryConfiguration {
            total_pages: 0,
            available_pages: 0,
            reserved_pages: 0,
            page_size: 4096, // Standard 4KB pages
            max_physical_address: 0,
        }
    }

    /// Check if memory configuration is valid
    pub fn is_valid(&self) -> bool {
        self.total_pages > 0
            && self.available_pages > 0
            && self.page_size > 0
            && self.max_physical_address > 0
    }

    /// Get total memory in bytes
    pub fn total_memory_bytes(&self) -> u64 {
        (self.total_pages as u64) * (self.page_size as u64)
    }

    /// Get available memory in bytes
    pub fn available_memory_bytes(&self) -> u64 {
        (self.available_pages as u64) * (self.page_size as u64)
    }

    /// Get memory utilization percentage
    pub fn utilization_percent(&self) -> u32 {
        if self.total_pages == 0 {
            return 0;
        }
        let used = self.total_pages - self.available_pages;
        ((used as u64 * 100) / (self.total_pages as u64)) as u32
    }
}

impl fmt::Display for MemoryConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Memory {{ total: {} MB, available: {} MB, utilization: {}% }}",
            self.total_memory_bytes() / 1048576,
            self.available_memory_bytes() / 1048576,
            self.utilization_percent()
        )
    }
}

impl Default for MemoryConfiguration {
    fn default() -> Self {
        Self::new()
    }
}

/// Firmware information
#[derive(Debug, Clone)]
pub struct FirmwareInfo {
    pub firmware_type: String,    // BIOS, UEFI, etc.
    pub version: String,
    pub manufacturer: String,
    pub checksum: u32,
    pub verified: bool,
}

impl FirmwareInfo {
    /// Create new firmware info
    pub fn new(firmware_type: &str) -> Self {
        FirmwareInfo {
            firmware_type: String::from(firmware_type),
            version: String::new(),
            manufacturer: String::new(),
            checksum: 0,
            verified: false,
        }
    }

    /// Set firmware version
    pub fn set_version(&mut self, version: &str) {
        self.version = String::from(version);
    }

    /// Set manufacturer
    pub fn set_manufacturer(&mut self, manufacturer: &str) {
        self.manufacturer = String::from(manufacturer);
    }

    /// Validate firmware
    pub fn validate(&mut self) -> bool {
        if self.firmware_type.is_empty()
            || self.version.is_empty()
            || self.checksum == 0
        {
            return false;
        }
        self.verified = true;
        true
    }
}

impl fmt::Display for FirmwareInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} ({}), verified: {}",
            self.firmware_type, self.version, self.manufacturer, self.verified
        )
    }
}

/// System validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub component: String,
    pub passed: bool,
    pub details: String,
}

impl ValidationResult {
    /// Create new validation result
    pub fn new(component: &str, passed: bool) -> Self {
        ValidationResult {
            component: String::from(component),
            passed,
            details: String::new(),
        }
    }

    /// Add details
    pub fn with_details(&mut self, details: &str) {
        self.details = String::from(details);
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.passed { "PASS" } else { "FAIL" };
        write!(f, "[{}] {}: {}", status, self.component, self.details)
    }
}

/// System Validator
pub struct SystemValidator {
    capabilities: Vec<HardwareCapability>,
    memory_config: MemoryConfiguration,
    firmware_info: FirmwareInfo,
    validation_results: Vec<ValidationResult>,
    overall_valid: bool,
}

impl SystemValidator {
    /// Create new system validator
    pub fn new() -> Self {
        SystemValidator {
            capabilities: Vec::new(),
            memory_config: MemoryConfiguration::new(),
            firmware_info: FirmwareInfo::new("Unknown"),
            validation_results: Vec::new(),
            overall_valid: false,
        }
    }

    /// Add hardware capability
    pub fn add_capability(&mut self, capability: HardwareCapability) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    /// Check if capability is supported
    pub fn has_capability(&self, capability: HardwareCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Set memory configuration
    pub fn set_memory_config(&mut self, config: MemoryConfiguration) {
        self.memory_config = config;
    }

    /// Set firmware info
    pub fn set_firmware_info(&mut self, firmware: FirmwareInfo) {
        self.firmware_info = firmware;
    }

    /// Validate CPU capabilities
    pub fn validate_cpu_capabilities(&mut self) -> bool {
        let required_caps = [
            HardwareCapability::PAE,
            HardwareCapability::MSR,
            HardwareCapability::TSC,
        ];

        let mut all_present = true;
        for cap in &required_caps {
            if !self.has_capability(*cap) {
                all_present = false;
                break;
            }
        }

        let result = ValidationResult::new(
            "CPU Capabilities",
            all_present
        );
        self.validation_results.push(result);
        all_present
    }

    /// Validate memory configuration
    pub fn validate_memory(&mut self) -> bool {
        let valid = self.memory_config.is_valid();
        let result = ValidationResult::new("Memory", valid);
        self.validation_results.push(result);
        valid
    }

    /// Validate firmware
    pub fn validate_firmware(&mut self) -> bool {
        let mut firmware = self.firmware_info.clone();
        let valid = firmware.validate();
        
        let mut result = ValidationResult::new("Firmware", valid);
        result.with_details(&format!("Type: {}", firmware.firmware_type));
        self.validation_results.push(result);
        valid
    }

    /// Run complete system validation
    pub fn validate_system(&mut self) -> bool {
        let cpu_ok = self.validate_cpu_capabilities();
        let mem_ok = self.validate_memory();
        let fw_ok = self.validate_firmware();

        self.overall_valid = cpu_ok && mem_ok && fw_ok;
        self.overall_valid
    }

    /// Get capability count
    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }

    /// Get all capabilities
    pub fn get_capabilities(&self) -> Vec<&HardwareCapability> {
        self.capabilities.iter().collect()
    }

    /// Get validation results
    pub fn get_results(&self) -> Vec<&ValidationResult> {
        self.validation_results.iter().collect()
    }

    /// Get failed validation count
    pub fn failed_count(&self) -> usize {
        self.validation_results.iter().filter(|r| !r.passed).count()
    }

    /// Get detailed validation report
    pub fn validation_report(&self) -> String {
        let mut report = String::from("=== System Validation Report ===\n");
        
        report.push_str(&format!("Overall Status: {}\n", 
            if self.overall_valid { "PASS" } else { "FAIL" }));
        
        report.push_str(&format!("\nCPU Capabilities: {}\n", self.capability_count()));
        for cap in &self.capabilities {
            report.push_str(&format!("  - {}\n", cap));
        }
        
        report.push_str(&format!("\n{}\n", self.memory_config));
        report.push_str(&format!("Firmware: {}\n", self.firmware_info));
        
        report.push_str("\nValidation Results:\n");
        for result in &self.validation_results {
            report.push_str(&format!("  {}\n", result));
        }
        
        report
    }

    /// Check if system is ready for boot
    pub fn is_ready_for_boot(&self) -> bool {
        self.overall_valid && self.failed_count() == 0
    }
}

impl fmt::Display for SystemValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SystemValidator {{ capabilities: {}, valid: {}, failed: {} }}",
            self.capability_count(),
            self.overall_valid,
            self.failed_count()
        )
    }
}

impl Default for SystemValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_capability_display() {
        assert_eq!(HardwareCapability::PAE.to_string(), "Physical Address Extension");
        assert_eq!(HardwareCapability::NX.to_string(), "No-Execute Bit");
    }

    #[test]
    fn test_memory_configuration_creation() {
        let mem = MemoryConfiguration::new();
        assert_eq!(mem.page_size, 4096);
    }

    #[test]
    fn test_memory_configuration_validity() {
        let mem = MemoryConfiguration::new();
        assert!(!mem.is_valid());
    }

    #[test]
    fn test_memory_configuration_total_bytes() {
        let mut mem = MemoryConfiguration::new();
        mem.total_pages = 256000; // 1 GB
        assert_eq!(mem.total_memory_bytes(), 1048576000);
    }

    #[test]
    fn test_memory_configuration_utilization() {
        let mut mem = MemoryConfiguration::new();
        mem.total_pages = 100;
        mem.available_pages = 50;
        assert_eq!(mem.utilization_percent(), 50);
    }

    #[test]
    fn test_firmware_info_creation() {
        let fw = FirmwareInfo::new("BIOS");
        assert_eq!(fw.firmware_type, "BIOS");
        assert!(!fw.verified);
    }

    #[test]
    fn test_firmware_info_validation() {
        let mut fw = FirmwareInfo::new("UEFI");
        fw.set_version("2.1");
        fw.set_manufacturer("AMI");
        fw.checksum = 0x12345678;
        
        assert!(fw.validate());
        assert!(fw.verified);
    }

    #[test]
    fn test_validation_result_creation() {
        let result = ValidationResult::new("Test", true);
        assert!(result.passed);
    }

    #[test]
    fn test_system_validator_creation() {
        let validator = SystemValidator::new();
        assert_eq!(validator.capability_count(), 0);
        assert!(!validator.overall_valid);
    }

    #[test]
    fn test_system_validator_add_capability() {
        let mut validator = SystemValidator::new();
        validator.add_capability(HardwareCapability::PAE);
        assert_eq!(validator.capability_count(), 1);
        assert!(validator.has_capability(HardwareCapability::PAE));
    }

    #[test]
    fn test_system_validator_has_capability() {
        let mut validator = SystemValidator::new();
        assert!(!validator.has_capability(HardwareCapability::NX));
        
        validator.add_capability(HardwareCapability::NX);
        assert!(validator.has_capability(HardwareCapability::NX));
    }

    #[test]
    fn test_system_validator_validate_cpu() {
        let mut validator = SystemValidator::new();
        validator.add_capability(HardwareCapability::PAE);
        validator.add_capability(HardwareCapability::MSR);
        validator.add_capability(HardwareCapability::TSC);
        
        assert!(validator.validate_cpu_capabilities());
    }

    #[test]
    fn test_system_validator_validate_memory() {
        let mut validator = SystemValidator::new();
        let mut mem = MemoryConfiguration::new();
        mem.total_pages = 1000;
        mem.available_pages = 800;
        mem.max_physical_address = 0xFFFFFFFF;
        
        validator.set_memory_config(mem);
        assert!(validator.validate_memory());
    }

    #[test]
    fn test_system_validator_validate_firmware() {
        let mut validator = SystemValidator::new();
        let mut fw = FirmwareInfo::new("BIOS");
        fw.set_version("1.0");
        fw.set_manufacturer("Test");
        fw.checksum = 0x12345678;
        
        validator.set_firmware_info(fw);
        assert!(validator.validate_firmware());
    }

    #[test]
    fn test_system_validator_complete_validation() {
        let mut validator = SystemValidator::new();
        
        // Set up CPU
        validator.add_capability(HardwareCapability::PAE);
        validator.add_capability(HardwareCapability::MSR);
        validator.add_capability(HardwareCapability::TSC);
        
        // Set up memory
        let mut mem = MemoryConfiguration::new();
        mem.total_pages = 1000;
        mem.available_pages = 800;
        mem.max_physical_address = 0xFFFFFFFF;
        validator.set_memory_config(mem);
        
        // Set up firmware
        let mut fw = FirmwareInfo::new("BIOS");
        fw.set_version("1.0");
        fw.set_manufacturer("Test");
        fw.checksum = 0x12345678;
        validator.set_firmware_info(fw);
        
        assert!(validator.validate_system());
        assert!(validator.is_ready_for_boot());
    }

    #[test]
    fn test_system_validator_failed_count() {
        let mut validator = SystemValidator::new();
        validator.validate_system();
        
        assert!(validator.failed_count() > 0);
    }

    #[test]
    fn test_system_validator_report() {
        let mut validator = SystemValidator::new();
        validator.validate_system();
        
        let report = validator.validation_report();
        assert!(report.contains("System Validation Report"));
        assert!(report.contains("Validation Results"));
    }
}
