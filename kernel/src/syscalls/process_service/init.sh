#!/bin/bash
# Process Management System Calls Module Initialization Script
# 初始化进程管理相关的系统调用模块

set -e

echo "Initializing Process Management System Calls Module..."

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSCALLS_DIR="$(dirname "$MODULE_DIR")"

# 创建必要的文件结构
echo "Creating module structure..."

# 创建模块声明文件
if [ ! -f "$MODULE_DIR/mod.rs" ]; then
    cat > "$MODULE_DIR/mod.rs" << 'EOF'
//! Process management system calls
//!
//! This module provides system call implementations for process lifecycle,
//! scheduling, and process information management.

pub mod process;  // Core process management
pub mod thread;   // Thread-related syscalls (to be migrated)
// pub mod advanced_thread;  // Advanced threading features (planned)

pub use process::*;
EOF
    echo "Created mod.rs"
fi

# 创建进程类型定义
if [ ! -f "$MODULE_DIR/types.rs" ]; then
    cat > "$MODULE_DIR/types.rs" << 'EOF'
//! Common types for process management syscalls

use crate::types::Pid;

/// Process creation flags
#[derive(Debug, Clone, Copy)]
pub enum CloneFlags {
    /// Standard fork behavior
    Fork = 0x01,
    /// Create new process group
    NewPidNamespace = 0x02,
    /// Share memory space
    VmShared = 0x04,
}

/// Process attributes for clone system call
#[derive(Debug, Clone)]
pub struct CloneArgs {
    pub flags: CloneFlags,
    pub stack: usize,
    pub tls: usize,
    pub parent_tidptr: usize,
    pub child_tidptr: usize,
}
EOF
    echo "Created types.rs"
fi

# 创建服务接口定义 (为未来重构做准备)
if [ ! -f "$MODULE_DIR/service.rs" ]; then
    cat > "$MODULE_DIR/service.rs" << 'EOF'
//! Process service abstraction interface
//!
//! This trait defines the interface for process management operations,
//! enabling dependency injection and service-oriented architecture.

use crate::types::{Pid, Result};
use crate::syscall::{SyscallResult, SystemCallHandler};

/// Process management service interface
#[async_trait::async_trait]
pub trait ProcessService: Send + Sync {
    /// Create a new child process (fork)
    async fn fork_process(&self) -> SyscallResult<Pid>;

    /// Execute a new program in the current process
    async fn exec_process(
        &self,
        path: &str,
        argv: &[&str],
        envp: &[&str]
    ) -> SyscallResult<()>;

    /// Wait for a child process to terminate
    async fn wait_process(&self, pid: Pid) -> SyscallResult<i32>;

    /// Terminate the current process
    async fn exit_process(&self, status: i32) -> SyscallResult<!>;

    /// Send a signal to a process
    async fn kill_process(&self, pid: Pid, signal: i32) -> SyscallResult<()>;

    /// Get the current process ID
    fn get_current_pid(&self) -> Pid;

    /// Get the parent process ID
    fn get_parent_pid(&self) -> Option<Pid>;

    /// Get the process group ID
    fn get_process_group(&self) -> Pid;

    /// Yield the processor
    async fn yield_processor(&self) -> SyscallResult<()>;
}

/// Future syscall handler implementations will use this service
pub struct ProcessSyscallHandler<T: ProcessService> {
    service: T,
}
EOF
    echo "Created service.rs"
fi

echo "Process Management module initialized successfully!"
echo "Next steps:"
echo "1. Move existing process.rs, thread.rs, advanced_thread.rs files to this directory"
echo "2. Update main syscalls/mod.rs to include this module"
echo "3. Implement ProcessService interface for existing code"