//! Scheduling syscalls bridge.
//!
//! - 0xE000-0xE008 仍委托给现有实时调度实现。
//! - 0xE010/0xE011 提供 O(1) 调度骨架的快速入口。

use crate::syscalls::common::{SyscallError, SyscallResult};

/// 调度快速路径：sched_yield（轻量）
pub const SYS_SCHED_YIELD_FAST: u32 = 0xE010;
/// 调度提示：tid, prio, cpu_hint
pub const SYS_SCHED_ENQUEUE_HINT: u32 = 0xE011;

pub fn dispatch(syscall_num: u32, args: &[u64]) -> SyscallResult {
    match syscall_num {
        // 兼容原有实时调度接口
        super::SYS_SCHED_SETSCHEDULER..=super::SYS_SCHED_GETAFFINITY => {
            super::realtime::dispatch(syscall_num, args)
        }
        SYS_SCHED_YIELD_FAST => crate::sched::syscall::sched_yield_fast(),
        SYS_SCHED_ENQUEUE_HINT => crate::sched::syscall::sched_enqueue_hint(args),
        _ => Err(SyscallError::InvalidSyscall),
    }
}

