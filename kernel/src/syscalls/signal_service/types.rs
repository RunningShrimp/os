//! 信号模块类型定义
//! 
//! 本模块定义了信号相关的类型、枚举和结构体，包括：
//! - 信号类型和编号
//! - 信号处理程序
//! - 信号掩码和集合
//! - 信号上下文信息

use alloc::string::String;
use alloc::vec::Vec;

/// 标准信号编号
/// 
/// 定义POSIX标准信号的编号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum StandardSignal {
    /// 挂起信号
    SIGHUP = 1,
    /// 中断信号
    SIGINT = 2,
    /// 退出信号
    SIGQUIT = 3,
    /// 非法指令
    SIGILL = 4,
    /// 断点陷阱
    SIGTRAP = 5,
    /// 异常终止
    SIGABRT = 6,
    /// 总线错误
    SIGBUS = 7,
    /// 浮点异常
    SIGFPE = 8,
    /// 进程杀死信号
    SIGKILL = 9,
    /// 用户定义信号1
    SIGUSR1 = 10,
    /// 段错误
    SIGSEGV = 11,
    /// 用户定义信号2
    SIGUSR2 = 12,
    /// 管道破裂
    SIGPIPE = 13,
    /// 定时器信号
    SIGALRM = 14,
    /// 软件终止
    SIGTERM = 15,
    /// 子进程状态改变
    SIGCHLD = 17,
    /// 继续执行
    SIGCONT = 18,
    /// 停止执行
    SIGSTOP = 19,
    /// 终端停止
    SIGTSTP = 20,
    /// 后台读取
    SIGTTIN = 21,
    /// 后台写入
    SIGTTOU = 22,
}

/// 实时信号编号
/// 
/// 定义实时信号的编号范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealtimeSignal {
    /// 信号编号
    pub signal_number: i32,
}

impl RealtimeSignal {
    /// 创建实时信号
    pub fn new(signal_number: i32) -> Option<Self> {
        if signal_number >= 32 && signal_number <= 64 {
            Some(Self { signal_number })
        } else {
            None
        }
    }

    /// 获取信号编号
    pub fn number(&self) -> i32 {
        self.signal_number
    }
}

/// 信号类型枚举
/// 
/// 定义信号的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    /// 标准信号
    Standard(StandardSignal),
    /// 实时信号
    Realtime(RealtimeSignal),
}

impl SignalType {
    /// 获取信号编号
    pub fn number(&self) -> i32 {
        match self {
            SignalType::Standard(sig) => *sig as i32,
            SignalType::Realtime(sig) => sig.number(),
        }
    }

    /// 从编号创建信号类型
    pub fn from_number(number: i32) -> Option<Self> {
        if let Some(rt_sig) = RealtimeSignal::new(number) {
            Some(SignalType::Realtime(rt_sig))
        } else if let Ok(standard_sig) = StandardSignal::try_from(number) {
            Some(SignalType::Standard(standard_sig))
        } else {
            None
        }
    }
}

/// 信号处理程序类型
/// 
/// 定义信号处理程序的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalHandlerType {
    /// 默认处理（终止进程）
    Default,
    /// 忽略信号
    Ignore,
    /// 捕获信号
    Catch,
}

/// 信号处理程序信息
/// 
/// 包含信号处理程序的详细信息。
#[derive(Debug, Clone)]
pub struct SignalHandler {
    /// 信号类型
    pub signal_type: SignalType,
    /// 处理程序类型
    pub handler_type: SignalHandlerType,
    /// 处理程序地址
    pub handler_address: u64,
    /// 处理程序标志
    pub handler_flags: u32,
    /// 注册进程ID
    pub creator_pid: u32,
}

/// 信号掩码
/// 
/// 用于阻塞或解除阻塞信号。
#[derive(Debug, Clone, Copy)]
pub struct SignalMask {
    /// 掩码位图
    pub mask: u64,
}

impl SignalMask {
    /// 创建新的信号掩码
    pub fn new() -> Self {
        Self { mask: 0 }
    }

    /// 创建全掩码（阻塞所有信号）
    pub fn full() -> Self {
        Self { mask: u64::MAX }
    }

    /// 创建空掩码（不阻塞任何信号）
    pub fn empty() -> Self {
        Self { mask: 0 }
    }

    /// 添加信号到掩码
    pub fn add_signal(&mut self, signal: SignalType) -> bool {
        let signal_bit = 1u64 << (signal.number() - 1);
        let was_blocked = (self.mask & signal_bit) != 0;
        self.mask |= signal_bit;
        was_blocked
    }

    /// 从掩码移除信号
    pub fn remove_signal(&mut self, signal: SignalType) -> bool {
        let signal_bit = 1u64 << (signal.number() - 1);
        let was_blocked = (self.mask & signal_bit) != 0;
        self.mask &= !signal_bit;
        was_blocked
    }

    /// 检查信号是否被阻塞
    pub fn is_blocked(&self, signal: SignalType) -> bool {
        let signal_bit = 1u64 << (signal.number() - 1);
        (self.mask & signal_bit) != 0
    }

    /// 获取阻塞的信号列表
    pub fn blocked_signals(&self) -> Vec<SignalType> {
        // println removed for no_std compatibility
        for i in 1..=65 {
            if let Some(signal_type) = SignalType::from_number(i) {
                if self.is_blocked(signal_type) {
                    // println removed for no_std compatibility
                }
            }
        }
        blocked
    }
}

/// 信号集合
/// 
/// 表示多个信号的集合。
#[derive(Debug, Clone)]
pub struct SignalSet {
    /// 信号位图
    pub signals: u64,
}

impl SignalSet {
    /// 创建新的信号集合
    pub fn new() -> Self {
        Self { signals: 0 }
    }

    /// 添加信号到集合
    pub fn add_signal(&mut self, signal: SignalType) {
        let signal_bit = 1u64 << (signal.number() - 1);
        self.signals |= signal_bit;
    }

    /// 从集合移除信号
    pub fn remove_signal(&mut self, signal: SignalType) {
        let signal_bit = 1u64 << (signal.number() - 1);
        self.signals &= !signal_bit;
    }

    /// 检查信号是否在集合中
    pub fn contains(&self, signal: SignalType) -> bool {
        let signal_bit = 1u64 << (signal.number() - 1);
        (self.signals & signal_bit) != 0
    }

    /// 获取集合中的信号列表
    pub fn signal_list(&self) -> Vec<SignalType> {
        // println removed for no_std compatibility
        for i in 1..=65 {
            if let Some(signal_type) = SignalType::from_number(i) {
                if self.contains(signal_type) {
                    // println removed for no_std compatibility
                }
            }
        }
        signals
    }
}

/// 信号上下文
/// 
/// 包含信号发送时的上下文信息。
#[derive(Debug, Clone)]
pub struct SignalContext {
    /// 发送进程ID
    pub sender_pid: u32,
    /// 发送进程用户ID
    pub sender_uid: u32,
    /// 信号值
    pub signal_value: i32,
    /// 信号代码
    pub signal_code: i32,
    /// 发送时间戳
    pub timestamp: u64,
    /// 附加数据指针
    pub data_ptr: u64,
    /// 上下文标志
    pub context_flags: u32,
}

/// 信号统计信息
/// 
/// 包含信号处理的统计信息。
#[derive(Debug, Clone)]
pub struct SignalStats {
    /// 发送的信号总数
    pub signals_sent: u64,
    /// 接收的信号总数
    pub signals_received: u64,
    /// 阻塞的信号数
    pub blocked_signals: u32,
    /// 处理的信号数
    pub handled_signals: u64,
    /// 忽略的信号数
    pub ignored_signals: u64,
    /// 挂起的信号数
    pub pending_signals: u32,
    /// 实时信号数
    pub realtime_signals: u32,
}

/// 信号操作参数
/// 
/// 包含信号操作所需的参数。
#[derive(Debug, Clone)]
pub struct SignalOperationParams {
    /// 目标进程ID
    pub target_pid: u32,
    /// 信号类型
    pub signal_type: SignalType,
    /// 信号值
    pub signal_value: i32,
    /// 发送标志
    pub send_flags: u32,
    /// 附加数据
    pub data: Option<Vec<u8>>,
}

impl Default for SignalOperationParams {
    fn default() -> Self {
        Self {
            target_pid: 0,
            signal_type: SignalType::Standard(StandardSignal::SIGTERM),
            signal_value: 0,
            send_flags: 0,
            data: None,
        }
    }
}

/// 信号错误类型
/// 
/// 定义信号模块特有的错误类型。
#[derive(Debug, Clone)]
pub enum SignalError {
    /// 无效信号
    InvalidSignal,
    /// 进程不存在
    ProcessNotFound,
    /// 权限不足
    PermissionDenied,
    /// 无效参数
    InvalidArgument,
    /// 信号队列满
    SignalQueueFull,
    /// 信号已存在
    SignalExists,
    /// 资源不足
    ResourceExhausted,
    /// 操作不被允许
    OperationNotPermitted,
    /// 系统调用不支持
    UnsupportedSyscall,
}

impl SignalError {
    /// 获取错误码
    pub fn error_code(&self) -> i32 {
        match self {
            SignalError::InvalidSignal => -22,
            SignalError::ProcessNotFound => -3,
            SignalError::PermissionDenied => -13,
            SignalError::InvalidArgument => -22,
            SignalError::SignalQueueFull => -105,
            SignalError::SignalExists => -17,
            SignalError::ResourceExhausted => -11,
            SignalError::OperationNotPermitted => -1,
            SignalError::UnsupportedSyscall => -38,
        }
    }

    /// 获取错误描述
    pub fn error_message(&self) -> &str {
        match self {
            SignalError::InvalidSignal => "Invalid signal",
            SignalError::ProcessNotFound => "Process not found",
            SignalError::PermissionDenied => "Permission denied",
            SignalError::InvalidArgument => "Invalid argument",
            SignalError::SignalQueueFull => "Signal queue full",
            SignalError::SignalExists => "Signal already exists",
            SignalError::ResourceExhausted => "Resource exhausted",
            SignalError::OperationNotPermitted => "Operation not permitted",
            SignalError::UnsupportedSyscall => "Unsupported syscall",
        }
    }
}

/// 信号管理特征
/// 
/// 定义信号管理的基本操作接口。
pub trait SignalManager: Send + Sync {
    /// 发送信号
    fn send_signal(&mut self, params: SignalOperationParams) -> Result<(), SignalError>;
    
    /// 发送信号到进程组
    fn send_signal_to_group(&mut self, group_id: u32, signal: SignalType) -> Result<(), SignalError>;
    
    /// 注册信号处理程序
    fn register_handler(&mut self, signal: SignalType, handler: SignalHandler) -> Result<(), SignalError>;
    
    /// 注销信号处理程序
    fn unregister_handler(&mut self, signal: SignalType) -> Result<(), SignalError>;
    
    /// 获取信号处理程序
    fn get_handler(&self, signal: SignalType) -> Option<SignalHandler>;
    
    /// 设置信号掩码
    fn set_signal_mask(&mut self, mask: SignalMask) -> Result<(), SignalError>;
    
    /// 获取信号掩码
    fn get_signal_mask(&self) -> SignalMask;
    
    /// 等待信号
    fn wait_for_signal(&mut self, mask: Option<SignalMask>) -> Result<SignalContext, SignalError>;
    
    /// 挂起执行直到信号
    fn suspend_until_signal(&mut self, mask: Option<SignalMask>) -> Result<SignalContext, SignalError>;
    
    /// 获取信号统计
    fn get_signal_stats(&self) -> SignalStats;
    
    /// 列出所有处理程序
    fn list_handlers(&self) -> Vec<SignalHandler>;
    
    /// 清除挂起的信号
    fn clear_pending_signals(&mut self, signal: Option<SignalType>) -> Result<(), SignalError>;
}