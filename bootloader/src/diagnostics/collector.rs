//! Diagnostic System - Integrated boot diagnostics and reporting
//!
//! Collects and reports on:
//! - Boot timing measurements
//! - Hardware detection results
//! - Boot failures and recovery
//! - Performance metrics

use crate::utils::error::Result;
use alloc::string::String;

/// Boot diagnostic information
#[derive(Debug, Clone)]
pub struct BootDiagnostics {
    /// Total boot time in milliseconds
    pub total_boot_time: u32,
    /// Hardware detected capabilities
    pub hardware_info: HardwareInfo,
    /// Boot phases executed
    pub phases_executed: u8,
    /// Any errors encountered
    pub errors: [Option<String>; 4],
    pub error_count: usize,
    /// Graphics initialization result
    pub graphics_status: GraphicsStatus,
}

/// Hardware detection results
#[derive(Debug, Clone, Copy)]
pub struct HardwareInfo {
    pub cpu_count: u8,
    pub total_memory_mb: u32,
    pub has_graphics: bool,
    pub has_network: bool,
    pub has_tpm: bool,
    pub max_resolution_width: u16,
    pub max_resolution_height: u16,
}

/// Graphics initialization status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsStatus {
    /// Not attempted
    NotAttempted,
    /// Graphics initialized successfully
    Success,
    /// Graphics failed
    Failed,
    /// Graphics not available on this hardware
    Unavailable,
}

impl BootDiagnostics {
    /// Create new diagnostics instance
    pub fn new() -> Self {
        Self {
            total_boot_time: 0,
            hardware_info: HardwareInfo {
                cpu_count: 1,
                total_memory_mb: 0,
                has_graphics: false,
                has_network: false,
                has_tpm: false,
                max_resolution_width: 0,
                max_resolution_height: 0,
            },
            phases_executed: 0,
            errors: [None, None, None, None],
            error_count: 0,
            graphics_status: GraphicsStatus::NotAttempted,
        }
    }

    /// Record boot timing
    pub fn set_boot_time(&mut self, time_ms: u32) {
        self.total_boot_time = time_ms;
    }

    /// Record phase execution
    pub fn record_phase(&mut self) {
        self.phases_executed = self.phases_executed.saturating_add(1);
    }

    /// Add error message
    pub fn add_error(&mut self, error: String) -> Result<()> {
        if self.error_count < 4 {
            self.errors[self.error_count] = Some(error);
            self.error_count += 1;
            Ok(())
        } else {
            Err(crate::utils::error::BootError::DeviceError("Too many boot errors"))
        }
    }

    /// Record graphics status
    pub fn set_graphics_status(&mut self, status: GraphicsStatus) {
        self.graphics_status = status;
    }

    /// Generate boot summary string
    pub fn summary(&self) -> String {
        let mut summary = alloc::format!(
            "Boot Summary:\n\
             Time: {}ms\n\
             CPU Count: {}\n\
             Memory: {}MB\n\
             Graphics: {:?}\n\
             Phases: {}",
            self.total_boot_time,
            self.hardware_info.cpu_count,
            self.hardware_info.total_memory_mb,
            self.graphics_status,
            self.phases_executed
        );

        if self.error_count > 0 {
            summary.push_str("\nErrors: ");
            for i in 0..self.error_count {
                if let Some(err) = &self.errors[i] {
                    summary.push_str(err);
                    if i < self.error_count - 1 {
                        summary.push_str(", ");
                    }
                }
            }
        }

        summary
    }

    /// Check if boot was successful
    pub fn is_successful(&self) -> bool {
        self.error_count == 0 && self.phases_executed >= 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostics_creation() {
        let diag = BootDiagnostics::new();
        assert_eq!(diag.total_boot_time, 0);
        assert!(diag.is_successful());
    }

    #[test]
    fn test_error_tracking() {
        let mut diag = BootDiagnostics::new();
        assert!(diag.add_error("Test error".into()).is_ok());
        assert!(!diag.is_successful());
    }
}
