/// Boot Diagnostics and Verification
///
/// Provides comprehensive diagnostics for boot process verification and debugging.
/// Tracks boot events, validates each stage, and provides detailed status reporting.

use crate::boot_stage::boot_loader::{BootEnvironment, BootStatus, LoadedKernel};

/// Boot event types for diagnostic tracking
#[derive(Debug, Clone, Copy)]
pub enum BootEvent {
    BootStarted,
    EnvironmentDetected,
    MemoryDetected,
    MemoryMapValid,
    DiskAccessOK,
    KernelLoadStarted,
    KernelLoaded,
    KernelSignatureValid,
    ChecksumValid,
    BootInfoCreated,
    AllChecksPassed,
    BootFailed,
}

impl BootEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BootStarted => "Boot started",
            Self::EnvironmentDetected => "Environment detected",
            Self::MemoryDetected => "Memory detected",
            Self::MemoryMapValid => "Memory map valid",
            Self::DiskAccessOK => "Disk access OK",
            Self::KernelLoadStarted => "Kernel load started",
            Self::KernelLoaded => "Kernel loaded",
            Self::KernelSignatureValid => "Kernel signature valid",
            Self::ChecksumValid => "Checksum valid",
            Self::BootInfoCreated => "Boot info created",
            Self::AllChecksPassed => "All checks passed",
            Self::BootFailed => "Boot failed",
        }
    }
}

/// Boot diagnostic record
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticRecord {
    pub event: BootEvent,
    pub timestamp: u32,  // In ticks (1/18 second)
    pub success: bool,
}

impl DiagnosticRecord {
    pub fn new(event: BootEvent) -> Self {
        Self {
            event,
            timestamp: 0,
            success: true,
        }
    }

    pub fn with_timestamp(mut self, timestamp: u32) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }
}

/// Boot diagnostics tracker
pub struct BootDiagnostics {
    events: [Option<DiagnosticRecord>; 32],
    event_count: usize,
    boot_start_time: u32,
}

impl BootDiagnostics {
    pub fn new() -> Self {
        // For now, we'll set boot_start_time to 0 since we don't have a real-time clock yet
        // This can be updated when RTC functionality is added
        Self {
            events: [None; 32],
            event_count: 0,
            boot_start_time: 0,
        }
    }

    /// Set the boot start time
    pub fn set_boot_start_time(&mut self, time: u32) {
        self.boot_start_time = time;
    }

    /// Log a boot event with current timestamp
    pub fn log_event(&mut self, event: BootEvent) -> Result<(), &'static str> {
        if self.event_count >= 32 {
            return Err("Event log full");
        }

        // For now, use event count as a simple timestamp
        // This can be updated to use real RTC time when available
        let timestamp = self.event_count as u32;
        let record = DiagnosticRecord::new(event).with_timestamp(timestamp);
        self.events[self.event_count] = Some(record);
        self.event_count += 1;

        Ok(())
    }

    /// Log event with success status
    pub fn log_event_result(&mut self, event: BootEvent, success: bool) -> Result<(), &'static str> {
        if self.event_count >= 32 {
            return Err("Event log full");
        }

        let record = DiagnosticRecord::new(event).with_success(success);
        self.events[self.event_count] = Some(record);
        self.event_count += 1;

        Ok(())
    }

    /// Get all logged events
    pub fn events(&self) -> &[Option<DiagnosticRecord>] {
        &self.events[..self.event_count]
    }

    /// Count successful events
    pub fn success_count(&self) -> usize {
        self.events()
            .iter()
            .filter(|e| e.as_ref().map(|r| r.success).unwrap_or(false))
            .count()
    }

    /// Count failed events
    pub fn failure_count(&self) -> usize {
        self.events()
            .iter()
            .filter(|e| e.as_ref().map(|r| !r.success).unwrap_or(false))
            .count()
    }

    /// Check if all events succeeded
    pub fn all_succeeded(&self) -> bool {
        self.failure_count() == 0
    }

    pub fn event_count(&self) -> usize {
        self.event_count
    }
}

/// Boot verification checklist
pub struct BootVerification {
    memory_detected: bool,
    memory_validated: bool,
    disk_accessible: bool,
    kernel_loaded: bool,
    kernel_valid: bool,
    boot_info_valid: bool,
}

impl BootVerification {
    pub fn new() -> Self {
        Self {
            memory_detected: false,
            memory_validated: false,
            disk_accessible: false,
            kernel_loaded: false,
            kernel_valid: false,
            boot_info_valid: false,
        }
    }

    /// Mark memory as detected
    pub fn check_memory(&mut self) {
        self.memory_detected = true;
    }

    /// Mark memory as validated
    pub fn validate_memory(&mut self) {
        self.memory_validated = true;
    }

    /// Mark disk as accessible
    pub fn check_disk(&mut self) {
        self.disk_accessible = true;
    }

    /// Mark kernel as loaded
    pub fn check_kernel_loaded(&mut self) {
        self.kernel_loaded = true;
    }

    /// Mark kernel as valid
    pub fn check_kernel_valid(&mut self) {
        self.kernel_valid = true;
    }

    /// Mark boot info as valid
    pub fn check_boot_info(&mut self) {
        self.boot_info_valid = true;
    }

    /// Check if all verifications passed
    pub fn all_checks_passed(&self) -> bool {
        self.memory_detected
            && self.memory_validated
            && self.disk_accessible
            && self.kernel_loaded
            && self.kernel_valid
            && self.boot_info_valid
    }

    /// Get status string
    pub fn status_string(&self) -> &'static str {
        if self.all_checks_passed() {
            "All checks passed - ready to boot"
        } else if self.kernel_valid && self.memory_validated {
            "Kernel ready - boot info pending"
        } else if self.kernel_loaded {
            "Kernel loaded - validation pending"
        } else if self.disk_accessible {
            "Disk accessible - kernel load pending"
        } else if self.memory_validated {
            "Memory validated - disk check pending"
        } else if self.memory_detected {
            "Memory detected - validation pending"
        } else {
            "Boot not initialized"
        }
    }

    /// Count passed checks
    pub fn passed_checks(&self) -> usize {
        let mut count = 0;
        if self.memory_detected { count += 1; }
        if self.memory_validated { count += 1; }
        if self.disk_accessible { count += 1; }
        if self.kernel_loaded { count += 1; }
        if self.kernel_valid { count += 1; }
        if self.boot_info_valid { count += 1; }
        count
    }
}

/// Boot status reporter
pub struct BootStatusReport {
    environment: BootEnvironment,
    status: BootStatus,
    total_memory: u64,
    kernel_size: u64,
    kernel_valid: bool,
}

impl BootStatusReport {
    pub fn new() -> Self {
        Self {
            environment: BootEnvironment::Unknown,
            status: BootStatus::NotStarted,
            total_memory: 0,
            kernel_size: 0,
            kernel_valid: false,
        }
    }

    /// Create report from current state
    pub fn from_state(
        environment: BootEnvironment,
        status: BootStatus,
        total_memory: u64,
        kernel: &LoadedKernel,
    ) -> Self {
        Self {
            environment,
            status,
            total_memory,
            kernel_size: kernel.size,
            kernel_valid: kernel.is_valid(),
        }
    }

    /// Get environment string
    pub fn environment_str(&self) -> &'static str {
        self.environment.as_str()
    }

    /// Get status string
    pub fn status_str(&self) -> &'static str {
        self.status.as_str()
    }

    /// Check if boot is successful
    pub fn is_successful(&self) -> bool {
        self.status.is_success() && self.kernel_valid
    }

    /// Get summary
    pub fn summary(&self) -> &'static str {
        if self.is_successful() {
            "Boot successful - ready to execute kernel"
        } else if self.kernel_valid {
            "Kernel valid - boot sequence pending"
        } else if self.total_memory > 0 {
            "Memory detected - kernel loading pending"
        } else {
            "Boot initialization in progress"
        }
    }

    /// Get kernel size in bytes
    pub fn kernel_size(&self) -> u64 {
        self.kernel_size
    }

    /// Check if kernel is properly loaded (size > 0)
    pub fn is_kernel_properly_loaded(&self) -> bool {
        self.kernel_size > 0 && self.kernel_valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_event_strings() {
        assert_eq!(BootEvent::BootStarted.as_str(), "Boot started");
        assert_eq!(BootEvent::KernelLoaded.as_str(), "Kernel loaded");
    }

    #[test]
    fn test_diagnostic_record() {
        let record = DiagnosticRecord::new(BootEvent::BootStarted)
            .with_timestamp(100)
            .with_success(true);
        assert_eq!(record.timestamp, 100);
        assert!(record.success);
    }

    #[test]
    fn test_boot_diagnostics() {
        let mut diag = BootDiagnostics::new();
        assert!(diag.log_event(BootEvent::BootStarted).is_ok());
        assert_eq!(diag.event_count(), 1);
        assert_eq!(diag.success_count(), 1);
    }

    #[test]
    fn test_boot_verification() {
        let mut verify = BootVerification::new();
        assert!(!verify.all_checks_passed());

        verify.check_memory();
        verify.validate_memory();
        verify.check_disk();
        verify.check_kernel_loaded();
        verify.check_kernel_valid();
        verify.check_boot_info();

        assert!(verify.all_checks_passed());
        assert_eq!(verify.passed_checks(), 6);
    }

    #[test]
    fn test_boot_status_report() {
        let report = BootStatusReport::new();
        assert_eq!(report.environment_str(), "Unknown");
        assert!(!report.is_successful());
    }
}
