//! Performance Counter Examples
//!
//! This file demonstrates usage of the performance counter system.

use alloc::sync::Arc;

use crate::perf::{
    counter_manager::{
        get_counter_manager, export_counters, get_counters_summary,
    },
    hardware::{
        HardwareCounterType, get_hw_counter_manager, increment_hw_counter,
    },
    software::{
        SoftwareCounterType, get_sw_counter_manager, increment_sw_counter,
    },
    init_all,
    SimpleCounter,
};

/// Example 1: Basic counter initialization and snapshot
pub fn example_basic_usage() {
    // Initialize all performance counters
    init_all();

    // Get counter manager
    let manager = get_counter_manager();

    // Simulate some work
    increment_hw_counter(HardwareCounterType::InstructionsRetired, 1000);
    increment_sw_counter(SoftwareCounterType::SyscallRead, 5);

    // Create snapshot
    let snapshot = manager.snapshot();
    println!("Snapshot timestamp: {}", snapshot.timestamp);

    // Export to /proc format
    let proc_output = export_counters();
    println!("{}\n", proc_output);

    // Get summary
    let summary = get_counters_summary();
    println!("Counters Summary:");
    for (key, value) in summary.iter() {
        println!("  {}: {}", key, value);
    }
}

/// Example 2: Custom counter registration
pub fn example_custom_counters() {
    init_all();
    let manager = get_counter_manager();

    // Create and register custom counter
    let counter = Arc::new(SimpleCounter::new_custom(
        "cache_coherency_traffic".to_string(),
        "Cache coherency operations".to_string(),
    ));

    match manager.register_counter(counter) {
        Ok(counter_id) => {
            println!("Registered custom counter with ID: {}", counter_id);

            // Increment counter
            manager.increment_custom("cache_coherency_traffic", 100);

            // Read value
            if let Some(value) = manager.get_custom_counter("cache_coherency_traffic") {
                println!("Custom counter value: {}", value);
            }
        }
        Err(e) => {
            println!("Failed to register counter: {}", e);
        }
    }
}

/// Example 3: Per-CPU statistics
pub fn example_per_cpu_stats() {
    init_all();

    // Get hardware and software managers
    let hw_manager = get_hw_counter_manager();
    let sw_manager = get_sw_counter_manager();

    // Display per-CPU metrics
    for cpu_id in 0..8 {
        let ipc = hw_manager.calculate_ipc(cpu_id);
        let l1_miss_rate = hw_manager.calculate_cache_miss_rate(cpu_id, 1);
        let l2_miss_rate = hw_manager.calculate_cache_miss_rate(cpu_id, 2);
        let l3_miss_rate = hw_manager.calculate_cache_miss_rate(cpu_id, 3);
        let branch_miss_rate = hw_manager.calculate_branch_miss_rate(cpu_id);
        let tlb_miss_rate = hw_manager.calculate_tlb_miss_rate(cpu_id);

        println!("CPU {} Metrics:", cpu_id);
        println!("  IPC: {:.2}", ipc);
        println!("  L1 miss rate: {:.2}%", l1_miss_rate);
        println!("  L2 miss rate: {:.2}%", l2_miss_rate);
        println!("  L3 miss rate: {:.2}%", l3_miss_rate);
        println!("  Branch miss rate: {:.2}%", branch_miss_rate);
        println!("  TLB miss rate: {:.2}%", tlb_miss_rate);
        println!();
    }
}

/// Example 4: Real-time monitoring
pub fn example_realtime_monitoring() {
    init_all();
    let manager = get_counter_manager();

    // Take initial snapshot
    let snapshot1 = manager.snapshot();

    // Simulate some workload
    for _ in 0..100 {
        increment_sw_counter(SoftwareCounterType::SyscallWrite, 1);
        increment_sw_counter(SoftwareCounterType::ContextSwitchTotal, 1);
    }

    // Take final snapshot
    let snapshot2 = manager.snapshot();

    // Calculate deltas
    let time_delta = snapshot2.timestamp - snapshot1.timestamp;
    let write_delta = snapshot2
        .aggregated_software
        .get("syscall_write")
        .unwrap_or(&0)
        - snapshot1.aggregated_software.get("syscall_write").unwrap_or(&0);
    let ctx_switch_delta = snapshot2
        .aggregated_software
        .get("context_switch_total")
        .unwrap_or(&0)
        - snapshot1
            .aggregated_software
            .get("context_switch_total")
            .unwrap_or(&0);

    println!("Monitoring Results (over {} ns):", time_delta);
    println!("  Write syscalls: {}", write_delta);
    println!("  Context switches: {}", ctx_switch_delta);
    println!("  Rate: {:.2} writes/ms", (write_delta as f64 / time_delta as f64) * 1_000_000.0);
}

/// Example 5: Export formats
pub fn example_export_formats() {
    init_all();
    let manager = get_counter_manager();

    // /proc format
    println!("=== /proc Format ===");
    println!("{}", manager.export_to_proc());
    println!();

    // JSON format
    println!("=== JSON Format ===");
    println!("{}", manager.export_to_json());
}

/// Example 6: Lock contention monitoring
pub fn example_lock_contention() {
    init_all();
    let sw_manager = get_sw_counter_manager();

    // Simulate lock operations
    increment_sw_counter(SoftwareCounterType::LockMutexAcquisitions, 1000);
    increment_sw_counter(SoftwareCounterType::LockMutexContentions, 50);

    increment_sw_counter(SoftwareCounterType::LockSpinAcquisitions, 500);
    increment_sw_counter(SoftwareCounterType::LockSpinContentions, 25);

    // Calculate contention rate
    let contention_rate = sw_manager.calculate_lock_contention_rate();
    println!("Lock Contention Rate: {:.2}%", contention_rate);
}

/// Example 7: System call profiling
pub fn example_syscall_profiling() {
    init_all();

    // Simulate various syscalls
    increment_sw_counter(SoftwareCounterType::SyscallRead, 150);
    increment_sw_counter(SoftwareCounterType::SyscallWrite, 100);
    increment_sw_counter(SoftwareCounterType::SyscallOpen, 20);
    increment_sw_counter(SoftwareCounterType::SyscallClose, 20);
    increment_sw_counter(SoftwareCounterType::SyscallMmap, 5);

    let manager = get_counter_manager();
    let snapshot = manager.snapshot();

    println!("System Call Profile:");
    let syscall_names = [
        "syscall_read",
        "syscall_write",
        "syscall_open",
        "syscall_close",
        "syscall_mmap",
    ];

    for name in &syscall_names {
        let value = snapshot.aggregated_software.get(*name).unwrap_or(&0);
        println!("  {}: {}", name, value);
    }
}

/// Example 8: Performance health check
pub fn example_health_check() {
    init_all();
    let manager = get_counter_manager();

    let hw_manager = manager.hardware_manager();
    let sw_manager = manager.software_manager();

    println!("Performance Health Check:");
    println!();

    // CPU efficiency
    let cpu_id = 0;
    let ipc = hw_manager.calculate_ipc(cpu_id);
    println!("  IPC: {:.2} (good if > 1.0)", ipc);

    // Cache efficiency
    let l1_miss = hw_manager.calculate_cache_miss_rate(cpu_id, 1);
    println!("  L1 miss rate: {:.2}% (good if < 5%)", l1_miss);

    // Branch prediction
    let branch_miss = hw_manager.calculate_branch_miss_rate(cpu_id);
    println!("  Branch miss rate: {:.2}% (good if < 5%)", branch_miss);

    // Lock contention
    let lock_contention = sw_manager.calculate_lock_contention_rate();
    println!("  Lock contention: {:.2}% (good if < 10%)", lock_contention);

    // Context switch rate
    let ctx_rate = sw_manager.calculate_context_switch_rate();
    println!("  Context switch rate: {:.2} per second", ctx_rate);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_usage() {
        example_basic_usage();
    }

    #[test]
    fn test_custom_counters() {
        example_custom_counters();
    }

    #[test]
    fn test_per_cpu_stats() {
        example_per_cpu_stats();
    }

    #[test]
    fn test_realtime_monitoring() {
        example_realtime_monitoring();
    }

    #[test]
    fn test_export_formats() {
        example_export_formats();
    }

    #[test]
    fn test_lock_contention() {
        example_lock_contention();
    }

    #[test]
    fn test_syscall_profiling() {
        example_syscall_profiling();
    }

    #[test]
    fn test_health_check() {
        example_health_check();
    }
}
