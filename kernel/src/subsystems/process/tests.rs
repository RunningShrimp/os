//! Process Tests
//!
//! Tests for process management

#[cfg(feature = "kernel_tests")]
pub mod process_tests {
    use alloc::string::String;
    use alloc::vec::Vec;
    use crate::{test_assert_eq, test_assert};
    use crate::tests::TestResult;
    use crate::process;

    /// Test process getpid
    pub fn test_getpid() -> TestResult {
        let pid = process::getpid();
        // PID should be non-negative
        test_assert!(pid >= 0);
        Ok(())
    }

    /// Test process allocation performance
    pub fn test_process_alloc_performance() -> TestResult {
        use crate::process::PROC_TABLE;

        // Test that process allocation is O(1)
        let start_time = crate::subsystems::time::get_ticks();
        let mut ptable = PROC_TABLE.lock();

        // Simulate allocation (actual allocation would require more setup)
        for _ in 0..10 {
            // Just test lookup performance for existing process
            let result = ptable.find_ref(1); // init process
            test_assert!(result.is_some());
        }

        let end_time = crate::subsystems::time::get_ticks();
        let duration = end_time - start_time;

        // Should be very fast (<100 ticks on test hardware)
        test_assert!(duration < 100, alloc::format!("Process lookup took too long: {} ticks", duration));

        Ok(())
    }

    /// Test process table initialization
    pub fn test_process_table_init() -> TestResult {
        let table = crate::process::ProcTable::new();

        // Check that all processes are initially unused
        for proc in table.iter() {
            test_assert_eq!(proc.state, crate::process::ProcState::Unused);
        }

        // Check that PID counter starts at 1
        test_assert_eq!(table.next_pid, 1);

        Ok(())
    }

    /// Test process allocation and deallocation
    pub fn test_process_alloc_dealloc() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        // Allocate first process
        let proc1 = table.alloc();
        test_assert!(proc1.is_some(), "First process allocation should succeed");
        let proc1_ref = proc1.unwrap();
        test_assert_eq!(proc1_ref.pid, 1);
        test_assert_eq!(proc1_ref.state, crate::process::ProcState::Used);

        // Allocate second process
        let proc2 = table.alloc();
        test_assert!(proc2.is_some(), "Second process allocation should succeed");
        let proc2_ref = proc2.unwrap();
        test_assert_eq!(proc2_ref.pid, 2);
        test_assert_eq!(proc2_ref.state, crate::process::ProcState::Used);

        // Free first process
        table.free(1);
        let proc1_after_free = table.find_ref(1);
        test_assert!(proc1_after_free.is_none(), "Process should be freed");

        // Allocate again - should reuse slot
        let proc3 = table.alloc();
        test_assert!(proc3.is_some(), "Third process allocation should succeed");
        let proc3_ref = proc3.unwrap();
        test_assert_eq!(proc3_ref.pid, 3);

        Ok(())
    }

    /// Test process table capacity limits
    pub fn test_process_table_capacity() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        // Allocate all processes
        let mut allocated = Vec::new();
        for _ in 0..crate::process::NPROC {
            let proc = table.alloc();
            test_assert!(proc.is_some(), "Process allocation should succeed");
            allocated.push(proc.unwrap().pid);
        }

        // Next allocation should fail
        let overflow_proc = table.alloc();
        test_assert!(overflow_proc.is_none(), "Process allocation should fail when table is full");

        // Free one process
        let freed_pid = allocated[0];
        table.free(freed_pid);

        // Now allocation should succeed again
        let new_proc = table.alloc();
        test_assert!(new_proc.is_some(), "Process allocation should succeed after freeing");

        // Clean up
        for pid in allocated.iter().skip(1) {
            table.free(*pid);
        }
        table.free(new_proc.unwrap().pid);

        Ok(())
    }

    /// Test process state transitions
    pub fn test_process_state_transitions() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        let proc = table.alloc().unwrap();
        test_assert_eq!(proc.state, crate::process::ProcState::Used);

        // Test state changes
        proc.state = crate::process::ProcState::Runnable;
        test_assert_eq!(proc.state, crate::process::ProcState::Runnable);

        proc.state = crate::process::ProcState::Running;
        test_assert_eq!(proc.state, crate::process::ProcState::Running);

        proc.state = crate::process::ProcState::Sleeping;
        test_assert_eq!(proc.state, crate::process::ProcState::Sleeping);

        proc.state = crate::process::ProcState::Zombie;
        test_assert_eq!(proc.state, crate::process::ProcState::Zombie);

        // Clean up
        table.free(proc.pid);

        Ok(())
    }

    /// Test process file descriptor management
    pub fn test_process_fd_management() -> TestResult {
        let mut table = crate::process::ProcTable::new();
        let proc = table.alloc().unwrap();

        // Initially all FDs should be None
        for i in 0..crate::process::NOFILE {
            test_assert!(proc.ofile[i].is_none(), alloc::format!("FD {} should be None initially", i));
        }

        // Allocate some FDs
        proc.ofile[0] = Some(5);
        proc.ofile[3] = Some(10);
        proc.ofile[15] = Some(20);

        test_assert_eq!(proc.ofile[0], Some(5));
        test_assert_eq!(proc.ofile[3], Some(10));
        test_assert_eq!(proc.ofile[15], Some(20));

        // Other FDs should still be None
        test_assert!(proc.ofile[1].is_none());
        test_assert!(proc.ofile[7].is_none());

        // Clean up
        table.free(proc.pid);

        Ok(())
    }

    /// Test process context initialization
    pub fn test_process_context_init() -> TestResult {
        let context = crate::process::Context::new();

        // Check that context is zero-initialized
        #[cfg(target_arch = "riscv64")]
        {
            test_assert_eq!(context.ra, 0);
            test_assert_eq!(context.sp, 0);
            test_assert_eq!(context.s0, 0);
            test_assert_eq!(context.s1, 0);
        }

        #[cfg(target_arch = "aarch64")]
        {
            test_assert_eq!(context.fp, 0);
            test_assert_eq!(context.lr, 0);
            test_assert_eq!(context.sp, 0);
        }

        #[cfg(target_arch = "x86_64")]
        {
            test_assert_eq!(context.rsp, 0);
            test_assert_eq!(context.rip, 0);
        }

        Ok(())
    }

    /// Test process trap frame initialization
    pub fn test_process_trapframe_init() -> TestResult {
        let tf = crate::process::TrapFrame::new();

        // Check that trap frame is zero-initialized
        #[cfg(target_arch = "riscv64")]
        {
            test_assert_eq!(tf.epc, 0);
            test_assert_eq!(tf.sp, 0);
            test_assert_eq!(tf.ra, 0);
        }

        #[cfg(target_arch = "aarch64")]
        {
            test_assert_eq!(tf.elr, 0);
            test_assert_eq!(tf.sp, 0);
        }

        #[cfg(target_arch = "x86_64")]
        {
            test_assert_eq!(tf.rip, 0);
            test_assert_eq!(tf.rsp, 0);
        }

        Ok(())
    }

    /// Test process lookup by PID
    pub fn test_process_lookup() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        // Allocate some processes
        let proc1 = table.alloc().unwrap();
        let pid1 = proc1.pid;
        let proc2 = table.alloc().unwrap();
        let pid2 = proc2.pid;

        // Test lookup
        let found1 = table.find_ref(pid1);
        test_assert!(found1.is_some(), "Should find process 1");
        test_assert_eq!(found1.unwrap().pid, pid1);

        let found2 = table.find_ref(pid2);
        test_assert!(found2.is_some(), "Should find process 2");
        test_assert_eq!(found2.unwrap().pid, pid2);

        // Test lookup of non-existent process
        let not_found = table.find_ref(9999);
        test_assert!(not_found.is_none(), "Should not find non-existent process");

        // Clean up
        table.free(pid1);
        table.free(pid2);

        Ok(())
    }

    /// Test process iterator
    pub fn test_process_iterator() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        // Allocate a few processes
        let pids = vec![
            table.alloc().unwrap().pid,
            table.alloc().unwrap().pid,
            table.alloc().unwrap().pid,
        ];

        // Count used processes
        let mut used_count = 0;
        for proc in table.iter() {
            if proc.state != crate::process::ProcState::Unused {
                used_count += 1;
            }
        }
        test_assert_eq!(used_count, 3, "Should find 3 used processes");

        // Test mutable iterator
        let mut modified_count = 0;
        for proc in table.iter_mut() {
            if proc.state != crate::process::ProcState::Unused {
                proc.state = crate::process::ProcState::Runnable;
                modified_count += 1;
            }
        }
        test_assert_eq!(modified_count, 3, "Should modify 3 processes");

        // Verify modifications
        let mut runnable_count = 0;
        for proc in table.iter() {
            if proc.state == crate::process::ProcState::Runnable {
                runnable_count += 1;
            }
        }
        test_assert_eq!(runnable_count, 3, "Should find 3 runnable processes");

        // Clean up
        for pid in pids {
            table.free(pid);
        }

        Ok(())
    }

    /// Test process resource limits initialization
    pub fn test_process_rlimits_init() -> TestResult {
        let proc = crate::process::Proc::new();

        // Check that resource limits are initialized to zero
        for rlimit in &proc.rlimits {
            test_assert_eq!(rlimit.rlim_cur, 0);
            test_assert_eq!(rlimit.rlim_max, 0);
        }

        Ok(())
    }

    /// Test process signal state initialization
    pub fn test_process_signal_init() -> TestResult {
        let proc = crate::process::Proc::new();

        // Signal state should be None initially
        test_assert!(proc.signals.is_none());

        Ok(())
    }

    /// Test process memory size tracking
    pub fn test_process_memory_tracking() -> TestResult {
        let mut table = crate::process::ProcTable::new();
        let proc = table.alloc().unwrap();

        // Initially memory size should be 0
        test_assert_eq!(proc.sz, 0);

        // Simulate memory allocation
        proc.sz = 4096;
        test_assert_eq!(proc.sz, 4096);

        proc.sz = 8192;
        test_assert_eq!(proc.sz, 8192);

        // Clean up
        table.free(proc.pid);

        Ok(())
    }

    /// Test process parent-child relationships
    pub fn test_process_parent_child() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        // Create parent process
        let parent = table.alloc().unwrap();
        let parent_pid = parent.pid;

        // Create child process
        let child = table.alloc().unwrap();
        let child_pid = child.pid;
        child.parent = Some(parent_pid);

        // Verify relationship
        test_assert_eq!(child.parent, Some(parent_pid));
        test_assert!(parent.parent.is_none()); // Parent has no parent

        // Test lookup by parent
        let mut child_count = 0;
        for proc in table.iter() {
            if proc.parent == Some(parent_pid) {
                child_count += 1;
            }
        }
        test_assert_eq!(child_count, 1, "Parent should have one child");

        // Clean up
        table.free(parent_pid);
        table.free(child_pid);

        Ok(())
    }

    /// Test process group and session management
    pub fn test_process_groups_sessions() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        let proc = table.alloc().unwrap();

        // Initially PGID and SID should equal PID
        test_assert_eq!(proc.pgid, proc.pid);
        test_assert_eq!(proc.sid, proc.pid);

        // Test changing process group
        proc.pgid = 100;
        test_assert_eq!(proc.pgid, 100);

        // Test changing session
        proc.sid = 200;
        test_assert_eq!(proc.sid, 200);

        // Clean up
        table.free(proc.pid);

        Ok(())
    }

    /// Test process killed flag
    pub fn test_process_killed_flag() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        let proc = table.alloc().unwrap();

        // Initially not killed
        test_assert!(!proc.killed);

        // Set killed flag
        proc.killed = true;
        test_assert!(proc.killed);

        // Reset killed flag
        proc.killed = false;
        test_assert!(!proc.killed);

        // Clean up
        table.free(proc.pid);

        Ok(())
    }

    /// Test process exit status tracking
    pub fn test_process_exit_status() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        let proc = table.alloc().unwrap();

        // Initially exit status should be 0
        test_assert_eq!(proc.xstate, 0);

        // Set various exit statuses
        proc.xstate = 42;
        test_assert_eq!(proc.xstate, 42);

        proc.xstate = -1;
        test_assert_eq!(proc.xstate, -1);

        // Clean up
        table.free(proc.pid);

        Ok(())
    }

    /// Test process working directory management
    pub fn test_process_cwd_management() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        let proc = table.alloc().unwrap();

        // Initially CWD should be None
        test_assert!(proc.cwd_path.is_none());
        test_assert!(proc.cwd.is_none());

        // Set working directory
        proc.cwd_path = Some(String::from("/home/user"));
        proc.cwd = Some(5); // File descriptor index

        test_assert_eq!(proc.cwd_path.as_ref().unwrap(), "/home/user");
        test_assert_eq!(proc.cwd, Some(5));

        // Change working directory
        proc.cwd_path = Some(String::from("/tmp"));
        proc.cwd = Some(10);

        test_assert_eq!(proc.cwd_path.as_ref().unwrap(), "/tmp");
        test_assert_eq!(proc.cwd, Some(10));

        // Clean up
        table.free(proc.pid);

        Ok(())
    }

    /// Test process sleep channel management
    pub fn test_process_sleep_channel() -> TestResult {
        let mut table = crate::process::ProcTable::new();

        let proc = table.alloc().unwrap();

        // Initially sleep channel should be 0
        test_assert_eq!(proc.chan, 0);

        // Set sleep channel
        proc.chan = 0x1000;
        test_assert_eq!(proc.chan, 0x1000);

        proc.chan = 0x2000;
        test_assert_eq!(proc.chan, 0x2000);

        // Clean up
        table.free(proc.pid);

        Ok(())
    }
}

// ============================================================================
// Memory management tests
// ============================================================================

