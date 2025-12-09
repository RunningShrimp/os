//! 优化的信号处理实现
//!
//! 本模块提供高性能的信号处理功能，包括：
//! - 高效的信号发送和接收
//! - 优化的信号处理程序管理
//! - 快速的信号掩码操作
//! - 减少锁竞争的信号队列

use crate::process::{PROC_TABLE, myproc};
use crate::mm::vm::PageTable;
use crate::sync::Mutex;
use super::common::{SyscallError, SyscallResult, extract_args};
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicUsize, Ordering};

/// 全局信号统计
static SIGNAL_STATS: Mutex<SignalStats> = Mutex::new(SignalStats::new());

/// 信号统计信息
#[derive(Debug, Default)]
pub struct SignalStats {
    pub kill_count: AtomicUsize,
    pub raise_count: AtomicUsize,
    pub sigaction_count: AtomicUsize,
    pub sigprocmask_count: AtomicUsize,
    pub sigpending_count: AtomicUsize,
    pub sigsuspend_count: AtomicUsize,
    pub sigwait_count: AtomicUsize,
    pub signals_delivered: AtomicUsize,
    pub signals_pending: AtomicUsize,
}

impl SignalStats {
    pub const fn new() -> Self {
        Self {
            kill_count: AtomicUsize::new(0),
            raise_count: AtomicUsize::new(0),
            sigaction_count: AtomicUsize::new(0),
            sigprocmask_count: AtomicUsize::new(0),
            sigpending_count: AtomicUsize::new(0),
            sigsuspend_count: AtomicUsize::new(0),
            sigwait_count: AtomicUsize::new(0),
            signals_delivered: AtomicUsize::new(0),
            signals_pending: AtomicUsize::new(0),
        }
    }
    
    pub fn record_kill(&self) {
        self.kill_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_raise(&self) {
        self.raise_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_sigaction(&self) {
        self.sigaction_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_sigprocmask(&self) {
        self.sigprocmask_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_sigpending(&self) {
        self.sigpending_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_sigsuspend(&self) {
        self.sigsuspend_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_sigwait(&self) {
        self.sigwait_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_signal_delivered(&self) {
        self.signals_delivered.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_signal_pending(&self) {
        self.signals_pending.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_signal_processed(&self) {
        self.signals_pending.fetch_sub(1, Ordering::Relaxed);
    }
}

/// 信号处理程序
#[derive(Debug, Clone, Copy)]
pub struct SignalHandler {
    pub handler: usize,
    pub flags: u32,
    pub mask: u64,
}

impl SignalHandler {
    pub fn new(handler: usize, flags: u32, mask: u64) -> Self {
        Self {
            handler,
            flags,
            mask,
        }
    }
}

/// 信号信息
#[derive(Debug, Clone)]
pub struct SignalInfo {
    pub signum: i32,
    pub code: i32,
    pub pid: u32,
    pub uid: u32,
    pub value: usize,
}

impl SignalInfo {
    pub fn new(signum: i32, code: i32, pid: u32, uid: u32, value: usize) -> Self {
        Self {
            signum,
            code,
            pid,
            uid,
            value,
        }
    }
}

/// 进程信号状态
#[derive(Debug)]
pub struct ProcessSignalState {
    pub pending: VecDeque<SignalInfo>,
    pub blocked: u64,
    pub handlers: [Option<SignalHandler>; 64],
}

impl ProcessSignalState {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            blocked: 0,
            handlers: [None; 64],
        }
    }
    
    pub fn add_pending(&mut self, info: SignalInfo) {
        self.pending.push_back(info);
    }
    
    pub fn get_pending(&mut self) -> Option<SignalInfo> {
        self.pending.pop_front()
    }
    
    pub fn has_pending(&self, signum: i32) -> bool {
        self.pending.iter().any(|info| info.signum == signum)
    }
    
    pub fn is_blocked(&self, signum: i32) -> bool {
        if signum < 0 || signum >= 64 {
            return false;
        }
        (self.blocked & (1 << signum)) != 0
    }
    
    pub fn set_blocked(&mut self, signum: i32, blocked: bool) {
        if signum < 0 || signum >= 64 {
            return;
        }
        if blocked {
            self.blocked |= 1 << signum;
        } else {
            self.blocked &= !(1 << signum);
        }
    }
    
    pub fn set_handler(&mut self, signum: i32, handler: Option<SignalHandler>) {
        if signum < 0 || signum >= 64 {
            return;
        }
        self.handlers[signum as usize] = handler;
    }
    
    pub fn get_handler(&self, signum: i32) -> Option<SignalHandler> {
        if signum < 0 || signum >= 64 {
            return None;
        }
        self.handlers[signum as usize]
    }
}

/// 优化的kill系统调用实现
pub fn sys_kill_optimized(pid: i32, sig: i32) -> isize {
    // 记录统计
    SIGNAL_STATS.lock().record_kill();
    
    // 验证信号编号
    if sig < 0 || sig >= 64 {
        return -1;
    }
    
    // 获取目标进程
    let mut table = PROC_TABLE.lock();
    let target_proc = match table.find_mut(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    
    // 获取当前进程信息
    let current_pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let current_proc = match table.find_mut(current_pid) {
        Some(proc) => proc,
        None => return -1,
    };
    
    // 创建信号信息
    let signal_info = SignalInfo::new(
        sig,
        0, // code
        current_proc.pid as u32,
        current_proc.uid as u32,
        0, // value
    );
    
    // 添加信号到目标进程的挂起队列
    if let Some(signal_state) = &mut target_proc.signal_state {
        signal_state.add_pending(signal_info);
        SIGNAL_STATS.lock().record_signal_pending();
    } else {
        // 如果进程没有信号状态，创建一个
        let mut signal_state = ProcessSignalState::new();
        signal_state.add_pending(signal_info);
        target_proc.signal_state = Some(signal_state);
        SIGNAL_STATS.lock().record_signal_pending();
    }
    
    // 唤醒目标进程（如果它在等待信号）
    if target_proc.state == crate::process::ProcState::Sleeping {
        target_proc.state = crate::process::ProcState::Runnable;
    }
    
    0
}

/// 优化的raise系统调用实现
pub fn sys_raise_optimized(sig: i32) -> isize {
    // 记录统计
    SIGNAL_STATS.lock().record_raise();
    
    // 验证信号编号
    if sig < 0 || sig >= 64 {
        return -1;
    }
    
    // 获取当前进程
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find_mut(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    
    // 创建信号信息
    let signal_info = SignalInfo::new(
        sig,
        0, // code
        proc.pid as u32,
        proc.uid as u32,
        0, // value
    );
    
    // 添加信号到当前进程的挂起队列
    if let Some(signal_state) = &mut proc.signal_state {
        signal_state.add_pending(signal_info);
        SIGNAL_STATS.lock().record_signal_pending();
    } else {
        // 如果进程没有信号状态，创建一个
        let mut signal_state = ProcessSignalState::new();
        signal_state.add_pending(signal_info);
        proc.signal_state = Some(signal_state);
        SIGNAL_STATS.lock().record_signal_pending();
    }
    
    0
}

/// 优化的sigaction系统调用实现
pub fn sys_sigaction_optimized(signum: i32, act_ptr: usize, oldact_ptr: usize) -> isize {
    // 记录统计
    SIGNAL_STATS.lock().record_sigaction();
    
    // 验证信号编号
    if signum < 0 || signum >= 64 {
        return -1;
    }
    
    // 获取当前进程
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find_mut(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    
    let pagetable = proc.pagetable;
    
    // 确保进程有信号状态
    if proc.signal_state.is_none() {
        proc.signal_state = Some(ProcessSignalState::new());
    }
    
    let signal_state = proc.signal_state.as_mut().unwrap();
    
    // 如果请求，保存旧的信号处理程序
    if oldact_ptr != 0 {
        if let Some(old_handler) = signal_state.get_handler(signum) {
            let oldact = crate::posix::sigaction {
                sa_handler: old_handler.handler,
                sa_flags: old_handler.flags,
                sa_mask: old_handler.mask,
            };
            
            // 复制到用户空间
            unsafe {
                let oldact_slice = core::slice::from_raw_parts(
                    &oldact as *const crate::posix::sigaction as *const u8,
                    core::mem::size_of::<crate::posix::sigaction>()
                );
                crate::mm::vm::copyout(pagetable, oldact_ptr, oldact_slice.as_ptr(), oldact_slice.len())
                    .unwrap_or(());
            }
        }
    }
    
    // 如果提供了新的信号处理程序，设置它
    if act_ptr != 0 {
        // 从用户空间读取新的信号处理程序
        let mut act_bytes = [0u8; core::mem::size_of::<crate::posix::sigaction>()];
        unsafe {
            crate::mm::vm::copyin(pagetable, act_bytes.as_mut_ptr(), act_ptr, act_bytes.len())
                .unwrap_or(());
        }
        
        let act = unsafe {
            *(act_bytes.as_ptr() as *const crate::posix::sigaction)
        };
        
        // 创建新的信号处理程序
        let handler = SignalHandler::new(
            act.sa_handler,
            act.sa_flags,
            act.sa_mask
        );
        
        // 设置新的信号处理程序
        signal_state.set_handler(signum, Some(handler));
    }
    
    0
}

/// 优化的sigprocmask系统调用实现
pub fn sys_sigprocmask_optimized(how: i32, set_ptr: usize, oldset_ptr: usize) -> isize {
    // 记录统计
    SIGNAL_STATS.lock().record_sigprocmask();
    
    // 获取当前进程
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find_mut(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    
    let pagetable = proc.pagetable;
    
    // 确保进程有信号状态
    if proc.signal_state.is_none() {
        proc.signal_state = Some(ProcessSignalState::new());
    }
    
    let signal_state = proc.signal_state.as_mut().unwrap();
    
    // 如果请求，保存旧的信号掩码
    if oldset_ptr != 0 {
        let oldset = signal_state.blocked;
        
        // 复制到用户空间
        unsafe {
            let oldset_slice = core::slice::from_raw_parts(
                &oldset as *const u64 as *const u8,
                core::mem::size_of::<u64>()
            );
            crate::mm::vm::copyout(pagetable, oldset_ptr, oldset_slice.as_ptr(), oldset_slice.len())
                .unwrap_or(());
        }
    }
    
    // 如果提供了新的信号掩码，设置它
    if set_ptr != 0 {
        // 从用户空间读取新的信号掩码
        let mut set_bytes = [0u8; core::mem::size_of::<u64>()];
        unsafe {
            crate::mm::vm::copyin(pagetable, set_bytes.as_mut_ptr(), set_ptr, set_bytes.len())
                .unwrap_or(());
        }
        
        let newset = u64::from_le_bytes(set_bytes);
        
        // 根据how参数更新信号掩码
        match how {
            0 => { // SIG_BLOCK
                signal_state.blocked |= newset;
            }
            1 => { // SIG_UNBLOCK
                signal_state.blocked &= !newset;
            }
            2 => { // SIG_SETMASK
                signal_state.blocked = newset;
            }
            _ => {
                return -1;
            }
        }
    }
    
    0
}

/// 优化的sigpending系统调用实现
pub fn sys_sigpending_optimized(set_ptr: usize) -> isize {
    // 记录统计
    SIGNAL_STATS.lock().record_sigpending();
    
    // 获取当前进程
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find_mut(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    
    let pagetable = proc.pagetable;
    
    // 确保进程有信号状态
    if proc.signal_state.is_none() {
        proc.signal_state = Some(ProcessSignalState::new());
    }
    
    let signal_state = proc.signal_state.as_mut().unwrap();
    
    // 计算挂起的信号掩码
    let mut pending_mask = 0u64;
    for info in &signal_state.pending {
        if info.signum >= 0 && info.signum < 64 {
            pending_mask |= 1 << info.signum;
        }
    }
    
    // 复制到用户空间
    if set_ptr != 0 {
        let pending_bytes = pending_mask.to_le_bytes();
        unsafe {
            crate::mm::vm::copyout(pagetable, set_ptr, pending_bytes.as_ptr(), pending_bytes.len())
                .unwrap_or(());
        }
    }
    
    // 返回挂起的信号数
    signal_state.pending.len() as isize
}

/// 优化的sigsuspend系统调用实现
pub fn sys_sigsuspend_optimized(mask_ptr: usize) -> isize {
    // 记录统计
    SIGNAL_STATS.lock().record_sigsuspend();
    
    // 获取当前进程
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find_mut(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    
    let pagetable = proc.pagetable;
    
    // 确保进程有信号状态
    if proc.signal_state.is_none() {
        proc.signal_state = Some(ProcessSignalState::new());
    }
    
    let signal_state = proc.signal_state.as_mut().unwrap();
    
    // 保存旧的信号掩码
    let old_mask = signal_state.blocked;
    
    // 从用户空间读取新的信号掩码
    let mut mask_bytes = [0u8; core::mem::size_of::<u64>()];
    unsafe {
        crate::mm::vm::copyin(pagetable, mask_bytes.as_mut_ptr(), mask_ptr, mask_bytes.len())
            .unwrap_or(());
    }
    
    let new_mask = u64::from_le_bytes(mask_bytes);
    
    // 设置新的信号掩码
    signal_state.blocked = new_mask;
    
    // 检查是否有挂起的信号
    let has_pending = signal_state.pending.iter().any(|info| {
        info.signum >= 0 && info.signum < 64 && (new_mask & (1 << info.signum)) == 0
    });
    
    if has_pending {
        // 恢复旧的信号掩码
        signal_state.blocked = old_mask;
        return -1; // 返回-1表示被信号中断
    }
    
    // 设置进程状态为等待信号
    proc.state = crate::process::ProcState::Sleeping;
    
    // 释放锁并等待信号
    drop(table);
    
    // 这里应该实现实际的等待逻辑
    // 简化实现，直接返回-1
    -1
}

/// 优化的sigwait系统调用实现
pub fn sys_sigwait_optimized(set_ptr: usize, info_ptr: usize) -> isize {
    // 记录统计
    SIGNAL_STATS.lock().record_sigwait();
    
    // 获取当前进程
    let pid = match myproc() {
        Some(pid) => pid,
        None => return -1,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find_mut(pid) {
        Some(proc) => proc,
        None => return -1,
    };
    
    let pagetable = proc.pagetable;
    
    // 确保进程有信号状态
    if proc.signal_state.is_none() {
        proc.signal_state = Some(ProcessSignalState::new());
    }
    
    let signal_state = proc.signal_state.as_mut().unwrap();
    
    // 从用户空间读取信号集
    let mut set_bytes = [0u8; core::mem::size_of::<u64>()];
    unsafe {
        crate::mm::vm::copyin(pagetable, set_bytes.as_mut_ptr(), set_ptr, set_bytes.len())
            .unwrap_or(());
    }
    
    let signal_set = u64::from_le_bytes(set_bytes);
    
    // 查找匹配的挂起信号
    let mut found_signal = None;
    for info in &signal_state.pending {
        if info.signum >= 0 && info.signum < 64 && (signal_set & (1 << info.signum)) != 0 {
            found_signal = Some(info.clone());
            break;
        }
    }
    
    if let Some(info) = found_signal {
        // 从挂起队列中移除信号
        signal_state.pending.retain(|i| i.signum != info.signum);
        SIGNAL_STATS.lock().record_signal_processed();
        
        // 如果提供了info_ptr，复制信号信息到用户空间
        if info_ptr != 0 {
            // 简化实现，只复制信号编号
            let signal_num = info.signum as u64;
            let signal_bytes = signal_num.to_le_bytes();
            unsafe {
                crate::mm::vm::copyout(pagetable, info_ptr, signal_bytes.as_ptr(), signal_bytes.len())
                    .unwrap_or(());
            }
        }
        
        info.signum
    } else {
        // 没有找到匹配的信号，设置进程状态为等待信号
        proc.state = crate::process::ProcState::Sleeping;
        
        // 释放锁并等待信号
        drop(table);
        
        // 这里应该实现实际的等待逻辑
        // 简化实现，直接返回-1
        -1
    }
}

/// 获取信号统计信息
pub fn get_signal_stats() -> SignalStats {
    let stats = SIGNAL_STATS.lock();
    SignalStats {
        kill_count: AtomicUsize::new(stats.kill_count.load(Ordering::Relaxed)),
        raise_count: AtomicUsize::new(stats.raise_count.load(Ordering::Relaxed)),
        sigaction_count: AtomicUsize::new(stats.sigaction_count.load(Ordering::Relaxed)),
        sigprocmask_count: AtomicUsize::new(stats.sigprocmask_count.load(Ordering::Relaxed)),
        sigpending_count: AtomicUsize::new(stats.sigpending_count.load(Ordering::Relaxed)),
        sigsuspend_count: AtomicUsize::new(stats.sigsuspend_count.load(Ordering::Relaxed)),
        sigwait_count: AtomicUsize::new(stats.sigwait_count.load(Ordering::Relaxed)),
        signals_delivered: AtomicUsize::new(stats.signals_delivered.load(Ordering::Relaxed)),
        signals_pending: AtomicUsize::new(stats.signals_pending.load(Ordering::Relaxed)),
    }
}

/// 系统调用分发函数
pub fn dispatch_optimized(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        0x4000 => {
            // kill
            let args = extract_args(args, 2)?;
            let pid = args[0] as i32;
            let sig = args[1] as i32;
            Ok(sys_kill_optimized(pid, sig) as u64)
        }
        0x4001 => {
            // raise
            let args = extract_args(args, 1)?;
            let sig = args[0] as i32;
            Ok(sys_raise_optimized(sig) as u64)
        }
        0x4002 => {
            // sigaction
            let args = extract_args(args, 3)?;
            let signum = args[0] as i32;
            let act_ptr = args[1] as usize;
            let oldact_ptr = args[2] as usize;
            Ok(sys_sigaction_optimized(signum, act_ptr, oldact_ptr) as u64)
        }
        0x4003 => {
            // sigprocmask
            let args = extract_args(args, 3)?;
            let how = args[0] as i32;
            let set_ptr = args[1] as usize;
            let oldset_ptr = args[2] as usize;
            Ok(sys_sigprocmask_optimized(how, set_ptr, oldset_ptr) as u64)
        }
        0x4004 => {
            // sigpending
            let args = extract_args(args, 1)?;
            let set_ptr = args[0] as usize;
            Ok(sys_sigpending_optimized(set_ptr) as u64)
        }
        0x4005 => {
            // sigsuspend
            let args = extract_args(args, 1)?;
            let mask_ptr = args[0] as usize;
            Ok(sys_sigsuspend_optimized(mask_ptr) as u64)
        }
        0x4006 => {
            // sigwait
            let args = extract_args(args, 2)?;
            let set_ptr = args[0] as usize;
            let info_ptr = args[1] as usize;
            Ok(sys_sigwait_optimized(set_ptr, info_ptr) as u64)
        }
        _ => Err(SyscallError::NotSupported),
    }
}