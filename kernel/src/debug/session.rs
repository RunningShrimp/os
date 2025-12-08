// Debug Session Management Module
//
// 调试会话管理模块
// 提供调试会话、事件、进程信息等管理功能

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// 调试会话
#[derive(Debug, Clone)]
pub struct DebugSession {
    /// 会话ID
    pub id: String,
    /// 会话名称
    pub name: String,
    /// 会话类型
    pub session_type: DebugSessionType,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: Option<u64>,
    /// 会话状态
    pub status: DebugSessionStatus,
    /// 目标进程
    pub target_process: Option<ProcessInfo>,
    /// 调试事件
    pub debug_events: Vec<DebugEvent>,
    /// 调试数据
    pub debug_data: BTreeMap<String, Vec<u8>>,
    /// 会话配置
    pub config: SessionConfig,
    /// 性能数据
    pub performance_data: Vec<crate::debug::analyzer::PerformanceSnapshot>,
}

/// 调试会话类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugSessionType {
    /// 系统调试
    SystemDebug,
    /// 进程调试
    ProcessDebug,
    /// 内核调试
    KernelDebug,
    /// 内存调试
    MemoryDebug,
    /// 性能调试
    PerformanceDebug,
    /// 网络调试
    NetworkDebug,
    /// 实时监控
    RealTimeMonitor,
    /// 自定义调试
    CustomDebug,
}

/// 调试会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugSessionStatus {
    /// 初始化中
    Initializing,
    /// 活动中
    Active,
    /// 暂停
    Paused,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
    /// 出错
    Error,
}

/// 进程信息
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// 进程ID
    pub pid: u32,
    /// 进程名称
    pub name: String,
    /// 进程状态
    pub state: ProcessState,
    /// 父进程ID
    pub parent_pid: u32,
    /// 进程路径
    pub exec_path: String,
    /// 命令行参数
    pub args: Vec<String>,
    /// 环境变量
    pub env_vars: BTreeMap<String, String>,
    /// 线程数
    pub thread_count: u32,
    /// 内存使用
    pub memory_usage: ProcessMemoryUsage,
    /// CPU使用
    pub cpu_usage: f64,
}

/// 进程状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// 运行中
    Running,
    /// 睡眠中
    Sleeping,
    /// 等待中
    Waiting,
    /// 僵尸进程
    Zombie,
    /// 停止
    Stopped,
    /// 未知
    Unknown,
}

/// 进程内存使用
#[derive(Debug, Clone)]
pub struct ProcessMemoryUsage {
    /// 虚拟内存大小（字节）
    pub virtual_size: u64,
    /// 物理内存大小（字节）
    pub resident_size: u64,
    /// 共享内存大小（字节）
    pub shared_size: u64,
    /// 代码段大小（字节）
    pub text_size: u64,
    /// 数据段大小（字节）
    pub data_size: u64,
    /// 栈段大小（字节）
    pub stack_size: u64,
}

/// 调试事件
#[derive(Debug, Clone)]
pub struct DebugEvent {
    /// 事件ID
    pub id: String,
    /// 事件时间
    pub timestamp: u64,
    /// 事件类型
    pub event_type: DebugEventType,
    /// 事件级别
    pub level: DebugLevel,
    /// 事件描述
    pub description: String,
    /// 事件数据
    pub event_data: BTreeMap<String, String>,
    /// 来源组件
    pub source: String,
    /// 线程信息
    pub thread_info: Option<ThreadInfo>,
}

/// 调试事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugEventType {
    /// 断点命中
    BreakpointHit,
    /// 异常触发
    ExceptionTriggered,
    /// 系统调用
    SystemCall,
    /// 内存访问
    MemoryAccess,
    /// 函数调用
    FunctionCall,
    /// 变量变化
    VariableChange,
    /// 性能事件
    PerformanceEvent,
    /// 错误事件
    ErrorEvent,
    /// 信息事件
    InfoEvent,
    /// 自定义事件
    CustomEvent,
}

/// 调试级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugLevel {
    /// 调试
    Debug,
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 致命
    Fatal,
}

/// 线程信息
#[derive(Debug, Clone)]
pub struct ThreadInfo {
    /// 线程ID
    pub tid: u32,
    /// 线程名称
    pub name: String,
    /// 线程状态
    pub state: ThreadState,
    /// 线程优先级
    pub priority: u8,
    /// 栈指针
    pub stack_pointer: u64,
    /// 指令指针
    pub instruction_pointer: u64,
}

/// 线程状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// 运行中
    Running,
    /// 就绪
    Ready,
    /// 等待
    Waiting,
    /// 阻塞
    Blocked,
    /// 终止
    Terminated,
    /// 未知
    Unknown,
}

/// 会话配置
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// 自动启动
    pub auto_start: bool,
    /// 优先级
    pub priority: u32,
    /// 启用实时监控
    pub enable_real_time_monitoring: bool,
    /// 启用符号解析
    pub enable_symbol_resolution: bool,
    /// 启用性能分析
    pub enable_performance_analysis: bool,
    /// 启用内存分析
    pub enable_memory_analysis: bool,
    /// 调试级别
    pub debug_level: DebugLevel,
    /// 自定义配置
    pub custom_config: BTreeMap<String, String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_start: true,
            priority: 5,
            enable_real_time_monitoring: false,
            enable_symbol_resolution: true,
            enable_performance_analysis: true,
            enable_memory_analysis: true,
            debug_level: DebugLevel::Info,
            custom_config: BTreeMap::new(),
        }
    }
}

