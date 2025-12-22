/// Complete Boot Execution Coordinator
///
/// Integrates all boot components and manages the complete boot-to-kernel transition.

use crate::boot_stage::boot_loader::BootLoader;
use crate::boot_stage::boot_diagnostics::{BootDiagnostics, BootVerification, BootStatusReport, BootEvent};
use crate::bios::e820_detection::E820MemoryMap;

/// Boot execution phase
#[derive(Debug, Clone, Copy)]
pub enum ExecutionPhase {
    Initialization,
    EnvironmentDetection,
    MemoryDetection,
    DiskValidation,
    KernelLoading,
    KernelValidation,
    BootInfoSetup,
    FinalVerification,
    KernelExecution,
    Failed,
}

impl ExecutionPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Initialization => "Initialization",
            Self::EnvironmentDetection => "Environment Detection",
            Self::MemoryDetection => "Memory Detection",
            Self::DiskValidation => "Disk Validation",
            Self::KernelLoading => "Kernel Loading",
            Self::KernelValidation => "Kernel Validation",
            Self::BootInfoSetup => "Boot Info Setup",
            Self::FinalVerification => "Final Verification",
            Self::KernelExecution => "Kernel Execution",
            Self::Failed => "Failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::KernelExecution | Self::Failed)
    }
}

/// Boot execution coordinator
pub struct BootExecutor {
    phase: ExecutionPhase,
    bootloader: BootLoader,
    diagnostics: BootDiagnostics,
    verification: BootVerification,
    error_message: Option<&'static str>,
}

impl BootExecutor {
    /// Create new boot executor
    pub fn new() -> Self {
        Self {
            phase: ExecutionPhase::Initialization,
            bootloader: BootLoader::new(),
            diagnostics: BootDiagnostics::new(),
            verification: BootVerification::new(),
            error_message: None,
        }
    }

    /// Execute boot phase: environment detection
    pub fn execute_environment_detection(&mut self) -> Result<(), &'static str> {
        self.phase = ExecutionPhase::EnvironmentDetection;
        self.diagnostics.log_event(BootEvent::EnvironmentDetected)?;

        self.bootloader.detect_environment()?;
        self.verification.check_memory();

        Ok(())
    }

    /// Execute boot phase: memory detection
    pub fn execute_memory_detection(
        &mut self,
        memory_map: E820MemoryMap,
    ) -> Result<(), &'static str> {
        self.phase = ExecutionPhase::MemoryDetection;

        if memory_map.count == 0 {
            return Err("No memory detected");
        }

        self.diagnostics.log_event(BootEvent::MemoryDetected)?;
        self.bootloader.set_memory_map(memory_map)?;
        self.diagnostics.log_event(BootEvent::MemoryMapValid)?;

        self.verification.validate_memory();

        Ok(())
    }

    /// Execute boot phase: disk validation
    pub fn execute_disk_validation(&mut self) -> Result<(), &'static str> {
        self.phase = ExecutionPhase::DiskValidation;
        self.diagnostics.log_event(BootEvent::DiskAccessOK)?;

        self.verification.check_disk();

        Ok(())
    }

    /// Execute boot phase: kernel loading
    pub fn execute_kernel_loading(&mut self, kernel_address: u64) -> Result<(), &'static str> {
        self.phase = ExecutionPhase::KernelLoading;
        self.diagnostics.log_event(BootEvent::KernelLoadStarted)?;

        self.bootloader.load_kernel(kernel_address)?;
        self.diagnostics.log_event(BootEvent::KernelLoaded)?;

        self.verification.check_kernel_loaded();

        Ok(())
    }

    /// Execute boot phase: kernel validation
    pub fn execute_kernel_validation(&mut self) -> Result<(), &'static str> {
        self.phase = ExecutionPhase::KernelValidation;

        self.bootloader.validate_kernel()?;
        self.diagnostics.log_event(BootEvent::KernelSignatureValid)?;
        self.diagnostics.log_event(BootEvent::ChecksumValid)?;

        self.verification.check_kernel_valid();

        Ok(())
    }

    /// Execute boot phase: boot info setup
    pub fn execute_boot_info_setup(&mut self) -> Result<(), &'static str> {
        self.phase = ExecutionPhase::BootInfoSetup;

        self.bootloader.prepare_boot_info()?;
        self.diagnostics.log_event(BootEvent::BootInfoCreated)?;

        self.verification.check_boot_info();

        Ok(())
    }

    /// Execute boot phase: final verification
    pub fn execute_final_verification(&mut self) -> Result<(), &'static str> {
        self.phase = ExecutionPhase::FinalVerification;

        self.bootloader.verify_prerequisites()?;
        self.bootloader.mark_ready()?;
        self.diagnostics.log_event(BootEvent::AllChecksPassed)?;

        Ok(())
    }

    /// Execute complete boot sequence
    pub fn execute_complete(
        &mut self,
        memory_map: E820MemoryMap,
        kernel_address: u64,
    ) -> Result<(), &'static str> {
        // Phase 1: Environment Detection
        if let Err(e) = self.execute_environment_detection() {
            self.phase = ExecutionPhase::Failed;
            self.error_message = Some(e);
            return Err(e);
        }

        // Phase 2: Memory Detection
        if let Err(e) = self.execute_memory_detection(memory_map) {
            self.phase = ExecutionPhase::Failed;
            self.error_message = Some(e);
            return Err(e);
        }

        // Phase 3: Disk Validation
        if let Err(e) = self.execute_disk_validation() {
            self.phase = ExecutionPhase::Failed;
            self.error_message = Some(e);
            return Err(e);
        }

        // Phase 4: Kernel Loading
        if let Err(e) = self.execute_kernel_loading(kernel_address) {
            self.phase = ExecutionPhase::Failed;
            self.error_message = Some(e);
            return Err(e);
        }

        // Phase 5: Kernel Validation
        if let Err(e) = self.execute_kernel_validation() {
            self.phase = ExecutionPhase::Failed;
            self.error_message = Some(e);
            return Err(e);
        }

        // Phase 6: Boot Info Setup
        if let Err(e) = self.execute_boot_info_setup() {
            self.phase = ExecutionPhase::Failed;
            self.error_message = Some(e);
            return Err(e);
        }

        // Phase 7: Final Verification
        if let Err(e) = self.execute_final_verification() {
            self.phase = ExecutionPhase::Failed;
            self.error_message = Some(e);
            return Err(e);
        }

        self.phase = ExecutionPhase::KernelExecution;
        Ok(())
    }

    /// Get current execution phase
    pub fn phase(&self) -> ExecutionPhase {
        self.phase
    }

    /// Get boot loader reference
    pub fn bootloader(&self) -> &BootLoader {
        &self.bootloader
    }

    /// Get diagnostics reference
    pub fn diagnostics(&self) -> &BootDiagnostics {
        &self.diagnostics
    }

    /// Get verification reference
    pub fn verification(&self) -> &BootVerification {
        &self.verification
    }

    /// Get error message if any
    pub fn error_message(&self) -> Option<&'static str> {
        self.error_message
    }

    /// Check if execution succeeded
    pub fn is_successful(&self) -> bool {
        matches!(self.phase, ExecutionPhase::KernelExecution)
    }

    /// Check if execution failed
    pub fn is_failed(&self) -> bool {
        matches!(self.phase, ExecutionPhase::Failed)
    }

    /// Get status report
    pub fn get_report(&self) -> BootStatusReport {
        BootStatusReport::from_state(
            self.bootloader.environment(),
            self.bootloader.status(),
            self.bootloader.total_memory().unwrap_or(0),
            self.bootloader.kernel(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_phase_strings() {
        assert_eq!(ExecutionPhase::Initialization.as_str(), "Initialization");
        assert_eq!(ExecutionPhase::KernelExecution.as_str(), "Kernel Execution");
    }

    #[test]
    fn test_execution_phase_terminal() {
        assert!(!ExecutionPhase::MemoryDetection.is_terminal());
        assert!(ExecutionPhase::KernelExecution.is_terminal());
        assert!(ExecutionPhase::Failed.is_terminal());
    }

    #[test]
    fn test_boot_executor_creation() {
        let executor = BootExecutor::new();
        assert!(matches!(executor.phase(), ExecutionPhase::Initialization));
        assert!(!executor.is_successful());
        assert!(!executor.is_failed());
    }

    #[test]
    fn test_boot_executor_environment() {
        let mut executor = BootExecutor::new();
        assert!(executor.execute_environment_detection().is_ok());
        assert!(!matches!(executor.phase(), ExecutionPhase::Initialization));
    }
}
