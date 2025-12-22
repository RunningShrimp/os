/// Advanced Boot Flow Manager v2
///
/// Orchestrates the complete boot sequence from disk to kernel execution.
/// Integrates all boot components into single cohesive workflow.

use alloc::format;
use alloc::string::String;

/// Boot flow stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootFlowStage {
    DetectBootMedia,
    ReadPartitionTable,
    LocateKernel,
    LoadKernel,
    ValidateKernel,
    ConfigureModes,
    BuildBootInfo,
    FinalVerification,
    ReadyForTransfer,
    KernelExecution,
    Failed,
}

impl BootFlowStage {
    pub fn description(&self) -> &'static str {
        match self {
            Self::DetectBootMedia => "Detecting boot media",
            Self::ReadPartitionTable => "Reading partition table",
            Self::LocateKernel => "Locating kernel",
            Self::LoadKernel => "Loading kernel",
            Self::ValidateKernel => "Validating kernel",
            Self::ConfigureModes => "Configuring CPU modes",
            Self::BuildBootInfo => "Building boot information",
            Self::FinalVerification => "Final system verification",
            Self::ReadyForTransfer => "Ready for kernel transfer",
            Self::KernelExecution => "Kernel execution",
            Self::Failed => "Boot failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::KernelExecution | Self::Failed)
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::ReadyForTransfer | Self::KernelExecution)
    }

    pub fn next_stage(&self) -> Option<Self> {
        match self {
            Self::DetectBootMedia => Some(Self::ReadPartitionTable),
            Self::ReadPartitionTable => Some(Self::LocateKernel),
            Self::LocateKernel => Some(Self::LoadKernel),
            Self::LoadKernel => Some(Self::ValidateKernel),
            Self::ValidateKernel => Some(Self::ConfigureModes),
            Self::ConfigureModes => Some(Self::BuildBootInfo),
            Self::BuildBootInfo => Some(Self::FinalVerification),
            Self::FinalVerification => Some(Self::ReadyForTransfer),
            Self::ReadyForTransfer => Some(Self::KernelExecution),
            _ => None,
        }
    }
}

/// Boot flow statistics
#[derive(Debug, Clone)]
pub struct BootFlowStats {
    pub current_stage: BootFlowStage,
    pub stages_completed: u32,
    pub total_stages: u32,
    pub errors: u32,
    pub warnings: u32,
}

impl BootFlowStats {
    pub fn new() -> Self {
        Self {
            current_stage: BootFlowStage::DetectBootMedia,
            stages_completed: 0,
            total_stages: 10,
            errors: 0,
            warnings: 0,
        }
    }

    pub fn progress_percent(&self) -> u32 {
        if self.total_stages == 0 {
            0
        } else {
            (self.stages_completed * 100) / self.total_stages
        }
    }

    pub fn is_on_track(&self) -> bool {
        self.errors == 0 && self.warnings < 3
    }
}

/// Complete boot flow manager
pub struct BootFlowManager {
    stats: BootFlowStats,
    stage_errors: [Option<&'static str>; 11],
    stage_error_count: usize,
}

impl BootFlowManager {
    /// Create new boot flow manager
    pub fn new() -> Self {
        Self {
            stats: BootFlowStats::new(),
            stage_errors: [None; 11],
            stage_error_count: 0,
        }
    }

    /// Progress to next stage
    pub fn next_stage(&mut self) -> Result<(), &'static str> {
        match self.stats.current_stage.next_stage() {
            Some(next) => {
                self.stats.current_stage = next;
                self.stats.stages_completed += 1;
                Ok(())
            }
            None => Err("No next stage"),
        }
    }

    /// Record stage error
    pub fn record_stage_error(&mut self, error: &'static str) {
        if self.stage_error_count < self.stage_errors.len() {
            self.stage_errors[self.stage_error_count] = Some(error);
            self.stage_error_count += 1;
        }

        self.stats.errors += 1;
    }

    /// Record stage warning
    pub fn record_stage_warning(&mut self) {
        self.stats.warnings += 1;
    }

    /// Get current stage
    pub fn current_stage(&self) -> BootFlowStage {
        self.stats.current_stage
    }

    /// Check if boot can proceed
    pub fn can_proceed(&self) -> bool {
        self.stats.errors == 0 && self.stats.warnings < 5
    }

    /// Get boot flow status
    pub fn status(&self) -> BootFlowStatus {
        BootFlowStatus {
            current_stage: self.stats.current_stage,
            progress: self.stats.progress_percent(),
            can_proceed: self.can_proceed(),
            errors: self.stats.errors,
            warnings: self.stats.warnings,
        }
    }

    /// Get detailed report
    pub fn detailed_report(&self) -> String {
        let mut report = String::new();

        report.push_str("Boot Flow Status:\n");
        report.push_str(&format!(
            "  Current Stage: {} ({}%)\n",
            self.stats.current_stage.description(),
            self.stats.progress_percent()
        ));
        report.push_str(&format!(
            "  Progress: {}/{}\n",
            self.stats.stages_completed, self.stats.total_stages
        ));
        report.push_str(&format!("  Errors: {}\n", self.stats.errors));
        report.push_str(&format!("  Warnings: {}\n", self.stats.warnings));
        report.push_str(&format!(
            "  On Track: {}\n",
            if self.stats.is_on_track() { "Yes" } else { "No" }
        ));

        if self.stage_error_count > 0 {
            report.push_str("  Error Log:\n");
            for (i, error) in self.stage_errors[..self.stage_error_count].iter().enumerate() {
                if let Some(e) = error {
                    report.push_str(&format!("    {}: {}\n", i + 1, e));
                }
            }
        }

        report
    }

    /// Fail boot with error
    pub fn fail_boot(&mut self, error: &'static str) {
        self.stats.current_stage = BootFlowStage::Failed;
        self.record_stage_error(error);
    }

    /// Complete boot successfully
    pub fn complete_boot(&mut self) {
        self.stats.current_stage = BootFlowStage::KernelExecution;
    }

    /// Check if boot succeeded
    pub fn is_successful(&self) -> bool {
        self.stats.current_stage.is_success()
    }

    /// Check if boot failed
    pub fn has_failed(&self) -> bool {
        self.stats.current_stage == BootFlowStage::Failed
    }
}

/// Boot flow status
#[derive(Debug, Clone)]
pub struct BootFlowStatus {
    pub current_stage: BootFlowStage,
    pub progress: u32,
    pub can_proceed: bool,
    pub errors: u32,
    pub warnings: u32,
}

impl BootFlowStatus {
    pub fn summary(&self) -> String {
        format!(
            "Boot: {} ({}% - E:{} W:{})",
            self.current_stage.description(),
            self.progress,
            self.errors,
            self.warnings
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_flow_stage_description() {
        assert!(BootFlowStage::DetectBootMedia
            .description()
            .contains("media"));
        assert!(BootFlowStage::LoadKernel
            .description()
            .contains("kernel"));
    }

    #[test]
    fn test_boot_flow_stage_is_terminal() {
        assert!(!BootFlowStage::LoadKernel.is_terminal());
        assert!(BootFlowStage::KernelExecution.is_terminal());
    }

    #[test]
    fn test_boot_flow_stage_is_success() {
        assert!(BootFlowStage::ReadyForTransfer.is_success());
        assert!(!BootFlowStage::ValidateKernel.is_success());
    }

    #[test]
    fn test_boot_flow_stage_next() {
        assert_eq!(
            BootFlowStage::DetectBootMedia.next_stage(),
            Some(BootFlowStage::ReadPartitionTable)
        );
    }

    #[test]
    fn test_boot_flow_stats_progress() {
        let mut stats = BootFlowStats::new();
        stats.stages_completed = 5;
        assert_eq!(stats.progress_percent(), 50);
    }

    #[test]
    fn test_boot_flow_manager_next_stage() {
        let mut manager = BootFlowManager::new();
        assert!(manager.next_stage().is_ok());
        assert_eq!(manager.current_stage(), BootFlowStage::ReadPartitionTable);
    }

    #[test]
    fn test_boot_flow_manager_record_error() {
        let mut manager = BootFlowManager::new();
        manager.record_stage_error("Test error");

        assert_eq!(manager.stats.errors, 1);
        assert!(!manager.can_proceed());
    }

    #[test]
    fn test_boot_flow_manager_fail_boot() {
        let mut manager = BootFlowManager::new();
        manager.fail_boot("Test failure");

        assert!(manager.has_failed());
    }

    #[test]
    fn test_boot_flow_manager_complete_boot() {
        let mut manager = BootFlowManager::new();
        manager.complete_boot();

        assert!(manager.is_successful());
    }

    #[test]
    fn test_boot_flow_status_summary() {
        let status = BootFlowStatus {
            current_stage: BootFlowStage::LoadKernel,
            progress: 50,
            can_proceed: true,
            errors: 0,
            warnings: 0,
        };

        let summary = status.summary();
        assert!(summary.contains("50%"));
    }
}
