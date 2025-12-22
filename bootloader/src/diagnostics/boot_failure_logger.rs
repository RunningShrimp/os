//! Boot Failure Logger - Persistent error recording and recovery hints
//!
//! Provides:
//! - Error logging
//! - Error categorization
//! - Recovery suggestions
//! - Error diagnostics

/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Hardware error
    Hardware,
    /// Firmware error
    Firmware,
    /// Memory error
    Memory,
    /// Device error
    Device,
    /// Configuration error
    Configuration,
    /// Timeout error
    Timeout,
    /// Unknown error
    Unknown,
}

/// Error component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorComponent {
    /// CPU component
    Cpu,
    /// Memory component
    Memory,
    /// Disk component
    Disk,
    /// Network component
    Network,
    /// BIOS component
    Bios,
    /// Power component
    Power,
    /// Thermal component
    Thermal,
    /// Unknown component
    Unknown,
}

/// Boot error entry
#[derive(Debug, Clone, Copy)]
pub struct BootError {
    /// Error timestamp
    pub timestamp: u64,
    /// Error category
    pub category: ErrorCategory,
    /// Error component
    pub component: ErrorComponent,
    /// Error code
    pub error_code: u16,
    /// Error message length
    pub message_len: u8,
    /// Valid flag
    pub valid: bool,
    /// Recovery possible
    pub recoverable: bool,
}

impl BootError {
    /// Create boot error
    pub fn new(timestamp: u64, category: ErrorCategory, component: ErrorComponent) -> Self {
        BootError {
            timestamp,
            category,
            component,
            error_code: 0,
            message_len: 0,
            valid: true,
            recoverable: false,
        }
    }

    /// Set error code
    pub fn set_error_code(&mut self, code: u16) {
        self.error_code = code;
    }

    /// Set recoverable flag
    pub fn set_recoverable(&mut self, recoverable: bool) {
        self.recoverable = recoverable;
    }
}

/// Boot failure logger
pub struct BootFailureLogger {
    /// Error log
    errors: [Option<BootError>; 256],
    /// Error count
    error_count: usize,
    /// Critical error flag
    critical_error: bool,
    /// Boot success flag
    boot_success: bool,
    /// Hardware errors
    hardware_errors: u32,
    /// Firmware errors
    firmware_errors: u32,
    /// Memory errors
    memory_errors: u32,
    /// Configuration errors
    config_errors: u32,
}

impl BootFailureLogger {
    /// Create boot failure logger
    pub fn new() -> Self {
        BootFailureLogger {
            errors: [None; 256],
            error_count: 0,
            critical_error: false,
            boot_success: false,
            hardware_errors: 0,
            firmware_errors: 0,
            memory_errors: 0,
            config_errors: 0,
        }
    }

    /// Log error
    pub fn log_error(&mut self, error: BootError) -> bool {
        if self.error_count < 256 {
            self.errors[self.error_count] = Some(error);
            self.error_count += 1;

            // Update category counters
            match error.category {
                ErrorCategory::Hardware => self.hardware_errors += 1,
                ErrorCategory::Firmware => self.firmware_errors += 1,
                ErrorCategory::Memory => self.memory_errors += 1,
                ErrorCategory::Configuration => self.config_errors += 1,
                _ => {}
            }

            // Check critical
            if matches!(error.category, ErrorCategory::Hardware | ErrorCategory::Firmware)
                && !error.recoverable
            {
                self.critical_error = true;
            }

            true
        } else {
            false
        }
    }

    /// Get error by index
    pub fn get_error(&self, index: usize) -> Option<&BootError> {
        if index < self.error_count {
            self.errors[index].as_ref()
        } else {
            None
        }
    }

    /// Get error count
    pub fn get_error_count(&self) -> usize {
        self.error_count
    }

    /// Get errors by category
    pub fn get_errors_by_category(&self, category: ErrorCategory) -> u32 {
        match category {
            ErrorCategory::Hardware => self.hardware_errors,
            ErrorCategory::Firmware => self.firmware_errors,
            ErrorCategory::Memory => self.memory_errors,
            ErrorCategory::Configuration => self.config_errors,
            _ => 0,
        }
    }

    /// Check critical error
    pub fn has_critical_error(&self) -> bool {
        self.critical_error
    }

    /// Set boot success
    pub fn set_boot_success(&mut self, success: bool) {
        self.boot_success = success;
    }

    /// Check boot success
    pub fn is_boot_success(&self) -> bool {
        self.boot_success
    }

    /// Get recovery suggestion
    pub fn get_recovery_suggestion(&self, error: &BootError) -> u32 {
        match (error.category, error.component) {
            (ErrorCategory::Hardware, ErrorComponent::Memory) => {
                // Memory error recovery
                1
            }
            (ErrorCategory::Hardware, ErrorComponent::Cpu) => {
                // CPU error recovery
                2
            }
            (ErrorCategory::Firmware, _) => {
                // Firmware error recovery
                3
            }
            (ErrorCategory::Configuration, _) => {
                // Configuration error recovery
                4
            }
            _ => 0,
        }
    }

    /// Get most critical error
    pub fn get_most_critical_error(&self) -> Option<&BootError> {
        let mut most_critical = None;
        let mut max_severity = 0;

        for i in 0..self.error_count {
            if let Some(e) = &self.errors[i] {
                let severity = match e.category {
                    ErrorCategory::Hardware => 10,
                    ErrorCategory::Firmware => 9,
                    ErrorCategory::Memory => 8,
                    ErrorCategory::Configuration => 5,
                    _ => 1,
                };

                if severity > max_severity && !e.recoverable {
                    max_severity = severity;
                    most_critical = Some(e);
                }
            }
        }

        most_critical
    }

    /// Get recoverable error count
    pub fn get_recoverable_error_count(&self) -> u32 {
        let mut count = 0;
        for i in 0..self.error_count {
            if let Some(e) = &self.errors[i] {
                if e.recoverable {
                    count += 1;
                }
            }
        }
        count
    }

    /// Get first error
    pub fn get_first_error(&self) -> Option<&BootError> {
        if self.error_count > 0 {
            self.errors[0].as_ref()
        } else {
            None
        }
    }

    /// Get last error
    pub fn get_last_error(&self) -> Option<&BootError> {
        if self.error_count > 0 {
            self.errors[self.error_count - 1].as_ref()
        } else {
            None
        }
    }

    /// Clear error log
    pub fn clear_error_log(&mut self) {
        for i in 0..256 {
            self.errors[i] = None;
        }
        self.error_count = 0;
        self.critical_error = false;
        self.hardware_errors = 0;
        self.firmware_errors = 0;
        self.memory_errors = 0;
        self.config_errors = 0;
    }

    /// Check if error log is full
    pub fn is_full(&self) -> bool {
        self.error_count >= 256
    }

    /// Get errors by component
    pub fn get_errors_by_component(&self, component: ErrorComponent) -> u32 {
        let mut count = 0;
        for i in 0..self.error_count {
            if let Some(e) = &self.errors[i] {
                if e.component == component {
                    count += 1;
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_creation() {
        let category = ErrorCategory::Hardware;
        assert_eq!(category, ErrorCategory::Hardware);
    }

    #[test]
    fn test_error_component_creation() {
        let component = ErrorComponent::Memory;
        assert_eq!(component, ErrorComponent::Memory);
    }

    #[test]
    fn test_boot_error_creation() {
        let error = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        assert_eq!(error.category, ErrorCategory::Hardware);
        assert!(error.valid);
    }

    #[test]
    fn test_boot_error_set_code() {
        let mut error = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        error.set_error_code(42);
        assert_eq!(error.error_code, 42);
    }

    #[test]
    fn test_boot_error_set_recoverable() {
        let mut error = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        error.set_recoverable(true);
        assert!(error.recoverable);
    }

    #[test]
    fn test_logger_creation() {
        let logger = BootFailureLogger::new();
        assert_eq!(logger.get_error_count(), 0);
        assert!(!logger.has_critical_error());
    }

    #[test]
    fn test_log_single_error() {
        let mut logger = BootFailureLogger::new();
        let error = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        assert!(logger.log_error(error));
        assert_eq!(logger.get_error_count(), 1);
    }

    #[test]
    fn test_get_error_by_index() {
        let mut logger = BootFailureLogger::new();
        let error = BootError::new(100, ErrorCategory::Firmware, ErrorComponent::Bios);
        logger.log_error(error);
        assert!(logger.get_error(0).is_some());
    }

    #[test]
    fn test_log_multiple_errors() {
        let mut logger = BootFailureLogger::new();
        for i in 0..5 {
            let error = BootError::new(i as u64, ErrorCategory::Hardware, ErrorComponent::Cpu);
            logger.log_error(error);
        }
        assert_eq!(logger.get_error_count(), 5);
    }

    #[test]
    fn test_get_errors_by_category() {
        let mut logger = BootFailureLogger::new();
        let error1 = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        let error2 = BootError::new(1, ErrorCategory::Hardware, ErrorComponent::Cpu);
        logger.log_error(error1);
        logger.log_error(error2);
        assert_eq!(logger.get_errors_by_category(ErrorCategory::Hardware), 2);
    }

    #[test]
    fn test_critical_error_detection() {
        let mut logger = BootFailureLogger::new();
        let mut error = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        error.set_recoverable(false);
        logger.log_error(error);
        assert!(logger.has_critical_error());
    }

    #[test]
    fn test_set_boot_success() {
        let mut logger = BootFailureLogger::new();
        logger.set_boot_success(true);
        assert!(logger.is_boot_success());
    }

    #[test]
    fn test_get_most_critical_error() {
        let mut logger = BootFailureLogger::new();
        let mut error1 = BootError::new(0, ErrorCategory::Configuration, ErrorComponent::Unknown);
        error1.set_recoverable(false);
        let mut error2 = BootError::new(1, ErrorCategory::Hardware, ErrorComponent::Memory);
        error2.set_recoverable(false);
        logger.log_error(error1);
        logger.log_error(error2);
        
        let critical = logger.get_most_critical_error();
        assert!(critical.is_some());
    }

    #[test]
    fn test_get_recoverable_error_count() {
        let mut logger = BootFailureLogger::new();
        let mut error = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        error.set_recoverable(true);
        logger.log_error(error);
        assert_eq!(logger.get_recoverable_error_count(), 1);
    }

    #[test]
    fn test_get_first_error() {
        let mut logger = BootFailureLogger::new();
        let error = BootError::new(100, ErrorCategory::Firmware, ErrorComponent::Bios);
        logger.log_error(error);
        let first = logger.get_first_error();
        assert!(first.is_some());
    }

    #[test]
    fn test_get_last_error() {
        let mut logger = BootFailureLogger::new();
        let error1 = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        let error2 = BootError::new(1, ErrorCategory::Firmware, ErrorComponent::Bios);
        logger.log_error(error1);
        logger.log_error(error2);
        let last = logger.get_last_error();
        assert!(last.is_some());
    }

    #[test]
    fn test_clear_error_log() {
        let mut logger = BootFailureLogger::new();
        let error = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        logger.log_error(error);
        logger.clear_error_log();
        assert_eq!(logger.get_error_count(), 0);
    }

    #[test]
    fn test_is_full() {
        let mut logger = BootFailureLogger::new();
        for i in 0..256 {
            let error = BootError::new(i as u64, ErrorCategory::Hardware, ErrorComponent::Memory);
            let _ = logger.log_error(error);
        }
        assert!(logger.is_full());
    }

    #[test]
    fn test_get_errors_by_component() {
        let mut logger = BootFailureLogger::new();
        let error1 = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        let error2 = BootError::new(1, ErrorCategory::Hardware, ErrorComponent::Memory);
        logger.log_error(error1);
        logger.log_error(error2);
        assert_eq!(logger.get_errors_by_component(ErrorComponent::Memory), 2);
    }

    #[test]
    fn test_get_recovery_suggestion() {
        let logger = BootFailureLogger::new();
        let error = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        let suggestion = logger.get_recovery_suggestion(&error);
        assert!(suggestion > 0);
    }

    #[test]
    fn test_multiple_categories() {
        let mut logger = BootFailureLogger::new();
        let error1 = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Memory);
        let error2 = BootError::new(1, ErrorCategory::Firmware, ErrorComponent::Bios);
        let error3 = BootError::new(2, ErrorCategory::Configuration, ErrorComponent::Unknown);
        logger.log_error(error1);
        logger.log_error(error2);
        logger.log_error(error3);
        assert_eq!(logger.get_error_count(), 3);
    }

    #[test]
    fn test_error_timestamp_tracking() {
        let mut logger = BootFailureLogger::new();
        let error1 = BootError::new(1000, ErrorCategory::Hardware, ErrorComponent::Cpu);
        let error2 = BootError::new(2000, ErrorCategory::Hardware, ErrorComponent::Memory);
        logger.log_error(error1);
        logger.log_error(error2);
        
        let first = logger.get_first_error().unwrap();
        let last = logger.get_last_error().unwrap();
        assert!(last.timestamp > first.timestamp);
    }

    #[test]
    fn test_firmware_error_categorization() {
        let mut logger = BootFailureLogger::new();
        let mut error = BootError::new(0, ErrorCategory::Firmware, ErrorComponent::Bios);
        error.set_recoverable(false);
        logger.log_error(error);
        assert!(logger.has_critical_error());
    }

    #[test]
    fn test_recoverable_vs_non_recoverable() {
        let mut logger = BootFailureLogger::new();
        let mut error1 = BootError::new(0, ErrorCategory::Hardware, ErrorComponent::Thermal);
        error1.set_recoverable(true);
        let mut error2 = BootError::new(1, ErrorCategory::Hardware, ErrorComponent::Power);
        error2.set_recoverable(false);
        logger.log_error(error1);
        logger.log_error(error2);
        
        assert_eq!(logger.get_recoverable_error_count(), 1);
    }

    #[test]
    fn test_error_none_state() {
        let logger = BootFailureLogger::new();
        assert!(logger.get_error(0).is_none());
    }

    #[test]
    fn test_all_error_categories() {
        let mut logger = BootFailureLogger::new();
        let categories = vec![
            ErrorCategory::Hardware,
            ErrorCategory::Firmware,
            ErrorCategory::Memory,
            ErrorCategory::Configuration,
        ];
        
        for cat in categories {
            let error = BootError::new(0, cat, ErrorComponent::Unknown);
            logger.log_error(error);
        }
        
        assert_eq!(logger.get_error_count(), 4);
    }
}
