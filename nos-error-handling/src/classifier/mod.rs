//! Error classifier
//!
//! This module provides error classification and analysis functionality.

use crate::Result;

use nos_api::collections::BTreeMap;

use spin::Mutex;

/// Error classifier
pub struct ErrorClassifier {
    /// Classification rules
    rules: BTreeMap<u32, ClassificationRule>,
    /// Classification statistics
    stats: Mutex<ClassificationStats>,
}

impl Default for ErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorClassifier {
    /// Create a new error classifier
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
            stats: Mutex::new(ClassificationStats::default()),
        }
    }

    /// Add a classification rule
    pub fn add_rule(&mut self, rule: ClassificationRule) {
        self.rules.insert(rule.error_code, rule);
    }

    /// Classify an error
    pub fn classify_error(&self, error_record: &mut crate::types::ErrorRecord) -> Result<()> {
        // Get the rule for this error code
        let rule = self.rules.get(&error_record.code);
        
        if let Some(rule) = rule {
            // Apply the rule
            error_record.category = rule.category;
            error_record.severity = rule.severity;
            error_record.error_type = rule.error_type;
            
            // Update statistics
            let mut stats = self.stats.lock();
            stats.total_classified += 1;
            *stats.classifications_by_category.entry(rule.category).or_insert(0) += 1;
            *stats.classifications_by_severity.entry(rule.severity).or_insert(0) += 1;
        } else {
            // Default classification
            error_record.category = crate::types::ErrorCategory::System;
            error_record.severity = crate::types::ErrorSeverity::Error;
            error_record.error_type = crate::types::ErrorType::RuntimeError;
            
            // Update statistics
            let mut stats = self.stats.lock();
            stats.total_unclassified += 1;
        }
        
        Ok(())
    }

    /// Get classification statistics
    pub fn get_stats(&self) -> ClassificationStats {
        self.stats.lock().clone()
    }

    /// Initialize the classifier
    pub fn init(&mut self) -> Result<()> {
        // Add default classification rules
        self.add_default_rules();
        Ok(())
    }

    /// Shutdown the classifier
    pub fn shutdown(&mut self) -> Result<()> {
        // TODO: Shutdown classifier
        Ok(())
    }

    /// Add default classification rules
    fn add_default_rules(&mut self) {
        // Memory errors
        self.add_rule(ClassificationRule {
            error_code: 12, // ENOMEM
            category: crate::types::ErrorCategory::Memory,
            severity: crate::types::ErrorSeverity::High,
            error_type: crate::types::ErrorType::MemoryError,
        });
        
        // File system errors
        self.add_rule(ClassificationRule {
            error_code: 2, // ENOENT
            category: crate::types::ErrorCategory::FileSystem,
            severity: crate::types::ErrorSeverity::Medium,
            error_type: crate::types::ErrorType::IOError,
        });
        
        // Network errors
        self.add_rule(ClassificationRule {
            error_code: 101, // ENETUNREACH
            category: crate::types::ErrorCategory::Network,
            severity: crate::types::ErrorSeverity::Medium,
            error_type: crate::types::ErrorType::NetworkError,
        });
        
        // Permission errors
        self.add_rule(ClassificationRule {
            error_code: 13, // EACCES
            category: crate::types::ErrorCategory::Security,
            severity: crate::types::ErrorSeverity::High,
            error_type: crate::types::ErrorType::PermissionError,
        });
    }
}

/// Classification rule
#[derive(Debug, Clone)]
pub struct ClassificationRule {
    /// Error code
    pub error_code: u32,
    /// Error category
    pub category: crate::types::ErrorCategory,
    /// Error severity
    pub severity: crate::types::ErrorSeverity,
    /// Error type
    pub error_type: crate::types::ErrorType,
}

/// Classification statistics
#[derive(Debug, Clone, Default)]
pub struct ClassificationStats {
    /// Total classified errors
    pub total_classified: u64,
    /// Total unclassified errors
    pub total_unclassified: u64,
    /// Classifications by category
    pub classifications_by_category: BTreeMap<crate::types::ErrorCategory, u64>,
    /// Classifications by severity
    pub classifications_by_severity: BTreeMap<crate::types::ErrorSeverity, u64>,
}

/// Global error classifier
static GLOBAL_CLASSIFIER: spin::Once<Mutex<ErrorClassifier>> = spin::Once::new();

/// Initialize the global error classifier
pub fn init_classifier() -> Result<()> {
    GLOBAL_CLASSIFIER.call_once(|| {
        Mutex::new(ErrorClassifier::new())
    });
    
    // Initialize the classifier
    GLOBAL_CLASSIFIER.get().unwrap().lock().init()
}

/// Get the global error classifier
pub fn get_classifier() -> &'static Mutex<ErrorClassifier> {
    GLOBAL_CLASSIFIER.get().expect("Error classifier not initialized")
}



/// Shutdown the global error classifier
pub fn shutdown_classifier() -> Result<()> {
    // Note: spin::Once doesn't provide a way to reset, so we just return Ok(())
    // In a real implementation, you might want to provide a different approach
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classifier() {
        let mut classifier = ErrorClassifier::new();
        
        // Add a test rule
        let rule = ClassificationRule {
            error_code: 100,
            category: crate::types::ErrorCategory::System,
            severity: crate::types::ErrorSeverity::Error,
            error_type: crate::types::ErrorType::RuntimeError,
        };
        classifier.add_rule(rule);
        
        // Classify an error
        let mut error_record = crate::types::ErrorRecord {
            code: 100,
            ..Default::default()
        };
        
        assert!(classifier.classify_error(&mut error_record).is_ok());
        assert_eq!(error_record.category, crate::types::ErrorCategory::System);
        assert_eq!(error_record.severity, crate::types::ErrorSeverity::Error);
        assert_eq!(error_record.error_type, crate::types::ErrorType::RuntimeError);
    }

    #[test]
    fn test_classification_stats() {
        let stats = ClassificationStats::default();
        assert_eq!(stats.total_classified, 0);
        assert_eq!(stats.total_unclassified, 0);
        assert!(stats.classifications_by_category.is_empty());
        assert!(stats.classifications_by_severity.is_empty());
    }
}