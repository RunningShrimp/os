/// Boot Loader Execution - Complete Boot Flow Implementation
///
/// Orchestrates the complete boot process including environment detection,
/// kernel loading validation, and boot parameter preparation.

use crate::core::boot_sequence::BootMemoryLayout;
use crate::bios::e820_detection::E820MemoryMap;
use crate::kernel_if::kernel_handoff::BootInformation;

/// Boot environment detection result
#[derive(Debug, Clone, Copy)]
pub enum BootEnvironment {
    BIOS,           // Legacy BIOS boot
    UEFI,           // UEFI firmware
    Multiboot2,     // Multiboot2 compliant bootloader
    Unknown,        // Unknown boot environment
}

impl BootEnvironment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BIOS => "BIOS",
            Self::UEFI => "UEFI",
            Self::Multiboot2 => "Multiboot2",
            Self::Unknown => "Unknown",
        }
    }
}

/// Kernel signature validation
#[derive(Debug, Clone, Copy)]
pub struct KernelSignature {
    pub magic: u32,
    pub version: u32,
    pub flags: u32,
    pub checksum: u32,
}

impl KernelSignature {
    pub fn new() -> Self {
        Self {
            magic: 0,
            version: 0,
            flags: 0,
            checksum: 0,
        }
    }

    /// Validate Multiboot2 kernel signature
    pub fn is_multiboot2(&self) -> bool {
        self.magic == 0xE85250D6
    }

    /// Validate checksum (magic + version + flags + checksum should equal 0)
    pub fn validate_checksum(&self) -> bool {
        (self.magic as i32)
            .wrapping_add(self.version as i32)
            .wrapping_add(self.flags as i32)
            .wrapping_add(self.checksum as i32)
            == 0
    }
}

/// Loaded kernel information
#[derive(Debug, Clone, Copy)]
pub struct LoadedKernel {
    pub load_address: u64,
    pub size: u64,
    pub entry_point: u64,
    pub signature: KernelSignature,
    pub valid: bool,
}

impl LoadedKernel {
    pub fn new() -> Self {
        Self {
            load_address: 0,
            size: 0,
            entry_point: 0,
            signature: KernelSignature::new(),
            valid: false,
        }
    }

    /// Validate kernel at given address
    pub fn validate_at(&mut self, address: u64) -> Result<(), &'static str> {
        // Read signature from kernel header
        unsafe {
            let sig_ptr = address as *const KernelSignature;
            self.signature = *sig_ptr;
        }

        if !self.signature.is_multiboot2() {
            return Err("Invalid kernel signature");
        }

        if !self.signature.validate_checksum() {
            return Err("Kernel checksum mismatch");
        }

        self.load_address = address;
        self.valid = true;
        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        self.valid && self.signature.is_multiboot2()
    }
}

/// Boot completion status
#[derive(Debug, Clone, Copy)]
pub enum BootStatus {
    NotStarted,
    EnvironmentDetected,
    MemoryDetected,
    KernelLoaded,
    KernelValidated,
    BootInfoPrepared,
    ReadyToExecute,
    Failed,
}

impl BootStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "Not started",
            Self::EnvironmentDetected => "Environment detected",
            Self::MemoryDetected => "Memory detected",
            Self::KernelLoaded => "Kernel loaded",
            Self::KernelValidated => "Kernel validated",
            Self::BootInfoPrepared => "Boot info prepared",
            Self::ReadyToExecute => "Ready to execute",
            Self::Failed => "Failed",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::ReadyToExecute)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// Complete boot loader state
pub struct BootLoader {
    status: BootStatus,
    environment: BootEnvironment,
    memory_layout: BootMemoryLayout,
    memory_map: Option<E820MemoryMap>,
    kernel: LoadedKernel,
    boot_info: Option<BootInformation>,
}

impl BootLoader {
    /// Create new bootloader instance
    pub fn new() -> Self {
        Self {
            status: BootStatus::NotStarted,
            environment: BootEnvironment::Unknown,
            memory_layout: BootMemoryLayout::new(),
            memory_map: None,
            kernel: LoadedKernel::new(),
            boot_info: None,
        }
    }

    /// Detect boot environment
    pub fn detect_environment(&mut self) -> Result<(), &'static str> {
        // Framework: detect based on bootloader signatures
        // For now, assume BIOS
        self.environment = BootEnvironment::BIOS;
        self.status = BootStatus::EnvironmentDetected;
        Ok(())
    }

    /// Set memory map from detection
    pub fn set_memory_map(&mut self, map: E820MemoryMap) -> Result<(), &'static str> {
        self.memory_layout.validate()?;
        self.memory_map = Some(map);
        self.status = BootStatus::MemoryDetected;
        Ok(())
    }

    /// Load kernel from disk
    pub fn load_kernel(&mut self, address: u64) -> Result<(), &'static str> {
        // Framework: would load from disk via INT 0x13
        self.kernel.load_address = address;
        self.status = BootStatus::KernelLoaded;
        Ok(())
    }

    /// Validate loaded kernel
    pub fn validate_kernel(&mut self) -> Result<(), &'static str> {
        self.kernel.validate_at(self.kernel.load_address)?;
        self.status = BootStatus::KernelValidated;
        Ok(())
    }

    /// Prepare boot information structure
    pub fn prepare_boot_info(&mut self) -> Result<(), &'static str> {
        let memory_map = self.memory_map.as_ref()
            .ok_or("Memory map not available")?;

        let mut boot_info = BootInformation::new(self.kernel.entry_point);
        
        // Add memory entries
        for entry in &memory_map.entries {
            if let Some(mem_entry) = entry {
                if let Err(e) = boot_info.add_memory_entry(crate::kernel_if::kernel_handoff::MemoryMapEntry {
                    base: mem_entry.base_address,
                    length: mem_entry.length,
                    region_type: mem_entry.entry_type,
                }) {
                    log::warn!("Failed to add memory entry: {}", e);
                }
            }
        }

        // Add bootloader name
        boot_info.set_bootloader_name("NOS Bootloader");

        // Validate boot info
        boot_info.validate()?;

        self.boot_info = Some(boot_info);
        self.status = BootStatus::BootInfoPrepared;
        Ok(())
    }

    /// Get current status
    pub fn status(&self) -> BootStatus {
        self.status
    }

    /// Check if ready to execute kernel
    pub fn is_ready(&self) -> bool {
        self.kernel.is_valid()
            && self.memory_map.is_some()
            && self.boot_info.is_some()
    }

    /// Mark as ready for execution
    pub fn mark_ready(&mut self) -> Result<(), &'static str> {
        if !self.is_ready() {
            return Err("Boot sequence incomplete");
        }
        self.status = BootStatus::ReadyToExecute;
        Ok(())
    }

    /// Mark boot as failed
    pub fn mark_failed(&mut self) {
        self.status = BootStatus::Failed;
    }

    /// Get environment
    pub fn environment(&self) -> BootEnvironment {
        self.environment
    }

    /// Get memory map
    pub fn memory_map(&self) -> Option<&E820MemoryMap> {
        self.memory_map.as_ref()
    }

    /// Get loaded kernel
    pub fn kernel(&self) -> &LoadedKernel {
        &self.kernel
    }

    /// Get boot information
    pub fn boot_info(&self) -> Option<&BootInformation> {
        self.boot_info.as_ref()
    }

    /// Get total usable memory
    pub fn total_memory(&self) -> Option<u64> {
        self.memory_map.as_ref().map(|map| map.total_ram())
    }

    /// Verify boot prerequisites
    pub fn verify_prerequisites(&self) -> Result<(), &'static str> {
        // Check minimum memory
        let total_mem = self.total_memory().ok_or("Memory not detected")?;
        if total_mem < 4 * 1024 * 1024 {
            return Err("Insufficient memory (need 4MB)");
        }

        // Check kernel validation
        if !self.kernel.is_valid() {
            return Err("Kernel not valid");
        }

        // Check boot info
        if self.boot_info.is_none() {
            return Err("Boot info not prepared");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_environment_strings() {
        assert_eq!(BootEnvironment::BIOS.as_str(), "BIOS");
        assert_eq!(BootEnvironment::UEFI.as_str(), "UEFI");
    }

    #[test]
    fn test_boot_status_strings() {
        assert!(!BootStatus::NotStarted.is_success());
        assert!(!BootStatus::Failed.is_success());
        assert!(BootStatus::ReadyToExecute.is_success());
        assert!(BootStatus::Failed.is_failed());
    }

    #[test]
    fn test_kernel_signature() {
        let mut sig = KernelSignature::new();
        assert!(!sig.is_multiboot2());

        sig.magic = 0xE85250D6;
        assert!(sig.is_multiboot2());
    }

    #[test]
    fn test_kernel_creation() {
        let kernel = LoadedKernel::new();
        assert!(!kernel.is_valid());
        assert_eq!(kernel.load_address, 0);
    }

    #[test]
    fn test_bootloader_creation() {
        let bl = BootLoader::new();
        assert!(!bl.is_ready());
        assert!(matches!(bl.status(), BootStatus::NotStarted));
    }

    #[test]
    fn test_bootloader_environment_detection() {
        let mut bl = BootLoader::new();
        assert!(bl.detect_environment().is_ok());
        assert!(matches!(bl.status(), BootStatus::EnvironmentDetected));
    }

    #[test]
    fn test_bootloader_prerequisites() {
        let bl = BootLoader::new();
        let result = bl.verify_prerequisites();
        assert!(result.is_err());
    }
}
