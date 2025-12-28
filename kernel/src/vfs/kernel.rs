//! Kernel information in /sys/kernel

extern crate alloc;
use alloc::{boxed::Box, string::{String, ToString}, sync::Arc};
use crate::vfs::{
    error::*,
    types::*,
    fs::InodeOps,
};
use super::fs::SysFsInode;

/// Create /sys/kernel root
pub fn create_root() -> VfsResult<Arc<dyn InodeOps>> {
    let root = Arc::new(SysFsInode::new_dir(2000));
    
    // Add common kernel files
    let mut children = root.children().lock();
    
    // /sys/kernel/version - kernel version
    children.insert("version".to_string(), Arc::new(SysFsInode::new_file(
        2001,
        Box::new(|| {
            format!(
                "NOS {}\n",
                env!("CARGO_PKG_VERSION")
            )
        })
    )));
    
    // /sys/kernel/hostname - system hostname
    children.insert("hostname".to_string(), Arc::new(SysFsInode::new_file(
        2002,
        Box::new(|| {
            "nos\n".to_string()
        })
    )));
    
    // /sys/kernel/domainname - system domain name
    children.insert("domainname".to_string(), Arc::new(SysFsInode::new_file(
        2003,
        Box::new(|| {
            "(none)\n".to_string()
        })
    )));
    
    // /sys/kernel/osrelease - kernel release version
    children.insert("osrelease".to_string(), Arc::new(SysFsInode::new_file(
        2004,
        Box::new(|| {
            format!("{}\n", env!("CARGO_PKG_VERSION"))
        })
    )));
    
    // /sys/kernel/ostype - operating system type
    children.insert("ostype".to_string(), Arc::new(SysFsInode::new_file(
        2005,
        Box::new(|| {
            "NOS\n".to_string()
        })
    )));
    
    // /sys/kernel/metrics - runtime metrics
    children.insert("metrics".to_string(), Arc::new(SysFsInode::new_file(
        2006,
        Box::new(|| {
            format_kernel_metrics()
        })
    )));
    
    drop(children);
    
    Ok(root)
}

/// Format /sys/kernel/metrics content
fn format_kernel_metrics() -> String {
    use crate::monitoring::metrics::get_metrics_collector;
    
    let collector = get_metrics_collector();
    collector.update_system_metrics();
    let metrics = collector.collect_metrics();
    
    // Get kernel-specific metrics
    let error_stats = crate::error::get_error_stats();
    
    // Get process statistics
    let proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
    let total_processes = proc_table.iter().count();
    let running_processes = proc_table.iter()
        .filter(|p| p.state == crate::subsystems::process::manager::ProcState::Running)
        .count();
    drop(proc_table);
    
    // Get memory statistics
    let (free_pages, total_pages) = crate::subsystems::mm::phys::mem_stats();
    let used_pages = total_pages.saturating_sub(free_pages);
    let page_size = 4096;
    let total_bytes = total_pages * page_size;
    let free_bytes = free_pages * page_size;
    let used_bytes = used_pages * page_size;
    
    // Format metrics in key-value format (simpler than Prometheus format)
    let mut output = String::new();
    
    output.push_str("# Kernel Runtime Metrics\n");
    output.push_str("# Format: key=value\n\n");
    
    // Process metrics
    output.push_str(&format!("processes_total={}\n", total_processes));
    output.push_str(&format!("processes_running={}\n", running_processes));
    
    // Memory metrics
    output.push_str(&format!("memory_total_bytes={}\n", total_bytes));
    output.push_str(&format!("memory_free_bytes={}\n", free_bytes));
    output.push_str(&format!("memory_used_bytes={}\n", used_bytes));
    output.push_str(&format!("memory_total_pages={}\n", total_pages));
    output.push_str(&format!("memory_free_pages={}\n", free_pages));
    output.push_str(&format!("memory_used_pages={}\n", used_pages));
    
    // Error metrics
    output.push_str(&format!("errors_total={}\n", error_stats.total_errors));
    output.push_str(&format!("errors_critical={}\n", error_stats.critical_errors));
    
    // Metrics from collector
    output.push_str("\n# Metrics from metrics collector\n");
    for (name, value) in metrics.iter() {
        output.push_str(&format!("{}={}\n", name, value));
    }
    
    // Uptime
    let uptime_secs = crate::subsystems::time::uptime_ms() as f64 / 1000.0;
    output.push_str(&format!("uptime_seconds={:.2}\n", uptime_secs));
    
    // CPU count
    let cpu_count = crate::cpu::ncpus();
    output.push_str(&format!("cpu_count={}\n", cpu_count));
    
    output
}

/// Create /sys/module root
pub fn create_module_root() -> VfsResult<Arc<dyn InodeOps>> {
    Ok(Arc::new(SysFsInode::new_dir(3000)))
}

