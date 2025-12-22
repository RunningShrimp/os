/// Advanced Boot Control and Execution Module
///
/// Provides high-level control over complete boot process with detailed
/// status tracking, error recovery, and boot event monitoring.

use alloc::format;
use alloc::string::String;
use crate::boot_stage::boot_manager::BootManager;
use crate::boot_stage::boot_validation::FinalSystemCheck;
use crate::bios::e820_detection::E820MemoryMap;

/// Boot execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    Uninitialized,
    Initializing,
    DetectingEnvironment,
    CheckingMemory,
    ValidatingDisk,
    LoadingKernel,
    VerifyingKernel,
    PreparingBoot,
    VerifyingSystem,
    ReadyForTransfer,
    TransferringControl,
    Failed,
    Halted,
}

impl ExecutionState {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Uninitialized => "Uninitialized",
            Self::Initializing => "Initializing boot system",
            Self::DetectingEnvironment => "Detecting boot environment",
            Self::CheckingMemory => "Checking memory configuration",
            Self::ValidatingDisk => "Validating disk interface",
            Self::LoadingKernel => "Loading kernel image",
            Self::VerifyingKernel => "Verifying kernel integrity",
            Self::PreparingBoot => "Preparing boot information",
            Self::VerifyingSystem => "Verifying system readiness",
            Self::ReadyForTransfer => "Ready to transfer control",
            Self::TransferringControl => "Transferring control to kernel",
            Self::Failed => "Boot failed",
            Self::Halted => "System halted",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::ReadyForTransfer
                | Self::TransferringControl
                | Self::Failed
                | Self::Halted
        )
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::ReadyForTransfer)
    }
}

/// Advanced boot controller
pub struct BootController {
    state: ExecutionState,
    boot_manager: BootManager,
    system_check: FinalSystemCheck,
    boot_count: u32,
    #[allow(dead_code)]
    error_history: [Option<&'static str>; 5],
    #[allow(dead_code)]
    error_index: usize,
}

impl BootController {
    /// Create new boot controller
    pub fn new() -> Self {
        Self {
            state: ExecutionState::Uninitialized,
            boot_manager: BootManager::new(),
            system_check: FinalSystemCheck::new(),
            boot_count: 0,
            error_history: [None; 5],
            error_index: 0,
        }
    }

    /// Initialize boot system
    pub fn init_system(&mut self) -> Result<(), &'static str> {
        self.state = ExecutionState::Initializing;
        self.boot_count += 1;

        if self.boot_manager.initialize().is_err() {
            self.state = ExecutionState::Failed;
            return Err("Boot manager initialization failed");
        }

        self.state = ExecutionState::DetectingEnvironment;
        Ok(())
    }

    /// Execute memory detection
    pub fn detect_memory(&mut self, memory_map: E820MemoryMap) -> Result<(), &'static str> {
        self.state = ExecutionState::CheckingMemory;

        if self.boot_manager.detect_memory(memory_map).is_err() {
            self.state = ExecutionState::Failed;
            return Err("Memory detection failed");
        }

        Ok(())
    }

    /// Validate bootable media
    pub fn validate_media(&mut self) -> Result<(), &'static str> {
        self.state = ExecutionState::ValidatingDisk;

        if self.boot_manager.validate_bootable_media().is_err() {
            self.state = ExecutionState::Failed;
            return Err("Media validation failed");
        }

        Ok(())
    }

    /// Load kernel from disk
    pub fn load_kernel(&mut self, kernel_address: u64) -> Result<(), &'static str> {
        self.state = ExecutionState::LoadingKernel;

        if self.boot_manager.load_kernel(kernel_address).is_err() {
            self.state = ExecutionState::Failed;
            return Err("Kernel loading failed");
        }

        self.state = ExecutionState::VerifyingKernel;
        Ok(())
    }

    /// Verify kernel integrity
    pub fn verify_kernel(
        &mut self,
        signature: u32,
        checksum: u32,
    ) -> Result<(), &'static str> {
        if self.boot_manager.verify_kernel(signature, checksum).is_err() {
            self.state = ExecutionState::Failed;
            return Err("Kernel verification failed");
        }

        self.state = ExecutionState::PreparingBoot;
        Ok(())
    }

    /// Setup boot information
    pub fn setup_boot(&mut self) -> Result<(), &'static str> {
        if self.boot_manager.setup_boot_info().is_err() {
            self.state = ExecutionState::Failed;
            return Err("Boot setup failed");
        }

        self.state = ExecutionState::VerifyingSystem;
        Ok(())
    }

    /// Run all system checks
    pub fn run_system_checks(&mut self) -> Result<(), &'static str> {
        if self.system_check.run_all_checks().is_err() {
            self.state = ExecutionState::Failed;
            return Err("System checks failed");
        }

        if !self.system_check.is_system_ready() {
            self.state = ExecutionState::Failed;
            return Err("System not ready");
        }

        Ok(())
    }

    /// Verify all prerequisites
    pub fn verify_all(&mut self) -> Result<(), &'static str> {
        if self.boot_manager.verify_all_systems().is_err() {
            self.state = ExecutionState::Failed;
            return Err("Final verification failed");
        }

        self.state = ExecutionState::ReadyForTransfer;
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
        self.init_system()?;
        self.detect_memory(memory_map)?;
        self.validate_media()?;
        self.load_kernel(kernel_address)?;
        self.verify_kernel(signature, checksum)?;
        self.setup_boot()?;
        self.run_system_checks()?;
        self.verify_all()?;

        Ok(())
    }

    /// Record error
    #[allow(dead_code)]
    fn record_error(&mut self, error: &'static str) {
        self.error_history[self.error_index] = Some(error);
        self.error_index = (self.error_index + 1) % self.error_history.len();
    }

    /// Get current state
    pub fn state(&self) -> ExecutionState {
        self.state
    }

    /// Check if ready to transfer to kernel
    pub fn is_ready_to_transfer(&self) -> bool {
        self.state == ExecutionState::ReadyForTransfer && self.boot_manager.is_ready()
    }

    /// Check if boot failed
    pub fn has_failed(&self) -> bool {
        self.state == ExecutionState::Failed
    }

    /// Get boot statistics
    pub fn get_statistics(&self) -> BootStatistics {
        BootStatistics {
            boot_count: self.boot_count,
            current_state: self.state,
            is_ready: self.is_ready_to_transfer(),
            is_failed: self.has_failed(),
        }
    }

    /// Get detailed status report
    pub fn status_report(&self) -> String {
        let mut report = String::new();

        report.push_str("Boot Controller Status:\n");
        report.push_str(&format!("  State: {}\n", self.state.description()));
        report.push_str(&format!("  Boot Count: {}\n", self.boot_count));
        report.push_str(&format!(
            "  Ready: {}\n",
            if self.is_ready_to_transfer() {
                "Yes"
            } else {
                "No"
            }
        ));
        report.push_str(&format!(
            "  Failed: {}\n",
            if self.has_failed() { "Yes" } else { "No" }
        ));

        report
    }
}

/// Boot statistics
#[derive(Debug, Clone)]
pub struct BootStatistics {
    pub boot_count: u32,
    pub current_state: ExecutionState,
    pub is_ready: bool,
    pub is_failed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_state_description() {
        assert!(ExecutionState::Initializing
            .description()
            .contains("Initializing"));
    }

    #[test]
    fn test_execution_state_is_terminal() {
        assert!(!ExecutionState::Initializing.is_terminal());
        assert!(ExecutionState::ReadyForTransfer.is_terminal());
        assert!(ExecutionState::Failed.is_terminal());
    }

    #[test]
    fn test_execution_state_is_ready() {
        assert!(ExecutionState::ReadyForTransfer.is_ready());
        assert!(!ExecutionState::Initializing.is_ready());
    }

    #[test]
    fn test_boot_controller_creation() {
        let controller = BootController::new();
        assert_eq!(controller.state(), ExecutionState::Uninitialized);
        assert!(!controller.is_ready_to_transfer());
    }

    #[test]
    fn test_boot_controller_init() {
        let mut controller = BootController::new();
        assert!(controller.init_system().is_ok());
        assert_ne!(controller.state(), ExecutionState::Uninitialized);
    }

    #[test]
    fn test_boot_controller_statistics() {
        let controller = BootController::new();
        let stats = controller.get_statistics();

        assert_eq!(stats.boot_count, 0);
        assert!(!stats.is_ready);
    }

    #[test]
    fn test_boot_controller_status_report() {
        let controller = BootController::new();
        let report = controller.status_report();

        assert!(report.contains("Boot Controller Status"));
        assert!(report.contains("Uninitialized"));
    }
}
