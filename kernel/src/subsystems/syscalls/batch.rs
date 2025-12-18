//! 系统调用批处理机制
//!
//! 提供高效的系统调用批处理功能，包括：
//! - 批处理接口设计
//! - 减少系统调用开销和上下文切换
//! - 批处理错误处理机制
//! - 性能监控和统计

extern crate alloc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// 批处理系统调用类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchSyscallType {
    /// 文件I/O操作
    FileIO,
    /// 内存管理操作
    Memory,
    /// 进程管理操作
    Process,
    /// 网络操作
    Network,
    /// 信号操作
    Signal,
    /// 混合类型
    Mixed,
}

/// 单个批处理系统调用描述
#[derive(Debug, Clone)]
pub struct BatchSyscall {
    /// 系统调用编号
    pub syscall_num: u32,
    /// 系统调用参数
    pub args: [u64; 6],
    /// 系统调用类型
    pub syscall_type: BatchSyscallType,
    /// 预期结果大小（字节）
    pub expected_result_size: usize,
    /// 是否允许部分失败
    pub allow_partial_failure: bool,
}

impl BatchSyscall {
    /// 创建新的批处理系统调用
    #[inline]
    pub const fn new(
        syscall_num: u32,
        args: [u64; 6],
        syscall_type: BatchSyscallType,
    ) -> Self {
        Self {
            syscall_num,
            args,
            syscall_type,
            expected_result_size: 0,
            allow_partial_failure: false,
        }
    }

    /// 设置预期结果大小
    #[inline]
    pub fn with_result_size(mut self, size: usize) -> Self {
        self.expected_result_size = size;
        self
    }

    /// 设置是否允许部分失败
    #[inline]
    pub fn allow_partial_failure(mut self, allow: bool) -> Self {
        self.allow_partial_failure = allow;
        self
    }
}

/// 批处理系统调用结果
#[derive(Debug, Clone)]
pub struct BatchSyscallResult {
    /// 系统调用编号
    pub syscall_num: u32,
    /// 执行结果
    pub result: isize,
    /// 错误码（如果失败）
    pub error_code: Option<i32>,
    /// 实际结果数据
    pub result_data: Option<Vec<u8>>,
    /// 执行时间（纳秒）
    pub execution_time_ns: u64,
}

impl BatchSyscallResult {
    /// 创建成功的系统调用结果
    #[inline]
    pub const fn success(syscall_num: u32, result: isize) -> Self {
        Self {
            syscall_num,
            result,
            error_code: None,
            result_data: None,
            execution_time_ns: 0,
        }
    }

    /// 创建失败的系统调用结果
    #[inline]
    pub const fn failure(syscall_num: u32, result: isize, error_code: i32) -> Self {
        Self {
            syscall_num,
            result,
            error_code: Some(error_code),
            result_data: None,
            execution_time_ns: 0,
        }
    }

    /// 创建带数据的系统调用结果
    #[inline]
    pub fn with_data(syscall_num: u32, result: isize, data: Vec<u8>) -> Self {
        Self {
            syscall_num,
            result,
            error_code: None,
            result_data: Some(data),
            execution_time_ns: 0,
        }
    }

    /// 设置执行时间
    #[inline]
    pub fn with_execution_time(mut self, time_ns: u64) -> Self {
        self.execution_time_ns = time_ns;
        self
    }

    /// 检查是否成功
    #[inline]
    pub fn is_success(&self) -> bool {
        self.error_code.is_none()
    }

    /// 检查是否允许部分失败
    #[inline]
    pub fn is_partial_failure_allowed(&self) -> bool {
        // 这里需要从原始调用中获取，简化实现
        false
    }
}

/// 批处理请求
#[derive(Debug, Clone)]
pub struct BatchRequest {
    /// 批处理ID
    pub batch_id: u64,
    /// 系统调用列表
    pub syscalls: Vec<BatchSyscall>,
    /// 批处理类型
    pub batch_type: BatchSyscallType,
    /// 是否原子执行（全部成功或全部失败）
    pub atomic: bool,
    /// 超时时间（毫秒）
    pub timeout_ms: u32,
}

impl BatchRequest {
    /// 创建新的批处理请求
    #[inline]
    pub fn new(batch_id: u64, syscalls: Vec<BatchSyscall>, batch_type: BatchSyscallType) -> Self {
        Self {
            batch_id,
            syscalls,
            batch_type,
            atomic: false,
            timeout_ms: 5000, // 默认5秒超时
        }
    }

    /// 设置原子执行标志
    #[inline]
    pub fn atomic(mut self, atomic: bool) -> Self {
        self.atomic = atomic;
        self
    }

    /// 设置超时时间
    #[inline]
    pub fn with_timeout(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// 批处理响应
#[derive(Debug, Clone)]
pub struct BatchResponse {
    /// 批处理ID
    pub batch_id: u64,
    /// 系统调用结果列表
    pub results: Vec<BatchSyscallResult>,
    /// 成功的调用数量
    pub success_count: usize,
    /// 失败的调用数量
    pub failure_count: usize,
    /// 总执行时间（纳秒）
    pub total_execution_time_ns: u64,
    /// 批处理状态
    pub status: BatchStatus,
}

impl BatchResponse {
    /// 创建新的批处理响应
    #[inline]
    pub fn new(batch_id: u64, results: Vec<BatchSyscallResult>) -> Self {
        let success_count = results.iter().filter(|r| r.is_success()).count();
        let failure_count = results.len() - success_count;
        let total_time = results.iter().map(|r| r.execution_time_ns).sum();

        Self {
            batch_id,
            results,
            success_count,
            failure_count,
            total_execution_time_ns: total_time,
            status: BatchStatus::Completed,
        }
    }

    /// 创建失败的批处理响应
    #[inline]
    pub fn failure(batch_id: u64, status: BatchStatus) -> Self {
        Self {
            batch_id,
            results: Vec::new(),
            success_count: 0,
            failure_count: 0,
            total_execution_time_ns: 0,
            status,
        }
    }

    /// 计算成功率
    #[inline]
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            0.0
        } else {
            self.success_count as f64 / total as f64
        }
    }

    /// 计算平均执行时间
    #[inline]
    pub fn avg_execution_time_ns(&self) -> u64 {
        let count = self.success_count + self.failure_count;
        if count == 0 {
            0
        } else {
            self.total_execution_time_ns / count as u64
        }
    }
}

/// 批处理状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchStatus {
    /// 批处理成功完成
    Completed,
    /// 批处理部分失败
    PartialFailure,
    /// 批处理完全失败
    Failed,
    /// 批处理超时
    Timeout,
    /// 批处理被取消
    Cancelled,
    /// 批处理参数错误
    InvalidParameters,
}

/// 批处理统计信息
#[derive(Debug)]
pub struct BatchStats {
    /// 总批处理请求数
    pub total_batches: AtomicU64,
    /// 成功的批处理数
    pub successful_batches: AtomicU64,
    /// 失败的批处理数
    pub failed_batches: AtomicU64,
    /// 超时的批处理数
    pub timeout_batches: AtomicU64,
    /// 总系统调用数
    pub total_syscalls: AtomicU64,
    /// 成功的系统调用数
    pub successful_syscalls: AtomicU64,
    /// 失败的系统调用数
    pub failed_syscalls: AtomicU64,
    /// 总节省的时间（纳秒）
    pub total_time_saved_ns: AtomicU64,
}

impl BatchStats {
    #[inline]
    pub const fn new() -> Self {
        Self {
            total_batches: AtomicU64::new(0),
            successful_batches: AtomicU64::new(0),
            failed_batches: AtomicU64::new(0),
            timeout_batches: AtomicU64::new(0),
            total_syscalls: AtomicU64::new(0),
            successful_syscalls: AtomicU64::new(0),
            failed_syscalls: AtomicU64::new(0),
            total_time_saved_ns: AtomicU64::new(0),
        }
    }

    /// 记录批处理开始
    #[inline]
    pub fn record_batch_start(&self) {
        self.total_batches.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录批处理成功
    #[inline]
    pub fn record_batch_success(&self, syscall_count: usize, time_saved_ns: u64) {
        self.successful_batches.fetch_add(1, Ordering::Relaxed);
        self.total_syscalls.fetch_add(syscall_count as u64, Ordering::Relaxed);
        self.successful_syscalls.fetch_add(syscall_count as u64, Ordering::Relaxed);
        self.total_time_saved_ns.fetch_add(time_saved_ns, Ordering::Relaxed);
    }

    /// 记录批处理失败
    #[inline]
    pub fn record_batch_failure(&self, syscall_count: usize) {
        self.failed_batches.fetch_add(1, Ordering::Relaxed);
        self.total_syscalls.fetch_add(syscall_count as u64, Ordering::Relaxed);
        self.failed_syscalls.fetch_add(syscall_count as u64, Ordering::Relaxed);
    }

    /// 记录批处理超时
    #[inline]
    pub fn record_batch_timeout(&self, syscall_count: usize) {
        self.timeout_batches.fetch_add(1, Ordering::Relaxed);
        self.total_syscalls.fetch_add(syscall_count as u64, Ordering::Relaxed);
    }

    /// 获取统计信息快照
    pub fn get_snapshot(&self) -> BatchStatsSnapshot {
        BatchStatsSnapshot {
            total_batches: self.total_batches.load(Ordering::Relaxed),
            successful_batches: self.successful_batches.load(Ordering::Relaxed),
            failed_batches: self.failed_batches.load(Ordering::Relaxed),
            timeout_batches: self.timeout_batches.load(Ordering::Relaxed),
            total_syscalls: self.total_syscalls.load(Ordering::Relaxed),
            successful_syscalls: self.successful_syscalls.load(Ordering::Relaxed),
            failed_syscalls: self.failed_syscalls.load(Ordering::Relaxed),
            total_time_saved_ns: self.total_time_saved_ns.load(Ordering::Relaxed),
        }
    }
}

/// 批处理统计快照
#[derive(Debug, Clone)]
pub struct BatchStatsSnapshot {
    pub total_batches: u64,
    pub successful_batches: u64,
    pub failed_batches: u64,
    pub timeout_batches: u64,
    pub total_syscalls: u64,
    pub successful_syscalls: u64,
    pub failed_syscalls: u64,
    pub total_time_saved_ns: u64,
}

impl BatchStatsSnapshot {
    /// 计算批处理成功率
    #[inline]
    pub fn batch_success_rate(&self) -> f64 {
        if self.total_batches == 0 {
            0.0
        } else {
            self.successful_batches as f64 / self.total_batches as f64
        }
    }

    /// 计算系统调用成功率
    #[inline]
    pub fn syscall_success_rate(&self) -> f64 {
        if self.total_syscalls == 0 {
            0.0
        } else {
            self.successful_syscalls as f64 / self.total_syscalls as f64
        }
    }

    /// 计算平均节省时间（每个批处理）
    #[inline]
    pub fn avg_time_saved_per_batch(&self) -> u64 {
        if self.successful_batches == 0 {
            0
        } else {
            self.total_time_saved_ns / self.successful_batches
        }
    }
}

/// 批处理配置
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// 最大批处理大小
    pub max_batch_size: usize,
    /// 是否启用自动批处理
    pub enable_auto_batching: bool,
    /// 批处理超时时间（毫秒）
    pub default_timeout_ms: u32,
    /// 是否启用批处理统计
    pub enable_stats: bool,
    /// 是否启用原子批处理
    pub enable_atomic_batches: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 32,
            enable_auto_batching: true,
            default_timeout_ms: 5000,
            enable_stats: true,
            enable_atomic_batches: false,
        }
    }
}

/// 批处理器
pub struct BatchProcessor {
    config: BatchConfig,
    stats: BatchStats,
    next_batch_id: core::sync::atomic::AtomicU64,
}

impl BatchProcessor {
    /// 创建新的批处理器
    #[inline]
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            stats: BatchStats::new(),
            next_batch_id: core::sync::atomic::AtomicU64::new(1),
        }
    }

    /// 使用默认配置创建批处理器
    #[inline]
    pub fn with_defaults() -> Self {
        Self::new(BatchConfig::default())
    }

    /// 执行批处理请求
    pub fn execute_batch(&self, request: BatchRequest) -> BatchResponse {
        let start_time = crate::time::hrtime_nanos();
        self.stats.record_batch_start();

        // 验证批处理请求
        if request.syscalls.is_empty() {
            return BatchResponse::failure(request.batch_id, BatchStatus::InvalidParameters);
        }

        if request.syscalls.len() > self.config.max_batch_size {
            return BatchResponse::failure(request.batch_id, BatchStatus::InvalidParameters);
        }

        // 执行批处理
        let results = if request.atomic {
            self.execute_atomic_batch(&request)
        } else {
            self.execute_normal_batch(&request)
        };

        let end_time = crate::time::hrtime_nanos();
        let total_time = end_time - start_time;

        // 更新统计信息
        let success_count = results.iter().filter(|r| r.is_success()).count();
        let failure_count = results.len() - success_count;
        
        if failure_count == 0 {
            self.stats.record_batch_success(results.len(), total_time);
        } else {
            self.stats.record_batch_failure(results.len());
        }

        // Set execution time in the response
        let status = if failure_count == 0 {
            BatchStatus::Completed
        } else if failure_count == results.len() {
            BatchStatus::Failed
        } else {
            BatchStatus::PartialFailure
        };

        let response = BatchResponse {
            batch_id: request.batch_id,
            results,
            success_count,
            failure_count,
            total_execution_time_ns: total_time,
            status,
        };
        response
    }

    /// 执行原子批处理（全部成功或全部失败）
    fn execute_atomic_batch(&self, request: &BatchRequest) -> Vec<BatchSyscallResult> {
        let mut results = Vec::with_capacity(request.syscalls.len());

        // 预检查所有系统调用的有效性
        for syscall in &request.syscalls {
            if !self.is_syscall_valid(syscall) {
                // 原子批处理中，一个无效调用导致整个批处理失败
                return vec![BatchSyscallResult::failure(
                    syscall.syscall_num,
                    -1,
                    crate::reliability::errno::EINVAL
                ); request.syscalls.len()];
            }
        }

        // 执行所有系统调用
        for syscall in &request.syscalls {
            let result = self.execute_single_syscall(syscall);
            
            // Check if result is success before pushing it into the vector
            let is_success = result.is_success();
            
            results.push(result);
            
            // 如果原子批处理且有一个失败，停止后续调用
            if !is_success {
                // 填充剩余的调用为失败
                for remaining_syscall in request.syscalls.iter().skip(results.len()) {
                    results.push(BatchSyscallResult::failure(
                        remaining_syscall.syscall_num,
                        -1,
                        crate::reliability::errno::ECANCELED
                    ));
                }
                break;
            }
        }

        results
    }

    /// 执行普通批处理（允许部分失败）
    fn execute_normal_batch(&self, request: &BatchRequest) -> Vec<BatchSyscallResult> {
        let mut results = Vec::with_capacity(request.syscalls.len());

        for syscall in &request.syscalls {
            if !self.is_syscall_valid(syscall) {
                results.push(BatchSyscallResult::failure(
                    syscall.syscall_num,
                    -1,
                    crate::reliability::errno::EINVAL
                ));
                continue;
            }

            let result = self.execute_single_syscall(syscall);
            results.push(result);
        }

        results
    }

    /// 验证系统调用是否有效
    fn is_syscall_valid(&self, syscall: &BatchSyscall) -> bool {
        // 检查系统调用编号范围
        match syscall.syscall_type {
            BatchSyscallType::FileIO => {
                syscall.syscall_num >= 0x2000 && syscall.syscall_num <= 0x2FFF
            }
            BatchSyscallType::Memory => {
                syscall.syscall_num >= 0x3000 && syscall.syscall_num <= 0x3FFF
            }
            BatchSyscallType::Process => {
                syscall.syscall_num >= 0x1000 && syscall.syscall_num <= 0x1FFF
            }
            BatchSyscallType::Network => {
                syscall.syscall_num >= 0x4000 && syscall.syscall_num <= 0x4FFF
            }
            BatchSyscallType::Signal => {
                syscall.syscall_num >= 0x5000 && syscall.syscall_num <= 0x5FFF
            }
            BatchSyscallType::Mixed => true, // 混合类型允许所有调用
        }
    }

    /// 执行单个系统调用
    fn execute_single_syscall(&self, syscall: &BatchSyscall) -> BatchSyscallResult {
        let start_time = crate::time::hrtime_nanos();
        
        // 调用实际的系统调用分发器
        let args_usize: [usize; 6] = syscall.args.map(|arg| arg as usize);
        let result = crate::syscalls::dispatch(syscall.syscall_num as usize, &args_usize);
        
        let end_time = crate::time::hrtime_nanos();
        let execution_time = end_time - start_time;

        // Handle the result from dispatch which returns isize directly
        // In system call conventions, negative values indicate errors
        if result >= 0 {
            BatchSyscallResult::success(syscall.syscall_num, result)
        } else {
            // Convert negative result to error code (absolute value)
            let errno = (-result) as i32;
            BatchSyscallResult::failure(syscall.syscall_num, result, errno)
        }
        .with_execution_time(execution_time)
    }

    /// 获取批处理统计信息
    #[inline]
    pub fn get_stats(&self) -> &BatchStats {
        &self.stats
    }

    /// 重置统计信息
    #[inline]
    pub fn reset_stats(&mut self) {
        self.stats = BatchStats::new();
    }

    /// 生成下一个批处理ID
    #[inline]
    pub fn next_batch_id(&self) -> u64 {
        self.next_batch_id.fetch_add(1, Ordering::Relaxed)
    }
}

/// 全局批处理器实例
static mut GLOBAL_BATCH_PROCESSOR: Option<BatchProcessor> = None;
static BATCH_PROCESSOR_INITIALIZED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// 获取全局批处理器
pub fn get_global_batch_processor() -> &'static mut BatchProcessor {
    unsafe {
        if !BATCH_PROCESSOR_INITIALIZED.load(core::sync::atomic::Ordering::Relaxed) {
            GLOBAL_BATCH_PROCESSOR = Some(BatchProcessor::with_defaults());
            BATCH_PROCESSOR_INITIALIZED.store(true, core::sync::atomic::Ordering::Relaxed);
        }
        GLOBAL_BATCH_PROCESSOR.as_mut().unwrap()
    }
}

/// 批处理系统调用接口
pub fn batch_syscalls(syscalls: Vec<BatchSyscall>) -> BatchResponse {
    unsafe {
        let processor = get_global_batch_processor();
        let batch_id = processor.next_batch_id();
        
        // 确定批处理类型
        let batch_type = if syscalls.is_empty() {
            BatchSyscallType::Mixed
        } else {
            // 使用第一个系统调用的类型作为批处理类型
            syscalls[0].syscall_type
        };
        
        let request = BatchRequest::new(batch_id, syscalls, batch_type);
        processor.execute_batch(request)
    }
}

/// 便捷的文件I/O批处理接口
pub fn batch_file_operations(operations: Vec<(u32, [u64; 6])>) -> BatchResponse {
    let syscalls: Vec<BatchSyscall> = operations
        .into_iter()
        .map(|(syscall_num, args)| {
            BatchSyscall::new(syscall_num, args, BatchSyscallType::FileIO)
        })
        .collect();
    
    batch_syscalls(syscalls)
}

/// 便捷的内存管理批处理接口
pub fn batch_memory_operations(operations: Vec<(u32, [u64; 6])>) -> BatchResponse {
    let syscalls: Vec<BatchSyscall> = operations
        .into_iter()
        .map(|(syscall_num, args)| {
            BatchSyscall::new(syscall_num, args, BatchSyscallType::Memory)
        })
        .collect();
    
    batch_syscalls(syscalls)
}

/// 获取批处理统计信息
pub fn get_batch_stats() -> BatchStatsSnapshot {
    unsafe {
        get_global_batch_processor().get_stats().get_snapshot()
    }
}

/// 重置批处理统计信息
pub fn reset_batch_stats() {
    unsafe {
        get_global_batch_processor().reset_stats();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_syscall_creation() {
        let args = [1u64, 2, 3, 4, 5, 6];
        let syscall = BatchSyscall::new(0x2001, args, BatchSyscallType::FileIO)
            .with_result_size(1024)
            .allow_partial_failure(true);
        
        assert_eq!(syscall.syscall_num, 0x2001);
        assert_eq!(syscall.args, args);
        assert_eq!(syscall.syscall_type, BatchSyscallType::FileIO);
        assert_eq!(syscall.expected_result_size, 1024);
        assert!(syscall.allow_partial_failure);
    }

    #[test]
    fn test_batch_request_creation() {
        let syscalls = vec![
            BatchSyscall::new(0x2001, [1, 2, 3, 4, 5, 6], BatchSyscallType::FileIO),
            BatchSyscall::new(0x2002, [7, 8, 9, 10, 11, 12], BatchSyscallType::FileIO),
        ];
        
        let request = BatchRequest::new(123, syscalls.clone(), BatchSyscallType::FileIO)
            .atomic(true)
            .with_timeout(10000);
        
        assert_eq!(request.batch_id, 123);
        assert_eq!(request.syscalls.len(), 2);
        assert_eq!(request.batch_type, BatchSyscallType::FileIO);
        assert!(request.atomic);
        assert_eq!(request.timeout_ms, 10000);
    }

    #[test]
    fn test_batch_processor_creation() {
        let processor = BatchProcessor::with_defaults();
        let stats = processor.get_stats();
        let snapshot = stats.get_snapshot();
        
        assert_eq!(snapshot.total_batches, 0);
        assert_eq!(snapshot.successful_batches, 0);
        assert_eq!(snapshot.failed_batches, 0);
    }

    #[test]
    fn test_batch_stats() {
        let stats = BatchStats::new();
        
        stats.record_batch_start();
        stats.record_batch_success(5, 1000000); // 1ms saved
        stats.record_batch_failure(3);
        stats.record_batch_timeout(2);
        
        let snapshot = stats.get_snapshot();
        assert_eq!(snapshot.total_batches, 3);
        assert_eq!(snapshot.successful_batches, 1);
        assert_eq!(snapshot.failed_batches, 1);
        assert_eq!(snapshot.timeout_batches, 1);
        assert_eq!(snapshot.total_syscalls, 10); // 5 + 3 + 2
        assert_eq!(snapshot.successful_syscalls, 5);
        assert_eq!(snapshot.failed_syscalls, 5); // 3 + 2
        assert_eq!(snapshot.total_time_saved_ns, 1000000);
        
        assert!((snapshot.batch_success_rate() - 0.333).abs() < f64::EPSILON);
        assert!((snapshot.syscall_success_rate() - 0.5).abs() < f64::EPSILON);
    }
}
