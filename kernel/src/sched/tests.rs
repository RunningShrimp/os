//! # Scheduler Test Suite
//!
//! Comprehensive tests for the O(1) scheduler implementation,
//! covering basic functionality, real-time scheduling, context switching,
//! and load balancing.

use crate::prelude::*;
use crate::sched::mod::{O1Scheduler, PerCpuScheduler, SchedulerStats, StatsSnapshot, DEFAULT_TIMESLICE, MAX_PRIORITY, MAX_CPUS};
use crate::platform::arch::cpuid;
use crate::subsystems::sync::spinlock::SpinLock;

/// Test helper to create a mock task ID
#[cfg(test)]
fn mock_task_id(cpu: usize, index: usize) -> usize {
    (cpu << 16) | index
}

// ============================================================================
// O(1) Scheduling Tests
// ============================================================================

#[cfg(test)]
mod o1_scheduling_tests {
    use super::*;

    /// Test O(1) pick next task - basic functionality
    #[test]
    fn test_o1_pick_next_task() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Enqueue tasks with different priorities
        let task_low = mock_task_id(cpu_id, 1);
        let task_mid = mock_task_id(cpu_id, 2);
        let task_high = mock_task_id(cpu_id, 3);

        // Lower number = higher priority in typical schedulers
        scheduler.enqueue_task(task_low, 100);
        scheduler.enqueue_task(task_mid, 50);
        scheduler.enqueue_task(task_high, 10);

        // Should pick highest priority (lowest number) first
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_high), "Should pick highest priority task");

        // Next should be mid priority
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_mid), "Should pick mid priority task");

        // Finally low priority
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_low), "Should pick low priority task");

        // Queue should be empty
        let next = scheduler.pick_next_task();
        assert_eq!(next, None, "Should return None when queue is empty");
    }

    /// Test O(1) enqueue/dequeue operations
    #[test]
    fn test_o1_enqueue_dequeue() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Test enqueue
        for i in 0..10 {
            let task_id = mock_task_id(cpu_id, i);
            scheduler.enqueue_task(task_id, i * 10);
        }

        // Verify task count
        let stats = scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.task_count(), 10, "Should have 10 tasks enqueued");

        // Test dequeue - should come out in priority order (0 first)
        for i in 0..10 {
            let task_id = scheduler.pick_next_task();
            assert_eq!(task_id, Some(mock_task_id(cpu_id, i)), "Task {} should be picked", i);
        }

        // Verify all tasks dequeued
        let next = scheduler.pick_next_task();
        assert_eq!(next, None, "All tasks should be dequeued");
    }

    /// Test O(1) priority bitmap operations
    #[test]
    fn test_o1_priority_bitmap() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Enqueue tasks at specific priorities
        scheduler.enqueue_task(mock_task_id(cpu_id, 1), 0);
        scheduler.enqueue_task(mock_task_id(cpu_id, 2), 31);
        scheduler.enqueue_task(mock_task_id(cpu_id, 3), 63);

        // Verify bitmap has correct bits set
        let stats = scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        let bitmap = snapshot.priority_bitmap();

        assert!(bitmap & (1 << 0) != 0, "Priority 0 bit should be set");
        assert!(bitmap & (1 << 31) != 0, "Priority 31 bit should be set");
        assert!(bitmap & (1 << 63) != 0, "Priority 63 bit should be set");
    }

    /// Test O(1) scheduler with empty queues
    #[test]
    fn test_o1_empty_queue() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Empty queue should return None
        let next = scheduler.pick_next_task();
        assert_eq!(next, None, "Empty queue should return None");

        // Stats should reflect empty state
        let stats = scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.task_count(), 0, "Task count should be 0");
    }

    /// Test O(1) scheduler with invalid priorities
    #[test]
    fn test_o1_invalid_priority() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Enqueue with invalid priority (>= MAX_PRIORITY)
        scheduler.enqueue_task(mock_task_id(cpu_id, 1), MAX_PRIORITY);
        scheduler.enqueue_task(mock_task_id(cpu_id, 2), MAX_PRIORITY + 100);

        // These should be ignored
        let next = scheduler.pick_next_task();
        assert_eq!(next, None, "Invalid priority tasks should be ignored");
    }
}

// ============================================================================
// Real-Time Scheduling Tests
// ============================================================================

#[cfg(test)]
mod realtime_scheduling_tests {
    use super::*;

    /// Test Earliest Deadline First (EDF) scheduling
    #[test]
    fn test_edf_scheduling() {
        // EDF: tasks with earlier deadlines should be scheduled first
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // In our implementation, lower priority number = higher priority
        // Simulate EDF by using deadline as priority (earlier = lower number)
        let task_urgent = mock_task_id(cpu_id, 1);   // Deadline: 100 ticks
        let task_normal = mock_task_id(cpu_id, 2);   // Deadline: 200 ticks
        let task_relaxed = mock_task_id(cpu_id, 3);  // Deadline: 300 ticks

        scheduler.enqueue_task(task_urgent, 100);   // Highest priority
        scheduler.enqueue_task(task_normal, 200);
        scheduler.enqueue_task(task_relaxed, 300);  // Lowest priority

        // Should pick urgent (earliest deadline) first
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_urgent), "Should pick task with earliest deadline");

        // Then normal
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_normal), "Should pick task with normal deadline");

        // Finally relaxed
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_relaxed), "Should pick task with relaxed deadline");
    }

    /// Test Rate Monotonic scheduling
    #[test]
    fn test_rate_monotonic() {
        // Rate Monotonic: shorter period = higher priority
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Simulate tasks with different periods
        // Shorter period = lower priority number
        let task_fast = mock_task_id(cpu_id, 1);    // Period: 10ms
        let task_medium = mock_task_id(cpu_id, 2);   // Period: 50ms
        let task_slow = mock_task_id(cpu_id, 3);     // Period: 100ms

        scheduler.enqueue_task(task_fast, 10);    // Highest priority (fastest period)
        scheduler.enqueue_task(task_medium, 50);
        scheduler.enqueue_task(task_slow, 100);   // Lowest priority (slowest period)

        // Should pick fast (shortest period) first
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_fast), "Should pick task with shortest period");

        // Then medium
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_medium), "Should pick task with medium period");

        // Finally slow
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_slow), "Should pick task with longest period");
    }

    /// Test real-time task preemption
    #[test]
    fn test_rt_preemption() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Running a low priority task
        let task_low = mock_task_id(cpu_id, 1);
        scheduler.enqueue_task(task_low, 100);

        let _current = scheduler.pick_next_task();
        assert_eq!(_current, Some(task_low), "Low priority task should be running");

        // High priority real-time task arrives
        let task_high = mock_task_id(cpu_id, 2);
        scheduler.enqueue_task(task_high, 10);

        // Next pick should be high priority task (preemption)
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_high), "High priority task should preempt low priority task");
    }
}

// ============================================================================
// Context Switch Tests
// ============================================================================

#[cfg(test)]
mod context_switch_tests {
    use super::*;
    use crate::subsystems::time::get_time_ns;

    /// Test context switch latency
    #[test]
    fn test_switch_latency() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Measure time to switch tasks
        let start = get_time_ns();

        // Switch between tasks
        for i in 0..100 {
            let task_id = mock_task_id(cpu_id, i);
            scheduler.enqueue_task(task_id, i % MAX_PRIORITY);
            let _ = scheduler.pick_next_task();
        }

        let end = get_time_ns();
        let latency_ns = end.saturating_sub(start);
        let avg_latency_ns = latency_ns / 100;

        // Target: context switch < 5μs (5000ns)
        assert!(avg_latency_ns < 5000, "Average context switch latency should be < 5μs, got: {}ns", avg_latency_ns);

        // Check latency histogram
        let stats = scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        // Most switches should be in fast buckets
        let fast_switches = snapshot.latency_hist()[0] + snapshot.latency_hist()[1];
        assert!(fast_switches > 50, "At least 50% of switches should be fast (<10μs)");
    }

    /// Test voluntary context switch (yield)
    #[test]
    fn test_voluntary_switch() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Enqueue a task
        let task1 = mock_task_id(cpu_id, 1);
        let task2 = mock_task_id(cpu_id, 2);
        scheduler.enqueue_task(task1, 50);
        scheduler.enqueue_task(task2, 50);

        // Pick first task
        let current = scheduler.pick_next_task();
        assert_eq!(current, Some(task1), "Should pick task1");

        // Task1 yields voluntarily
        scheduler.yield_task(task1);

        // Should pick task2
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task2), "Should pick task2 after task1 yields");

        // Verify voluntary switch was recorded
        let stats = scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert!(snapshot.voluntary_switches() > 0, "Voluntary switches should be recorded");
    }

    /// Test preemption
    #[test]
    fn test_preemption() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Low priority task running
        let task_low = mock_task_id(cpu_id, 1);
        scheduler.enqueue_task(task_low, 100);
        let _current = scheduler.pick_next_task();

        // High priority task preempts
        let task_high = mock_task_id(cpu_id, 2);
        scheduler.enqueue_task(task_high, 10);

        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_high), "High priority task should preempt");

        // Verify preemption was recorded
        let stats = scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert!(snapshot.preemptions() > 0, "Preemptions should be recorded");
    }

    /// Test rapid context switches
    #[test]
    fn test_rapid_switches() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Simulate rapid switching between two tasks
        let task_a = mock_task_id(cpu_id, 1);
        let task_b = mock_task_id(cpu_id, 2);

        for _ in 0..1000 {
            scheduler.enqueue_task(task_a, 50);
            let _ = scheduler.pick_next_task();
            scheduler.yield_task(task_a);

            scheduler.enqueue_task(task_b, 50);
            let _ = scheduler.pick_next_task();
            scheduler.yield_task(task_b);
        }

        // Verify scheduler handles rapid switches
        let stats = scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert!(snapshot.voluntary_switches() >= 2000, "Should handle 2000+ rapid switches");
    }
}

// ============================================================================
// Load Balancing Tests
// ============================================================================

#[cfg(test)]
mod load_balancing_tests {
    use super::*;

    /// Test CPU migration
    #[test]
    fn test_cpu_migration() {
        let source_cpu = 0;
        let target_cpu = 1;

        // Add task to source CPU
        let task_id = mock_task_id(source_cpu, 1);
        O1Scheduler::add_task_to_cpu(task_id, 50, source_cpu);

        // Verify task is on source CPU
        let source_scheduler = O1Scheduler::get_cpu_scheduler(source_cpu);
        let stats = source_scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.task_count(), 1, "Task should be on source CPU");

        // Migrate task to target CPU
        O1Scheduler::migrate_task(task_id, source_cpu, target_cpu);

        // Verify task moved to target CPU
        let target_scheduler = O1Scheduler::get_cpu_scheduler(target_cpu);
        let stats = target_scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.task_count(), 1, "Task should be on target CPU");

        // Verify source CPU is empty
        let source_scheduler = O1Scheduler::get_cpu_scheduler(source_cpu);
        let stats = source_scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.task_count(), 0, "Source CPU should be empty");
    }

    /// Test multi-CPU load distribution
    #[test]
    fn test_multicpu_load_distribution() {
        let num_cpus = 4;
        let tasks_per_cpu = 10;

        // Distribute tasks across CPUs
        for cpu in 0..num_cpus {
            for i in 0..tasks_per_cpu {
                let task_id = mock_task_id(cpu, i);
                O1Scheduler::add_task_to_cpu(task_id, i % MAX_PRIORITY, cpu);
            }
        }

        // Verify load distribution
        for cpu in 0..num_cpus {
            let scheduler = O1Scheduler::get_cpu_scheduler(cpu);
            let stats = scheduler.detailed_stats();
            let snapshot = stats.snapshot();
            assert_eq!(snapshot.task_count(), tasks_per_cpu,
                "CPU {} should have {} tasks", cpu, tasks_per_cpu);
        }
    }

    /// Test CPU hotplug
    #[test]
    fn test_hotplug() {
        // Simulate CPU hotplug: CPU going offline and online
        let cpu_id = 2;

        // Add tasks to CPU
        for i in 0..5 {
            let task_id = mock_task_id(cpu_id, i);
            O1Scheduler::add_task_to_cpu(task_id, i * 10, cpu_id);
        }

        // CPU goes offline - migrate tasks
        let source_scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);
        let tasks: Vec<usize> = (0..5).map(|i| mock_task_id(cpu_id, i)).collect();

        for task_id in tasks {
            O1Scheduler::migrate_task(task_id, cpu_id, 0); // Migrate to CPU 0
        }

        // Verify source CPU is empty
        let stats = source_scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.task_count(), 0, "Offline CPU should have no tasks");

        // Verify tasks are on CPU 0
        let target_scheduler = O1Scheduler::get_cpu_scheduler(0);
        let stats = target_scheduler.detailed_stats();
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.task_count(), 5, "CPU 0 should have migrated tasks");

        // CPU comes back online
        // (In real implementation, would re-enable scheduling on this CPU)
    }

    /// Test load balancing heuristic
    #[test]
    fn test_load_balancing_heuristic() {
        // Create imbalanced load
        let busy_cpu = 0;
        let idle_cpu = 1;

        // Overload busy CPU
        for i in 0..20 {
            let task_id = mock_task_id(busy_cpu, i);
            O1Scheduler::add_task_to_cpu(task_id, i % MAX_PRIORITY, busy_cpu);
        }

        // Check if load balancing should trigger
        let busy_scheduler = O1Scheduler::get_cpu_scheduler(busy_cpu);
        let idle_scheduler = O1Scheduler::get_cpu_scheduler(idle_cpu);

        let busy_stats = busy_scheduler.detailed_stats();
        let idle_stats = idle_scheduler.detailed_stats();

        let busy_load = busy_stats.snapshot().task_count();
        let idle_load = idle_stats.snapshot().task_count();

        let load_imbalance = busy_load.saturating_sub(idle_load);

        // If imbalance is significant, should trigger migration
        assert!(load_imbalance > 15, "Should detect significant load imbalance");

        // Perform load balancing
        if load_imbalance > 10 {
            let migrate_count = load_imbalance / 2;

            for i in 0..migrate_count {
                // Peek next task on busy CPU and migrate
                if let Some(task_id) = O1Scheduler::peek_next_on_cpu(busy_cpu) {
                    O1Scheduler::migrate_task(task_id, busy_cpu, idle_cpu);
                }
            }
        }

        // Verify load is more balanced
        let busy_stats_after = busy_scheduler.detailed_stats();
        let idle_stats_after = idle_scheduler.detailed_stats();

        let busy_load_after = busy_stats_after.snapshot().task_count();
        let idle_load_after = idle_stats_after.snapshot().task_count();

        let imbalance_after = busy_load_after.saturating_sub(idle_load_after);

        assert!(imbalance_after < load_imbalance, "Load should be more balanced after migration");
    }
}

// ============================================================================
// Statistics and Performance Tests
// ============================================================================

#[cfg(test)]
mod statistics_tests {
    use super::*;

    /// Test scheduler statistics tracking
    #[test]
    fn test_scheduler_stats() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Perform some scheduling operations
        for i in 0..10 {
            let task_id = mock_task_id(cpu_id, i);
            scheduler.enqueue_task(task_id, i * 10);
            let _ = scheduler.pick_next_task();
        }

        // Check stats
        let stats = scheduler.detailed_stats();
        let snapshot = stats.snapshot();

        assert!(snapshot.ticks() > 0, "Should have recorded scheduler ticks");
        assert_eq!(snapshot.task_count(), 0, "All tasks should be processed");
    }

    /// Test latency histogram
    #[test]
    fn test_latency_histogram() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);
        let stats = scheduler.detailed_stats();

        // Record some latencies
        stats.record_latency(500);    // < 1μs
        stats.record_latency(5000);   // < 10μs
        stats.record_latency(50000);  // < 100μs
        stats.record_latency(500000); // < 1ms
        stats.record_latency(5000000); // < 10ms

        let snapshot = stats.snapshot();
        let hist = snapshot.latency_hist();

        assert_eq!(hist[0], 1, "Should have 1 entry in bucket 0");
        assert_eq!(hist[1], 1, "Should have 1 entry in bucket 1");
        assert_eq!(hist[2], 1, "Should have 1 entry in bucket 2");
        assert_eq!(hist[3], 1, "Should have 1 entry in bucket 3");
        assert_eq!(hist[4], 1, "Should have 1 entry in bucket 4");
    }

    /// Test scheduler throughput
    #[test]
    fn test_scheduler_throughput() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Target: > 100K context switches per second
        let start = crate::subsystems::time::get_time_ns();
        let iterations = 10000;

        for i in 0..iterations {
            let task_id = mock_task_id(cpu_id, i);
            scheduler.enqueue_task(task_id, i % MAX_PRIORITY);
            let _ = scheduler.pick_next_task();
        }

        let end = crate::subsystems::time::get_time_ns();
        let elapsed_ns = end.saturating_sub(start);
        let elapsed_sec = elapsed_ns as f64 / 1_000_000_000.0;
        let throughput = iterations as f64 / elapsed_sec;

        // Should achieve > 100K switches/sec
        assert!(throughput > 100_000.0, "Scheduler throughput should be > 100K switches/sec, got: {:.2}", throughput);
    }
}

// ============================================================================
// Edge Cases and Stress Tests
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    /// Test single task scheduling
    #[test]
    fn test_single_task() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        let task_id = mock_task_id(cpu_id, 1);
        scheduler.enqueue_task(task_id, 50);

        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_id), "Should pick single task");

        let next = scheduler.pick_next_task();
        assert_eq!(next, None, "Should be empty after single task");
    }

    /// Test all tasks at same priority
    #[test]
    fn test_same_priority() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        let priority = 50;

        // Enqueue multiple tasks at same priority
        for i in 0..10 {
            let task_id = mock_task_id(cpu_id, i);
            scheduler.enqueue_task(task_id, priority);
        }

        // All should be picked (FIFO order within same priority)
        let mut picked = Vec::new();
        while let Some(task_id) = scheduler.pick_next_task() {
            picked.push(task_id);
        }

        assert_eq!(picked.len(), 10, "Should pick all 10 tasks");
    }

    /// Test extreme priorities (0 and MAX-1)
    #[test]
    fn test_extreme_priorities() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        let task_highest = mock_task_id(cpu_id, 1);
        let task_lowest = mock_task_id(cpu_id, 2);

        scheduler.enqueue_task(task_lowest, MAX_PRIORITY - 1);
        scheduler.enqueue_task(task_highest, 0);

        // Highest priority (0) should be picked first
        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_highest), "Should pick priority 0 task first");

        let next = scheduler.pick_next_task();
        assert_eq!(next, Some(task_lowest), "Should pick priority MAX-1 task second");
    }

    /// Test concurrent enqueue/dequeue (stress test)
    #[test]
    fn test_concurrent_operations() {
        let cpu_id = cpuid();
        let scheduler = O1Scheduler::get_cpu_scheduler(cpu_id);

        // Simulate concurrent operations from multiple "threads"
        for iteration in 0..100 {
            // Enqueue batch
            for i in 0..10 {
                let task_id = mock_task_id(cpu_id, iteration * 10 + i);
                scheduler.enqueue_task(task_id, i % MAX_PRIORITY);
            }

            // Dequeue batch
            for _ in 0..10 {
                let _ = scheduler.pick_next_task();
            }
        }

        // Should be empty after all operations
        let next = scheduler.pick_next_task();
        assert_eq!(next, None, "All tasks should be processed");
    }
}
