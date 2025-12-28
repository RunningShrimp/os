//! 系统调用类型模块
//!
//! 本模块提供系统调用相关的类型定义。

use alloc::string::String;
use alloc::vec::Vec;

/// 系统调用信息
#[derive(Debug, Clone)]
pub struct SyscallInfo {
    /// 系统调用号
    pub number: usize,
    /// 系统调用名称
    pub name: String,
    /// 系统调用描述
    pub description: String,
    /// 参数数量
    pub arg_count: usize,
    /// 参数类型
    pub arg_types: Vec<SyscallArgType>,
    /// 返回类型
    pub return_type: SyscallReturnType,
}

/// 系统调用参数类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallArgType {
    /// 无符号整数
    UnsignedInt,
    /// 有符号整数
    SignedInt,
    /// 指针
    Pointer,
    /// 字符串
    String,
    /// 缓冲区
    Buffer,
    /// 文件描述符
    FileDescriptor,
    /// 进程ID
    ProcessId,
    /// 用户ID
    UserId,
    /// 组ID
    GroupId,
    /// 权限
    Permissions,
    /// 标志
    Flags,
}

/// 系统调用返回类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallReturnType {
    /// 无符号整数
    UnsignedInt,
    /// 有符号整数
    SignedInt,
    /// 指针
    Pointer,
    /// 文件描述符
    FileDescriptor,
    /// 进程ID
    ProcessId,
    /// 无返回值
    Void,
    /// 错误码
    ErrorCode,
}

/// 系统调用上下文
#[derive(Debug, Clone)]
pub struct SyscallContext {
    /// 系统调用号
    pub syscall_num: usize,
    /// 系统调用参数
    pub args: Vec<usize>,
    /// 调用者进程ID
    pub caller_pid: usize,
    /// 调用者用户ID
    pub caller_uid: usize,
    /// 调用者组ID
    pub caller_gid: usize,
    /// 调用时间戳
    pub timestamp: u64,
    /// 调用标志
    pub flags: usize,
}

/// 系统调用结果
#[derive(Debug, Clone)]
pub struct SyscallResult {
    /// 返回值
    pub return_value: isize,
    /// 错误码
    pub error_code: isize,
    /// 执行时间（纳秒）
    pub execution_time_ns: u64,
    /// 结果标志
    pub flags: usize,
}

/// 系统调用统计信息
#[derive(Debug, Clone)]
pub struct SyscallStatistics {
    /// 总调用次数
    pub total_calls: u64,
    /// 成功调用次数
    pub successful_calls: u64,
    /// 失败调用次数
    pub failed_calls: u64,
    /// 平均执行时间（纳秒）
    pub avg_execution_time_ns: u64,
    /// 最小执行时间（纳秒）
    pub min_execution_time_ns: u64,
    /// 最大执行时间（纳秒）
    pub max_execution_time_ns: u64,
    /// 各系统调用统计
    pub calls_by_type: alloc::collections::BTreeMap<usize, u64>,
}

impl SyscallStatistics {
    /// 创建新的系统调用统计信息
    pub fn new() -> Self {
        Self {
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            avg_execution_time_ns: 0,
            min_execution_time_ns: u64::MAX,
            max_execution_time_ns: 0,
            calls_by_type: alloc::collections::BTreeMap::new(),
        }
    }
    
    /// 更新统计信息
    pub fn update(&mut self, syscall_num: usize, execution_time_ns: u64, success: bool) {
        self.total_calls += 1;
        
        if success {
            self.successful_calls += 1;
        } else {
            self.failed_calls += 1;
        }
        
        // 更新执行时间统计
        if execution_time_ns < self.min_execution_time_ns {
            self.min_execution_time_ns = execution_time_ns;
        }
        
        if execution_time_ns > self.max_execution_time_ns {
            self.max_execution_time_ns = execution_time_ns;
        }
        
        // 更新平均执行时间
        let total_time = self.avg_execution_time_ns * (self.total_calls - 1) + execution_time_ns;
        self.avg_execution_time_ns = total_time / self.total_calls;
        
        // 更新各系统调用统计
        *self.calls_by_type.entry(syscall_num).or_insert(0) += 1;
    }
    
    /// 重置统计信息
    pub fn reset(&mut self) {
        self.total_calls = 0;
        self.successful_calls = 0;
        self.failed_calls = 0;
        self.avg_execution_time_ns = 0;
        self.min_execution_time_ns = u64::MAX;
        self.max_execution_time_ns = 0;
        self.calls_by_type.clear();
    }
}

impl Default for SyscallStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// 系统调用过滤器
pub trait SyscallFilter: Send + Sync {
    /// 检查是否允许执行系统调用
    fn allow_syscall(&self, context: &SyscallContext) -> bool;
    
    /// 获取过滤器名称
    fn name(&self) -> &str;
    
    /// 获取过滤器优先级
    fn priority(&self) -> u32 {
        100
    }
}

/// 系统调用拦截器
pub trait SyscallInterceptor: Send + Sync {
    /// 在系统调用执行前拦截
    fn before_syscall(&self, context: &SyscallContext) -> Option<isize> {
        None
    }
    
    /// 在系统调用执行后拦截
    fn after_syscall(&self, context: &SyscallContext, result: &mut SyscallResult) {
        // 默认实现：不做任何操作
    }
    
    /// 获取拦截器名称
    fn name(&self) -> &str;
    
    /// 获取拦截器优先级
    fn priority(&self) -> u32 {
        100
    }
}

/// 系统调用日志记录器
pub trait SyscallLogger: Send + Sync {
    /// 记录系统调用
    fn log_syscall(&self, context: &SyscallContext, result: &SyscallResult);
    
    /// 获取日志记录器名称
    fn name(&self) -> &str;
    
    /// 获取日志级别
    fn level(&self) -> SyscallLogLevel {
        SyscallLogLevel::Info
    }
}

/// 系统调用日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyscallLogLevel {
    /// 调试
    Debug = 0,
    /// 信息
    Info = 1,
    /// 警告
    Warning = 2,
    /// 错误
    Error = 3,
    /// 严重
    Critical = 4,
}