//! 进程模块类型定义
//! 
//! 本模块定义了进程管理相关的类型、枚举和结构体，包括：
//! - 进程状态和属性
//! - 进程创建参数
//! - 进程调度信息
//! - 进程间通信相关类型

use alloc::string::String;
use alloc::vec::Vec;

/// 进程状态枚举
/// 
/// 定义进程可能的状态，用于进程调度和管理。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// 新创建，未就绪
    New,
    /// 就绪，等待调度
    Ready,
    /// 运行中
    Running,
    /// 阻塞，等待事件
    Blocked,
    /// 已终止
    Terminated,
    /// 僵尸状态
    Zombie,
}

/// 进程优先级
/// 
/// 定义进程的调度优先级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProcessPriority {
    /// 空闲优先级
    Idle = 0,
    /// 低优先级
    Low = 25,
    /// 普通优先级
    Normal = 50,
    /// 高优先级
    High = 75,
    /// 实时优先级
    Realtime = 100,
}

/// 进程创建参数
/// 
/// 包含创建新进程所需的所有参数。
#[derive(Debug, Clone)]
pub struct ProcessCreateParams {
    /// 进程名称
    pub name: String,
    /// 程序路径
    pub executable_path: String,
    /// 命令行参数
    pub args: Vec<String>,
    /// 环境变量
    pub env_vars: Vec<String>,
    /// 工作目录
    pub working_dir: String,
    /// 标准输入重定向
    pub stdin: Option<String>,
    /// 标准输出重定向
    pub stdout: Option<String>,
    /// 标准错误重定向
    pub stderr: Option<String>,
    /// 进程优先级
    pub priority: ProcessPriority,
    /// 是否创建为新进程组
    pub new_process_group: bool,
}

impl Default for ProcessCreateParams {
    fn default() -> Self {
        Self {
            name: String::new(),
            executable_path: String::new(),
            args: Vec::new(),
            env_vars: Vec::new(),
            working_dir: String::from("/"),
            stdin: None,
            stdout: None,
            stderr: None,
            priority: ProcessPriority::Normal,
            new_process_group: false,
        }
    }
}

/// 进程信息结构体
/// 
/// 包含进程的详细状态和属性信息。
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// 进程ID
    pub pid: u32,
    /// 父进程ID
    pub ppid: u32,
    /// 进程组ID
    pub pgid: u32,
    /// 会话ID
    pub sid: u32,
    /// 进程状态
    pub state: ProcessState,
    /// 进程优先级
    pub priority: ProcessPriority,
    /// 进程名称
    pub name: String,
    /// 可执行文件路径
    pub executable_path: String,
    /// 创建时间戳
    pub create_time: u64,
    /// 用户CPU时间
    pub user_cpu_time: u64,
    /// 系统CPU时间
    pub system_cpu_time: u64,
    /// 内存使用量（字节）
    pub memory_usage: u64,
    /// 退出码（如果已终止）
    pub exit_code: Option<i32>,
}

/// 进程统计信息
/// 
/// 包含进程的运行统计信息。
#[derive(Debug, Clone)]
pub struct ProcessStats {
    /// 总进程数
    pub total_processes: u32,
    /// 运行中进程数
    pub running_processes: u32,
    /// 阻塞进程数
    pub blocked_processes: u32,
    /// 僵尸进程数
    pub zombie_processes: u32,
    /// 系统平均负载
    pub load_average: f64,
}

/// 进程调度策略
/// 
/// 定义进程的调度策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// 普通调度
    Normal,
    /// 先进先出
    FIFO,
    /// 轮转调度
    RoundRobin,
    /// 批处理调度
    Batch,
    /// 空闲调度
    Idle,
}

/// 进程信号掩码
/// 
/// 用于管理进程的信号处理。
#[derive(Debug, Clone)]
pub struct ProcessSignalMask {
    /// 阻塞的信号
    pub blocked_signals: u64,
    /// 挂起的信号
    pub pending_signals: u64,
}

/// 进程资源限制
/// 
/// 定义进程的资源使用限制。
#[derive(Debug, Clone)]
pub struct ProcessResourceLimits {
    /// 最大CPU时间（秒）
    pub max_cpu_time: Option<u64>,
    /// 最大内存使用（字节）
    pub max_memory: Option<u64>,
    /// 最大文件描述符数
    pub max_file_descriptors: Option<u32>,
    /// 最大进程数
    pub max_processes: Option<u32>,
    /// 最大文件大小（字节）
    pub max_file_size: Option<u64>,
}

impl Default for ProcessResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_time: None,
            max_memory: None,
            max_file_descriptors: None,
            max_processes: None,
            max_file_size: None,
        }
    }
}

/// 进程系统调用错误类型
/// 
/// 定义进程模块特有的错误类型。
#[derive(Debug, Clone)]
pub enum ProcessError {
    /// 无效的进程ID
    InvalidPid,
    /// 进程不存在
    ProcessNotFound,
    /// 权限不足
    PermissionDenied,
    /// 资源不足
    ResourceExhausted,
    /// 无效参数
    InvalidArgument,
    /// 进程已存在
    ProcessExists,
    /// 系统调用不支持
    UnsupportedSyscall,
}

impl ProcessError {
    /// 获取错误码
    pub fn error_code(&self) -> i32 {
        match self {
            ProcessError::InvalidPid => -1,
            ProcessError::ProcessNotFound => -2,
            ProcessError::PermissionDenied => -3,
            ProcessError::ResourceExhausted => -4,
            ProcessError::InvalidArgument => -5,
            ProcessError::ProcessExists => -6,
            ProcessError::UnsupportedSyscall => -7,
        }
    }

    /// 获取错误描述
    pub fn error_message(&self) -> &str {
        match self {
            ProcessError::InvalidPid => "Invalid process ID",
            ProcessError::ProcessNotFound => "Process not found",
            ProcessError::PermissionDenied => "Permission denied",
            ProcessError::ResourceExhausted => "Resource exhausted",
            ProcessError::InvalidArgument => "Invalid argument",
            ProcessError::ProcessExists => "Process already exists",
            ProcessError::UnsupportedSyscall => "Unsupported syscall",
        }
    }
}