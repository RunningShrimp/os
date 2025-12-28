//! System information files in /proc
//!
//! Provides system-wide information files like /proc/meminfo, /proc/stat, etc.

extern crate alloc;
use alloc::{boxed::Box, string::String, sync::Arc};
use crate::subsystems::sync::Mutex;
use crate::vfs::{
    error::*,
    types::*,
    fs::InodeOps,
};
use super::fs::ProcFsInode;

/// Create root /proc/sys inode
pub fn create_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(ProcFsInode::new_dir(1000)))
}

/// Create /proc/meminfo file
pub fn create_meminfo() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(ProcFsInode::new_file(1001, Box::new(|| {
        format_meminfo()
    }))))
}

/// Create /proc/stat file
pub fn create_stat() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(ProcFsInode::new_file(1002, Box::new(|| {
        format_stat()
    }))))
}

/// Create /proc/version file
pub fn create_version() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(ProcFsInode::new_file(1003, Box::new(|| {
        format_version()
    }))))
}

/// Create /proc/uptime file
pub fn create_uptime() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(ProcFsInode::new_file(1004, Box::new(|| {
        format_uptime()
    }))))
}

/// Create /proc/loadavg file
pub fn create_loadavg() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(ProcFsInode::new_file(1005, Box::new(|| {
        format_loadavg()
    }))))
}

/// Create /proc/metrics file
pub fn create_metrics() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(ProcFsInode::new_file(1006, Box::new(|| {
        format_metrics()
    }))))
}

/// Format /proc/meminfo content
fn format_meminfo() -> String {
    // Get memory statistics
    let (free_pages, total_pages) = crate::subsystems::mm::phys::mem_stats();
    let used_pages = total_pages.saturating_sub(free_pages);
    
    // Convert to KB (assuming page size 4KB)
    let page_size = 4096;
    let total_kb = total_pages * page_size / 1024;
    let free_kb = free_pages * page_size / 1024;
    let used_kb = used_pages * page_size / 1024;
    
    format!(
        "MemTotal:       {} kB\n\
         MemFree:        {} kB\n\
         MemAvailable:   {} kB\n\
         Buffers:        0 kB\n\
         Cached:         0 kB\n\
         SwapTotal:      0 kB\n\
         SwapFree:       0 kB\n\
         Dirty:          0 kB\n\
         Writeback:      0 kB\n\
         AnonPages:      {} kB\n\
         Mapped:         0 kB\n\
         Shmem:          0 kB\n\
         Slab:           0 kB\n\
         SReclaimable:   0 kB\n\
         SUnreclaim:     0 kB\n\
         KernelStack:    0 kB\n\
         PageTables:     0 kB\n\
         NFS_Unstable:   0 kB\n\
         Bounce:         0 kB\n\
         WritebackTmp:   0 kB\n\
         CommitLimit:    {} kB\n\
         Committed_AS:   {} kB\n\
         VmallocTotal:   0 kB\n\
         VmallocUsed:    0 kB\n\
         VmallocChunk:   0 kB\n",
        total_kb, free_kb, free_kb, used_kb, total_kb, used_kb
    )
}

/// Format /proc/stat content
fn format_stat() -> String {
    // Get CPU statistics (simplified)
    let cpu_user = 0u64;
    let cpu_nice = 0u64;
    let cpu_system = 0u64;
    let cpu_idle = 0u64;
    let cpu_iowait = 0u64;
    let cpu_irq = 0u64;
    let cpu_softirq = 0u64;
    
    format!(
        "cpu  {} {} {} {} {} {} {}\n\
         cpu0 {} {} {} {} {} {} {}\n\
         intr 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n\
         ctxt 0\n\
         btime 0\n\
         processes 0\n\
         procs_running 0\n\
         procs_blocked 0\n\
         softirq 0 0 0 0 0 0 0 0 0 0 0\n",
        cpu_user, cpu_nice, cpu_system, cpu_idle, cpu_iowait, cpu_irq, cpu_softirq,
        cpu_user, cpu_nice, cpu_system, cpu_idle, cpu_iowait, cpu_irq, cpu_softirq
    )
}

/// Format /proc/version content
fn format_version() -> String {
    format!(
        "NOS version {} {} {} {} {}\n",
        env!("CARGO_PKG_VERSION"),
        option_env!("BUILD_DATE").unwrap_or("unknown"),
        option_env!("BUILD_USER").unwrap_or("unknown"),
        option_env!("BUILD_HOST").unwrap_or("unknown"),
        option_env!("BUILD_ARCH").unwrap_or("unknown")
    )
}

/// Format /proc/uptime content
fn format_uptime() -> String {
    // Get uptime in seconds (convert from milliseconds)
    let uptime_ms = crate::subsystems::time::uptime_ms();
    let uptime_secs = uptime_ms as f64 / 1000.0;
    let uptime_idle = 0.0; // Idle time
    
    format!("{:.2} {:.2}\n", uptime_secs, uptime_idle)
}

/// Format /proc/loadavg content
fn format_loadavg() -> String {
    // Get load average (simplified)
    let load1 = 0.0;
    let load5 = 0.0;
    let load15 = 0.0;
    
    // Get running/total processes
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let total_procs = proc_table.len();
    let running_procs = proc_table.iter()
        .filter(|proc| proc.state == crate::process::manager::ProcState::Running)
        .count();
    drop(proc_table);
    
    format!("{:.2} {:.2} {:.2} {}/{} 0\n", load1, load5, load15, running_procs, total_procs)
}

/// Format /proc/metrics content
fn format_metrics() -> String {
    use crate::monitoring::metrics::get_metrics_collector;
    
    let collector = get_metrics_collector();
    collector.update_system_metrics();
    let metrics = collector.collect_metrics();
    
    // Get error statistics
    let error_stats = crate::error::get_error_stats();
    
    // Get process statistics
    let proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    let total_processes = proc_table.iter().count();
    let running_processes = proc_table.iter()
        .filter(|p| p.state == crate::subsystems::process::manager::ProcState::Running)
        .count();
    let sleeping_processes = proc_table.iter()
        .filter(|p| p.state == crate::subsystems::process::manager::ProcState::Sleeping)
        .count();
    let zombie_processes = proc_table.iter()
        .filter(|p| p.state == crate::subsystems::process::manager::ProcState::Zombie)
        .count();
    drop(proc_table);
    
    // Get memory statistics
    let (free_pages, total_pages) = crate::subsystems::mm::phys::mem_stats();
    let used_pages = total_pages.saturating_sub(free_pages);
    let page_size = 4096;
    let total_bytes = total_pages * page_size;
    let free_bytes = free_pages * page_size;
    let used_bytes = used_pages * page_size;
    
    // Get CPU statistics
    let cpu_count = crate::cpu::ncpus();
    
    // Format metrics in Prometheus-like format
    let mut output = String::new();
    
    // System metrics
    output.push_str("# HELP system_processes_total Total number of processes\n");
    output.push_str("# TYPE system_processes_total gauge\n");
    output.push_str(&format!("system_processes_total {}\n", total_processes));
    
    output.push_str("# HELP system_processes_running Number of running processes\n");
    output.push_str("# TYPE system_processes_running gauge\n");
    output.push_str(&format!("system_processes_running {}\n", running_processes));
    
    output.push_str("# HELP system_processes_sleeping Number of sleeping processes\n");
    output.push_str("# TYPE system_processes_sleeping gauge\n");
    output.push_str(&format!("system_processes_sleeping {}\n", sleeping_processes));
    
    output.push_str("# HELP system_processes_zombie Number of zombie processes\n");
    output.push_str("# TYPE system_processes_zombie gauge\n");
    output.push_str(&format!("system_processes_zombie {}\n", zombie_processes));
    
    // Memory metrics
    output.push_str("# HELP system_memory_total_bytes Total memory in bytes\n");
    output.push_str("# TYPE system_memory_total_bytes gauge\n");
    output.push_str(&format!("system_memory_total_bytes {}\n", total_bytes));
    
    output.push_str("# HELP system_memory_free_bytes Free memory in bytes\n");
    output.push_str("# TYPE system_memory_free_bytes gauge\n");
    output.push_str(&format!("system_memory_free_bytes {}\n", free_bytes));
    
    output.push_str("# HELP system_memory_used_bytes Used memory in bytes\n");
    output.push_str("# TYPE system_memory_used_bytes gauge\n");
    output.push_str(&format!("system_memory_used_bytes {}\n", used_bytes));
    
    // CPU metrics
    output.push_str("# HELP system_cpu_count Number of CPUs\n");
    output.push_str("# TYPE system_cpu_count gauge\n");
    output.push_str(&format!("system_cpu_count {}\n", cpu_count));
    
    // Error metrics
    output.push_str("# HELP system_errors_total Total number of errors\n");
    output.push_str("# TYPE system_errors_total counter\n");
    output.push_str(&format!("system_errors_total {}\n", error_stats.total_errors));
    
    output.push_str("# HELP system_errors_critical_total Total number of critical errors\n");
    output.push_str("# TYPE system_errors_critical_total counter\n");
    output.push_str(&format!("system_errors_critical_total {}\n", error_stats.critical_errors));
    
    // Metrics from collector
    for (name, value) in metrics.iter() {
        output.push_str(&format!("# HELP {} Metric from metrics collector\n", name));
        output.push_str(&format!("# TYPE {} gauge\n", name));
        output.push_str(&format!("{} {}\n", name, value));
    }
    
    // Uptime
    let uptime_secs = crate::subsystems::time::uptime_ms() as f64 / 1000.0;
    output.push_str("# HELP system_uptime_seconds System uptime in seconds\n");
    output.push_str("# TYPE system_uptime_seconds gauge\n");
    output.push_str(&format!("system_uptime_seconds {:.2}\n", uptime_secs));
    
    output
}

