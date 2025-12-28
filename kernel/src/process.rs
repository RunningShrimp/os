//! Process module
//!
//! This module provides process management functionality.

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Sleeping,
    Blocked,
    Zombie,
    Stopped,
}

/// Process structure
#[derive(Debug, Clone)]
pub struct Process {
    pub pid: i32,
    pub parent_pid: i32,
    pub state: ProcessState,
}

impl Process {
    pub fn new(pid: i32, parent_pid: i32) -> Self {
        Self {
            pid,
            parent_pid,
            state: ProcessState::Running,
        }
    }
}
