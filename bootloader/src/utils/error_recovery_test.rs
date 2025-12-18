//! 错误恢复机制测试
//!
//! 测试多级错误恢复策略的正确性和有效性。

use crate::utils::error::{BootError, Result as BootResult};
use crate::utils::error_recovery::{ErrorRecoveryManager, ErrorSeverity, OutputMode, RecoveryStatus};

/// 测试错误严重程度评估
#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用的错误恢复管理器
    fn create_test_recovery_manager() -> ErrorRecoveryManager {
        ErrorRecoveryManager::new()
    }

    #[test]
    fn test_low_severity_error_assessment() {
        let manager = create_test_recovery_manager();
        let error = BootError::Timeout;
        let severity = manager.assess_error_severity(&error);
        assert_eq!(severity, ErrorSeverity::Low);
    }

    #[test]
    fn test_medium_severity_error_assessment() {
        let manager = create_test_recovery_manager();
        let error = BootError::MemoryAllocationFailed;
        let severity = manager.assess_error_severity(&error);
        assert_eq!(severity, ErrorSeverity::Medium);
    }

    #[test]
    fn test_high_severity_error_assessment() {
        let manager = create_test_recovery_manager();
        let error = BootError::InitializationFailed("Test failure");
        let severity = manager.assess_error_severity(&error);
        assert_eq!(severity, ErrorSeverity::High);
    }

    #[test]
    fn test_critical_severity_error_assessment() {
        let manager = create_test_recovery_manager();
        let error = BootError::OutOfMemory;
        let severity = manager.assess_error_severity(&error);
        assert_eq!(severity, ErrorSeverity::Critical);
    }

    #[test]
    fn test_output_mode_fallback() {
        let mut manager = create_test_recovery_manager();
        
        // 初始模式应该是高分辨率图形模式
        assert_eq!(manager.current_mode(), OutputMode::HighResolutionGraphics);
        
        // 模拟中等严重性错误
        let error = BootError::MemoryAllocationFailed;
        let result = manager.recover_from_error(&error);
        
        // 应该成功降级到低分辨率图形模式
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), OutputMode::LowResolutionGraphics);
        assert_eq!(manager.current_mode(), OutputMode::LowResolutionGraphics);
    }

    #[test]
    fn test_multiple_fallbacks() {
        let mut manager = create_test_recovery_manager();
        
        // 第一次降级
        let error1 = BootError::MemoryAllocationFailed;
        let result1 = manager.recover_from_error(&error1);
        assert!(result1.is_ok());
        assert_eq!(manager.current_mode(), OutputMode::LowResolutionGraphics);
        
        // 第二次降级
        let error2 = BootError::MemoryAllocationFailed;
        let result2 = manager.recover_from_error(&error2);
        assert!(result2.is_ok());
        assert_eq!(manager.current_mode(), OutputMode::TextMode);
        
        // 第三次降级
        let error3 = BootError::MemoryAllocationFailed;
        let result3 = manager.recover_from_error(&error3);
        assert!(result3.is_ok());
        assert_eq!(manager.current_mode(), OutputMode::SerialConsole);
        
        // 第四次降级
        let error4 = BootError::MemoryAllocationFailed;
        let result4 = manager.recover_from_error(&error4);
        assert!(result4.is_ok());
        assert_eq!(manager.current_mode(), OutputMode::Silent);
    }

    #[test]
    fn test_recovery_status_tracking() {
        let mut manager = create_test_recovery_manager();
        
        // 初始状态应该是无恢复
        assert_eq!(manager.recovery_status(), RecoveryStatus::NoRecovery);
        
        // 触发恢复
        let error = BootError::MemoryAllocationFailed;
        let _ = manager.recover_from_error(&error);
        
        // 状态应该变为恢复成功
        assert_eq!(manager.recovery_status(), RecoveryStatus::RecoverySuccessful);
    }

    #[test]
    fn test_max_retry_limit() {
        let mut manager = create_test_recovery_manager();
        
        // 尝试超过最大重试次数
        for _ in 0..=manager.max_retries {
            let error = BootError::MemoryAllocationFailed;
            let result = manager.recover_from_error(&error);
            if result.is_err() {
                assert_eq!(result.unwrap_err(), BootError::RecoveryModeFailed);
                assert_eq!(manager.recovery_status(), RecoveryStatus::RecoveryFailed);
                return;
            }
        }
        
        panic!("Expected recovery to fail after max retries");
    }

    #[test]
    fn test_recovery_reset() {
        let mut manager = create_test_recovery_manager();
        
        // 触发恢复
        let error = BootError::MemoryAllocationFailed;
        let _ = manager.recover_from_error(&error);
        assert_ne!(manager.current_mode(), OutputMode::HighResolutionGraphics);
        
        // 重置恢复状态
        manager.reset();
        assert_eq!(manager.current_mode(), OutputMode::HighResolutionGraphics);
        assert_eq!(manager.recovery_status(), RecoveryStatus::NoRecovery);
    }

    #[test]
    fn test_high_severity_error_recovery() {
        let mut manager = create_test_recovery_manager();
        
        // 高严重性错误应该直接跳到文本模式
        let error = BootError::InitializationFailed("Critical failure");
        let result = manager.recover_from_error(&error);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), OutputMode::TextMode);
        assert_eq!(manager.recovery_status(), RecoveryStatus::PartialRecovery);
    }

    #[test]
    fn test_critical_error_recovery() {
        let mut manager = create_test_recovery_manager();
        
        // 致命错误应该直接跳到串行控制台
        let error = BootError::OutOfMemory;
        let result = manager.recover_from_error(&error);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), OutputMode::SerialConsole);
        assert_eq!(manager.recovery_status(), RecoveryStatus::PartialRecovery);
    }

    #[test]
    fn test_low_severity_error_no_fallback() {
        let mut manager = create_test_recovery_manager();
        
        // 低严重性错误不应该触发降级
        let error = BootError::Timeout;
        let result = manager.recover_from_error(&error);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), OutputMode::HighResolutionGraphics);
        assert_eq!(manager.current_mode(), OutputMode::HighResolutionGraphics);
    }
}

/// 集成测试 - 测试与主引导流程的集成
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_vga_initialization_recovery_flow() {
        // 这个测试模拟VGA初始化失败时的恢复流程
        let mut manager = create_test_recovery_manager();
        
        // 模拟VGA初始化失败
        let vga_error = BootError::InitializationFailed("VGA initialization failed");
        
        // 尝试恢复
        let recovery_result = manager.recover_from_error(&vga_error);
        
        // 验证恢复结果
        assert!(recovery_result.is_ok());
        let recovered_mode = recovery_result.unwrap();
        
        // 应该降级到文本模式或串行控制台
        assert!(
            recovered_mode == OutputMode::TextMode || 
            recovered_mode == OutputMode::SerialConsole
        );
        
        // 验证恢复状态
        assert_eq!(manager.recovery_status(), RecoveryStatus::PartialRecovery);
    }

    #[test]
    fn test_multiple_component_failures() {
        // 测试多个组件失败时的恢复流程
        let mut manager = create_test_recovery_manager();
        
        // 模拟多个组件初始化失败
        let errors = vec![
            BootError::InitializationFailed("VGA initialization failed"),
            BootError::MemoryAllocationFailed,
            BootError::ProtocolInitializationFailed("BIOS services failed"),
        ];
        
        let mut successful_recoveries = 0;
        for error in errors {
            if manager.recover_from_error(&error).is_ok() {
                successful_recoveries += 1;
            }
        }
        
        // 至少应该有一次成功的恢复
        assert!(successful_recoveries > 0);
        
        // 最终状态应该是某种恢复状态
        assert_ne!(manager.recovery_status(), RecoveryStatus::NoRecovery);
    }

    #[test]
    fn test_recovery_exhaustion() {
        // 测试恢复资源耗尽的情况
        let mut manager = create_test_recovery_manager();
        
        // 尝试超过最大重试次数的恢复
        let mut failure_count = 0;
        for _ in 0..=manager.max_retries + 1 {
            let error = BootError::InitializationFailed("Persistent failure");
            if manager.recover_from_error(&error).is_err() {
                failure_count += 1;
            }
        }
        
        // 应该至少有一次失败
        assert!(failure_count > 0);
        
        // 最终状态应该是恢复失败
        assert_eq!(manager.recovery_status(), RecoveryStatus::RecoveryFailed);
    }
}

/// 性能测试 - 测试错误恢复的性能影响
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_recovery_performance() {
        let mut manager = create_test_recovery_manager();
        let error = BootError::MemoryAllocationFailed;
        
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = manager.recover_from_error(&error);
            manager.reset();
        }
        let duration = start.elapsed();
        
        // 恢复操作应该在合理时间内完成
        assert!(duration.as_millis() < 100); // 100ms for 1000 operations
    }

    #[test]
    fn test_severity_assessment_performance() {
        let manager = create_test_recovery_manager();
        let error = BootError::InitializationFailed("Test error");
        
        let start = Instant::now();
        for _ in 0..10000 {
            let _ = manager.assess_error_severity(&error);
        }
        let duration = start.elapsed();
        
        // 严重程度评估应该非常快
        assert!(duration.as_millis() < 10); // 10ms for 10000 operations
    }
}