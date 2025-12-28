//! 信号系统调用类型定义
//! 
//! 本模块定义了信号系统调用相关的数据类型，包括：
//! - 信号编号和信号集
//! - 信号处理程序定义
//! - 信号相关常量
//! - 信号操作类型

use alloc::vec::Vec;
use core::fmt;

/// 信号编号类型
pub type SignalNumber = i32;

/// 信号集
/// 
/// 表示一组信号，用于信号掩码和挂起信号集合
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalSet {
    bits: u64,
}

impl SignalSet {
    /// 创建空信号集
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    /// 创建包含所有信号的信号集
    pub fn full() -> Self {
        Self { bits: u64::MAX }
    }

    /// 添加信号到信号集
    pub fn add(&mut self, sig: SignalNumber) {
        if sig > 0 && sig <= 64 {
            self.bits |= 1 << (sig - 1);
        }
    }

    /// 从信号集中移除信号
    pub fn remove(&mut self, sig: SignalNumber) {
        if sig > 0 && sig <= 64 {
            self.bits &= !(1 << (sig - 1));
        }
    }

    /// 检查信号是否在信号集中
    pub fn contains(&self, sig: SignalNumber) -> bool {
        if sig > 0 && sig <= 64 {
            (self.bits & (1 << (sig - 1))) != 0
        } else {
            false
        }
    }

    /// 清空信号集
    pub fn clear(&mut self) {
        self.bits = 0;
    }

    /// 填充信号集（包含所有信号）
    pub fn fill(&mut self) {
        self.bits = u64::MAX;
    }

    /// 检查信号集是否为空
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// 检查信号集是否为满（包含所有信号）
    pub fn is_full(&self) -> bool {
        self.bits == u64::MAX
    }

    /// 获取信号集中包含的信号列表
    pub fn to_vec(&self) -> Vec<SignalNumber> {
        let mut signals = Vec::new();
        for sig in 1..=64 {
            if self.contains(sig) {
                signals.push(sig);
            }
        }
        signals
    }

    /// 从信号列表创建信号集
    pub fn from_vec(signals: &[SignalNumber]) -> Self {
        let mut set = Self::empty();
        for &sig in signals {
            set.add(sig);
        }
        set
    }

    /// 信号集的按位与操作
    pub fn and(&self, other: &SignalSet) -> SignalSet {
        SignalSet {
            bits: self.bits & other.bits,
        }
    }

    /// 信号集的按位或操作
    pub fn or(&self, other: &SignalSet) -> SignalSet {
        SignalSet {
            bits: self.bits | other.bits,
        }
    }

    /// 信号集的按位异或操作
    pub fn xor(&self, other: &SignalSet) -> SignalSet {
        SignalSet {
            bits: self.bits ^ other.bits,
        }
    }

    /// 信号集的按位取反操作
    pub fn not(&self) -> SignalSet {
        SignalSet { bits: !self.bits }
    }

    /// 获取信号集的原始位表示
    pub fn bits(&self) -> u64 {
        self.bits
    }

    /// 从原始位表示创建信号集
    pub fn from_bits(bits: u64) -> Self {
        Self { bits }
    }
}

impl Default for SignalSet {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for SignalSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SignalSet[")?;
        let mut first = true;
        for sig in 1..=64 {
            if self.contains(sig) {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "{}", sig)?;
                first = false;
            }
        }
        write!(f, "]")
    }
}

/// 信号处理程序
/// 
/// 定义了进程接收到信号时的处理方式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalAction {
    /// 使用默认处理方式
    Default,
    /// 忽略信号
    Ignore,
    /// 使用自定义处理函数
    Handler {
        /// 处理函数地址
        handler: usize,
        /// 信号掩码（在执行处理程序时屏蔽的信号）
        mask: SignalSet,
        /// 处理程序标志
        flags: SignalFlags,
    },
}

impl SignalAction {
    /// 创建默认处理程序
    pub fn default() -> Self {
        SignalAction::Default
    }

    /// 创建忽略处理程序
    pub fn ignore() -> Self {
        SignalAction::Ignore
    }

    /// 创建自定义处理程序
    pub fn handler(handler: usize) -> Self {
        SignalAction::Handler {
            handler,
            mask: SignalSet::default(),
            flags: SignalFlags::empty(),
        }
    }

    /// 创建带掩码和标志的自定义处理程序
    pub fn handler_with_mask_and_flags(
        handler: usize,
        mask: SignalSet,
        flags: SignalFlags,
    ) -> Self {
        SignalAction::Handler {
            handler,
            mask,
            flags,
        }
    }

    /// 检查是否是默认处理程序
    pub fn is_default(&self) -> bool {
        matches!(self, SignalAction::Default)
    }

    /// 检查是否是忽略处理程序
    pub fn is_ignore(&self) -> bool {
        matches!(self, SignalAction::Ignore)
    }

    /// 检查是否是自定义处理程序
    pub fn is_handler(&self) -> bool {
        matches!(self, SignalAction::Handler { .. })
    }

    /// 获取处理程序地址（如果是自定义处理程序）
    pub fn handler_address(&self) -> Option<usize> {
        match self {
            SignalAction::Handler { handler, .. } => Some(*handler),
            _ => None,
        }
    }

    /// 获取信号掩码（如果是自定义处理程序）
    pub fn mask(&self) -> Option<&SignalSet> {
        match self {
            SignalAction::Handler { mask, .. } => Some(mask),
            _ => None,
        }
    }

    /// 获取处理程序标志（如果是自定义处理程序）
    pub fn flags(&self) -> Option<&SignalFlags> {
        match self {
            SignalAction::Handler { flags, .. } => Some(flags),
            _ => None,
        }
    }
}

impl Default for SignalAction {
    fn default() -> Self {
        Self::default()
    }
}

/// 信号处理程序标志
/// 
/// 控制信号处理程序的行为
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalFlags {
    bits: u32,
}

impl SignalFlags {
    /// 创建空标志集合
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    /// 创建包含所有标志的集合
    pub fn all() -> Self {
        Self { bits: u32::MAX }
    }

    /// 添加标志
    pub fn add(&mut self, flag: SignalFlag) {
        self.bits |= flag as u32;
    }

    /// 移除标志
    pub fn remove(&mut self, flag: SignalFlag) {
        self.bits &= !(flag as u32);
    }

    /// 检查是否包含标志
    pub fn contains(&self, flag: SignalFlag) -> bool {
        (self.bits & flag as u32) != 0
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// 获取原始位表示
    pub fn bits(&self) -> u32 {
        self.bits
    }

    /// 从原始位表示创建标志集合
    pub fn from_bits(bits: u32) -> Self {
        Self { bits }
    }
}

impl Default for SignalFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// 信号处理程序标志位
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalFlag {
    /// 在信号处理程序执行期间，不自动屏蔽该信号
    SA_NODEFER = 0x40000000,
    /// 使用替代信号栈
    SA_ONSTACK = 0x08000000,
    /// 重启被中断的系统调用
    SA_RESTART = 0x10000000,
    /// 重置处理程序为默认值
    SA_RESETHAND = 0x80000000,
    /// 提供关于信号的附加信息
    SA_SIGINFO = 0x00000004,
}

/// 信号掩码操作类型
/// 
/// 用于sigprocmask系统调用，指定如何修改信号掩码
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigmaskHow {
    /// 阻塞信号集
    SIG_BLOCK = 0,
    /// 解除阻塞信号集
    SIG_UNBLOCK = 1,
    /// 设置信号集
    SIG_SETMASK = 2,
}

/// 信号上下文
/// 
/// 保存信号处理时的进程上下文信息
#[derive(Debug, Clone)]
pub struct SignalContext {
    /// 信号编号
    pub signal: SignalNumber,
    /// 错误码
    pub error_code: i32,
    /// 信号码
    pub code: i32,
    /// 发送信号的进程ID
    pub pid: u32,
    /// 发送信号的用户ID
    pub uid: u32,
    /// 信号值
    pub value: usize,
    /// 信号处理程序返回地址
    pub return_address: usize,
    /// 寄存器状态
    pub registers: SignalRegisters,
}

/// 信号寄存器状态
/// 
/// 保存信号处理时的CPU寄存器状态
#[derive(Debug, Clone)]
pub struct SignalRegisters {
    /// 通用寄存器
    pub general: [u64; 16],
    /// 程序计数器
    pub pc: u64,
    /// 栈指针
    pub sp: u64,
    /// 状态寄存器
    pub status: u64,
}

impl Default for SignalRegisters {
    fn default() -> Self {
        Self {
            general: [0; 16],
            pc: 0,
            sp: 0,
            status: 0,
        }
    }
}

/// 信号栈
/// 
/// 用于信号处理的替代栈
#[derive(Debug, Clone)]
pub struct SignalStack {
    /// 栈底地址
    pub base: usize,
    /// 栈大小
    pub size: usize,
    /// 栈标志
    pub flags: SignalStackFlags,
}

/// 信号栈标志
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalStackFlag {
    /// 禁用信号栈
    SS_DISABLE = 2,
    /// 在信号栈上
    SS_ONSTACK = 1,
}

/// 信号栈标志集合
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalStackFlags {
    bits: u32,
}

impl SignalStackFlags {
    /// 创建空标志集合
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    /// 添加标志
    pub fn add(&mut self, flag: SignalStackFlag) {
        self.bits |= flag as u32;
    }

    /// 移除标志
    pub fn remove(&mut self, flag: SignalStackFlag) {
        self.bits &= !(flag as u32);
    }

    /// 检查是否包含标志
    pub fn contains(&self, flag: SignalStackFlag) -> bool {
        (self.bits & flag as u32) != 0
    }

    /// 获取原始位表示
    pub fn bits(&self) -> u32 {
        self.bits
    }

    /// 从原始位表示创建标志集合
    pub fn from_bits(bits: u32) -> Self {
        Self { bits }
    }
}

impl Default for SignalStackFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// 标准信号定义
pub mod signals {
    use super::SignalNumber;

    /// 终止信号
    pub const SIGTERM: SignalNumber = 15;
    /// 中断信号
    pub const SIGINT: SignalNumber = 2;
    /// 退出信号
    pub const SIGQUIT: SignalNumber = 3;
    /// 非法指令信号
    pub const SIGILL: SignalNumber = 4;
    /// 总线错误信号
    pub const SIGBUS: SignalNumber = 7;
    /// 浮点异常信号
    pub const SIGFPE: SignalNumber = 8;
    /// 段错误信号
    pub const SIGSEGV: SignalNumber = 11;
    /// 管道破裂信号
    pub const SIGPIPE: SignalNumber = 13;
    /// 时钟信号
    pub const SIGALRM: SignalNumber = 14;
    /// 终止信号（不可捕获）
    pub const SIGKILL: SignalNumber = 9;
    /// 停止信号（不可捕获）
    pub const SIGSTOP: SignalNumber = 19;
    /// 子进程状态改变信号
    pub const SIGCHLD: SignalNumber = 20;
    /// 用户自定义信号1
    pub const SIGUSR1: SignalNumber = 10;
    /// 用户自定义信号2
    pub const SIGUSR2: SignalNumber = 12;
    /// 终端断开信号
    pub const SIGHUP: SignalNumber = 1;
    /// 终端停止信号
    pub const SIGTSTP: SignalNumber = 18;
    /// 继续执行信号
    pub const SIGCONT: SignalNumber = 18;
    /// 后台进程读信号
    pub const SIGTTIN: SignalNumber = 21;
    /// 后台进程写信号
    pub const SIGTTOU: SignalNumber = 22;

    /// 获取信号名称
    pub fn signal_name(sig: SignalNumber) -> &'static str {
        match sig {
            SIGTERM => "SIGTERM",
            SIGINT => "SIGINT",
            SIGQUIT => "SIGQUIT",
            SIGILL => "SIGILL",
            SIGBUS => "SIGBUS",
            SIGFPE => "SIGFPE",
            SIGSEGV => "SIGSEGV",
            SIGPIPE => "SIGPIPE",
            SIGALRM => "SIGALRM",
            SIGKILL => "SIGKILL",
            SIGSTOP => "SIGSTOP",
            SIGCHLD => "SIGCHLD",
            SIGUSR1 => "SIGUSR1",
            SIGUSR2 => "SIGUSR2",
            SIGHUP => "SIGHUP",
            SIGTSTP => "SIGTSTP",
            SIGCONT => "SIGCONT",
            SIGTTIN => "SIGTTIN",
            SIGTTOU => "SIGTTOU",
            _ => "UNKNOWN",
        }
    }

    /// 检查信号是否是标准信号
    pub fn is_standard_signal(sig: SignalNumber) -> bool {
        sig >= 1 && sig <= 31
    }

    /// 检查信号是否是实时信号
    pub fn is_realtime_signal(sig: SignalNumber) -> bool {
        sig >= 32 && sig <= 64
    }

    /// 检查信号是否可以被捕获
    pub fn is_catchable(sig: SignalNumber) -> bool {
        sig != SIGKILL && sig != SIGSTOP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_set() {
        let mut set = SignalSet::empty();
        
        // 测试添加和检查信号
        assert!(!set.contains(1));
        set.add(1);
        assert!(set.contains(1));
        
        // 测试移除信号
        set.remove(1);
        assert!(!set.contains(1));
        
        // 测试空和满
        assert!(set.is_empty());
        set.fill();
        assert!(set.is_full());
        
        // 测试转换为向量
        let signals = set.to_vec();
        assert_eq!(signals.len(), 64);
        
        // 测试从向量创建
        let set2 = SignalSet::from_vec(&[1, 2, 3]);
        assert!(set2.contains(1));
        assert!(set2.contains(2));
        assert!(set2.contains(3));
        assert!(!set2.contains(4));
    }

    #[test]
    fn test_signal_action() {
        // 测试默认处理程序
        let action = SignalAction::default();
        assert!(action.is_default());
        assert!(!action.is_ignore());
        assert!(!action.is_handler());
        
        // 测试忽略处理程序
        let action = SignalAction::ignore();
        assert!(action.is_ignore());
        assert!(!action.is_default());
        assert!(!action.is_handler());
        
        // 测试自定义处理程序
        let action = SignalAction::handler(0x12345678);
        assert!(action.is_handler());
        assert!(!action.is_default());
        assert!(!action.is_ignore());
        assert_eq!(action.handler_address(), Some(0x12345678));
    }

    #[test]
    fn test_signal_flags() {
        let mut flags = SignalFlags::empty();
        
        // 测试添加和检查标志
        assert!(!flags.contains(SignalFlag::SA_RESTART));
        flags.add(SignalFlag::SA_RESTART);
        assert!(flags.contains(SignalFlag::SA_RESTART));
        
        // 测试移除标志
        flags.remove(SignalFlag::SA_RESTART);
        assert!(!flags.contains(SignalFlag::SA_RESTART));
        
        // 测试空
        assert!(flags.is_empty());
    }

    #[test]
    fn test_signal_stack_flags() {
        let mut flags = SignalStackFlags::empty();
        
        // 测试添加和检查标志
        assert!(!flags.contains(SignalStackFlag::SS_DISABLE));
        flags.add(SignalStackFlag::SS_DISABLE);
        assert!(flags.contains(SignalStackFlag::SS_DISABLE));
        
        // 测试移除标志
        flags.remove(SignalStackFlag::SS_DISABLE);
        assert!(!flags.contains(SignalStackFlag::SS_DISABLE));
    }

    #[test]
    fn test_signals() {
        use signals::*;
        
        // 测试信号名称
        assert_eq!(signal_name(SIGTERM), "SIGTERM");
        assert_eq!(signal_name(999), "UNKNOWN");
        
        // 测试信号类型检查
        assert!(is_standard_signal(SIGINT));
        assert!(!is_standard_signal(32));
        assert!(is_realtime_signal(32));
        assert!(!is_realtime_signal(SIGINT));
        
        // 测试可捕获性
        assert!(is_catchable(SIGINT));
        assert!(!is_catchable(SIGKILL));
        assert!(!is_catchable(SIGSTOP));
    }
}