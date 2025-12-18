//! Core error handling functionality
//!
//! This module provides the core error handling engine and infrastructure.



use crate::Error;
use crate::Result;
use crate::types::{ErrorRecord, ErrorCategory, ErrorSeverity, RecoveryStrategy};

#[cfg(feature = "alloc")]
use alloc::sync::Arc;
#[cfg(feature = "alloc")]
use spin::Mutex;
#[cfg(feature = "alloc")]
use alloc::string::ToString;

/// Error handling engine
#[cfg(feature = "alloc")]
pub struct ErrorHandlingEngine {
    /// Engine configuration
    config: ErrorHandlingConfig,
    /// Error records
    error_records: alloc::vec::Vec<ErrorRecord>,
    /// Error registry
    error_registry: Arc<Mutex<crate::registry::ErrorRegistry>>,
    /// Error classifier
    error_classifier: Arc<Mutex<crate::classifier::ErrorClassifier>>,
    /// Recovery manager
    recovery_manager: Arc<Mutex<crate::recovery::RecoveryManager>>,
    /// Diagnostic analyzer
    diagnostic_analyzer: Arc<Mutex<crate::diagnostics::DiagnosticAnalyzer>>,
    /// Error reporter
    error_reporter: Arc<Mutex<crate::reporting::ErrorReporter>>,
    /// System health monitor
    health_monitor: Arc<Mutex<crate::health::HealthMonitor>>,
    /// Error statistics
    stats: Arc<Mutex<crate::types::ErrorHandlingStats>>,
    /// Error counter
    error_counter: core::sync::atomic::AtomicU64,
    /// Running flag
    running: core::sync::atomic::AtomicBool,
}

#[cfg(feature = "alloc")]
impl ErrorHandlingEngine {
    /// Create a new error handling engine
    pub fn new(config: ErrorHandlingConfig) -> Self {
        Self {
            config,
            error_records: alloc::vec::Vec::new(),
            error_registry: Arc::new(Mutex::new(crate::registry::ErrorRegistry::new())),
            error_classifier: Arc::new(Mutex::new(crate::classifier::ErrorClassifier::new())),
            recovery_manager: Arc::new(Mutex::new(crate::recovery::RecoveryManager::new())),
            diagnostic_analyzer: Arc::new(Mutex::new(crate::diagnostics::DiagnosticAnalyzer::new())),
            error_reporter: Arc::new(Mutex::new(crate::reporting::ErrorReporter::new())),
            health_monitor: Arc::new(Mutex::new(crate::health::HealthMonitor::new())),
            stats: Arc::new(Mutex::new(crate::types::ErrorHandlingStats::default())),
            error_counter: core::sync::atomic::AtomicU64::new(1),
            running: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Initialize the error handling engine
    pub fn init(&mut self) -> Result<()> {
        self.running.store(true, core::sync::atomic::Ordering::SeqCst);

        // Initialize all components
        self.error_registry.lock().init()?;
        self.error_classifier.lock().init()?;
        self.recovery_manager.lock().init()?;
        self.diagnostic_analyzer.lock().init()?;
        self.error_reporter.lock().init()?;
        self.health_monitor.lock().init()?;

        Ok(())
    }

    /// Record an error
    pub fn record_error(&mut self, mut error_record: ErrorRecord) -> Result<u64> {
        if !self.running.load(core::sync::atomic::Ordering::SeqCst) {
            return Err(Error::InvalidState("Error handling engine is not running".to_string()));
        }

        let error_id = self.error_counter.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        error_record.id = error_id;

        // Classify the error
        {
            let classifier = self.error_classifier.lock();
            classifier.classify_error(&mut error_record)?;
        }

        // Analyze the error
        {
            let analyzer = self.diagnostic_analyzer.lock();
            analyzer.analyze_error(&error_record)?;
        }

        // Add to records
        self.error_records.push(error_record.clone());

        // Limit records
        if self.error_records.len() > self.config.max_error_records {
            self.error_records.remove(0);
        }

        // Execute recovery actions
        if self.config.enable_recovery {
            self.execute_recovery_actions(&error_record)?;
        }

        // Report the error
        {
            let reporter = self.error_reporter.lock();
            reporter.report_error(&error_record)?;
        }

        // Update statistics
        self.update_statistics(&error_record);

        Ok(error_id)
    }

    /// Get error records
    pub fn get_error_records(&self, limit: Option<usize>, category: Option<ErrorCategory>, severity: Option<ErrorSeverity>) -> alloc::vec::Vec<ErrorRecord> {
        let mut records = self.error_records.clone();

        // Filter by category
        if let Some(cat) = category {
            records.retain(|r| r.category == cat);
        }

        // Filter by severity
        if let Some(sev) = severity {
            records.retain(|r| r.severity == sev);
        }

        // Sort by timestamp (newest first)
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Limit results
        if let Some(limit) = limit {
            records.truncate(limit);
        }

        records
    }

    /// Get error handling statistics
    pub fn get_statistics(&self) -> crate::types::ErrorHandlingStats {
        self.stats.lock().clone()
    }

    /// Get system health status
    pub fn get_health_status(&self) -> crate::health::HealthStatus {
        let monitor = self.health_monitor.lock();
        monitor.get_current_status()
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ErrorHandlingConfig) -> Result<()> {
        self.config = config;
        Ok(())
    }

    /// Execute recovery actions
    fn execute_recovery_actions(&self, error_record: &ErrorRecord) -> Result<()> {
        let recovery_manager = self.recovery_manager.lock();
        
        for action in &error_record.recovery_actions {
            recovery_manager.execute_recovery_action(action)?;
        }

        // Apply automatic recovery strategies
        for strategy in &self.config.auto_recovery_strategies {
            recovery_manager.apply_recovery_strategy(strategy, error_record)?;
        }

        Ok(())
    }

    /// Update statistics
    fn update_statistics(&self, error_record: &ErrorRecord) {
        let mut stats = self.stats.lock();

        stats.total_errors += 1;
        *stats.errors_by_category.entry(error_record.category).or_insert(0) += 1;
        *stats.errors_by_severity.entry(error_record.severity).or_insert(0) += 1;

        if error_record.resolved {
            stats.recovered_errors += 1;
        }
    }

    /// Shutdown the error handling engine
    pub fn shutdown(&mut self) -> Result<()> {
        self.running.store(false, core::sync::atomic::Ordering::SeqCst);

        // Shutdown all components
        self.error_registry.lock().shutdown()?;
        self.error_classifier.lock().shutdown()?;
        self.recovery_manager.lock().shutdown()?;
        self.diagnostic_analyzer.lock().shutdown()?;
        self.error_reporter.lock().shutdown()?;
        self.health_monitor.lock().shutdown()?;

        Ok(())
    }
}

/// Error handling configuration
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub struct ErrorHandlingConfig {
    /// Enable error recovery
    pub enable_recovery: bool,
    /// Maximum number of error records
    pub max_error_records: usize,
    /// Automatic recovery strategies
    pub auto_recovery_strategies: alloc::vec::Vec<RecoveryStrategy>,
    /// Error retention period (seconds)
    pub retention_period_seconds: u64,
}

#[cfg(feature = "alloc")]
impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            enable_recovery: true,
            max_error_records: 10000,
            auto_recovery_strategies: alloc::vec::Vec::new(),
            retention_period_seconds: 86400 * 7, // 7 days
        }
    }
}

/// Global error handling engine
#[cfg(feature = "alloc")]
static GLOBAL_ENGINE: spin::Once<Mutex<ErrorHandlingEngine>> = spin::Once::new();

/// Initialize the global error handling engine
#[cfg(feature = "alloc")]
pub fn init_engine() -> Result<()> {
    GLOBAL_ENGINE.call_once(|| {
        Mutex::new(ErrorHandlingEngine::new(ErrorHandlingConfig::default()))
    });
    Ok(())
}

/// Get the global error handling engine
#[cfg(feature = "alloc")]
pub fn get_engine() -> &'static Mutex<ErrorHandlingEngine> {
    GLOBAL_ENGINE.get().expect("Error handling engine not initialized")
}

/// Shutdown the global error handling engine
#[cfg(feature = "alloc")]
pub fn shutdown_engine() -> Result<()> {
    // With spin::Once, we cannot reset the initialization
    // In a real implementation, you might want to handle this differently
    Ok(())
}

/// Get error handling statistics
#[cfg(feature = "alloc")]
pub fn get_stats() -> crate::types::ErrorHandlingStats {
    get_engine().lock().get_statistics()
}

/// Record an error
#[cfg(feature = "alloc")]
pub fn record_error(error_record: ErrorRecord) -> Result<u64> {
    get_engine().lock().record_error(error_record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_handling_engine() {
        let config = ErrorHandlingConfig::default();
        let mut engine = ErrorHandlingEngine::new(config);
        
        assert!(engine.init().is_ok());
        
        let stats = engine.get_statistics();
        assert_eq!(stats.total_errors, 0);
        
        assert!(engine.shutdown().is_ok());
    }

    #[test]
    fn test_error_handling_config() {
        let config = ErrorHandlingConfig::default();
        assert!(config.enable_recovery);
        assert_eq!(config.max_error_records, 10000);
        assert_eq!(config.retention_period_seconds, 86400 * 7);
    }
}