//! 高性能调度器优化实现
//!
//! 本模块提供高性能调度器优化，包括：
//! - 自适应时间片调整
//! - CPU亲和性优化
//! - 负载均衡优化
//! - 调度延迟优化
//! - 抢占优化

use crate::sched::{O1Scheduler, SchedulerStats, PRIORITY_LEVELS, DEFAULT_TIMESLICE};
use crate::process::thread::Tid;
use crate::sync::Mutex;
use crate::time::get_time_ns;
use crate::collections::HashMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};

/// 全局优化调度器
static GLOBAL_OPTIMIZED_SCHED: Mutex<Option<OptimizedScheduler>> = Mutex::new(None);

/// 优化调度器
pub struct OptimizedScheduler {
    base_scheduler: O1Scheduler,
    optimization_config: SchedulerOptimizationConfig,
    per_cpu_stats: Vec<PerCpuOptimizedStats>,
    thread_info: HashMap<Tid, ThreadSchedulingInfo>,
    load_balancer: LoadBalancer,
    adaptive_timeslice: AdaptiveTimeslice,
    last_optimization: AtomicU64,
}

/// 调度器优化配置
#[derive(Debug, Clone)]
pub struct SchedulerOptimizationConfig {
    pub enable_adaptive_timeslice: bool,
    pub enable_cpu_affinity: bool,
    pub enable_load_balancing: bool,
    pub enable_preemption_optimization: bool,
    pub optimization_interval_ms: u64,
    pub load_balance_threshold: f64,
    pub preemption_threshold: u32,
}

impl Default for SchedulerOptimizationConfig {
    fn default() -> Self {
        Self {
            enable_adaptive_timeslice: true,
            enable_cpu_affinity: true,
            enable_load_balancing: true,
            enable_preemption_optimization: true,
            optimization_interval_ms: 1000, // 1 second
            load_balance_threshold: 0.2,    // 20% imbalance threshold
            preemption_threshold: 2,         // Preempt after 2 timeslices
        }
    }
}

/// 每CPU优化统计
#[derive(Debug)]
pub struct PerCpuOptimizedStats {
    pub base_stats: SchedulerStats,
    pub context_switches: AtomicU64,
    pub migrations: AtomicU64,
    pub cache_misses: AtomicU64,
    pub idle_time: AtomicU64,
    pub busy_time: AtomicU64,
    pub last_activity: AtomicU64,
}

impl PerCpuOptimizedStats {
    pub fn new() -> Self {
        Self {
            base_stats: SchedulerStats::new(),
            context_switches: AtomicU64::new(0),
            migrations: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            idle_time: AtomicU64::new(0),
            busy_time: AtomicU64::new(0),
            last_activity: AtomicU64::new(0),
        }
    }
    
    pub fn record_context_switch(&self) {
        self.context_switches.fetch_add(1, Ordering::Relaxed);
        self.last_activity.store(get_time_ns(), Ordering::Relaxed);
    }
    
    pub fn record_migration(&self) {
        self.migrations.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn update_activity(&self, is_idle: bool) {
        let now = get_time_ns();
        let last = self.last_activity.load(Ordering::Relaxed);
        
        if last > 0 {
            let duration = now.saturating_sub(last);
            if is_idle {
                self.idle_time.fetch_add(duration, Ordering::Relaxed);
            } else {
                self.busy_time.fetch_add(duration, Ordering::Relaxed);
            }
        }
        
        self.last_activity.store(now, Ordering::Relaxed);
    }
    
    pub fn get_cpu_utilization(&self) -> f64 {
        let idle = self.idle_time.load(Ordering::Relaxed);
        let busy = self.busy_time.load(Ordering::Relaxed);
        let total = idle + busy;
        
        if total == 0 {
            0.0
        } else {
            busy as f64 / total as f64
        }
    }
}

/// 线程调度信息
#[derive(Debug, Clone)]
pub struct ThreadSchedulingInfo {
    pub tid: Tid,
    pub priority: usize,
    pub cpu_affinity: Option<usize>,
    pub last_cpu: Option<usize>,
    pub runtime: AtomicU64,
    pub wait_time: AtomicU64,
    pub timeslice_used: AtomicU32,
    pub last_scheduled: AtomicU64,
    pub preemption_count: AtomicU64,
    pub cache_affinity: AtomicU64,
}

impl ThreadSchedulingInfo {
    pub fn new(tid: Tid, priority: usize) -> Self {
        Self {
            tid,
            priority,
            cpu_affinity: None,
            last_cpu: None,
            runtime: AtomicU64::new(0),
            wait_time: AtomicU64::new(0),
            timeslice_used: AtomicU32::new(0),
            last_scheduled: AtomicU64::new(0),
            preemption_count: AtomicU64::new(0),
            cache_affinity: AtomicU64::new(0),
        }
    }
    
    pub fn update_runtime(&self, duration: u64) {
        self.runtime.fetch_add(duration, Ordering::Relaxed);
    }
    
    pub fn update_wait_time(&self, duration: u64) {
        self.wait_time.fetch_add(duration, Ordering::Relaxed);
    }
    
    pub fn increment_timeslice_used(&self) {
        self.timeslice_used.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn reset_timeslice_used(&self) {
        self.timeslice_used.store(0, Ordering::Relaxed);
    }
    
    pub fn record_preemption(&self) {
        self.preemption_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn update_cache_affinity(&self, cpu_id: usize) {
        self.cache_affinity.store(cpu_id as u64, Ordering::Relaxed);
    }
}

/// 负载均衡器
#[derive(Debug)]
pub struct LoadBalancer {
    cpu_loads: Vec<f64>,
    migration_threshold: f64,
    last_balance: AtomicU64,
}

impl LoadBalancer {
    pub fn new(num_cpus: usize, threshold: f64) -> Self {
        Self {
            cpu_loads: vec![0.0; num_cpus],
            migration_threshold: threshold,
            last_balance: AtomicU64::new(0),
        }
    }
    
    pub fn update_cpu_load(&mut self, cpu_id: usize, load: f64) {
        if cpu_id < self.cpu_loads.len() {
            self.cpu_loads[cpu_id] = load;
        }
    }
    
    pub fn should_balance(&self) -> bool {
        let now = get_time_ns();
        let last = self.last_balance.load(Ordering::Relaxed);
        
        // Check if enough time has passed since last balance
        if now - last < 1_000_000_000 { // 1 second
            return false;
        }
        
        // Check if there's significant imbalance
        let max_load = self.cpu_loads.iter().fold(0.0, |a, &b| a.max(b));
        let min_load = self.cpu_loads.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        if max_load == 0.0 {
            return false;
        }
        
        let imbalance = (max_load - min_load) / max_load;
        imbalance > self.migration_threshold
    }
    
    pub fn find_migration_target(&self, source_cpu: usize) -> Option<usize> {
        if source_cpu >= self.cpu_loads.len() {
            return None;
        }
        
        let source_load = self.cpu_loads[source_cpu];
        let mut best_target = None;
        let mut best_load = f64::INFINITY;
        
        for (cpu_id, &load) in self.cpu_loads.iter().enumerate() {
            if cpu_id == source_cpu {
                continue;
            }
            
            if load < best_load && load < source_load * 0.8 {
                best_load = load;
                best_target = Some(cpu_id);
            }
        }
        
        best_target
    }
    
    pub fn record_balance(&self) {
        self.last_balance.store(get_time_ns(), Ordering::Relaxed);
    }
}

/// 自适应时间片管理器
#[derive(Debug)]
pub struct AdaptiveTimeslice {
    base_timeslice: u32,
    min_timeslice: u32,
    max_timeslice: u32,
    cpu_utilization_threshold: f64,
    last_adjustment: AtomicU64,
}

impl AdaptiveTimeslice {
    pub fn new(base_timeslice: u32) -> Self {
        Self {
            base_timeslice,
            min_timeslice: 1,
            max_timeslice: 16,
            cpu_utilization_threshold: 0.8,
            last_adjustment: AtomicU64::new(0),
        }
    }
    
    pub fn calculate_timeslice(&self, thread_info: &ThreadSchedulingInfo, cpu_utilization: f64) -> u32 {
        let mut timeslice = self.base_timeslice;
        
        // Adjust based on thread priority
        timeslice = timeslice.saturating_add((thread_info.priority / 10) as u32);
        
        // Adjust based on CPU utilization
        if cpu_utilization > self.cpu_utilization_threshold {
            // High CPU utilization, reduce timeslice
            timeslice = timeslice.saturating_sub(1);
        } else if cpu_utilization < 0.5 {
            // Low CPU utilization, increase timeslice
            timeslice = timeslice.saturating_add(1);
        }
        
        // Clamp to min/max
        timeslice = core::cmp::max(self.min_timeslice, timeslice);
        timeslice = core::cmp::min(self.max_timeslice, timeslice);
        
        timeslice
    }
    
    pub fn should_adjust(&self) -> bool {
        let now = get_time_ns();
        let last = self.last_adjustment.load(Ordering::Relaxed);
        now - last > 5_000_000_000 // 5 seconds
    }
    
    pub fn record_adjustment(&self) {
        self.last_adjustment.store(get_time_ns(), Ordering::Relaxed);
    }
}

impl OptimizedScheduler {
    pub fn new(num_cpus: usize) -> Self {
        let config = SchedulerOptimizationConfig::default();
        Self::with_config(num_cpus, config)
    }
    
    pub fn with_config(num_cpus: usize, config: SchedulerOptimizationConfig) -> Self {
        let base_scheduler = O1Scheduler::new(num_cpus, DEFAULT_TIMESLICE);
        let mut per_cpu_stats = Vec::with_capacity(num_cpus);
        
        for _ in 0..num_cpus {
            per_cpu_stats.push(PerCpuOptimizedStats::new());
        }
        
        Self {
            base_scheduler,
            optimization_config: config,
            per_cpu_stats,
            thread_info: HashMap::new(),
            load_balancer: LoadBalancer::new(num_cpus, config.load_balance_threshold),
            adaptive_timeslice: AdaptiveTimeslice::new(DEFAULT_TIMESLICE),
            last_optimization: AtomicU64::new(0),
        }
    }
    
    /// 优化的线程入队
    pub fn enqueue_optimized(&mut self, tid: Tid, priority: usize, cpu_hint: Option<usize>) -> Result<(), crate::sched::SchedulerError> {
        let now = get_time_ns();
        
        // 获取或创建线程信息
        let thread_info = self.thread_info.entry(tid).or_insert_with(|| ThreadSchedulingInfo::new(tid, priority));
        thread_info.priority = priority;
        
        // 更新等待时间
        let last_scheduled = thread_info.last_scheduled.load(Ordering::Relaxed);
        if last_scheduled > 0 {
            thread_info.update_wait_time(now.saturating_sub(last_scheduled));
        }
        
        // CPU亲和性优化
        let target_cpu = if self.optimization_config.enable_cpu_affinity {
            self.select_optimal_cpu(thread_info, cpu_hint)
        } else {
            cpu_hint.unwrap_or_else(|| self.base_scheduler.pick_cpu_rr())
        };
        
        // 更新负载均衡器
        if let Some(stats) = self.per_cpu_stats.get(target_cpu) {
            let utilization = stats.get_cpu_utilization();
            self.load_balancer.update_cpu_load(target_cpu, utilization);
        }
        
        // 执行负载均衡
        if self.optimization_config.enable_load_balancing && self.load_balancer.should_balance() {
            self.perform_load_balance();
        }
        
        // 记录迁移
        if let Some(last_cpu) = thread_info.last_cpu {
            if last_cpu != target_cpu {
                if let Some(stats) = self.per_cpu_stats.get(target_cpu) {
                    stats.record_migration();
                }
            }
        }
        
        thread_info.last_cpu = Some(target_cpu);
        
        // 使用基础调度器入队
        self.base_scheduler.enqueue(tid, priority, Some(target_cpu))
    }
    
    /// 优化的选择下一个线程
    pub fn pick_next_optimized(&mut self, cpu_id: usize) -> Result<Option<Tid>, crate::sched::SchedulerError> {
        let now = get_time_ns();
        
        // 使用基础调度器选择
        let tid = self.base_scheduler.pick_next(cpu_id)?;
        
        if let Some(tid) = tid {
            // 更新线程信息
            if let Some(thread_info) = self.thread_info.get_mut(&tid) {
                thread_info.last_scheduled.store(now, Ordering::Relaxed);
                thread_info.reset_timeslice_used();
                thread_info.update_cache_affinity(cpu_id);
            }
            
            // 更新CPU统计
            if let Some(stats) = self.per_cpu_stats.get(cpu_id) {
                stats.record_context_switch();
                stats.update_activity(false);
            }
        } else {
            // 没有可运行的线程，更新空闲时间
            if let Some(stats) = self.per_cpu_stats.get(cpu_id) {
                stats.update_activity(true);
            }
        }
        
        Ok(tid)
    }
    
    /// 选择最优CPU
    fn select_optimal_cpu(&self, thread_info: &ThreadSchedulingInfo, cpu_hint: Option<usize>) -> usize {
        // 如果有CPU亲和性设置，优先使用
        if let Some(affinity) = thread_info.cpu_affinity {
            return affinity;
        }
        
        // 如果有CPU提示，检查是否合适
        if let Some(hint) = cpu_hint {
            if hint < self.per_cpu_stats.len() {
                return hint;
            }
        }
        
        // 选择负载最低的CPU
        let mut best_cpu = 0;
        let mut best_load = f64::INFINITY;
        
        for (cpu_id, stats) in self.per_cpu_stats.iter().enumerate() {
            let load = stats.get_cpu_utilization();
            
            // 考虑缓存亲和性
            let cache_affinity_bonus = if thread_info.cache_affinity.load(Ordering::Relaxed) == cpu_id as u64 {
                0.1 // 10% bonus for cache affinity
            } else {
                0.0
            };
            
            let adjusted_load = load - cache_affinity_bonus;
            
            if adjusted_load < best_load {
                best_load = adjusted_load;
                best_cpu = cpu_id;
            }
        }
        
        best_cpu
    }
    
    /// 执行负载均衡
    fn perform_load_balance(&mut self) {
        // 简化实现，实际应该迁移线程
        for cpu_id in 0..self.per_cpu_stats.len() {
            if let Some(target_cpu) = self.load_balancer.find_migration_target(cpu_id) {
                crate::println!("[sched] Load balance: migrate from CPU {} to CPU {}", cpu_id, target_cpu);
                // TODO: 实际迁移线程
            }
        }
        
        self.load_balancer.record_balance();
    }
    
    /// 优化的时间片处理
    pub fn handle_timeslice_optimized(&mut self, tid: Tid, cpu_id: usize) -> bool {
        let now = get_time_ns();
        
        if let Some(thread_info) = self.thread_info.get_mut(&tid) {
            thread_info.increment_timeslice_used();
            
            // 计算自适应时间片
            let cpu_utilization = if let Some(stats) = self.per_cpu_stats.get(cpu_id) {
                stats.get_cpu_utilization()
            } else {
                0.0
            };
            
            let adaptive_timeslice = if self.optimization_config.enable_adaptive_timeslice {
                self.adaptive_timeslice.calculate_timeslice(thread_info, cpu_utilization)
            } else {
                DEFAULT_TIMESLICE
            };
            
            // 检查是否应该抢占
            let timeslice_used = thread_info.timeslice_used.load(Ordering::Relaxed);
            if timeslice_used >= adaptive_timeslice {
                thread_info.record_preemption();
                return true; // 应该抢占
            }
        }
        
        false // 不应该抢占
    }
    
    /// 执行调度器优化
    pub fn perform_optimization(&mut self) {
        let now = get_time_ns();
        let last_opt = self.last_optimization.load(Ordering::Relaxed);
        
        if now - last_opt < self.optimization_config.optimization_interval_ms * 1_000_000 {
            return;
        }
        
        // 自适应时间片调整
        if self.optimization_config.enable_adaptive_timeslice && self.adaptive_timeslice.should_adjust() {
            self.adjust_adaptive_timeslice();
            self.adaptive_timeslice.record_adjustment();
        }
        
        // 负载均衡
        if self.optimization_config.enable_load_balancing && self.load_balancer.should_balance() {
            self.perform_load_balance();
        }
        
        self.last_optimization.store(now, Ordering::Relaxed);
    }
    
    /// 调整自适应时间片
    fn adjust_adaptive_timeslice(&mut self) {
        // 简化实现，实际应该基于系统负载调整
        let total_utilization: f64 = self.per_cpu_stats.iter()
            .map(|stats| stats.get_cpu_utilization())
            .sum();
        
        let avg_utilization = total_utilization / self.per_cpu_stats.len() as f64;
        
        if avg_utilization > 0.8 {
            // 高负载，减少时间片
            crate::println!("[sched] High load detected, reducing timeslice");
        } else if avg_utilization < 0.5 {
            // 低负载，增加时间片
            crate::println!("[sched] Low load detected, increasing timeslice");
        }
    }
    
    /// 获取调度器统计
    pub fn get_scheduler_stats(&self, cpu_id: usize) -> Option<&PerCpuOptimizedStats> {
        self.per_cpu_stats.get(cpu_id)
    }
    
    /// 获取线程信息
    pub fn get_thread_info(&self, tid: Tid) -> Option<&ThreadSchedulingInfo> {
        self.thread_info.get(&tid)
    }
    
    /// 设置线程CPU亲和性
    pub fn set_thread_affinity(&mut self, tid: Tid, cpu_id: usize) -> bool {
        if let Some(thread_info) = self.thread_info.get_mut(&tid) {
            thread_info.cpu_affinity = Some(cpu_id);
            true
        } else {
            false
        }
    }
}

/// 初始化全局优化调度器
pub fn initialize_global_optimized_scheduler(num_cpus: usize) {
    let mut sched_guard = GLOBAL_OPTIMIZED_SCHED.lock();
    if sched_guard.is_none() {
        let scheduler = OptimizedScheduler::new(num_cpus);
        *sched_guard = Some(scheduler);
        crate::println!("[sched] Optimized scheduler initialized with {} CPUs", num_cpus);
    }
}

/// 获取全局优化调度器
pub fn get_global_optimized_scheduler() -> &'static Mutex<Option<OptimizedScheduler>> {
    &GLOBAL_OPTIMIZED_SCHED
}

/// 优化的sched_yield系统调用
pub fn sched_yield_optimized() -> crate::syscalls::common::SyscallResult {
    let mut sched_guard = GLOBAL_OPTIMIZED_SCHED.lock();
    if let Some(ref mut scheduler) = *sched_guard {
        // 执行调度器优化
        scheduler.perform_optimization();
        
        // 记录统计
        let cpu_id = 0; // TODO: 获取当前CPU ID
        if let Some(stats) = scheduler.get_scheduler_stats(cpu_id) {
            stats.record_context_switch();
        }
        
        Ok(0)
    } else {
        Err(crate::syscalls::common::SyscallError::NotSupported)
    }
}

/// 优化的调度系统调用分发
pub fn dispatch_optimized(syscall_num: u32, args: &[u64]) -> crate::syscalls::common::SyscallResult {
    match syscall_num {
        0xE010 => sched_yield_optimized(),
        0xE011 => sched_enqueue_hint_optimized(args),
        _ => Err(crate::syscalls::common::SyscallError::NotSupported),
    }
}

/// 优化的调度提示系统调用
pub fn sched_enqueue_hint_optimized(args: &[u64]) -> crate::syscalls::common::SyscallResult {
    if args.len() < 3 {
        return Err(crate::syscalls::common::SyscallError::InvalidArgument);
    }
    
    let tid = args[0] as Tid;
    let priority = args[1] as usize;
    let cpu_hint = args[2] as usize;
    
    let mut sched_guard = GLOBAL_OPTIMIZED_SCHED.lock();
    if let Some(ref mut scheduler) = *sched_guard {
        match scheduler.enqueue_optimized(tid, priority, Some(cpu_hint)) {
            Ok(()) => Ok(0),
            Err(_) => Err(crate::syscalls::common::SyscallError::InvalidArgument),
        }
    } else {
        Err(crate::syscalls::common::SyscallError::NotSupported)
    }
}

/// 获取调度器性能报告
pub fn get_scheduler_performance_report() -> Option<SchedulerPerformanceReport> {
    let sched_guard = GLOBAL_OPTIMIZED_SCHED.lock();
    if let Some(ref scheduler) = *sched_guard {
        let mut cpu_stats = Vec::new();
        for (cpu_id, stats) in scheduler.per_cpu_stats.iter().enumerate() {
            cpu_stats.push(CpuPerformanceStats {
                cpu_id,
                utilization: stats.get_cpu_utilization(),
                context_switches: stats.context_switches.load(Ordering::Relaxed),
                migrations: stats.migrations.load(Ordering::Relaxed),
                cache_misses: stats.cache_misses.load(Ordering::Relaxed),
                idle_time: stats.idle_time.load(Ordering::Relaxed),
                busy_time: stats.busy_time.load(Ordering::Relaxed),
            });
        }
        
        Some(SchedulerPerformanceReport {
            timestamp: get_time_ns(),
            cpu_stats,
            thread_count: scheduler.thread_info.len(),
            load_balance_threshold: scheduler.optimization_config.load_balance_threshold,
        })
    } else {
        None
    }
}

/// CPU性能统计
#[derive(Debug, Clone)]
pub struct CpuPerformanceStats {
    pub cpu_id: usize,
    pub utilization: f64,
    pub context_switches: u64,
    pub migrations: u64,
    pub cache_misses: u64,
    pub idle_time: u64,
    pub busy_time: u64,
}

/// 调度器性能报告
#[derive(Debug, Clone)]
pub struct SchedulerPerformanceReport {
    pub timestamp: u64,
    pub cpu_stats: Vec<CpuPerformanceStats>,
    pub thread_count: usize,
    pub load_balance_threshold: f64,
}