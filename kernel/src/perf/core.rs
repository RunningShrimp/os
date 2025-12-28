#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! 性能核心模块
//! 
//! 提供统一的系统调用性能统计功能

use alloc::collections::BTreeMap;
use spin::Mutex;

/// 系统调用类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallType {
    Read = 0,
    Write = 1,
    Open = 2,
    Close = 3,
    Stat = 4,
    Fstat = 5,
    Lstat = 6,
    Poll = 7,
    Lseek = 8,
    Mmap = 9,
    Mprotect = 10,
    Munmap = 11,
    Brk = 12,
    RtSigaction = 13,
    Ioctl = 14,
    Pread64 = 15,
    Pwrite64 = 16,
    Readv = 17,
    Writev = 18,
    Access = 19,
    Pipe = 20,
    Select = 21,
    SchedYield = 22,
    Mremap = 23,
    Msync = 24,
    Mincore = 25,
    Madvise = 26,
    Dup = 27,
    Dup2 = 28,
    Pause = 29,
    Nanosleep = 30,
    Getitimer = 31,
    Alarm = 32,
    Setitimer = 33,
    Getpid = 34,
    Sendfile = 35,
    Socket = 36,
    Connect = 37,
    Accept = 38,
    Sendto = 39,
    Recvfrom = 40,
    Sendmsg = 41,
    Recvmsg = 42,
    Shutdown = 43,
    Bind = 44,
    Listen = 45,
    Getsockname = 46,
    Getpeername = 47,
    Socketpair = 48,
    Setsockopt = 49,
    Getsockopt = 50,
    Clone = 51,
    Fork = 52,
    Vfork = 53,
    Execve = 54,
    Exit = 55,
    Wait4 = 56,
    Kill = 57,
    Unlink = 58,
    Other(u32),
}

impl From<u32> for SyscallType {
    fn from(value: u32) -> Self {
        match value {
            0 => SyscallType::Read,
            1 => SyscallType::Write,
            2 => SyscallType::Open,
            3 => SyscallType::Close,
            4 => SyscallType::Stat,
            5 => SyscallType::Fstat,
            6 => SyscallType::Lstat,
            7 => SyscallType::Poll,
            8 => SyscallType::Lseek,
            9 => SyscallType::Mmap,
            10 => SyscallType::Mprotect,
            11 => SyscallType::Munmap,
            12 => SyscallType::Brk,
            13 => SyscallType::RtSigaction,
            14 => SyscallType::Ioctl,
            15 => SyscallType::Pread64,
            16 => SyscallType::Pwrite64,
            17 => SyscallType::Readv,
            18 => SyscallType::Writev,
            19 => SyscallType::Access,
            20 => SyscallType::Pipe,
            21 => SyscallType::Select,
            22 => SyscallType::SchedYield,
            23 => SyscallType::Mremap,
            24 => SyscallType::Msync,
            25 => SyscallType::Mincore,
            26 => SyscallType::Madvise,
            27 => SyscallType::Dup,
            28 => SyscallType::Dup2,
            29 => SyscallType::Pause,
            30 => SyscallType::Nanosleep,
            31 => SyscallType::Getitimer,
            32 => SyscallType::Alarm,
            33 => SyscallType::Setitimer,
            34 => SyscallType::Getpid,
            35 => SyscallType::Sendfile,
            36 => SyscallType::Socket,
            37 => SyscallType::Connect,
            38 => SyscallType::Accept,
            39 => SyscallType::Sendto,
            40 => SyscallType::Recvfrom,
            41 => SyscallType::Sendmsg,
            42 => SyscallType::Recvmsg,
            43 => SyscallType::Shutdown,
            44 => SyscallType::Bind,
            45 => SyscallType::Listen,
            46 => SyscallType::Getsockname,
            47 => SyscallType::Getpeername,
            48 => SyscallType::Socketpair,
            49 => SyscallType::Setsockopt,
            50 => SyscallType::Getsockopt,
            51 => SyscallType::Clone,
            52 => SyscallType::Fork,
            53 => SyscallType::Vfork,
            54 => SyscallType::Execve,
            55 => SyscallType::Exit,
            56 => SyscallType::Wait4,
            57 => SyscallType::Kill,
            58 => SyscallType::Unlink,
            _ => SyscallType::Other(value),
        }
    }
}

/// 系统调用统计
#[derive(Debug, Clone)]
pub struct SyscallStats {
    /// 系统调用号
    pub syscall_number: u32,
    /// 调用次数
    pub call_count: u64,
    /// 成功次数
    pub success_count: u64,
    /// 失败次数
    pub error_count: u64,
    /// 总耗时（纳秒）
    pub total_time_ns: u64,
}

/// 统一的系统调用统计
#[derive(Debug, Clone)]
pub struct UnifiedSyscallStats {
    /// 总系统调用数
    pub total_syscalls: u64,
    /// 成功的系统调用数
    pub successful_syscalls: u64,
    /// 失败的系统调用数
    pub failed_syscalls: u64,
    /// 各系统调用的统计
    pub syscall_stats: BTreeMap<u32, SyscallStats>,
}

impl Default for UnifiedSyscallStats {
    fn default() -> Self {
        Self {
            total_syscalls: 0,
            successful_syscalls: 0,
            failed_syscalls: 0,
            syscall_stats: BTreeMap::new(),
        }
    }
}

impl UnifiedSyscallStats {
    /// 创建新的系统调用统计
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 记录系统调用
    pub fn record_syscall(&mut self, syscall_number: u32, success: bool, time_ns: u64) {
        self.total_syscalls += 1;
        if success {
            self.successful_syscalls += 1;
        } else {
            self.failed_syscalls += 1;
        }
        
        let stats = self.syscall_stats.entry(syscall_number).or_insert_with(|| SyscallStats {
            syscall_number,
            call_count: 0,
            success_count: 0,
            error_count: 0,
            total_time_ns: 0,
        });
        
        stats.call_count += 1;
        if success {
            stats.success_count += 1;
        } else {
            stats.error_count += 1;
        }
        stats.total_time_ns += time_ns;
    }
    
    /// 清空统计
    pub fn clear(&mut self) {
        self.total_syscalls = 0;
        self.successful_syscalls = 0;
        self.failed_syscalls = 0;
        self.syscall_stats.clear();
    }
}

/// 系统调用统计快照
#[derive(Debug, Clone)]
pub struct SyscallStatsSnapshot {
    /// 时间戳
    pub timestamp: u64,
    /// 系统调用统计
    pub stats: UnifiedSyscallStats,
}

impl SyscallStatsSnapshot {
    /// 创建新的快照
    pub fn new(stats: UnifiedSyscallStats) -> Self {
        Self {
            timestamp: crate::subsystems::time::hrtime_nanos(),
            stats,
        }
    }
}

/// 全局系统调用统计
static mut GLOBAL_SYSCALL_STATS: Option<spin::Mutex<UnifiedSyscallStats>> = None;

/// 初始化系统调用统计
pub fn init_syscall_stats() {
    unsafe {
        if GLOBAL_SYSCALL_STATS.is_none() {
            GLOBAL_SYSCALL_STATS = Some(spin::Mutex::new(UnifiedSyscallStats::new()));
        }
    }
}

/// 获取系统调用统计
pub fn get_syscall_stats() -> &'static spin::Mutex<UnifiedSyscallStats> {
    unsafe {
        if GLOBAL_SYSCALL_STATS.is_none() {
            init_syscall_stats();
        }
        GLOBAL_SYSCALL_STATS.as_ref().unwrap()
    }
}

/// 记录系统调用
pub fn record_syscall(syscall_number: u32, success: bool, time_ns: u64) {
    let stats = get_syscall_stats();
    let mut stats = stats.lock();
    stats.record_syscall(syscall_number, success, time_ns);
}

/// 获取系统调用统计快照
pub fn get_snapshot() -> SyscallStatsSnapshot {
    let stats = get_syscall_stats();
    let stats = stats.lock();
    SyscallStatsSnapshot::new(stats.clone())
}

/// 重置系统调用统计
pub fn reset_syscall_stats() {
    let stats = get_syscall_stats();
    let mut stats = stats.lock();
    stats.clear();
}
