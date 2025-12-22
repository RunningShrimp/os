//! Boot Recovery System
//!
//! Provides boot recovery and error handling including:
//! - Error diagnosis and reporting
//! - System state preservation
//! - Recovery action execution
//! - Event logging and history

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;


/// Boot recovery error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryErrorCode {
    NoError,
    InvalidBootDevice,
    KernelLoadFailed,
    ValidationFailed,
    MemoryAllocationFailed,
    InterruptSetupFailed,
    ModeSwitchFailed,
    InvalidElfImage,
    MissingBootProtocol,
    UnrecoverableError,
}

impl fmt::Display for RecoveryErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryErrorCode::NoError => write!(f, "No Error"),
            RecoveryErrorCode::InvalidBootDevice => write!(f, "Invalid Boot Device"),
            RecoveryErrorCode::KernelLoadFailed => write!(f, "Kernel Load Failed"),
            RecoveryErrorCode::ValidationFailed => write!(f, "Validation Failed"),
            RecoveryErrorCode::MemoryAllocationFailed => write!(f, "Memory Allocation Failed"),
            RecoveryErrorCode::InterruptSetupFailed => write!(f, "Interrupt Setup Failed"),
            RecoveryErrorCode::ModeSwitchFailed => write!(f, "Mode Switch Failed"),
            RecoveryErrorCode::InvalidElfImage => write!(f, "Invalid ELF Image"),
            RecoveryErrorCode::MissingBootProtocol => write!(f, "Missing Boot Protocol"),
            RecoveryErrorCode::UnrecoverableError => write!(f, "Unrecoverable Error"),
        }
    }
}

/// Recovery action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    Retry,
    SkipComponent,
    UseDefaults,
    ReduceMemory,
    DisableFeature,
    BootInSafeMode,
    SystemHalt,
}

impl fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryAction::Retry => write!(f, "Retry"),
            RecoveryAction::SkipComponent => write!(f, "Skip Component"),
            RecoveryAction::UseDefaults => write!(f, "Use Defaults"),
            RecoveryAction::ReduceMemory => write!(f, "Reduce Memory"),
            RecoveryAction::DisableFeature => write!(f, "Disable Feature"),
            RecoveryAction::BootInSafeMode => write!(f, "Boot in Safe Mode"),
            RecoveryAction::SystemHalt => write!(f, "System Halt"),
        }
    }
}

/// Boot event for logging
#[derive(Debug, Clone)]
pub struct BootEvent {
    pub event_type: String,
    pub severity: u8, // 0=info, 1=warning, 2=error, 3=critical
    pub message: String,
    pub timestamp: u64,
    pub context: String,
}

impl BootEvent {
    /// Create new boot event
    pub fn new(event_type: &str, severity: u8, message: &str) -> Self {
        BootEvent {
            event_type: String::from(event_type),
            severity,
            message: String::from(message),
            timestamp: 0,
            context: String::new(),
        }
    }

    /// Create info event
    pub fn info(message: &str) -> Self {
        Self::new("INFO", 0, message)
    }

    /// Create warning event
    pub fn warning(message: &str) -> Self {
        Self::new("WARNING", 1, message)
    }

    /// Create error event
    pub fn error(message: &str) -> Self {
        Self::new("ERROR", 2, message)
    }

    /// Create critical event
    pub fn critical(message: &str) -> Self {
        Self::new("CRITICAL", 3, message)
    }

    /// Set event context
    pub fn with_context(&mut self, context: &str) {
        self.context = String::from(context);
    }

    /// Get severity name
    pub fn severity_name(&self) -> &'static str {
        match self.severity {
            0 => "INFO",
            1 => "WARNING",
            2 => "ERROR",
            3 => "CRITICAL",
            _ => "UNKNOWN",
        }
    }
}

impl fmt::Display for BootEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.severity_name(),
            self.event_type,
            self.message
        )
    }
}

/// Recovery context for state preservation
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    pub error_code: RecoveryErrorCode,
    pub affected_component: String,
    pub last_successful_state: String,
    pub system_flags: u32,
    pub memory_available: u64,
}

impl RecoveryContext {
    /// Create new recovery context
    pub fn new(error_code: RecoveryErrorCode) -> Self {
        RecoveryContext {
            error_code,
            affected_component: String::new(),
            last_successful_state: String::new(),
            system_flags: 0,
            memory_available: 0,
        }
    }

    /// Set affected component
    pub fn set_component(&mut self, component: &str) {
        self.affected_component = String::from(component);
    }

    /// Set last successful state
    pub fn set_last_state(&mut self, state: &str) {
        self.last_successful_state = String::from(state);
    }

    /// Check if recovery possible
    pub fn can_recover(&self) -> bool {
        self.error_code != RecoveryErrorCode::UnrecoverableError
            && !self.affected_component.is_empty()
    }
}

impl fmt::Display for RecoveryContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RecoveryContext {{ error: {}, component: {}, memory: {} }}",
            self.error_code, self.affected_component, self.memory_available
        )
    }
}

/// Boot recovery system
pub struct BootRecovery {
    events: Vec<BootEvent>,
    contexts: Vec<RecoveryContext>,
    recovery_actions: Vec<RecoveryAction>,
    last_error: RecoveryErrorCode,
    recovery_count: u32,
    max_recovery_attempts: u32,
    safe_mode_enabled: bool,
}

impl BootRecovery {
    /// Create new boot recovery system
    pub fn new() -> Self {
        BootRecovery {
            events: Vec::new(),
            contexts: Vec::new(),
            recovery_actions: Vec::new(),
            last_error: RecoveryErrorCode::NoError,
            recovery_count: 0,
            max_recovery_attempts: 5,
            safe_mode_enabled: false,
        }
    }

    /// Log boot event
    pub fn log_event(&mut self, event: BootEvent) -> bool {
        self.events.push(event);
        true
    }

    /// Log diagnostic information
    pub fn log_diagnostic(&mut self, category: &str, message: &str) {
        let mut event = BootEvent::new(category, 0, message);
        event.timestamp = self.events.len() as u64;
        self.log_event(event);
    }

    /// Report error with context
    pub fn report_error(&mut self, error: RecoveryErrorCode, component: &str) -> bool {
        let mut context = RecoveryContext::new(error);
        context.set_component(component);
        
        let message = format!("Error in {}: {}", component, error);
        self.log_event(BootEvent::error(&message));
        
        self.last_error = error;
        self.contexts.push(context);
        true
    }

    /// Attempt recovery action
    pub fn attempt_recovery(&mut self, action: RecoveryAction) -> bool {
        if self.recovery_count >= self.max_recovery_attempts {
            let event = BootEvent::critical("Max recovery attempts exceeded");
            self.log_event(event);
            return false;
        }

        self.recovery_count += 1;
        self.recovery_actions.push(action);

        match action {
            RecoveryAction::BootInSafeMode => {
                self.safe_mode_enabled = true;
                self.log_diagnostic("RECOVERY", "Entering safe mode");
                true
            }
            RecoveryAction::Retry => {
                self.log_diagnostic("RECOVERY", "Retrying failed operation");
                true
            }
            RecoveryAction::UseDefaults => {
                self.log_diagnostic("RECOVERY", "Using default configuration");
                true
            }
            RecoveryAction::SkipComponent => {
                self.log_diagnostic("RECOVERY", "Skipping failed component");
                true
            }
            _ => {
                self.log_diagnostic("RECOVERY", &format!("Executing: {}", action));
                true
            }
        }
    }

    /// Get recovery recommendation
    pub fn get_recovery_recommendation(&self) -> Option<RecoveryAction> {
        match self.last_error {
            RecoveryErrorCode::NoError => None,
            RecoveryErrorCode::KernelLoadFailed => Some(RecoveryAction::Retry),
            RecoveryErrorCode::ValidationFailed => Some(RecoveryAction::SkipComponent),
            RecoveryErrorCode::MemoryAllocationFailed => Some(RecoveryAction::ReduceMemory),
            RecoveryErrorCode::ModeSwitchFailed => Some(RecoveryAction::BootInSafeMode),
            RecoveryErrorCode::UnrecoverableError => Some(RecoveryAction::SystemHalt),
            _ => Some(RecoveryAction::UseDefaults),
        }
    }

    /// Check if safe mode is enabled
    pub fn is_safe_mode(&self) -> bool {
        self.safe_mode_enabled
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.events.iter().filter(|e| e.severity >= 2).count()
    }

    /// Get all events
    pub fn get_events(&self) -> Vec<&BootEvent> {
        self.events.iter().collect()
    }

    /// Get last error
    pub fn get_last_error(&self) -> RecoveryErrorCode {
        self.last_error
    }

    /// Get recovery statistics
    pub fn get_stats(&self) -> (u32, usize, usize) {
        (self.recovery_count, self.event_count(), self.error_count())
    }

    /// Get detailed diagnostic report
    pub fn diagnostic_report(&self) -> String {
        format!(
            "BootRecovery {{ last_error: {}, recovery_count: {}, events: {}, errors: {}, safe_mode: {} }}",
            self.last_error,
            self.recovery_count,
            self.event_count(),
            self.error_count(),
            self.safe_mode_enabled
        )
    }

    /// Clear recovery history
    pub fn clear_history(&mut self) {
        self.events.clear();
        self.contexts.clear();
        self.recovery_actions.clear();
        self.recovery_count = 0;
    }

    /// Set max recovery attempts
    pub fn set_max_attempts(&mut self, max: u32) {
        self.max_recovery_attempts = max;
    }

    /// Export event log as string
    pub fn export_event_log(&self) -> String {
        let mut log = String::from("=== Boot Event Log ===\n");
        for event in &self.events {
            log.push_str(&format!("{}\n", event));
        }
        log.push_str(&format!("\n{}\n", self.diagnostic_report()));
        log
    }
}

impl fmt::Display for BootRecovery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.diagnostic_report())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_error_code_display() {
        assert_eq!(RecoveryErrorCode::NoError.to_string(), "No Error");
        assert_eq!(RecoveryErrorCode::KernelLoadFailed.to_string(), "Kernel Load Failed");
    }

    #[test]
    fn test_recovery_action_display() {
        assert_eq!(RecoveryAction::Retry.to_string(), "Retry");
        assert_eq!(RecoveryAction::BootInSafeMode.to_string(), "Boot in Safe Mode");
    }

    #[test]
    fn test_boot_event_creation() {
        let event = BootEvent::info("Boot started");
        assert_eq!(event.severity, 0);
        assert_eq!(event.message, "Boot started");
    }

    #[test]
    fn test_boot_event_levels() {
        let info = BootEvent::info("test");
        let warning = BootEvent::warning("test");
        let error = BootEvent::error("test");
        let critical = BootEvent::critical("test");

        assert_eq!(info.severity, 0);
        assert_eq!(warning.severity, 1);
        assert_eq!(error.severity, 2);
        assert_eq!(critical.severity, 3);
    }

    #[test]
    fn test_boot_event_severity_name() {
        let event = BootEvent::error("test");
        assert_eq!(event.severity_name(), "ERROR");
    }

    #[test]
    fn test_boot_event_context() {
        let mut event = BootEvent::info("test");
        event.with_context("kernel_load");
        assert_eq!(event.context, "kernel_load");
    }

    #[test]
    fn test_recovery_context_creation() {
        let ctx = RecoveryContext::new(RecoveryErrorCode::KernelLoadFailed);
        assert_eq!(ctx.error_code, RecoveryErrorCode::KernelLoadFailed);
    }

    #[test]
    fn test_recovery_context_component() {
        let mut ctx = RecoveryContext::new(RecoveryErrorCode::ValidationFailed);
        ctx.set_component("kernel_validator");
        assert_eq!(ctx.affected_component, "kernel_validator");
    }

    #[test]
    fn test_recovery_context_can_recover() {
        let mut ctx = RecoveryContext::new(RecoveryErrorCode::ModeSwitchFailed);
        ctx.set_component("cpu");
        assert!(ctx.can_recover());

        let unrecoverable = RecoveryContext::new(RecoveryErrorCode::UnrecoverableError);
        assert!(!unrecoverable.can_recover());
    }

    #[test]
    fn test_boot_recovery_creation() {
        let recovery = BootRecovery::new();
        assert_eq!(recovery.event_count(), 0);
        assert!(!recovery.is_safe_mode());
    }

    #[test]
    fn test_boot_recovery_log_event() {
        let mut recovery = BootRecovery::new();
        let event = BootEvent::info("Test event");
        assert!(recovery.log_event(event));
        assert_eq!(recovery.event_count(), 1);
    }

    #[test]
    fn test_boot_recovery_log_diagnostic() {
        let mut recovery = BootRecovery::new();
        recovery.log_diagnostic("BOOT", "BIOS initialized");
        assert_eq!(recovery.event_count(), 1);
    }

    #[test]
    fn test_boot_recovery_report_error() {
        let mut recovery = BootRecovery::new();
        assert!(recovery.report_error(RecoveryErrorCode::KernelLoadFailed, "kernel_loader"));
        assert_eq!(recovery.get_last_error(), RecoveryErrorCode::KernelLoadFailed);
        assert!(recovery.event_count() > 0);
    }

    #[test]
    fn test_boot_recovery_attempt_recovery() {
        let mut recovery = BootRecovery::new();
        assert!(recovery.attempt_recovery(RecoveryAction::Retry));
        assert_eq!(recovery.recovery_count, 1);
    }

    #[test]
    fn test_boot_recovery_safe_mode() {
        let mut recovery = BootRecovery::new();
        assert!(!recovery.is_safe_mode());
        recovery.attempt_recovery(RecoveryAction::BootInSafeMode);
        assert!(recovery.is_safe_mode());
    }

    #[test]
    fn test_boot_recovery_recommendation() {
        let mut recovery = BootRecovery::new();
        recovery.last_error = RecoveryErrorCode::KernelLoadFailed;
        
        let recommendation = recovery.get_recovery_recommendation();
        assert_eq!(recommendation, Some(RecoveryAction::Retry));
    }

    #[test]
    fn test_boot_recovery_max_attempts() {
        let mut recovery = BootRecovery::new();
        recovery.set_max_attempts(2);
        
        assert!(recovery.attempt_recovery(RecoveryAction::Retry));
        assert!(recovery.attempt_recovery(RecoveryAction::Retry));
        assert!(!recovery.attempt_recovery(RecoveryAction::Retry));
    }

    #[test]
    fn test_boot_recovery_error_count() {
        let mut recovery = BootRecovery::new();
        recovery.log_event(BootEvent::info("info"));
        recovery.log_event(BootEvent::error("error"));
        recovery.log_event(BootEvent::critical("critical"));
        
        assert_eq!(recovery.error_count(), 2);
    }

    #[test]
    fn test_boot_recovery_statistics() {
        let mut recovery = BootRecovery::new();
        recovery.log_event(BootEvent::info("event1"));
        recovery.attempt_recovery(RecoveryAction::Retry);
        
        let (recovery_count, event_count, _error_count) = recovery.get_stats();
        assert_eq!(recovery_count, 1);
        assert_eq!(event_count, 1);
    }

    #[test]
    fn test_boot_recovery_clear_history() {
        let mut recovery = BootRecovery::new();
        recovery.log_event(BootEvent::info("test"));
        recovery.attempt_recovery(RecoveryAction::Retry);
        
        assert!(recovery.event_count() > 0);
        recovery.clear_history();
        assert_eq!(recovery.event_count(), 0);
        assert_eq!(recovery.recovery_count, 0);
    }

    #[test]
    fn test_boot_recovery_event_log_export() {
        let mut recovery = BootRecovery::new();
        recovery.log_event(BootEvent::info("Boot started"));
        
        let log = recovery.export_event_log();
        assert!(log.contains("Boot Event Log"));
        assert!(log.contains("Boot started"));
    }
}
