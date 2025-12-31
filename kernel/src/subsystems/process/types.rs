//! Process Type Definitions
//!
//! Unified type definitions for process management

use alloc::sync::Arc;

use spin::Mutex;

/// Process ID type
pub type ProcessId = u64;

/// Thread ID type  
pub type ThreadId = u64;

/// Process structure
#[derive(Debug)]
pub struct Process {
    pub pid: ProcessId,
    pub parent_pid: ProcessId,
    pub name: alloc::string::String,
}

impl Process {
    pub fn new(pid: ProcessId, parent_pid: ProcessId, name: &str) -> Self {
        Self { pid, parent_pid, name: alloc::string::String::from(name) }
    }

    pub fn id(&self) -> ProcessId {
        self.pid
    }
}

/// Thread structure
#[derive(Debug)]
pub struct Thread {
    pub tid: ThreadId,
    pub process_id: ProcessId,
}

impl Thread {
    pub fn new(tid: ThreadId, process_id: ProcessId) -> Self {
        Self { tid, process_id }
    }

    pub fn id(&self) -> ThreadId {
        self.tid
    }
}

/// Get current process
pub fn get_current_process() -> Option<Arc<Mutex<Process>>> {
    // GH-#1330: Implement proper process lookup
    // See: https://github.com/npos/kernel/issues/1330
    // This should return the currently executing process
    None
}

/// Get process by PID
pub fn get_process_by_pid(pid: ProcessId) -> Option<Arc<Mutex<Process>>> {
    // GH-#1331: Implement process table lookup by PID
    // See: https://github.com/npos/kernel/issues/1331
    // This should search the global process table and return the process
    let _ = pid; // Parameter will be used when process table is implemented
    None
}
