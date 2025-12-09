//! O(1) Scheduler skeleton with per-CPU runqueues and syscall bridge.
//!
//! 设计目标：
//! - 每 CPU 就绪队列 + 优先级位图实现 O(1) 选取。
//! - 轻量骨架，便于后续接入调度策略与抢占逻辑。
//! - 提供全局访问封装，供系统调用层快速落地。

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::array;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::sync::atomic::AtomicU64;

use crate::process::thread::Tid;
use crate::sync::{Mutex, Once};

/// 支持的优先级数量（0 为最低，越大优先级越高）
pub const PRIORITY_LEVELS: usize = 64;
/// 默认时间片（单位：ticks）
pub const DEFAULT_TIMESLICE: u32 = 4;
/// 延迟直方图桶数
pub const LAT_BUCKETS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    InvalidCpu,
    InvalidPriority,
}

#[derive(Debug)]
struct PerCpuRunQueues {
    queues: [VecDeque<Tid>; PRIORITY_LEVELS],
    bitmap: u64,
    current: Option<Tid>,
    time_slice: u32,
}

impl PerCpuRunQueues {
    fn new(time_slice: u32) -> Self {
        Self {
            queues: array::from_fn(|_| VecDeque::new()),
            bitmap: 0,
            current: None,
            time_slice,
        }
    }

    fn enqueue(&mut self, tid: Tid, prio: usize) -> Result<(), SchedulerError> {
        if prio >= PRIORITY_LEVELS {
            return Err(SchedulerError::InvalidPriority);
        }
        self.queues[prio].push_back(tid);
        self.bitmap |= 1u64 << prio;
        Ok(())
    }

    fn pick_next(&mut self) -> Option<Tid> {
        let next_prio = Self::highest_priority(self.bitmap)?;
        let queue = &mut self.queues[next_prio];
        let tid = queue.pop_front();
        if queue.is_empty() {
            self.bitmap &= !(1u64 << next_prio);
        }
        if tid.is_some() {
            self.current = tid;
        }
        tid
    }

    fn current(&self) -> Option<Tid> {
        self.current
    }

    #[inline]
    fn highest_priority(bitmap: u64) -> Option<usize> {
        if bitmap == 0 {
            None
        } else {
            Some(63 - bitmap.leading_zeros() as usize)
        }
    }
}

/// O(1) 调度器主体
pub struct O1Scheduler {
    cpus: Vec<Mutex<PerCpuRunQueues>>,
    default_slice: u32,
    rr_cursor: AtomicUsize,
    stats: Vec<SchedulerStats>,
}

impl O1Scheduler {
    pub fn new(num_cpus: usize, default_slice: u32) -> Self {
        let cpu_count = core::cmp::max(1, num_cpus);
        let mut cpus = Vec::with_capacity(cpu_count);
        let mut stats = Vec::with_capacity(cpu_count);
        for _ in 0..cpu_count {
            cpus.push(Mutex::new(PerCpuRunQueues::new(default_slice)));
            stats.push(SchedulerStats::new());
        }
        Self {
            cpus,
            default_slice: core::cmp::max(1, default_slice),
            rr_cursor: AtomicUsize::new(0),
            stats,
        }
    }

    #[inline]
    fn pick_cpu_rr(&self) -> usize {
        let next = self.rr_cursor.fetch_add(1, Ordering::Relaxed);
        next % self.cpus.len()
    }

    pub fn enqueue(
        &self,
        tid: Tid,
        prio: usize,
        cpu_hint: Option<usize>,
    ) -> Result<(), SchedulerError> {
        let target = cpu_hint.unwrap_or_else(|| self.pick_cpu_rr());
        let Some(cpu_lock) = self.cpus.get(target) else {
            return Err(SchedulerError::InvalidCpu);
        };
        let mut guard = cpu_lock.lock();
        guard.enqueue(tid, prio)
    }

    pub fn pick_next(&self, cpu_id: usize) -> Result<Option<Tid>, SchedulerError> {
        let Some(cpu_lock) = self.cpus.get(cpu_id) else {
            return Err(SchedulerError::InvalidCpu);
        };
        let mut guard = cpu_lock.lock();
        Ok(guard.pick_next())
    }

    pub fn current(&self, cpu_id: usize) -> Result<Option<Tid>, SchedulerError> {
        let Some(cpu_lock) = self.cpus.get(cpu_id) else {
            return Err(SchedulerError::InvalidCpu);
        };
        let guard = cpu_lock.lock();
        Ok(guard.current())
    }

    pub fn cpu_count(&self) -> usize {
        self.cpus.len()
    }

    pub fn default_slice(&self) -> u32 {
        self.default_slice
    }

    pub fn stats(&self, cpu_id: usize) -> Option<&SchedulerStats> {
        self.stats.get(cpu_id)
    }
}

/// 轻量统计信息，后续可挂载到 /proc/trace
#[derive(Debug)]
pub struct SchedulerStats {
    pub ticks: AtomicU64,
    pub preemptions: AtomicU64,
    pub voluntary_switches: AtomicU64,
    pub latency_hist: [AtomicU64; LAT_BUCKETS],
}

impl SchedulerStats {
    pub const fn new() -> Self {
        Self {
            ticks: AtomicU64::new(0),
            preemptions: AtomicU64::new(0),
            voluntary_switches: AtomicU64::new(0),
            latency_hist: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }

    pub fn record_tick(&self, preempt: bool) {
        self.ticks.fetch_add(1, Ordering::Relaxed);
        if preempt {
            self.preemptions.fetch_add(1, Ordering::Relaxed);
        } else {
            self.voluntary_switches.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_latency(&self, nanos: u64) {
        let bucket = if nanos < 1_000 {
            0
        } else if nanos < 10_000 {
            1
        } else if nanos < 100_000 {
            2
        } else if nanos < 1_000_000 {
            3
        } else if nanos < 10_000_000 {
            4
        } else if nanos < 100_000_000 {
            5
        } else if nanos < 1_000_000_000 {
            6
        } else {
            7
        };
        self.latency_hist[bucket].fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> StatsSnapshot {
        let mut buckets = [0u64; LAT_BUCKETS];
        for (i, b) in self.latency_hist.iter().enumerate() {
            buckets[i] = b.load(Ordering::Relaxed);
        }
        StatsSnapshot {
            ticks: self.ticks.load(Ordering::Relaxed),
            preemptions: self.preemptions.load(Ordering::Relaxed),
            voluntary_switches: self.voluntary_switches.load(Ordering::Relaxed),
            latency_hist: buckets,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StatsSnapshot {
    pub ticks: u64,
    pub preemptions: u64,
    pub voluntary_switches: u64,
    pub latency_hist: [u64; LAT_BUCKETS],
}

/// 全局调度器句柄（延迟初始化）
static GLOBAL_SCHED: Mutex<Option<O1Scheduler>> = Mutex::new(None);
static GLOBAL_INIT: Once = Once::new();

pub fn init_global(num_cpus: usize) {
    GLOBAL_INIT.call_once(|| {
        *GLOBAL_SCHED.lock() = Some(O1Scheduler::new(num_cpus, DEFAULT_TIMESLICE));
    });
}

pub fn with_global<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&O1Scheduler) -> R,
{
    let guard = GLOBAL_SCHED.lock();
    guard.as_ref().map(f)
}

fn ensure_global() -> Option<()> {
    init_global(1);
    with_global(|_| ())
}

/// 系统调用侧快速接口
pub mod syscall {
    use super::{ensure_global, with_global, SchedulerError};
    use crate::process::thread::Tid;
    use crate::syscalls::common::{SyscallError, SyscallResult};
    use crate::time::get_time_ns;

    /// 用户态 hint 调度：tid, prio, cpu_hint
    pub const SYS_SCHED_ENQUEUE_HINT: u32 = 0xE011;
    /// 轻量 sched_yield（不做上下文切换，只记录意愿）
    pub const SYS_SCHED_YIELD_FAST: u32 = 0xE010;

    pub fn sched_yield_fast() -> SyscallResult {
        ensure_global().ok_or(SyscallError::NotSupported)?;
        if let Some(res) = with_global(|sched| {
            let id = 0; // 暂时使用0代替cpu_id()，待后续实现
            if let Some(stats) = sched.stats(id) {
                stats.record_tick(false);
            }
        }) {
            res
        }
        Ok(0)
    }

    pub fn sched_enqueue_hint(args: &[u64]) -> SyscallResult {
        if args.len() < 3 {
            return Err(SyscallError::InvalidArgument);
        }
        let tid = args[0] as Tid;
        let prio = args[1] as usize;
        let cpu = args[2] as usize;

        ensure_global().ok_or(SyscallError::NotSupported)?;
        match with_global(|sched| {
            let start = get_time_ns();
            let r = sched.enqueue(tid, prio, Some(cpu));
            if let Some(stats) = sched.stats(cpu.min(sched.cpu_count().saturating_sub(1))) {
                let end = get_time_ns();
                stats.record_latency(end.saturating_sub(start));
            }
            r
        }) {
            Some(Ok(())) => Ok(0),
            Some(Err(SchedulerError::InvalidCpu)) => Err(SyscallError::InvalidArgument),
            Some(Err(SchedulerError::InvalidPriority)) => Err(SyscallError::InvalidArgument),
            None => Err(SyscallError::NotSupported),
        }
    }

    pub fn sched_pick_next(cpu_id: usize) -> Option<Tid> {
        ensure_global()?;
        with_global(|sched| sched.pick_next(cpu_id).ok().flatten())?
    }
}

