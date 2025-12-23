//! Error handling tests

use nos_error_handling::core;
use nos_error_handling::types;
use nos_error_handling::kernel_integration;
use std::collections::BTreeMap;

#[test]
fn test_error_handling_init() {
    // Test error handling initialization
    assert!(init_error_handling().is_ok());
}

#[test]
fn test_error_stats() {
    // Test error handling statistics
    let stats = get_error_stats();
    assert_eq!(stats.total_errors, 0);
    assert_eq!(stats.recovered_errors, 0);
    assert_eq!(stats.recovery_success_rate, 0.0);
    assert_eq!(stats.avg_recovery_time, 0);
    assert_eq!(stats.health_score, 100.0);
    assert!(stats.errors_by_category.is_empty());
    assert!(stats.errors_by_severity.is_empty());
}

#[test]
fn test_error_record() {
    // Test error record creation
    let error_record = types::ErrorRecord {
        id: 1,
        code: 100,
        error_type: types::ErrorType::RuntimeError,
        category: types::ErrorCategory::System,
        severity: types::ErrorSeverity::Error,
        status: types::ErrorStatus::New,
        message: "Test error".to_string(),
        description: "Test error description".to_string(),
        source: types::ErrorSource {
            module: "test_module".to_string(),
            function: "test_function".to_string(),
            file: "test.rs".to_string(),
            line: 10,
            column: 5,
            process_id: 123,
            thread_id: 456,
            cpu_id: 0,
        },
        timestamp: 1234567890,
        context: types::ErrorContext {
            environment_variables: BTreeMap::new(),
            system_config: BTreeMap::new(),
            user_input: Some("test input".to_string()),
            related_data: vec![1, 2, 3, 4],
            operation_sequence: vec!["step1".to_string(), "step2".to_string()],
            preconditions: vec!["pre1".to_string()],
            postconditions: vec!["post1".to_string()],
        },
        recovery_actions: vec![],
        occurrence_count: 1,
        last_occurrence: 1234567890,
        resolved: false,
        resolution_time: None,
        resolution_method: None,
        metadata: BTreeMap::new(),
    };
    
    assert_eq!(error_record.id, 1);
    assert_eq!(error_record.code, 100);
    assert_eq!(error_record.error_type, types::ErrorType::RuntimeError);
    assert_eq!(error_record.category, types::ErrorCategory::System);
    assert_eq!(error_record.severity, types::ErrorSeverity::Error);
    assert_eq!(error_record.status, types::ErrorStatus::New);
    assert_eq!(error_record.message, "Test error");
    assert_eq!(error_record.description, "Test error description");
    assert_eq!(error_record.source.module, "test_module");
    assert_eq!(error_record.source.function, "test_function");
    assert_eq!(error_record.source.file, "test.rs");
    assert_eq!(error_record.source.line, 10);
    assert_eq!(error_record.source.column, 5);
    assert_eq!(error_record.source.process_id, 123);
    assert_eq!(error_record.source.thread_id, 456);
    assert_eq!(error_record.source.cpu_id, 0);
    assert_eq!(error_record.timestamp, 1234567890);
    assert_eq!(error_record.context.user_input, Some("test input".to_string()));
    assert_eq!(error_record.context.related_data, vec![1, 2, 3, 4]);
    assert_eq!(error_record.context.operation_sequence, vec!["step1".to_string(), "step2".to_string()]);
    assert_eq!(error_record.context.preconditions, vec!["pre1".to_string()]);
    assert_eq!(error_record.context.postconditions, vec!["post1".to_string()]);
    assert_eq!(error_record.recovery_actions, vec![]);
    assert_eq!(error_record.occurrence_count, 1);
    assert_eq!(error_record.last_occurrence, 1234567890);
    assert!(!error_record.resolved);
    assert_eq!(error_record.resolution_time, None);
    assert_eq!(error_record.resolution_method, None);
    assert!(error_record.metadata.is_empty());
}

#[test]
fn test_error_severity() {
    // Test error severity ordering
    assert!(types::ErrorSeverity::Info < types::ErrorSeverity::Low);
    assert!(types::ErrorSeverity::Low < types::ErrorSeverity::Warning);
    assert!(types::ErrorSeverity::Warning < types::ErrorSeverity::Medium);
    assert!(types::ErrorSeverity::Medium < types::ErrorSeverity::High);
    assert!(types::ErrorSeverity::High < types::ErrorSeverity::Error);
    assert!(types::ErrorSeverity::Error < types::ErrorSeverity::Critical);
    assert!(types::ErrorSeverity::Critical < types::ErrorSeverity::Fatal);
    
    assert_eq!(types::ErrorSeverity::default(), types::ErrorSeverity::Info);
}

#[test]
fn test_error_category() {
    // Test error category
    assert_eq!(types::ErrorCategory::System, types::ErrorCategory::System);
    assert_ne!(types::ErrorCategory::System, types::ErrorCategory::Memory);
    
    assert_eq!(types::ErrorCategory::default(), types::ErrorCategory::System);
}

#[test]
fn test_recovery_strategy() {
    // Test recovery strategy
    assert_eq!(types::RecoveryStrategy::default(), types::RecoveryStrategy::None);
    assert_eq!(types::RecoveryStrategy::None as u32, 0);
    assert_eq!(types::RecoveryStrategy::Retry as u32, 1);
    assert_eq!(types::RecoveryStrategy::Degrade as u32, 2);
    assert_eq!(types::RecoveryStrategy::Restart as u32, 3);
    assert_eq!(types::RecoveryStrategy::Release as u32, 4);
    assert_eq!(types::RecoveryStrategy::Failover as u32, 5);
    assert_eq!(types::RecoveryStrategy::Isolate as u32, 6);
    assert_eq!(types::RecoveryStrategy::Manual as u32, 7);
    assert_eq!(types::RecoveryStrategy::Ignore as u32, 8);
}

#[test]
fn test_error_handling_engine() {
    // Test error handling engine
    let config = kernel_integration::ErrorHandlingConfig::default();
    let mut engine = kernel_integration::ErrorHandlingEngine::new(config);
    
    assert!(engine.init().is_ok());
    
    let stats = engine.get_statistics();
    assert_eq!(stats.total_errors, 0);
    
    assert!(engine.shutdown().is_ok());
}

#[test]
fn test_error_handling_config() {
    // Test error handling configuration
    let config = kernel_integration::ErrorHandlingConfig::default();
    assert!(config.enable_recovery);
    assert_eq!(config.max_error_records, 10000);
    assert_eq!(config.retention_period_seconds, 86400 * 7);
    assert!(config.auto_recovery_strategies.is_empty());
}