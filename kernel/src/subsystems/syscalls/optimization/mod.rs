//! 系统调用优化模块
//!
//! 本模块提供系统调用性能优化的统一框架，包括：
//! - 核心数据结构和统计信息收集 (core)
//! - 通用优化工具和性能监控 (common)
//! - 统一优化框架和策略管理 (framework)
//! - 优化功能测试 (tests)
//!
//! ## 模块架构
//!
//! ### core
//! 提供优化基础设施的核心组件：
//! - `UnifiedSyscallStats`: 统一的统计信息收集
//! - `SyscallStatsSnapshot`: 统计快照
//! - `CacheEntry` 和 `UnifiedCache`: 通用缓存实现
//! - `OptimizationConfig`: 优化配置
//!
//! ### common
//! 提供通用优化组件：
//! - `FileDescriptorValidator`: 文件描述符验证
//! - `PerformanceMonitor`: 性能监控器
//! - 全局性能监控工具
//!
//! ### framework
//! 提供统一优化框架：
//! - `OptimizationStrategy`: 优化策略trait
//! - `UnifiedOptimizationManager`: 优化管理器
//! - 预定义优化策略（快速路径、缓存、批处理）
//!
//! ### tests
//! 提供优化功能测试：
//! - 功能一致性测试
//! - 性能对比测试
//! - 边界条件测试
//! - 压力测试

pub mod common;
pub mod core;
pub mod framework;
pub mod tests;

// 重新导出核心类型
pub use core::{
    CacheConfig, CacheEntry, EvictionPolicy, OptimizationConfig, SyscallStatsSnapshot,
    UnifiedCache, UnifiedSyscallStats, get_current_timestamp_ns,
};

pub use common::{
    FileDescriptorValidator, PerformanceMonitor, get_global_performance_monitor,
    init_global_performance_monitor, record_syscall_performance,
};
pub use framework::{
    BatchingOptimization, CachingOptimization, FastPathOptimization, OptimizationContext,
    OptimizationError, OptimizationRecord, OptimizationResult, OptimizationStrategy,
    OptimizationType, UnifiedOptimizationManager, check_and_apply_optimization,
    evaluate_optimization_effectiveness, get_global_optimization_manager,
    init_global_optimization_manager,
};
pub use tests::{
    run_all_syscall_optimization_tests, test_batch_syscall_optimization,
    test_non_fast_path_syscalls, test_syscall_boundary_conditions,
    test_syscall_function_consistency, test_syscall_performance_comparison, test_syscall_stress,
};
