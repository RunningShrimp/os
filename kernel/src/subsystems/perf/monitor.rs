use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

#[derive(Debug, Clone)]
pub struct CpuMetrics {
    pub total_cycles: u64,
    pub idle_cycles: u64,
    pub user_cycles: u64,
    pub kernel_cycles: u64,
    pub context_switches: u64,
    pub interrupts: u64,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub cached_bytes: u64,
    pub page_faults: u64,
    pub page_allocations: u64,
    pub page_deallocations: u64,
}

#[derive(Debug, Clone)]
pub struct IoMetrics {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub read_operations: u64,
    pub write_operations: u64,
    pub active_requests: u64,
}

#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub packets_received: u64,
    pub packets_sent: u64,
    pub active_connections: u64,
    pub total_connections: u64,
}

#[derive(Debug, Clone)]
pub struct SchedulerMetrics {
    pub total_tasks: u64,
    pub running_tasks: u64,
    pub runnable_tasks: u64,
    pub blocked_tasks: u64,
    pub context_switches: u64,
    pub average_load: f64,
}

#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    pub timestamp: u64,
    pub cpu_metrics: CpuMetrics,
    pub memory_metrics: MemoryMetrics,
    pub io_metrics: IoMetrics,
    pub network_metrics: NetworkMetrics,
    pub scheduler_metrics: SchedulerMetrics,
}

pub struct PerformanceMonitor {
    history: Mutex<Vec<PerformanceSnapshot>>,
    max_history_size: usize,
    last_snapshot: Mutex<Option<PerformanceSnapshot>>,
    
    cpu_total_cycles: AtomicU64,
    cpu_idle_cycles: AtomicU64,
    cpu_user_cycles: AtomicU64,
    cpu_kernel_cycles: AtomicU64,
    cpu_context_switches: AtomicU64,
    cpu_interrupts: AtomicU64,
    
    mem_page_faults: AtomicU64,
    mem_page_allocations: AtomicU64,
    mem_page_deallocations: AtomicU64,
    
    io_bytes_read: AtomicU64,
    io_bytes_written: AtomicU64,
    io_read_operations: AtomicU64,
    io_write_operations: AtomicU64,
    io_active_requests: AtomicUsize,
    
    net_bytes_received: AtomicU64,
    net_bytes_sent: AtomicU64,
    net_packets_received: AtomicU64,
    net_packets_sent: AtomicU64,
    net_total_connections: AtomicU64,
    
    scheduler_total_tasks: AtomicU64,
    scheduler_running_tasks: AtomicU64,
    scheduler_runnable_tasks: AtomicU64,
    scheduler_blocked_tasks: AtomicU64,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            history: Mutex::new(Vec::new()),
            max_history_size: 1000,
            last_snapshot: Mutex::new(None),
            
            cpu_total_cycles: AtomicU64::new(0),
            cpu_idle_cycles: AtomicU64::new(0),
            cpu_user_cycles: AtomicU64::new(0),
            cpu_kernel_cycles: AtomicU64::new(0),
            cpu_context_switches: AtomicU64::new(0),
            cpu_interrupts: AtomicU64::new(0),
            
            mem_page_faults: AtomicU64::new(0),
            mem_page_allocations: AtomicU64::new(0),
            mem_page_deallocations: AtomicU64::new(0),
            
            io_bytes_read: AtomicU64::new(0),
            io_bytes_written: AtomicU64::new(0),
            io_read_operations: AtomicU64::new(0),
            io_write_operations: AtomicU64::new(0),
            io_active_requests: AtomicUsize::new(0),
            
            net_bytes_received: AtomicU64::new(0),
            net_bytes_sent: AtomicU64::new(0),
            net_packets_received: AtomicU64::new(0),
            net_packets_sent: AtomicU64::new(0),
            net_total_connections: AtomicU64::new(0),
            
            scheduler_total_tasks: AtomicU64::new(0),
            scheduler_running_tasks: AtomicU64::new(0),
            scheduler_runnable_tasks: AtomicU64::new(0),
            scheduler_blocked_tasks: AtomicU64::new(0),
        }
    }
    
    pub fn record_cycle(&self, ty: CycleType) {
        let cycles = self.read_cpu_cycles();
        self.cpu_total_cycles.fetch_add(cycles, Ordering::Relaxed);
        
        match ty {
            CycleType::Idle => {
                self.cpu_idle_cycles.fetch_add(cycles, Ordering::Relaxed);
            }
            CycleType::User => {
                self.cpu_user_cycles.fetch_add(cycles, Ordering::Relaxed);
            }
            CycleType::Kernel => {
                self.cpu_kernel_cycles.fetch_add(cycles, Ordering::Relaxed);
            }
        }
    }
    
    pub fn record_context_switch(&self) {
        self.cpu_context_switches.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_interrupt(&self) {
        self.cpu_interrupts.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_page_fault(&self) {
        self.mem_page_faults.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_page_allocation(&self) {
        self.mem_page_allocations.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_page_deallocation(&self) {
        self.mem_page_deallocations.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_io_read(&self, bytes: u64) {
        self.io_bytes_read.fetch_add(bytes, Ordering::Relaxed);
        self.io_read_operations.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_io_write(&self, bytes: u64) {
        self.io_bytes_written.fetch_add(bytes, Ordering::Relaxed);
        self.io_write_operations.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn io_request_start(&self) {
        self.io_active_requests.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn io_request_complete(&self) {
        self.io_active_requests.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn record_network_receive(&self, bytes: u64, packets: u64) {
        self.net_bytes_received.fetch_add(bytes, Ordering::Relaxed);
        self.net_packets_received.fetch_add(packets, Ordering::Relaxed);
    }
    
    pub fn record_network_send(&self, bytes: u64, packets: u64) {
        self.net_bytes_sent.fetch_add(bytes, Ordering::Relaxed);
        self.net_packets_sent.fetch_add(packets, Ordering::Relaxed);
    }
    
    pub fn record_connection_open(&self) {
        self.net_total_connections.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn take_snapshot(&self) -> PerformanceSnapshot {
        let timestamp = self.get_timestamp();
        
        let cpu_metrics = CpuMetrics {
            total_cycles: self.cpu_total_cycles.load(Ordering::Relaxed),
            idle_cycles: self.cpu_idle_cycles.load(Ordering::Relaxed),
            user_cycles: self.cpu_user_cycles.load(Ordering::Relaxed),
            kernel_cycles: self.cpu_kernel_cycles.load(Ordering::Relaxed),
            context_switches: self.cpu_context_switches.load(Ordering::Relaxed),
            interrupts: self.cpu_interrupts.load(Ordering::Relaxed),
            frequency_mhz: self.get_cpu_frequency(),
        };
        
        let memory_metrics = MemoryMetrics {
            total_bytes: self.get_total_memory(),
            free_bytes: self.get_free_memory(),
            used_bytes: self.get_used_memory(),
            cached_bytes: self.get_cached_memory(),
            page_faults: self.mem_page_faults.load(Ordering::Relaxed),
            page_allocations: self.mem_page_allocations.load(Ordering::Relaxed),
            page_deallocations: self.mem_page_deallocations.load(Ordering::Relaxed),
        };
        
        let io_metrics = IoMetrics {
            bytes_read: self.io_bytes_read.load(Ordering::Relaxed),
            bytes_written: self.io_bytes_written.load(Ordering::Relaxed),
            read_operations: self.io_read_operations.load(Ordering::Relaxed),
            write_operations: self.io_write_operations.load(Ordering::Relaxed),
            active_requests: self.io_active_requests.load(Ordering::Relaxed) as u64,
        };
        
        let network_metrics = NetworkMetrics {
            bytes_received: self.net_bytes_received.load(Ordering::Relaxed),
            bytes_sent: self.net_bytes_sent.load(Ordering::Relaxed),
            packets_received: self.net_packets_received.load(Ordering::Relaxed),
            packets_sent: self.net_packets_sent.load(Ordering::Relaxed),
            active_connections: self.get_active_connections(),
            total_connections: self.net_total_connections.load(Ordering::Relaxed),
        };
        
        let scheduler_metrics = SchedulerMetrics {
            total_tasks: self.scheduler_total_tasks.load(Ordering::Relaxed),
            running_tasks: self.scheduler_running_tasks.load(Ordering::Relaxed),
            runnable_tasks: self.scheduler_runnable_tasks.load(Ordering::Relaxed),
            blocked_tasks: self.scheduler_blocked_tasks.load(Ordering::Relaxed),
            context_switches: self.cpu_context_switches.load(Ordering::Relaxed),
            average_load: self.calculate_average_load(),
        };
        
        PerformanceSnapshot {
            timestamp,
            cpu_metrics,
            memory_metrics,
            io_metrics,
            network_metrics,
            scheduler_metrics,
        }
    }
    
    pub fn snapshot_and_store(&self) {
        let snapshot = self.take_snapshot();
        let mut history = self.history.lock();
        history.push(snapshot.clone());
        
        if history.len() > self.max_history_size {
            history.remove(0);
        }
        
        *self.last_snapshot.lock() = Some(snapshot);
    }
    
    pub fn get_last_snapshot(&self) -> Option<PerformanceSnapshot> {
        self.last_snapshot.lock().clone()
    }
    
    pub fn get_history(&self, limit: Option<usize>) -> Vec<PerformanceSnapshot> {
        let history = self.history.lock();
        let len = limit.unwrap_or(history.len()).min(history.len());
        history.iter().cloned().rev().take(len).collect()
    }
    
    pub fn calculate_cpu_usage(&self) -> f64 {
        let total = self.cpu_total_cycles.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let idle = self.cpu_idle_cycles.load(Ordering::Relaxed);
        ((total - idle) as f64 / total as f64) * 100.0
    }
    
    pub fn calculate_memory_usage(&self) -> f64 {
        let total = self.get_total_memory();
        if total == 0 {
            return 0.0;
        }
        let used = self.get_used_memory();
        (used as f64 / total as f64) * 100.0
    }
    
    pub fn get_metrics_summary(&self) -> BTreeMap<String, String> {
        let snapshot = self.take_snapshot();
        let mut summary = BTreeMap::new();
        
        summary.insert("timestamp".to_string(), format!("{}", snapshot.timestamp));
        
        summary.insert("cpu_usage_percent".to_string(), format!("{:.2}", self.calculate_cpu_usage()));
        summary.insert("cpu_frequency_mhz".to_string(), format!("{}", snapshot.cpu_metrics.frequency_mhz));
        summary.insert("context_switches".to_string(), format!("{}", snapshot.cpu_metrics.context_switches));
        summary.insert("interrupts".to_string(), format!("{}", snapshot.cpu_metrics.interrupts));
        
        summary.insert("memory_usage_percent".to_string(), format!("{:.2}", self.calculate_memory_usage()));
        summary.insert("memory_used_bytes".to_string(), format!("{}", snapshot.memory_metrics.used_bytes));
        summary.insert("memory_free_bytes".to_string(), format!("{}", snapshot.memory_metrics.free_bytes));
        summary.insert("page_faults".to_string(), format!("{}", snapshot.memory_metrics.page_faults));
        
        summary.insert("io_bytes_read".to_string(), format!("{}", snapshot.io_metrics.bytes_read));
        summary.insert("io_bytes_written".to_string(), format!("{}", snapshot.io_metrics.bytes_written));
        summary.insert("io_active_requests".to_string(), format!("{}", snapshot.io_metrics.active_requests));
        
        summary.insert("network_bytes_received".to_string(), format!("{}", snapshot.network_metrics.bytes_received));
        summary.insert("network_bytes_sent".to_string(), format!("{}", snapshot.network_metrics.bytes_sent));
        summary.insert("network_active_connections".to_string(), format!("{}", snapshot.network_metrics.active_connections));
        
        summary.insert("scheduler_total_tasks".to_string(), format!("{}", snapshot.scheduler_metrics.total_tasks));
        summary.insert("scheduler_running_tasks".to_string(), format!("{}", snapshot.scheduler_metrics.running_tasks));
        summary.insert("scheduler_average_load".to_string(), format!("{:.2}", snapshot.scheduler_metrics.average_load));
        
        summary
    }
    
    pub fn reset(&self) {
        self.cpu_total_cycles.store(0, Ordering::Relaxed);
        self.cpu_idle_cycles.store(0, Ordering::Relaxed);
        self.cpu_user_cycles.store(0, Ordering::Relaxed);
        self.cpu_kernel_cycles.store(0, Ordering::Relaxed);
        self.cpu_context_switches.store(0, Ordering::Relaxed);
        self.cpu_interrupts.store(0, Ordering::Relaxed);
        
        self.mem_page_faults.store(0, Ordering::Relaxed);
        self.mem_page_allocations.store(0, Ordering::Relaxed);
        self.mem_page_deallocations.store(0, Ordering::Relaxed);
        
        self.io_bytes_read.store(0, Ordering::Relaxed);
        self.io_bytes_written.store(0, Ordering::Relaxed);
        self.io_read_operations.store(0, Ordering::Relaxed);
        self.io_write_operations.store(0, Ordering::Relaxed);
        
        self.net_bytes_received.store(0, Ordering::Relaxed);
        self.net_bytes_sent.store(0, Ordering::Relaxed);
        self.net_packets_received.store(0, Ordering::Relaxed);
        self.net_packets_sent.store(0, Ordering::Relaxed);
        self.net_total_connections.store(0, Ordering::Relaxed);
        
        self.history.lock().clear();
        *self.last_snapshot.lock() = None;
    }
    
    fn read_cpu_cycles(&self) -> u64 {
        1
    }
    
    fn get_timestamp(&self) -> u64 {
        0
    }
    
    fn get_cpu_frequency(&self) -> u64 {
        2400
    }
    
    fn get_total_memory(&self) -> u64 {
        0
    }
    
    fn get_free_memory(&self) -> u64 {
        0
    }
    
    fn get_used_memory(&self) -> u64 {
        0
    }
    
    fn get_cached_memory(&self) -> u64 {
        0
    }
    
    fn get_active_connections(&self) -> u64 {
        0
    }
    
    fn calculate_average_load(&self) -> f64 {
        let total = self.scheduler_total_tasks.load(Ordering::Relaxed);
        let running = self.scheduler_running_tasks.load(Ordering::Relaxed);
        let runnable = self.scheduler_runnable_tasks.load(Ordering::Relaxed);
        
        if total == 0 {
            return 0.0;
        }
        (running + runnable) as f64 / total as f64
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub static PERFORMANCE_MONITOR: PerformanceMonitor = PerformanceMonitor::new();

#[derive(Debug, Clone, Copy)]
pub enum CycleType {
    Idle,
    User,
    Kernel,
}
