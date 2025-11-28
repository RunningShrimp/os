//! Process Management Service for hybrid architecture
//! Separates process management functionality from kernel core

use crate::process::{
    ProcState, Context, TrapFrame, Proc, ProcTable, 
    NPROC, NOFILE, fork, exit, wait, kill, getpid,
};
use crate::sync::Mutex;
use crate::services::{service_register, ServiceInfo};

// ============================================================================
// Process Management Service State
// ============================================================================

/// Process service endpoint (IPC channel)
pub const PROCESS_SERVICE_ENDPOINT: usize = 0x2000;

/// Process service statistics
pub struct ProcessStats {
    pub total_processes: usize,
    pub runnable_processes: usize,
    pub sleeping_processes: usize,
    pub zombie_processes: usize,
}

// ============================================================================
// Public API
// ============================================================================

/// Initialize process management service
pub fn init() {
    // Register process management service
    service_register(
        "process_manager",
        "Process management service for process creation, scheduling, and lifecycle management",
        PROCESS_SERVICE_ENDPOINT
    );
    
    crate::println!("services/process: initialized");
}

/// Create a new process by forking
pub fn proc_fork() -> Option<usize> {
    fork()
}

/// Exit current process
pub fn proc_exit(status: i32) {
    exit(status);
}

/// Wait for child process to exit
pub fn proc_wait(status: *mut i32) -> Option<usize> {
    wait(status)
}

/// Kill a process
pub fn proc_kill(pid: usize) -> bool {
    kill(pid)
}

/// Get current process ID
pub fn proc_getpid() -> usize {
    getpid()
}

/// Get process statistics
pub fn proc_get_stats() -> ProcessStats {
    let table = crate::process::PROC_TABLE.lock();
    
    let mut stats = ProcessStats {
        total_processes: 0,
        runnable_processes: 0,
        sleeping_processes: 0,
        zombie_processes: 0,
    };
    
    for proc in table.iter() {
        match proc.state {
            ProcState::Runnable => {
                stats.total_processes += 1;
                stats.runnable_processes += 1;
            }
            ProcState::Sleeping => {
                stats.total_processes += 1;
                stats.sleeping_processes += 1;
            }
            ProcState::Zombie => {
                stats.total_processes += 1;
                stats.zombie_processes += 1;
            }
            ProcState::Running => {
                stats.total_processes += 1;
                stats.runnable_processes += 1;
            }
            _ => {}
        }
    }
    
    stats
}