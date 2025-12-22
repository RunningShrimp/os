//! Error Recovery Strategies
//!
//! This module provides configurable, dynamic error recovery strategies
//! that integrate with error statistics and monitoring.

use super::{UnifiedError, ErrorContext, ErrorSeverity, ErrorAction, ErrorStats};
use crate::subsystems::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

/// Recovery strategy configuration
#[derive(Debug, Clone)]
pub struct RecoveryStrategyConfig {
    /// Maximum number of recovery attempts before giving up
    pub max_attempts: usize,
    /// Timeout for recovery operations (in milliseconds)
    pub timeout_ms: u64,
    /// Whether to enable automatic recovery
    pub auto_recover: bool,
    /// Whether to log recovery attempts
    pub log_recovery: bool,
    /// Recovery action for each error type
    pub error_type_actions: BTreeMap<String, RecoveryAction>,
}

impl Default for RecoveryStrategyConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            timeout_ms: 1000,
            auto_recover: true,
            log_recovery: true,
            error_type_actions: BTreeMap::new(),
        }
    }
}

/// Recovery action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Retry the operation
    Retry,
    /// Fallback to alternative implementation
    Fallback,
    /// Degrade functionality gracefully
    Degrade,
    /// Reset the component
    Reset,
    /// Restart the subsystem
    Restart,
    /// No recovery (propagate error)
    NoRecovery,
}

/// Recovery attempt result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResult {
    /// Recovery succeeded
    Success,
    /// Recovery failed
    Failed,
    /// Recovery not applicable
    NotApplicable,
    /// Recovery timeout
    Timeout,
}

/// Recovery statistics
#[derive(Debug, Clone)]
pub struct RecoveryStats {
    /// Total recovery attempts
    pub total_attempts: usize,
    /// Successful recoveries
    pub successful_recoveries: usize,
    /// Failed recoveries
    pub failed_recoveries: usize,
    /// Recovery attempts by error type
    pub attempts_by_error_type: BTreeMap<String, usize>,
    /// Recovery successes by error type
    pub successes_by_error_type: BTreeMap<String, usize>,
}

impl Default for RecoveryStats {
    fn default() -> Self {
        Self {
            total_attempts: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
            attempts_by_error_type: BTreeMap::new(),
            successes_by_error_type: BTreeMap::new(),
        }
    }
}

/// Error recovery manager
pub struct RecoveryManager {
    /// Recovery strategy configuration
    config: Mutex<RecoveryStrategyConfig>,
    /// Recovery statistics
    stats: Mutex<RecoveryStats>,
    /// Error occurrence counts (for adaptive recovery)
    error_counts: Mutex<BTreeMap<String, usize>>,
    /// Recovery attempt counts per error
    recovery_attempts: Mutex<BTreeMap<String, usize>>,
    /// Whether recovery is enabled
    enabled: AtomicBool,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new() -> Self {
        Self {
            config: Mutex::new(RecoveryStrategyConfig::default()),
            stats: Mutex::new(RecoveryStats::default()),
            error_counts: Mutex::new(BTreeMap::new()),
            recovery_attempts: Mutex::new(BTreeMap::new()),
            enabled: AtomicBool::new(true),
        }
    }

    /// Update recovery strategy configuration
    pub fn update_config(&self, config: RecoveryStrategyConfig) {
        *self.config.lock() = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> RecoveryStrategyConfig {
        self.config.lock().clone()
    }

    /// Enable or disable recovery
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    /// Check if recovery is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Determine recovery action for an error
    pub fn determine_recovery_action(&self, error: &UnifiedError, context: &ErrorContext) -> Option<RecoveryAction> {
        if !self.is_enabled() {
            return None;
        }

        let config = self.config.lock();
        
        // Check if auto-recovery is disabled
        if !config.auto_recover {
            return None;
        }

        // Get error type string
        let error_type = format!("{:?}", error);
        
        // Check if we've exceeded max attempts for this error type
        let attempts = {
            let recovery_attempts = self.recovery_attempts.lock();
            recovery_attempts.get(&error_type).copied().unwrap_or(0)
        };
        
        if attempts >= config.max_attempts {
            if config.log_recovery {
                crate::log_warn!("Recovery: Max attempts ({}) exceeded for error type: {}", config.max_attempts, error_type);
            }
            return None;
        }

        // Determine recovery action based on error type and severity
        let action = match context.severity {
            ErrorSeverity::Info | ErrorSeverity::Warning => {
                // Low severity errors: usually no recovery needed
                None
            }
            ErrorSeverity::Error => {
                // Medium severity: try recovery based on error type
                self.get_recovery_action_for_error(error, &config)
            }
            ErrorSeverity::Critical => {
                // Critical errors: aggressive recovery
                Some(RecoveryAction::Reset)
            }
            ErrorSeverity::Fatal => {
                // Fatal errors: no recovery possible
                None
            }
        };

        if let Some(action) = action {
            // Update recovery attempt count
            {
                let mut recovery_attempts = self.recovery_attempts.lock();
                *recovery_attempts.entry(error_type.clone()).or_insert(0) += 1;
            }

            // Update statistics
            {
                let mut stats = self.stats.lock();
                stats.total_attempts += 1;
                *stats.attempts_by_error_type.entry(error_type).or_insert(0) += 1;
            }

            if config.log_recovery {
                crate::log_info!("Recovery: Attempting {:?} for error: {}", action, error_type);
            }
        }

        action
    }

    /// Get recovery action for a specific error type
    fn get_recovery_action_for_error(
        &self,
        error: &UnifiedError,
        config: &RecoveryStrategyConfig,
    ) -> Option<RecoveryAction> {
        let error_type = format!("{:?}", error);
        
        // Check if there's a specific action configured for this error type
        if let Some(action) = config.error_type_actions.get(&error_type) {
            return Some(*action);
        }

        // Default recovery actions based on error category
        match error {
            UnifiedError::OutOfMemory | UnifiedError::MemoryError(_) => {
                Some(RecoveryAction::Degrade)
            }
            UnifiedError::FileSystemError(_) => {
                Some(RecoveryAction::Retry)
            }
            UnifiedError::NetworkError(_) => {
                Some(RecoveryAction::Retry)
            }
            UnifiedError::ProcessError(_) => {
                Some(RecoveryAction::Reset)
            }
            UnifiedError::DriverError(_) => {
                Some(RecoveryAction::Reset)
            }
            UnifiedError::SecurityError(_) => {
                // Security errors: no automatic recovery
                None
            }
            _ => {
                // Default: try retry
                Some(RecoveryAction::Retry)
            }
        }
    }

    /// Execute a recovery action
    pub fn execute_recovery(
        &self,
        action: RecoveryAction,
        error: &UnifiedError,
        context: &ErrorContext,
    ) -> RecoveryResult {
        if !self.is_enabled() {
            return RecoveryResult::NotApplicable;
        }

        let config = self.config.lock();
        let error_type = format!("{:?}", error);

        match action {
            RecoveryAction::Retry => {
                // Retry: wait a bit and return success (caller should retry)
                if config.log_recovery {
                    crate::log_info!("Recovery: Retrying operation for error: {}", error_type);
                }
                RecoveryResult::Success
            }
            RecoveryAction::Fallback => {
                // Fallback: switch to alternative implementation
                if config.log_recovery {
                    crate::log_info!("Recovery: Using fallback implementation for error: {}", error_type);
                }
                self.execute_fallback(error, context)
            }
            RecoveryAction::Degrade => {
                // Degrade: reduce functionality gracefully
                if config.log_recovery {
                    crate::log_info!("Recovery: Degrading functionality for error: {}", error_type);
                }
                self.execute_degradation(error, context)
            }
            RecoveryAction::Reset => {
                // Reset: reset the component
                if config.log_recovery {
                    crate::log_info!("Recovery: Resetting component for error: {}", error_type);
                }
                self.execute_reset(error, context)
            }
            RecoveryAction::Restart => {
                // Restart: restart the subsystem
                if config.log_recovery {
                    crate::log_warn!("Recovery: Restarting subsystem for error: {}", error_type);
                }
                self.execute_restart(error, context)
            }
            RecoveryAction::NoRecovery => {
                RecoveryResult::NotApplicable
            }
        }
    }

    /// Execute fallback recovery
    fn execute_fallback(&self, _error: &UnifiedError, _context: &ErrorContext) -> RecoveryResult {
        // TODO: Implement fallback mechanisms
        // For now, just return success
        RecoveryResult::Success
    }

    /// Execute degradation recovery
    fn execute_degradation(&self, error: &UnifiedError, _context: &ErrorContext) -> RecoveryResult {
        match error {
            UnifiedError::OutOfMemory | UnifiedError::MemoryError(_) => {
                // Trigger memory pressure handling
                // This would integrate with graceful degradation system
                // For now, just log and return success
                if self.config.lock().log_recovery {
                    crate::log_info!("Recovery: Triggering memory pressure degradation");
                }
                RecoveryResult::Success
            }
            _ => {
                RecoveryResult::Failed
            }
        }
    }

    /// Execute reset recovery
    fn execute_reset(&self, error: &UnifiedError, _context: &ErrorContext) -> RecoveryResult {
        match error {
            UnifiedError::DriverError(_) => {
                // Reset driver
                // TODO: Implement driver reset
                RecoveryResult::Success
            }
            UnifiedError::ProcessError(_) => {
                // Reset process state
                // TODO: Implement process reset
                RecoveryResult::Success
            }
            _ => {
                RecoveryResult::Failed
            }
        }
    }

    /// Execute restart recovery
    fn execute_restart(&self, _error: &UnifiedError, _context: &ErrorContext) -> RecoveryResult {
        // Restart subsystem
        // TODO: Implement subsystem restart
        // This is a dangerous operation and should be carefully implemented
        RecoveryResult::Failed // For now, don't allow automatic restart
    }

    /// Record recovery success
    pub fn record_recovery_success(&self, error: &UnifiedError) {
        let error_type = format!("{:?}", error);
        let mut stats = self.stats.lock();
        stats.successful_recoveries += 1;
        *stats.successes_by_error_type.entry(error_type).or_insert(0) += 1;
        
        // Reset recovery attempt count on success
        let mut recovery_attempts = self.recovery_attempts.lock();
        recovery_attempts.remove(&format!("{:?}", error));
    }

    /// Record recovery failure
    pub fn record_recovery_failure(&self, error: &UnifiedError) {
        let mut stats = self.stats.lock();
        stats.failed_recoveries += 1;
    }

    /// Get recovery statistics
    pub fn get_stats(&self) -> RecoveryStats {
        self.stats.lock().clone()
    }

    /// Get error occurrence counts (for adaptive recovery)
    pub fn get_error_counts(&self) -> BTreeMap<String, usize> {
        self.error_counts.lock().clone()
    }

    /// Record error occurrence (for adaptive recovery)
    pub fn record_error(&self, error: &UnifiedError) {
        let error_type = format!("{:?}", error);
        let mut error_counts = self.error_counts.lock();
        *error_counts.entry(error_type).or_insert(0) += 1;
    }

    /// Reset recovery statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = RecoveryStats::default();
        
        let mut recovery_attempts = self.recovery_attempts.lock();
        recovery_attempts.clear();
        
        let mut error_counts = self.error_counts.lock();
        error_counts.clear();
    }

    /// Configure adaptive recovery based on error statistics
    pub fn configure_adaptive_recovery(&self, error_stats: &ErrorStats) {
        let mut config = self.config.lock();
        
        // Adjust max attempts based on error rate
        if error_stats.critical_errors > 10 {
            // High error rate: reduce recovery attempts to avoid thrashing
            config.max_attempts = 1;
            config.auto_recover = false;
            crate::log_warn!("Recovery: High error rate detected, disabling auto-recovery");
        } else if error_stats.total_errors > 100 {
            // Moderate error rate: reduce max attempts
            config.max_attempts = 2;
        } else {
            // Normal error rate: use default
            config.max_attempts = 3;
        }
    }
}

/// Global recovery manager
static mut RECOVERY_MANAGER: Option<RecoveryManager> = None;
static RECOVERY_MANAGER_INIT: Mutex<bool> = Mutex::new(false);

/// Initialize recovery manager
pub fn init_recovery_manager() {
    let mut init = RECOVERY_MANAGER_INIT.lock();
    if !*init {
        unsafe {
            RECOVERY_MANAGER = Some(RecoveryManager::new());
        }
        *init = true;
    }
}

/// Get recovery manager
pub fn get_recovery_manager() -> Option<&'static RecoveryManager> {
    unsafe {
        RECOVERY_MANAGER.as_ref()
    }
}

/// Determine recovery action for an error
pub fn determine_recovery_action(error: &UnifiedError, context: &ErrorContext) -> Option<RecoveryAction> {
    get_recovery_manager()
        .and_then(|mgr| mgr.determine_recovery_action(error, context))
}

/// Execute recovery action
pub fn execute_recovery(action: RecoveryAction, error: &UnifiedError, context: &ErrorContext) -> RecoveryResult {
    get_recovery_manager()
        .map(|mgr| mgr.execute_recovery(action, error, context))
        .unwrap_or(RecoveryResult::NotApplicable)
}

/// Record recovery success
pub fn record_recovery_success(error: &UnifiedError) {
    if let Some(mgr) = get_recovery_manager() {
        mgr.record_recovery_success(error);
    }
}

/// Record recovery failure
pub fn record_recovery_failure(error: &UnifiedError) {
    if let Some(mgr) = get_recovery_manager() {
        mgr.record_recovery_failure(error);
    }
}

/// Get recovery statistics
pub fn get_recovery_stats() -> Option<RecoveryStats> {
    get_recovery_manager().map(|mgr| mgr.get_stats())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_manager_default_config() {
        let mgr = RecoveryManager::new();
        let config = mgr.get_config();
        assert!(config.auto_recover);
        assert_eq!(config.max_attempts, 3);
    }

    #[test]
    fn test_determine_recovery_action_for_memory_error() {
        let mgr = RecoveryManager::new();
        let error = UnifiedError::OutOfMemory;
        let ctx = ErrorContext::new(error.clone(), "test_location");
        let action = mgr.determine_recovery_action(&error, &ctx);
        assert_eq!(action, Some(RecoveryAction::Degrade));
    }

    #[test]
    fn test_execute_recovery_retry_and_stats() {
        let mgr = RecoveryManager::new();
        let error = UnifiedError::OutOfMemory;
        let ctx = ErrorContext::new(error.clone(), "test_location");

        let result = mgr.execute_recovery(RecoveryAction::Retry, &error, &ctx);
        assert_eq!(result, RecoveryResult::Success);

        // Record success and check stats
        mgr.record_recovery_success(&error);
        let stats = mgr.get_stats();
        assert_eq!(stats.successful_recoveries, 1);
    }
}

