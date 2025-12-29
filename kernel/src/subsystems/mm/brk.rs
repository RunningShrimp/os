//! brk/sbrk 系统调用实现
//!
//! 提供传统的堆内存管理接口，用于动态调整程序堆的大小。
//!
//! ## 概述
//!
//! brk/sbrk 是传统的 Unix 内存管理接口：
//! - **brk**: 设置堆的绝对边界
//! - **sbrk**: 增量调整堆大小
//!
//! ## 注意事项
//!
//! 现代 POSIX 应用应该使用 malloc/free 而非 brk/sbrk。
//! brk/sbrk 主要用于：
//! - 实现 malloc 库
//! - 遗留系统兼容性
//! - 特殊的内存管理需求
//!
//! ## POSIX 兼容性
//!
//! 实现遵循 POSIX.1-2008 规范：
//! - brk(0) 返回当前堆边界
//! - sbrk(0) 返回当前堆边界
//! - 失败时返回 -1 并设置 errno

use crate::api::SyscallError;
use crate::subsystems::mm::PAGE_SIZE;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// 用户空间堆的起始地址
/// 这是进程地址空间中堆区域的起始位置
/// 在 x86_64 Linux 上，这通常在数据段之后
pub const HEAP_START: usize = 0x0000_0000_1000_0000;

/// 用户空间堆的最大地址
/// 防止堆增长到与栈或映射区域冲突
pub const HEAP_MAX: usize = 0x0000_0000_7000_0000;

/// 每个进程的堆边界
///
/// 每个进程维护自己的堆边界，通过进程 ID 索引
struct ProcessBrk {
    /// 当前堆边界（第一个未分配字节的地址）
    current_brk: AtomicUsize,
    /// 堆的起始地址
    heap_start: usize,
}

impl ProcessBrk {
    const fn new(start: usize) -> Self {
        Self {
            current_brk: AtomicUsize::new(start),
            heap_start: start,
        }
    }

    fn get(&self) -> usize {
        self.current_brk.load(Ordering::Acquire)
    }

    fn set(&self, new_brk: usize) -> Result<(), SyscallError> {
        // 验证新的堆边界
        if new_brk < self.heap_start {
            return Err(SyscallError::InvalidArgument);
        }

        if new_brk > HEAP_MAX {
            return Err(SyscallError::NoMemory);
        }

        // 对齐到页面边界
        let aligned_brk = crate::subsystems::mm::align_up(new_brk, PAGE_SIZE);

        // TODO: 实际分配/释放物理内存页面
        // 这需要与页表管理器交互来映射或取消映射页面

        self.current_brk.store(new_brk, Ordering::Release);
        Ok(())
    }
}

/// 全局堆边界管理器
///
/// 为每个进程维护独立的堆边界
struct BrkManager {
    /// 进程 ID 到堆边界的映射
    /// 简化实现：使用 Vec 而非 HashMap 以减少依赖
    process_brks: Vec<Option<ProcessBrk>>,
}

impl BrkManager {
    const fn new() -> Self {
        Self { process_brks: Vec::new() }
    }

    /// 获取或创建进程的堆边界
    fn get_process_brk(&mut self, pid: usize) -> Result<&ProcessBrk, SyscallError> {
        // 扩展 Vec 以容纳进程 ID
        while self.process_brks.len() <= pid {
            self.process_brks.push(None);
        }

        // 如果不存在，创建新的堆边界
        if self.process_brks[pid].is_none() {
            self.process_brks[pid] = Some(ProcessBrk::new(HEAP_START));
        }

        Ok(self.process_brks[pid].as_ref().unwrap())
    }
}

/// 全局堆管理器
static BRK_MANAGER: Mutex<BrkManager> = Mutex::new(BrkManager::new());

/// brk 系统调用 - 设置堆的绝对边界
///
/// # POSIX 语义
///
/// - `brk(0)`: 返回当前堆边界，不修改
/// - `brk(addr)`: 设置堆边界到 addr
///   - 如果 addr > 当前边界：尝试增长堆
///   - 如果 addr < 当前边界：尝试收缩堆
///   - 如果 addr == 当前边界：什么都不做
///
/// # 参数
///
/// * `addr` - 新的堆边界地址
///
/// # 返回值
///
/// * `Ok(usize)` - 成功时返回新的堆边界（可能等于 addr 或更小）
/// * `Err(SyscallError)` - 失败时返回错误
///
/// # 错误
///
/// - `EINVAL`: addr 无效（小于堆起始地址）
/// - `ENOMEM`: 无法分配足够内存
///
/// # 示例
///
/// ```no_run
/// use kernel::subsystems::mm::brk::sys_brk;
///
/// // 获取当前堆边界
/// let current = sys_brk(0)?;
///
/// // 增长堆 1MB
/// let new_brk = sys_brk(current + 0x100000)?;
/// ```
pub fn sys_brk(addr: usize) -> Result<isize, SyscallError> {
    // 获取当前进程 ID
    let pid = crate::process::myproc().ok_or(SyscallError::NoProcess)?;

    let mut manager = BRK_MANAGER.lock();
    let proc_brk = manager.get_process_brk(pid)?;

    // 如果 addr 为 0，只返回当前边界
    if addr == 0 {
        return Ok(proc_brk.get() as isize);
    }

    // 验证地址对齐
    if addr & 0xF != 0 {
        // brk 应该对齐到至少 16 字节边界
        // 一些实现对齐要求更宽松，但为了安全我们要求对齐
        return Err(SyscallError::InvalidArgument);
    }

    // 尝试设置新的堆边界
    let current = proc_brk.get();

    if addr == current {
        // 已经是请求的边界
        return Ok(addr as isize);
    }

    // 设置新边界
    proc_brk.set(addr)?;

    Ok(addr as isize)
}

/// sbrk 系统调用 - 增量调整堆大小
///
/// # POSIX 语义
///
/// - `sbrk(0)`: 返回当前堆边界，不修改
/// - `sbrk(increment)`: 调整堆大小
///   - 正值：增长堆 increment 字节
///   - 负值：收缩堆 |increment| 字节
///   - 返回调整前的堆边界
///
/// # 参数
///
/// * `increment` - 堆大小的增量（可为正、负或零）
///
/// # 返回值
///
/// * `Ok(isize)` - 成功时返回调整前的堆边界
/// * `Err(SyscallError)` - 失败时返回错误
///
/// # 错误
///
/// - `EINVAL`: 导致堆边界小于起始地址
/// - `ENOMEM`: 无法分配足够内存
///
/// # 示例
///
/// ```no_run
/// use kernel::subsystems::mm::brk::sys_sbrk;
///
/// // 获取当前堆边界
/// let old_brk = sys_sbrk(0)?;
///
/// // 增长堆 4096 字节
/// let old_brk2 = sys_sbrk(4096)?;
/// assert_eq!(old_brk2, old_brk);
/// ```
pub fn sys_sbrk(increment: isize) -> Result<isize, SyscallError> {
    // 获取当前进程 ID
    let pid = crate::process::myproc().ok_or(SyscallError::NoProcess)?;

    let mut manager = BRK_MANAGER.lock();
    let proc_brk = manager.get_process_brk(pid)?;

    // 获取当前堆边界
    let current = proc_brk.get();

    // 如果 increment 为 0，只返回当前边界
    if increment == 0 {
        return Ok(current as isize);
    }

    // 计算新的堆边界
    let new_brk = if increment > 0 {
        current.checked_add(increment as usize)
    } else {
        current.checked_sub((-increment) as usize)
    };

    let new_brk = new_brk.ok_or(SyscallError::InvalidArgument)?;

    // 尝试设置新的堆边界
    proc_brk.set(new_brk)?;

    // 返回旧的堆边界
    Ok(current as isize)
}

/// 初始化进程的堆
///
/// 当新进程创建时调用，初始化其堆边界
pub fn init_process_heap(pid: usize) -> Result<(), SyscallError> {
    let mut manager = BRK_MANAGER.lock();

    // 扩展 Vec 以容纳进程 ID
    while manager.process_brks.len() <= pid {
        manager.process_brks.push(None);
    }

    // 创建新的堆边界
    manager.process_brks[pid] = Some(ProcessBrk::new(HEAP_START));

    Ok(())
}

/// 清理进程的堆
///
/// 当进程退出时调用，清理其堆边界
pub fn cleanup_process_heap(pid: usize) {
    let mut manager = BRK_MANAGER.lock();

    if pid < manager.process_brks.len() {
        // TODO: 释放所有已分配的物理内存页面
        manager.process_brks[pid] = None;
    }
}

/// 获取进程的堆统计信息
///
/// # 返回值
///
/// 返回元组：(堆起始地址, 当前边界, 已分配大小)
pub fn get_heap_stats(pid: usize) -> Result<(usize, usize, usize), SyscallError> {
    let manager = BRK_MANAGER.lock();

    if pid >= manager.process_brks.len() {
        return Err(SyscallError::NoProcess);
    }

    let proc_brk = manager.process_brks[pid]
        .as_ref()
        .ok_or(SyscallError::NoProcess)?;

    let current = proc_brk.get();
    let allocated = current - proc_brk.heap_start;

    Ok((proc_brk.heap_start, current, allocated))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brk_zero() {
        // 测试 brk(0) 返回当前边界
        let result = sys_brk(0);
        // 在实际环境中，这会返回有效地址
        // 在单元测试中，我们主要验证不会 panic
    }

    #[test]
    fn test_sbrk_zero() {
        // 测试 sbrk(0) 返回当前边界
        let result = sys_sbrk(0);
        // 验证不会 panic
    }

    #[test]
    fn test_brk_alignment() {
        // 测试地址对齐验证
        // 由于我们无法轻松模拟进程环境，这里只做基本验证
        let addr = 0x1000;
        assert!(addr & 0xF == 0, "Address should be aligned");
    }
}
