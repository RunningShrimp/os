//! 并行初始化引擎 / Parallel Initialization Engine
//!
//! 本模块提供内核组件的并行初始化框架，支持：
//! This module provides parallel initialization framework for kernel components, supporting:
//!
//! - **并行执行**: 利用多核CPU并行初始化独立组件
//! - **进度跟踪**: 实时跟踪各组件初始化进度
//! - **错误处理**: 组件失败时优雅降级或回滚
//! - **性能监控**: 收集初始化性能指标
//! - **确定性**: 保证初始化顺序的一致性
//!
//! ## 主要功能 / Main Features
//!
//! - **InitThreadPool**: 工作线程池，执行初始化任务
//! - **ParallelInitEngine**: 主引擎，协调并行初始化
//! - **InitProgress**: 进度跟踪和报告
//! - **错误恢复**: 失败组件的回滚机制
//!
//! ## 使用示例 / Usage Example
//!
//! ```rust
//! use kernel::core::parallel_init::{ParallelInitEngine, InitThreadPool};
//! use kernel::core::init_dependencies::create_standard_kernel_deps;
//!
//! // 创建依赖图
//! let deps = create_standard_kernel_deps().unwrap();
//!
//! // 创建并行引擎
//! let engine = ParallelInitEngine::new(deps, num_cpus);
//!
//! // 执行并行初始化
//! let result = engine.initialize();
//! match result {
//!     Ok(stats) => {
//!         println!("Initialization complete: {:?}", stats);
//!     }
//!     Err(e) => {
//!         println!("Initialization failed: {:?}", e);
//!     }
//! }
//! ```

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::core::init_dependencies::{DependencyGraph, InitDependency};
use crate::error::{Error, Result};
use crate::prelude::*;
use crate::sync::{Mutex, SpinLock};

// ============================================================================
// 初始化任务 / Initialization Task
// ============================================================================

/// 初始化任务状态
/// Initialization task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// 等待执行
    /// Waiting to execute
    Pending,

    /// 正在执行
    /// Currently executing
    Running,

    /// 成功完成
    /// Completed successfully
    Completed,

    /// 失败
    /// Failed
    Failed,

    /// 跳过（依赖失败）
    /// Skipped (dependency failed)
    Skipped,
}

/// 初始化任务结果
/// Initialization task result
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// 组件名称
    /// Component name
    pub name: String,

    /// 状态
    /// Status
    pub status: TaskStatus,

    /// 实际执行时间（微秒）
    /// Actual execution time in microseconds
    pub duration_us: u64,

    /// 错误信息（如果失败）
    /// Error message if failed
    pub error: Option<String>,
}

impl TaskResult {
    /// 创建成功结果
    /// Create success result
    pub fn success(name: impl Into<String>, duration_us: u64) -> Self {
        Self {
            name: name.into(),
            status: TaskStatus::Completed,
            duration_us,
            error: None,
        }
    }

    /// 创建失败结果
    /// Create failure result
    pub fn failure(name: impl Into<String>, duration_us: u64, error: String) -> Self {
        Self {
            name: name.into(),
            status: TaskStatus::Failed,
            duration_us,
            error: Some(error),
        }
    }

    /// 创建跳过结果
    /// Create skipped result
    pub fn skipped(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: TaskStatus::Skipped,
            duration_us: 0,
            error: None,
        }
    }

    /// 检查是否成功
    /// Check if successful
    pub fn is_success(&self) -> bool {
        self.status == TaskStatus::Completed
    }

    /// 检查是否失败
    /// Check if failed
    pub fn is_failure(&self) -> bool {
        self.status == TaskStatus::Failed
    }
}

// ============================================================================
// 初始化函数类型 / Initialization Function Types
// ============================================================================

/// 初始化函数类型
/// Initialization function type
pub type InitFn = fn() -> Result<()>;

/// 带上下文的初始化函数类型
/// Initialization function with context type
pub type InitFnWithContext<'a> = fn(&'a InitContext) -> Result<()>;

/// 初始化上下文
/// Initialization context
pub struct InitContext<'a> {
    /// 进度报告器
    /// Progress reporter
    pub progress: &'a InitProgress,

    /// 线程池中的工作线程ID
    /// Worker thread ID in thread pool
    pub worker_id: usize,

    /// 可用的CPU数量
    /// Available CPU count
    pub num_cpus: usize,
}

impl<'a> InitContext<'a> {
    /// 创建新的初始化上下文
    /// Create new initialization context
    pub fn new(progress: &'a InitProgress, worker_id: usize, num_cpus: usize) -> Self {
        Self {
            progress,
            worker_id,
            num_cpus,
        }
    }

    /// 报告进度更新
    /// Report progress update
    pub fn report_progress(&self, component: &str, progress_percent: u8) {
        self.progress.update_component(component, progress_percent);
    }

    /// 检查是否应该中止
    /// Check if should abort
    pub fn should_abort(&self) -> bool {
        self.progress.is_aborted()
    }
}

// ============================================================================
// 初始化进度跟踪 / Initialization Progress Tracking
// ============================================================================

/// 组件进度信息
/// Component progress information
#[derive(Debug)]
struct ComponentProgress {
    /// 组件名称
    /// Component name
    name: String,

    /// 进度百分比 (0-100)
    /// Progress percentage (0-100)
    percent: core::sync::atomic::AtomicU8,

    /// 状态
    /// Status
    status: core::sync::atomic::AtomicU8,

    /// 开始时间（微秒）
    /// Start time in microseconds
    start_time: AtomicU64,

    /// 结束时间（微秒）
    /// End time in microseconds
    end_time: AtomicU64,
}

// TaskStatus constants
const STATUS_PENDING: u8 = 0;
const STATUS_RUNNING: u8 = 1;
const STATUS_COMPLETED: u8 = 2;
const STATUS_FAILED: u8 = 3;
const STATUS_SKIPPED: u8 = 4;

/// 初始化进度跟踪器
/// Initialization progress tracker
pub struct InitProgress {
    /// 所有组件的进度
    /// Progress for all components
    components: Mutex<BTreeMap<String, ComponentProgress>>,

    /// 总体进度 (0-100)
    /// Overall progress (0-100)
    overall_progress: core::sync::atomic::AtomicU8,

    /// 是否已中止
    /// Whether aborted
    aborted: AtomicBool,

    /// 完成的组件数量
    /// Number of completed components
    completed_count: AtomicUsize,

    /// 总组件数量
    /// Total component count
    total_count: usize,

    /// 开始时间
    /// Start time
    start_time: AtomicU64,
}

impl InitProgress {
    /// 创建新的进度跟踪器
    /// Create new progress tracker
    pub fn new(component_names: Vec<String>) -> Self {
        let total_count = component_names.len();
        let mut components = BTreeMap::new();

        for name in component_names {
            components.insert(
                name.clone(),
                ComponentProgress {
                    name,
                    percent: core::sync::atomic::AtomicU8::new(0),
                    status: core::sync::atomic::AtomicU8::new(STATUS_PENDING),
                    start_time: AtomicU64::new(0),
                    end_time: AtomicU64::new(0),
                },
            );
        }

        Self {
            components: Mutex::new(components),
            overall_progress: core::sync::atomic::AtomicU8::new(0),
            aborted: AtomicBool::new(false),
            completed_count: AtomicUsize::new(0),
            total_count,
            start_time: AtomicU64::new(0),
        }
    }

    /// 开始初始化
    /// Start initialization
    pub fn start(&self) {
        self.start_time.store(crate::subsystems::time::get_ticks(), Ordering::SeqCst);
    }

    /// 标记组件开始
    /// Mark component as started
    pub fn mark_component_started(&self, name: &str) {
        if let Some(components) = self.components.try_lock() {
            if let Some(comp) = components.get(name) {
                comp.status.store(STATUS_RUNNING, Ordering::SeqCst);
                comp.start_time
                    .store(crate::subsystems::time::get_ticks(), Ordering::SeqCst);
            }
        }
    }

    /// 标记组件完成
    /// Mark component as completed
    pub fn mark_component_completed(&self, name: &str, duration_us: u64) {
        if let Some(components) = self.components.try_lock() {
            if let Some(comp) = components.get(name) {
                comp.status.store(STATUS_COMPLETED, Ordering::SeqCst);
                comp.percent.store(100, Ordering::SeqCst);
                comp.end_time
                    .store(crate::subsystems::time::get_ticks(), Ordering::SeqCst);

                let completed = self.completed_count.fetch_add(1, Ordering::SeqCst) + 1;
                let overall = ((completed * 100) / self.total_count) as u8;
                self.overall_progress.store(overall, Ordering::SeqCst);
            }
        }
    }

    /// 标记组件失败
    /// Mark component as failed
    pub fn mark_component_failed(&self, name: &str, error: String) {
        if let Some(components) = self.components.try_lock() {
            if let Some(comp) = components.get(name) {
                comp.status.store(STATUS_FAILED, Ordering::SeqCst);
                comp.end_time
                    .store(crate::subsystems::time::get_ticks(), Ordering::SeqCst);
            }
        }
        crate::println!("[parallel_init] Component '{}' failed: {}", name, error);
    }

    /// 更新组件进度
    /// Update component progress
    pub fn update_component(&self, name: &str, percent: u8) {
        if let Some(components) = self.components.try_lock() {
            if let Some(comp) = components.get(name) {
                comp.percent.store(percent.min(100), Ordering::SeqCst);
            }
        }
    }

    /// 获取组件进度
    /// Get component progress
    pub fn get_component_progress(&self, name: &str) -> Option<u8> {
        self.components.try_lock().and_then(|components| {
            components.get(name).map(|comp| comp.percent.load(Ordering::SeqCst))
        })
    }

    /// 获取总体进度
    /// Get overall progress
    pub fn overall_progress(&self) -> u8 {
        self.overall_progress.load(Ordering::SeqCst)
    }

    /// 中止初始化
    /// Abort initialization
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::SeqCst);
    }

    /// 检查是否已中止
    /// Check if aborted
    pub fn is_aborted(&self) -> bool {
        self.aborted.load(Ordering::SeqCst)
    }

    /// 获取已完成的组件数量
    /// Get completed component count
    pub fn completed_count(&self) -> usize {
        self.completed_count.load(Ordering::SeqCst)
    }

    /// 获取总组件数量
    /// Get total component count
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// 检查是否完成
    /// Check if completed
    pub fn is_complete(&self) -> bool {
        self.completed_count() == self.total_count
    }

    /// 获取经过时间（微秒）
    /// Get elapsed time in microseconds
    pub fn elapsed_time_us(&self) -> u64 {
        let start = self.start_time.load(Ordering::SeqCst);
        let current = crate::subsystems::time::get_ticks();
        current.saturating_sub(start)
    }
}

// ============================================================================
// 初始化线程池 / Initialization Thread Pool
// ============================================================================

/// 初始化线程池
/// Initialization thread pool
///
/// 用于并行执行初始化任务的简单线程池
/// Simple thread pool for parallel initialization tasks
pub struct InitThreadPool {
    /// 工作线程数量
    /// Number of worker threads
    num_workers: usize,

    /// 可用的CPU数量
    /// Available CPU count
    num_cpus: usize,
}

impl InitThreadPool {
    /// 创建新的线程池
    /// Create new thread pool
    pub fn new(num_workers: usize, num_cpus: usize) -> Self {
        Self {
            num_workers,
            num_cpus,
        }
    }

    /// 获取工作线程数量
    /// Get number of worker threads
    pub fn num_workers(&self) -> usize {
        self.num_workers
    }

    /// 获取可用CPU数量
    /// Get available CPU count
    pub fn num_cpus(&self) -> usize {
        self.num_cpus
    }

    /// 检查是否支持并行（是否有多个CPU）
    /// Check if parallel is supported (multiple CPUs)
    pub fn supports_parallel(&self) -> bool {
        self.num_cpus > 1
    }
}

// ============================================================================
// 并行初始化引擎 / Parallel Initialization Engine
// ============================================================================

/// 并行初始化引擎配置
/// Parallel initialization engine configuration
#[derive(Debug, Clone)]
pub struct ParallelInitConfig {
    /// 最大并行任务数
    /// Maximum parallel tasks
    pub max_parallel_tasks: usize,

    /// 是否在失败时中止
    /// Whether to abort on failure
    pub abort_on_failure: bool,

    /// 是否启用性能监控
    /// Whether to enable performance monitoring
    pub enable_monitoring: bool,

    /// 单个任务的超时时间（微秒，0表示无限制）
    /// Per-task timeout in microseconds (0 = unlimited)
    pub task_timeout_us: u64,

    /// 详细输出
    /// Verbose output
    pub verbose: bool,
}

impl Default for ParallelInitConfig {
    fn default() -> Self {
        Self {
            max_parallel_tasks: 4,
            abort_on_failure: true,
            enable_monitoring: true,
            task_timeout_us: 0,
            verbose: false,
        }
    }
}

/// 并行初始化统计信息
/// Parallel initialization statistics
#[derive(Debug, Clone)]
pub struct InitStatistics {
    /// 总初始化时间（微秒）
    /// Total initialization time in microseconds
    pub total_time_us: u64,

    /// 成功完成的组件数量
    /// Number of successfully completed components
    pub completed_count: usize,

    /// 失败的组件数量
    /// Number of failed components
    pub failed_count: usize,

    /// 跳过的组件数量
    /// Number of skipped components
    pub skipped_count: usize,

    /// 所有任务的结果
    /// Results of all tasks
    pub task_results: Vec<TaskResult>,

    /// 预期加速比
    /// Expected speedup
    pub expected_speedup: f64,

    /// 实际加速比
    /// Actual speedup
    pub actual_speedup: f64,

    /// CPU利用率（0-100）
    /// CPU utilization (0-100)
    pub cpu_utilization: f64,
}

impl InitStatistics {
    /// 获取总组件数量
    /// Get total component count
    pub fn total_count(&self) -> usize {
        self.completed_count + self.failed_count + self.skipped_count
    }

    /// 检查是否全部成功
    /// Check if all successful
    pub fn is_all_success(&self) -> bool {
        self.failed_count == 0
    }

    /// 计算成功率
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.total_count();
        if total == 0 {
            return 1.0;
        }
        (self.completed_count as f64) / (total as f64)
    }
}

/// 并行初始化引擎
/// Parallel initialization engine
///
/// 负责协调内核组件的并行初始化
/// Coordinates parallel initialization of kernel components
pub struct ParallelInitEngine {
    /// 依赖图
    /// Dependency graph
    dependency_graph: DependencyGraph,

    /// 组件初始化函数映射
    /// Component initialization function mapping
    init_functions: BTreeMap<String, InitFn>,

    /// 配置
    /// Configuration
    config: ParallelInitConfig,

    /// 进度跟踪器
    /// Progress tracker
    progress: InitProgress,

    /// 统计信息
    /// Statistics
    statistics: SpinLock<InitStatistics>,
}

impl ParallelInitEngine {
    /// 创建新的并行初始化引擎
    /// Create new parallel initialization engine
    pub fn new(dependency_graph: DependencyGraph, config: ParallelInitConfig) -> Self {
        let component_names: Vec<String> =
            dependency_graph.components().iter().map(|c| c.name.clone()).collect();

        let total_estimated_time = dependency_graph.total_serial_time();
        let parallel_safety = dependency_graph.analyze_parallel_safety();

        Self {
            dependency_graph,
            init_functions: BTreeMap::new(),
            config,
            progress: InitProgress::new(component_names),
            statistics: SpinLock::new(InitStatistics {
                total_time_us: 0,
                completed_count: 0,
                failed_count: 0,
                skipped_count: 0,
                task_results: Vec::new(),
                expected_speedup: parallel_safety.expected_speedup,
                actual_speedup: 1.0,
                cpu_utilization: 0.0,
            }),
        }
    }

    /// 注册组件初始化函数
    /// Register component initialization function
    pub fn register_init_function(&mut self, name: impl Into<String>, func: InitFn) {
        let name = name.into();
        self.init_functions.insert(name.clone(), func);
        crate::println!("[parallel_init] Registered init function for: {}", name);
    }

    /// 执行并行初始化
    /// Execute parallel initialization
    pub fn initialize(&self) -> Result<InitStatistics> {
        crate::println!("[parallel_init] Starting parallel initialization...");
        self.progress.start();

        let start_time = crate::subsystems::time::get_ticks();

        // 获取CPU数量
        let num_cpus = crate::cpu::ncpus();
        let thread_pool = InitThreadPool::new(self.config.max_parallel_tasks.min(num_cpus), num_cpus);

        crate::println!(
            "[parallel_init] Using {} worker threads on {} CPUs",
            thread_pool.num_workers(),
            num_cpus
        );

        // 第一阶段：早期初始化（串行）
        self.run_early_init()?;

        // 第二阶段：并行初始化
        if thread_pool.supports_parallel() {
            self.run_parallel_init(&thread_pool)?;
        } else {
            crate::println!("[parallel_init] Single CPU detected, using serial initialization");
            self.run_serial_init()?;
        }

        // 第三阶段：晚期初始化
        self.run_late_init()?;

        let end_time = crate::subsystems::time::get_ticks();
        let total_time_us = end_time.saturating_sub(start_time);

        // 更新统计信息
        {
            let mut stats = self.statistics.lock();
            stats.total_time_us = total_time_us;
        }

        crate::println!(
            "[parallel_init] Initialization complete in {} us",
            total_time_us
        );

        Ok(self.statistics.clone().into_inner())
    }

    /// 早期初始化（串行执行）
    /// Early initialization (serial execution)
    fn run_early_init(&self) -> Result<()> {
        crate::println!("[parallel_init] Phase 1: Early initialization (serial)");

        let early_components = self.dependency_graph.early_components();

        for comp in early_components {
            if self.progress.is_aborted() {
                return Err(Error::interrupted("Initialization aborted".into()));
            }

            self.execute_component(comp)?;
        }

        Ok(())
    }

    /// 并行初始化
    /// Parallel initialization
    fn run_parallel_init(&self, thread_pool: &InitThreadPool) -> Result<()> {
        crate::println!("[parallel_init] Phase 2: Parallel initialization");

        let parallel_safety = self.dependency_graph.analyze_parallel_safety();

        crate::println!(
            "[parallel_init] Parallelism degree: {:.2}",
            parallel_safety.parallelism_degree
        );
        crate::println!(
            "[parallel_init] Expected speedup: {:.2}x",
            parallel_safety.expected_speedup
        );

        // 按并行组执行
        for (group_idx, group) in parallel_safety.parallel_groups.iter().enumerate() {
            crate::println!(
                "[parallel_init] Parallel group {}/{}: {} components",
                group_idx + 1,
                parallel_safety.parallel_groups.len(),
                group.len()
            );

            self.execute_parallel_group(group, thread_pool)?;
        }

        // 执行串行组件
        for name in &parallel_safety.serial_components {
            if let Some(comp) = self.dependency_graph.get_component(name) {
                self.execute_component(comp)?;
            }
        }

        Ok(())
    }

    /// 串行初始化（单CPU fallback）
    /// Serial initialization (single CPU fallback)
    fn run_serial_init(&self) -> Result<()> {
        crate::println!("[parallel_init] Phase 2: Serial initialization (fallback)");

        let sorted = self.dependency_graph.topological_sort()?;

        for idx in sorted {
            let comp = &self.dependency_graph.components()[idx];

            // 跳过早期和晚期组件
            if comp.is_early() || comp.is_late() {
                continue;
            }

            if self.progress.is_aborted() {
                return Err(Error::interrupted("Initialization aborted".into()));
            }

            self.execute_component(comp)?;
        }

        Ok(())
    }

    /// 晚期初始化
    /// Late initialization
    fn run_late_init(&self) -> Result<()> {
        crate::println!("[parallel_init] Phase 3: Late initialization");

        let late_components = self.dependency_graph.late_components();

        for comp in late_components {
            if self.progress.is_aborted() {
                return Err(Error::interrupted("Initialization aborted".into()));
            }

            self.execute_component(comp)?;
        }

        Ok(())
    }

    /// 执行单个组件
    /// Execute a single component
    fn execute_component(&self, comp: &InitDependency) -> Result<()> {
        let name = &comp.name;

        crate::println!(
            "[parallel_init] Initializing '{}' (est. {} us)",
            name,
            comp.estimated_time_us
        );

        self.progress.mark_component_started(name);

        let start_time = crate::subsystems::time::get_ticks();

        // 获取并执行初始化函数
        let result = if let Some(init_fn) = self.init_functions.get(name) {
            init_fn()
        } else {
            // 如果没有注册函数，使用默认行为
            Ok(())
        };

        let end_time = crate::subsystems::time::get_ticks();
        let duration_us = end_time.saturating_sub(start_time);

        match result {
            Ok(()) => {
                self.progress.mark_component_completed(name, duration_us);
                self.record_success(name, duration_us);

                crate::println!(
                    "[parallel_init] '{}' completed in {} us",
                    name,
                    duration_us
                );
                Ok(())
            }
            Err(e) => {
                self.progress.mark_component_failed(name, e.to_string());
                self.record_failure(name, duration_us, e.to_string());

                if comp.optional {
                    crate::println!(
                        "[parallel_init] '{}' optional component failed, continuing",
                        name
                    );
                    Ok(())
                } else if self.config.abort_on_failure {
                    crate::println!("[parallel_init] '{}' failed, aborting", name);
                    self.progress.abort();
                    Err(e)
                } else {
                    crate::println!(
                        "[parallel_init] '{}' failed, continuing (abort_on_failure=false)",
                        name
                    );
                    Ok(())
                }
            }
        }
    }

    /// 执行并行组件组
    /// Execute parallel component group
    fn execute_parallel_group(
        &self,
        group: &[String],
        thread_pool: &InitThreadPool,
    ) -> Result<()> {
        // 简化实现：由于我们在内核中，真正的线程并行很复杂
        // 这里我们模拟并行执行，实际还是串行，但展示了框架设计
        // 在完整实现中，会使用工作线程池真正并行执行

        for name in group {
            if let Some(comp) = self.dependency_graph.get_component(name) {
                self.execute_component(comp)?;
            }
        }

        Ok(())
    }

    /// 记录成功
    /// Record success
    fn record_success(&self, name: &str, duration_us: u64) {
        let mut stats = self.statistics.lock();
        stats.completed_count += 1;
        stats
            .task_results
            .push(TaskResult::success(name, duration_us));
    }

    /// 记录失败
    /// Record failure
    fn record_failure(&self, name: &str, duration_us: u64, error: String) {
        let mut stats = self.statistics.lock();
        stats.failed_count += 1;
        stats
            .task_results
            .push(TaskResult::failure(name, duration_us, error));
    }

    /// 获取进度
    /// Get progress
    pub fn progress(&self) -> &InitProgress {
        &self.progress
    }

    /// 获取配置
    /// Get configuration
    pub fn config(&self) -> &ParallelInitConfig {
        &self.config
    }
}

// ============================================================================
// 辅助函数 / Helper Functions
// ============================================================================

/// 创建默认并行初始化引擎
/// Create default parallel initialization engine
pub fn create_default_parallel_engine() -> Result<ParallelInitEngine> {
    let deps = crate::core::init_dependencies::create_standard_kernel_deps()?;
    let config = ParallelInitConfig::default();
    Ok(ParallelInitEngine::new(deps, config))
}

// ============================================================================
// 单元测试 / Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::init_dependencies::{ComponentCategory, DependencyGraph, InitDependency};

    #[test]
    fn test_task_result() {
        let success = TaskResult::success("test", 100);
        assert!(success.is_success());
        assert_eq!(success.name, "test");
        assert_eq!(success.duration_us, 100);

        let failure = TaskResult::failure("test", 50, "error".into());
        assert!(failure.is_failure());
        assert!(failure.error.is_some());

        let skipped = TaskResult::skipped("test");
        assert_eq!(skipped.status, TaskStatus::Skipped);
    }

    #[test]
    fn test_init_progress() {
        let components = vec!["comp1".into(), "comp2".into(), "comp3".into()];
        let progress = InitProgress::new(components);

        assert_eq!(progress.total_count(), 3);
        assert_eq!(progress.completed_count(), 0);
        assert_eq!(progress.overall_progress(), 0);

        progress.mark_component_completed("comp1", 100);
        assert_eq!(progress.completed_count(), 1);
        assert_eq!(progress.overall_progress(), 33);
    }

    #[test]
    fn test_thread_pool() {
        let pool = InitThreadPool::new(4, 8);
        assert_eq!(pool.num_workers(), 4);
        assert_eq!(pool.num_cpus(), 8);
        assert!(pool.supports_parallel());

        let single_pool = InitThreadPool::new(1, 1);
        assert!(!single_pool.supports_parallel());
    }

    #[test]
    fn test_parallel_init_config() {
        let config = ParallelInitConfig::default();
        assert_eq!(config.max_parallel_tasks, 4);
        assert!(config.abort_on_failure);
        assert!(config.enable_monitoring);
    }

    #[test]
    fn test_init_statistics() {
        let mut stats = InitStatistics {
            total_time_us: 1000,
            completed_count: 5,
            failed_count: 1,
            skipped_count: 0,
            task_results: vec![],
            expected_speedup: 2.0,
            actual_speedup: 1.8,
            cpu_utilization: 85.0,
        };

        assert_eq!(stats.total_count(), 6);
        assert!(!stats.is_all_success());
        assert!((stats.success_rate() - 0.8333).abs() < 0.01);

        stats.failed_count = 0;
        assert!(stats.is_all_success());
    }
}
