/// Advanced Boot Manager - Complete Boot System Integration
///
/// High-level boot manager that orchestrates the entire boot process,
/// integrating all bootloader components into a cohesive system.
/// Includes orchestration from boot_orchestrator, boot_orchestration, boot_coordinator.

use alloc::format;
use alloc::string::String;
use crate::boot_stage::boot_executor::BootExecutor;
use crate::boot_stage::boot_preparation::BootHandoff;
use crate::bios::e820_detection::E820MemoryMap;
use crate::boot_stage::boot_diagnostics::BootStatusReport;

/// Boot stages (from orchestrator)
#[derive(Debug, Clone, Copy)]
pub enum BootStage {
    Init,
    MemoryDetection,
    KernelLoad,
    KernelValidation,
    BootParamSetup,
    KernelEntry,
}

impl BootStage {
    pub fn description(&self) -> &'static str {
        match self {
            BootStage::Init => "Bootloader init",
            BootStage::MemoryDetection => "Memory detection",
            BootStage::KernelLoad => "Kernel loading",
            BootStage::KernelValidation => "Kernel validation",
            BootStage::BootParamSetup => "Boot parameter setup",
            BootStage::KernelEntry => "Kernel entry",
        }
    }
}

/// Boot manager status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootManagerStatus {
    Idle,
    Initializing,
    Executing,
    VerifyingKernel,
    PreparingHandoff,
    ReadyForKernel,
    Failed,
}

impl BootManagerStatus {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Initializing => "Initializing system",
            Self::Executing => "Executing boot sequence",
            Self::VerifyingKernel => "Verifying kernel",
            Self::PreparingHandoff => "Preparing kernel handoff",
            Self::ReadyForKernel => "Ready to execute kernel",
            Self::Failed => "Boot failed",
        }
    }

    pub fn is_final(&self) -> bool {
        matches!(self, Self::ReadyForKernel | Self::Failed)
    }
}

/// Complete boot manager
pub struct BootManager {
    status: BootManagerStatus,
    executor: BootExecutor,
    handoff: BootHandoff,
    system_ready: bool,
    error_log: [Option<&'static str>; 8],
    error_count: usize,
}

impl BootManager {
    /// Create new boot manager
    pub fn new() -> Self {
        Self {
            status: BootManagerStatus::Idle,
            executor: BootExecutor::new(),
            handoff: BootHandoff::new(),
            system_ready: false,
            error_log: [None; 8],
            error_count: 0,
        }
    }

    /// Initialize boot manager
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        self.status = BootManagerStatus::Initializing;

        if let Err(e) = self.executor.execute_environment_detection() {
            self.log_error(e);
            self.status = BootManagerStatus::Failed;
            return Err(e);
        }

        self.status = BootManagerStatus::Executing;
        Ok(())
    }

    /// Execute memory detection
    pub fn detect_memory(&mut self, memory_map: E820MemoryMap) -> Result<(), &'static str> {
        if self.executor.execute_memory_detection(memory_map).is_err() {
            self.status = BootManagerStatus::Failed;
            return Err("Memory detection failed");
        }

        Ok(())
    }

    /// Validate disk and bootable media
    pub fn validate_bootable_media(&mut self) -> Result<(), &'static str> {
        if self.executor.execute_disk_validation().is_err() {
            self.status = BootManagerStatus::Failed;
            return Err("Disk validation failed");
        }

        Ok(())
    }

    /// Load kernel from disk
    pub fn load_kernel(&mut self, kernel_address: u64) -> Result<(), &'static str> {
        if self.executor.execute_kernel_loading(kernel_address).is_err() {
            self.status = BootManagerStatus::Failed;
            return Err("Kernel loading failed");
        }

        self.status = BootManagerStatus::VerifyingKernel;
        Ok(())
    }

    /// Verify kernel signature and integrity
    pub fn verify_kernel(
        &mut self,
        signature: u32,
        checksum: u32,
    ) -> Result<(), &'static str> {
        // Execute kernel validation
        if self.executor.execute_kernel_validation().is_err() {
            self.status = BootManagerStatus::Failed;
            return Err("Kernel validation failed");
        }

        // Prepare handoff with signature/checksum verification
        if let Err(e) = self.handoff.prepare(0x100000, signature, checksum) {
            self.log_error(e);
            self.status = BootManagerStatus::Failed;
            return Err(e);
        }

        Ok(())
    }

    /// Set up boot information
    pub fn setup_boot_info(&mut self) -> Result<(), &'static str> {
        if self.executor.execute_boot_info_setup().is_err() {
            self.status = BootManagerStatus::Failed;
            return Err("Boot info setup failed");
        }

        self.status = BootManagerStatus::PreparingHandoff;
        Ok(())
    }

    /// Perform final verification
    pub fn verify_all_systems(&mut self) -> Result<(), &'static str> {
        if self.executor.execute_final_verification().is_err() {
            self.status = BootManagerStatus::Failed;
            return Err("Final verification failed");
        }

        if !self.handoff.is_ready_for_transfer() {
            self.log_error("Handoff not ready for transfer");
            self.status = BootManagerStatus::Failed;
            return Err("Handoff not ready");
        }

        self.system_ready = true;
        self.status = BootManagerStatus::ReadyForKernel;
        Ok(())
    }

    /// Execute complete boot sequence
    pub fn execute_complete(
        &mut self,
        memory_map: E820MemoryMap,
        kernel_address: u64,
        signature: u32,
        checksum: u32,
    ) -> Result<(), &'static str> {
        self.initialize()?;
        self.detect_memory(memory_map)?;
        self.validate_bootable_media()?;
        self.load_kernel(kernel_address)?;
        self.verify_kernel(signature, checksum)?;
        self.setup_boot_info()?;
        self.verify_all_systems()?;

        Ok(())
    }

    /// Log error
    fn log_error(&mut self, error: &'static str) {
        if self.error_count < self.error_log.len() {
            self.error_log[self.error_count] = Some(error);
            self.error_count += 1;
        }
    }

    /// Get current status
    pub fn status(&self) -> BootManagerStatus {
        self.status
    }

    /// Check if system is ready for kernel execution
    pub fn is_ready(&self) -> bool {
        self.system_ready && matches!(self.status, BootManagerStatus::ReadyForKernel)
    }

    /// Check if boot failed
    pub fn has_failed(&self) -> bool {
        matches!(self.status, BootManagerStatus::Failed)
    }

    /// Get error log
    pub fn error_log(&self) -> &[Option<&'static str>] {
        &self.error_log[..self.error_count]
    }

    /// Get boot report
    pub fn get_boot_report(&self) -> BootStatusReport {
        self.executor.get_report()
    }

    /// Get executor reference
    pub fn executor(&self) -> &BootExecutor {
        &self.executor
    }

    /// Get handoff reference
    pub fn handoff(&self) -> &BootHandoff {
        &self.handoff
    }

    /// Get diagnostics
    pub fn diagnostics_summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str("Boot Manager Diagnostics:\n");
        summary.push_str(&format!("  Status: {}\n", self.status.description()));
        summary.push_str(&format!(
            "  System Ready: {}\n",
            if self.system_ready { "Yes" } else { "No" }
        ));
        summary.push_str(&format!("  Errors: {}\n", self.error_count));

        if self.error_count > 0 {
            summary.push_str("  Error Log:\n");
            for (i, error) in self.error_log[..self.error_count].iter().enumerate() {
                if let Some(e) = error {
                    summary.push_str(&format!("    {}: {}\n", i + 1, e));
                }
            }
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_manager_status_description() {
        assert_eq!(BootManagerStatus::Idle.description(), "Idle");
        assert_eq!(
            BootManagerStatus::ReadyForKernel.description(),
            "Ready to execute kernel"
        );
    }

    #[test]
    fn test_boot_manager_status_is_final() {
        assert!(!BootManagerStatus::Idle.is_final());
        assert!(BootManagerStatus::ReadyForKernel.is_final());
        assert!(BootManagerStatus::Failed.is_final());
    }

    #[test]
    fn test_boot_manager_creation() {
        let manager = BootManager::new();
        assert_eq!(manager.status(), BootManagerStatus::Idle);
        assert!(!manager.is_ready());
    }

    #[test]
    fn test_boot_manager_initialize() {
        let mut manager = BootManager::new();
        assert!(manager.initialize().is_ok());
        assert!(!matches!(manager.status(), BootManagerStatus::Idle));
    }

    #[test]
    fn test_boot_manager_error_logging() {
        let mut manager = BootManager::new();
        manager.log_error("Test error");

        assert_eq!(manager.error_log().len(), 1);
        assert_eq!(manager.error_log()[0], Some("Test error"));
    }

    #[test]
    fn test_boot_manager_multiple_errors() {
        let mut manager = BootManager::new();

        for i in 0..5 {
            match i {
                0 => manager.log_error("Error 1"),
                1 => manager.log_error("Error 2"),
                2 => manager.log_error("Error 3"),
                3 => manager.log_error("Error 4"),
                4 => manager.log_error("Error 5"),
                _ => {}
            }
        }

        assert_eq!(manager.error_log().len(), 5);
    }

    #[test]
    fn test_boot_manager_diagnostics_summary() {
        let manager = BootManager::new();
        let summary = manager.diagnostics_summary();

        assert!(summary.contains("Boot Manager Diagnostics"));
        assert!(summary.contains("Idle"));
    }
}

