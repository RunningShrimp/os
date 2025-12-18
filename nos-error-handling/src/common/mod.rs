//! Common utilities
//!
//! This module provides common utilities for error handling.

/// Get current timestamp
pub fn get_timestamp() -> u64 {
    // TODO: Implement actual timestamp logic
    0
}

/// Validate error record
pub fn validate_error_record(error_record: &crate::types::ErrorRecord) -> bool {
    // Validate error record fields
    !error_record.message.is_empty() &&
    error_record.code > 0 && // Error code should be positive
    error_record.severity <= crate::types::ErrorSeverity::Fatal && // Severity should be valid
    error_record.category <= crate::types::ErrorCategory::Interface && // Category should be valid
    error_record.timestamp > 0 && // Timestamp should be positive
    error_record.occurrence_count > 0 // Occurrence count should be at least 1
}

/// Format error message
#[cfg(feature = "alloc")]
pub fn format_error_message(error_record: &crate::types::ErrorRecord) -> alloc::string::String {
    alloc::format!(
        "Error #{}{} ({:?}:{:?}) - {}",
        error_record.id,
        error_record.code,
        error_record.category,
        error_record.severity,
        error_record.message
    )
}

/// Format error message (non-alloc version)
#[cfg(not(feature = "alloc"))]
pub fn format_error_message(error_record: &crate::types::ErrorRecord) -> &'static str {
    // In non-alloc mode, we return a static string but use error_record for basic validation
    match error_record.severity {
        crate::types::ErrorSeverity::Info => "Info error",
        crate::types::ErrorSeverity::Warning => "Warning error",
        crate::types::ErrorSeverity::Error => "Error",
        crate::types::ErrorSeverity::Critical => "Critical error",
        crate::types::ErrorSeverity::Fatal => "Fatal error",
        _ => "Unknown error",
    }
}

/// Calculate error hash
pub fn calculate_error_hash(error_record: &crate::types::ErrorRecord) -> u64 {
    // Simple hash calculation using multiple fields
    let mut hash = error_record.id;
    hash = hash.wrapping_mul(31).wrapping_add(error_record.code as u64);
    hash = hash.wrapping_mul(31).wrapping_add(error_record.error_type as u64);
    hash = hash.wrapping_mul(31).wrapping_add(error_record.category as u64);
    hash = hash.wrapping_mul(31).wrapping_add(error_record.severity as u64);
    hash = hash.wrapping_mul(31).wrapping_add(error_record.timestamp);
    hash = hash.wrapping_mul(31).wrapping_add(error_record.occurrence_count as u64);
    hash
}

/// Compare error records
pub fn compare_error_records(a: &crate::types::ErrorRecord, b: &crate::types::ErrorRecord) -> bool {
    a.id == b.id &&
    a.code == b.code &&
    a.error_type == b.error_type &&
    a.category == b.category &&
    a.severity == b.severity &&
    a.message == b.message
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "alloc")]
    use alloc::string::ToString;

    #[test]
    fn test_format_error_message() {
        let error_record = crate::types::ErrorRecord {
            id: 1,
            code: 100,
            error_type: crate::types::ErrorType::RuntimeError,
            category: crate::types::ErrorCategory::System,
            severity: crate::types::ErrorSeverity::Error,
            message: "Test error".to_string(),
            ..Default::default()
        };
        
        let message = format_error_message(&error_record);
        assert!(message.contains("Error #1"));
        assert!(message.contains("100"));
        assert!(message.contains("Test error"));
    }

    #[test]
    fn test_validate_error_record() {
        let valid_error = crate::types::ErrorRecord {
            message: "Valid error".to_string(),
            ..Default::default()
        };
        assert!(validate_error_record(&valid_error));
        
        let invalid_error = crate::types::ErrorRecord {
            message: "".to_string(),
            ..Default::default()
        };
        assert!(!validate_error_record(&invalid_error));
    }

    #[test]
    fn test_calculate_error_hash() {
        let error_record = crate::types::ErrorRecord {
            id: 123,
            ..Default::default()
        };
        
        let hash = calculate_error_hash(&error_record);
        assert_eq!(hash, 123);
    }

    #[test]
    fn test_compare_error_records() {
        let error1 = crate::types::ErrorRecord {
            id: 1,
            code: 100,
            error_type: crate::types::ErrorType::RuntimeError,
            category: crate::types::ErrorCategory::System,
            severity: crate::types::ErrorSeverity::Error,
            message: "Test error".to_string(),
            ..Default::default()
        };
        
        let error2 = crate::types::ErrorRecord {
            id: 1,
            code: 100,
            error_type: crate::types::ErrorType::RuntimeError,
            category: crate::types::ErrorCategory::System,
            severity: crate::types::ErrorSeverity::Error,
            message: "Test error".to_string(),
            ..Default::default()
        };
        
        let error3 = crate::types::ErrorRecord {
            id: 2,
            code: 100,
            error_type: crate::types::ErrorType::RuntimeError,
            category: crate::types::ErrorCategory::System,
            severity: crate::types::ErrorSeverity::Error,
            message: "Test error".to_string(),
            ..Default::default()
        };
        
        assert!(compare_error_records(&error1, &error2));
        assert!(!compare_error_records(&error1, &error3));
    }
}