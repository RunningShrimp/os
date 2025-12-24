//! O(1) Scheduler skeleton with per-CPU runqueues and syscall bridge.
//!
//! 设计目标：
//! - 每 CPU 就绪队列 + 优先级位图实现 O(1) 选取。
//! - 轻量骨架，便于后续接入调度策略与抢占逻辑。
//! - 提供全局访问封装，供系统调用层快速落地。

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicUsize, AtomicU32, AtomicU64, Ordering};
use crate::subsystems::sync::SpinLock;
use crate::arch::cpuid;

/// 默认时间片（单位：ticks）
pub const DEFAULT_TIMESLICE: u32 = 4;
/// 延迟直方图桶数
pub const LAT_BUCKETS: usize = 8;
/// 最大优先级数
pub const MAX_PRIORITY: usize = 140;
/// 每个CPU的就绪队列
pub const MAX_CPUS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    InvalidCpu,
    InvalidPriority,
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

// 全局调度器已替换为静态每CPU调度器数组，保留此注释作为历史记录

/// 系统调用侧快速接口
pub mod syscall {
    use super::O1Scheduler;
    use crate::process::thread::Tid;
    use crate::syscalls::common::SyscallError;
    use nos_api::syscall::SyscallResult;
    use crate::subsystems::time::get_time_ns;
    use crate::arch::cpuid;

    /// 用户态 hint 调度：tid, prio, cpu_hint
    pub const SYS_SCHED_ENQUEUE_HINT: u32 = 0xE011;
    /// 轻量 sched_yield（不做上下文切换，只记录意愿）
    pub const SYS_SCHED_YIELD_FAST: u32 = 0xE010;

    pub fn sched_yield_fast() -> SyscallResult {
        // 记录自愿让出CPU
        let stats = O1Scheduler::get_detailed_stats();
        stats.record_tick(false);
        Ok(0)
    }

    pub fn sched_enqueue_hint(args: &[u64]) -> SyscallResult {
        if args.len() < 3 {
            return Err(SyscallError::InvalidArgument);
        }
        let tid = args[0] as usize;
        let prio = args[1] as usize;
        let cpu = args[2] as usize;

        // 检查优先级是否有效
        if prio >= MAX_PRIORITY {
            return Err(SyscallError::InvalidArgument);
        }

        // 记录开始时间
        let start = get_time_ns();
        
        // 添加任务到指定CPU
        O1Scheduler::add_task_to_cpu(tid, prio, cpu);
        
        // 记录延迟
        let end = get_time_ns();
        let stats = O1Scheduler::get_cpu_scheduler(cpu).detailed_stats();
        stats.record_latency(end.saturating_sub(start));
        
        Ok(0)
    }

    pub fn sched_pick_next(cpu_id: usize) -> Option<Tid> {
        O1Scheduler::peek_next_on_cpu(cpu_id).map(|tid| tid as Tid)
    }
    
    /// 获取调度器统计信息
    pub fn sched_get_stats(cpu_id: usize) -> Result<StatsSnapshot, SyscallError> {
        if cpu_id >= MAX_CPUS {
            return Err(SyscallError::InvalidArgument);
        }
        
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);
        Ok(scheduler.detailed_stats().snapshot())
    }
}


// 常量已移到文件顶部

// 每CPU调度器状态
#[repr(align(64))]
struct PerCpuScheduler {
    // 优先级位图：每个位表示对应优先级是否有就绪任务
    priority_bitmap: AtomicU32,
    // 就绪队列数组：每个优先级一个队列
    ready_queues: [SpinLock<VecDeque<usize>>; MAX_PRIORITY],
    // 当前运行的任务ID
    current_task: AtomicU32,
    // 任务计数
    task_count: AtomicUsize,
    // 统计信息
    stats: SchedulerStats,
    // 填充到缓存行
    _padding: [u8; 64 - (28 + core::mem::size_of::<SchedulerStats>())],
}

impl PerCpuScheduler {
    const fn new() -> Self {
        Self {
            priority_bitmap: AtomicU32::new(0),
            ready_queues: [const { SpinLock::new(VecDeque::new()) }; MAX_PRIORITY],
            current_task: AtomicU32::new(0),
            task_count: AtomicUsize::new(0),
            stats: SchedulerStats::new(),
            _padding: [0; 64 - (28 + core::mem::size_of::<SchedulerStats>())],
        }
    }
    
    /// 添加任务到就绪队列
    fn enqueue(&self, task_id: usize, priority: usize) {
        if priority >= MAX_PRIORITY {
            return;
        }
        
        let queue = &self.ready_queues[priority];
        queue.lock().push_back(task_id);
        
        // 设置优先级位
        let bitmask = 1u32 << (priority as u32 % 32);
        self.priority_bitmap.fetch_or(bitmask, Ordering::Release);
        
        self.task_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 从就绪队列取出任务
    fn dequeue(&self) -> Option<usize> {
        // 查找最高优先级非空队列
        let bitmap = self.priority_bitmap.load(Ordering::Acquire);
        if bitmap == 0 {
            return None;
        }
        
        // 找到最高设置位（最高优先级）
        let highest_priority = bitmap.trailing_zeros() as usize;
        if highest_priority >= MAX_PRIORITY {
            return None;
        }
        
        let queue = &self.ready_queues[highest_priority];
        let mut queue_guard = queue.lock();
        
        if let Some(task_id) = queue_guard.pop_front() {
            // 如果队列变空，清除位图对应位
            if queue_guard.is_empty() {
                let bitmask = 1u32 << (highest_priority as u32 % 32);
                self.priority_bitmap.fetch_and(!bitmask, Ordering::Release);
            }
            
            self.task_count.fetch_sub(1, Ordering::Relaxed);
            self.current_task.store(task_id as u32, Ordering::Relaxed);
            
            // 记录调度统计
            self.stats.record_tick(false);
            
            return Some(task_id);
        }
        
        None
    }
    
    /// 获取下一个要运行的任务（不取出）
    fn peek(&self) -> Option<usize> {
        let bitmap = self.priority_bitmap.load(Ordering::Acquire);
        if bitmap == 0 {
            return None;
        }
        
        let highest_priority = bitmap.trailing_zeros() as usize;
        if highest_priority >= MAX_PRIORITY {
            return None;
        }
        
        let queue = &self.ready_queues[highest_priority];
        let queue_guard = queue.lock();
        queue_guard.front().copied()
    }
    
    /// 移除特定任务
    fn remove(&self, task_id: usize) -> bool {
        for priority in 0..MAX_PRIORITY {
            let queue = &self.ready_queues[priority];
            let mut queue_guard = queue.lock();
            
            if let Some(pos) = queue_guard.iter().position(|&id| id == task_id) {
                queue_guard.remove(pos);
                
                // 如果队列变空，清除位图对应位
                if queue_guard.is_empty() {
                    let bitmask = 1u32 << (priority as u32 % 32);
                    self.priority_bitmap.fetch_and(!bitmask, Ordering::Release);
                }
                
                self.task_count.fetch_sub(1, Ordering::Relaxed);
                return true;
            }
        }
        
        false
    }
    
    /// 获取调度器统计
    fn stats(&self) -> (usize, usize, u32) {
        let count = self.task_count.load(Ordering::Relaxed);
        let current = self.current_task.load(Ordering::Relaxed);
        let bitmap = self.priority_bitmap.load(Ordering::Relaxed);
        
        // 计算非空队列数量
        let mut queue_count = 0;
        for priority in 0..MAX_PRIORITY {
            let queue = &self.ready_queues[priority];
            if !queue.lock().is_empty() {
                queue_count += 1;
            }
        }
        
        (count, queue_count, bitmap)
    }
    
    /// 获取详细统计信息
    fn detailed_stats(&self) -> &SchedulerStats {
        &self.stats
    }
}

// 全局每CPU调度器数组
static PER_CPU_SCHEDULERS: [PerCpuScheduler; MAX_CPUS] = 
    [const { PerCpuScheduler::new() }; MAX_CPUS];

/// 获取当前CPU的调度器
fn current_cpu_scheduler() -> &'static PerCpuScheduler {
    let cpu_id = cpuid() as usize;
    &PER_CPU_SCHEDULERS[cpu_id % MAX_CPUS]
}

/// O(1)调度器的主要接口
pub struct O1Scheduler;

impl O1Scheduler {
    /// 初始化调度器
    pub fn init() {
        // 初始化每个CPU的调度器
        for scheduler in &PER_CPU_SCHEDULERS {
            // 确保内存屏障
            core::sync::atomic::fence(Ordering::SeqCst);
        }
        info!("O(1) scheduler initialized");
    }
    
    /// 调度下一个任务
    pub fn schedule_next() -> Option<usize> {
        current_cpu_scheduler().dequeue()
    }
    
    /// 添加任务到就绪队列
    pub fn add_task(task_id: usize, priority: usize) {
        current_cpu_scheduler().enqueue(task_id, priority);
    }
    
    /// 移除任务
    pub fn remove_task(task_id: usize) -> bool {
        current_cpu_scheduler().remove(task_id)
    }
    
    /// 获取下一个要运行的任务（不调度）
    pub fn peek_next() -> Option<usize> {
        current_cpu_scheduler().peek()
    }
    
    /// 获取调度器统计信息
    pub fn get_stats() -> (usize, usize, u32) {
        current_cpu_scheduler().stats()
    }
    
    /// 获取详细统计信息
    pub fn get_detailed_stats() -> &'static SchedulerStats {
        current_cpu_scheduler().detailed_stats()
    }
    
    /// 获取指定CPU的调度器
    pub fn get_cpu_scheduler(cpu_id: usize) -> &'static PerCpuScheduler {
        &PER_CPU_SCHEDULERS[cpu_id % MAX_CPUS]
    }
    
    /// 添加任务到指定CPU的就绪队列
    pub fn add_task_to_cpu(task_id: usize, priority: usize, cpu_id: usize) {
        let scheduler = Self::get_cpu_scheduler(cpu_id);
        scheduler.enqueue(task_id, priority);
    }
    
    /// 从指定CPU移除任务
    pub fn remove_task_from_cpu(task_id: usize, cpu_id: usize) -> bool {
        let scheduler = Self::get_cpu_scheduler(cpu_id);
        scheduler.remove(task_id)
    }
    
    /// 获取指定CPU的下一个任务
    pub fn peek_next_on_cpu(cpu_id: usize) -> Option<usize> {
        let scheduler = Self::get_cpu_scheduler(cpu_id);
        scheduler.peek()
    }
    
    /// 负载均衡：将任务迁移到其他CPU
    pub fn load_balance() {
        // 简单的负载均衡策略：如果当前CPU任务过多，迁移一些到空闲CPU
        let (current_count, _, _) = current_cpu_scheduler().stats();
        let avg_load = Self::get_average_load();
        
        if current_count > avg_load * 3 / 2 {
            // 需要迁移任务
            Self::migrate_tasks(current_count - avg_load);
        }
    }
    
    /// 获取系统平均负载
    fn get_average_load() -> usize {
        let mut total = 0;
        let mut active_cpus = 0;
        
        for scheduler in &PER_CPU_SCHEDULERS[..MAX_CPUS] {
            let (count, _, _) = scheduler.stats();
            if count > 0 {
                total += count;
                active_cpus += 1;
            }
        }
        
        if active_cpus > 0 { total / active_cpus } else { 0 }
    }
    
    /// 迁移任务
    fn migrate_tasks(num_tasks: usize) {
        // 简化的任务迁移实现
        // 实际实现需要考虑任务亲和性等
        info!("Migrating {} tasks for load balancing", num_tasks);
    }

    /// 工作窃取：从其他CPU窃取任务
    pub fn work_steal() -> Option<usize> {
        let cpu_id = cpuid() as usize % MAX_CPUS;
        let local_scheduler = &PER_CPU_SCHEDULERS[cpu_id];

        // 如果本地CPU有足够工作，不窃取（反抖动）
        let (local_count, _, _) = local_scheduler.stats();
        if local_count > 2 {
            return None;
        }

        // 获取随机起始点（公平窃取）
        let random_start = Self::get_steal_random_offset() as usize % MAX_CPUS;

        // 工作窃取：负载感知选择
        for i in 0..MAX_CPUS.saturating_sub(1) {
            let steal_cpu_id = (random_start + i) % MAX_CPUS;

            // 跳过本地CPU
            if steal_cpu_id == cpu_id {
                continue;
            }

            let steal_scheduler = &PER_CPU_SCHEDULERS[steal_cpu_id];
            let (steal_count, _, _) = steal_scheduler.stats();

            // 只从负载更高的CPU窃取
            if steal_count <= local_count + 1 {
                continue;
            }

            // 尝试从该CPU窃取最高优先级任务
            if let Some(task_id) = steal_scheduler.peek() {
                // 成功窃取，从原CPU移除
                steal_scheduler.remove(task_id);
                return Some(task_id);
            }
        }

        None
    }

    /// 带工作窃取的调度
    pub fn schedule_next_with_steal() -> Option<usize> {
        // 先尝试本地调度
        if let Some(task_id) = Self::schedule_next() {
            return Some(task_id);
        }

        // 本地无任务，尝试工作窃取
        Self::work_steal()
    }

    /// 获取窃取随机偏移（简单哈希）
    fn get_steal_random_offset() -> u32 {
        use crate::subsystems::time::get_ticks;
        let timestamp = get_ticks();
        let cpu_id = cpuid() as u64;
        let combined = timestamp.wrapping_mul(31).wrapping_add(cpu_id);
        (combined as u32)
    }
}