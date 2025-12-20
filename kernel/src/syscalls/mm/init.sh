#!/bin/bash
# Memory Management System Calls Module Initialization Script
# 初始化内存管理相关的系统调用模块

set -e

echo "Initializing Memory Management System Calls Module..."

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSCALLS_DIR="$(dirname "$MODULE_DIR")"

# 创建必要的文件结构
echo "Creating module structure..."

# 创建模块声明文件
if [ ! -f "$MODULE_DIR/mod.rs" ]; then
    cat > "$MODULE_DIR/mod.rs" << 'EOF'
//! Memory management system calls
//!
//! This module provides system call implementations for virtual memory
//! management, memory mapping, protection, and advanced memory features.

pub mod memory;       // Core memory management operations
pub mod advanced_mmap; // Advanced mmap features and HugePage support

pub use memory::*;
pub use advanced_mmap::*;
EOF
    echo "Created mod.rs"
fi

# 创建内存类型定义
if [ ! -f "$MODULE_DIR/types.rs" ]; then
    cat > "$MODULE_DIR/types.rs" << 'EOF'
//! Common types for memory management syscalls

use crate::mm::Address;

/// Memory protection flags
#[derive(Debug, Clone, Copy)]
pub struct ProtectionFlags {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub none: bool,
}

impl ProtectionFlags {
    pub fn from_bits(prot: i32) -> Self {
        Self {
            read: (prot & 1) != 0,
            write: (prot & 2) != 0,
            execute: (prot & 4) != 0,
            none: prot == 0,
        }
    }
}

/// Memory mapping flags
#[derive(Debug, Clone, Copy)]
pub struct MapFlags {
    pub shared: bool,
    pub private: bool,
    pub fixed: bool,
    pub anonymous: bool,
    pub populate: bool,
    pub huge_page: bool,
}

impl MapFlags {
    pub fn from_bits(flags: i32) -> Self {
        Self {
            shared: (flags & 1) != 0,
            private: (flags & 2) != 0,
            fixed: (flags & 16) != 0,
            anonymous: (flags & 32) != 0,
            populate: (flags & 0x8000) != 0,
            huge_page: (flags & 0x40000) != 0,
        }
    }
}

/// Memory advice types for madvise
#[derive(Debug, Clone, Copy)]
pub enum MemoryAdvice {
    Normal = 0,
    Random = 1,
    Sequential = 2,
    WillNeed = 3,
    DontNeed = 4,
    Pageout = 8,
    HugePage = 14,
    DontFork = 10,
    DoFork = 11,
}

/// Memory mapping request structure
#[derive(Debug)]
pub struct MmapRequest {
    pub addr: Option<Address>,
    pub length: usize,
    pub prot: ProtectionFlags,
    pub flags: MapFlags,
    pub fd: Option<i64>,
    pub offset: u64,
}
EOF
    echo "Created types.rs"
fi

# 创建服务接口定义 (为未来重构做准备)
if [ ! -f "$MODULE_DIR/service.rs" ]; then
    cat > "$MODULE_DIR/service.rs" << 'EOF'
//! Memory management service abstraction interface
//!
//! This trait defines the interface for memory management operations,
//! enabling dependency injection and service-oriented architecture.

use crate::types::{Pid, Result};
use crate::syscall::{SyscallResult, SystemCallHandler};

/// Memory management service interface
#[async_trait::async_trait]
pub trait MemoryService: Send + Sync {
    /// Map memory pages
    async fn map_memory(
        &self,
        addr: Option<Address>,
        length: usize,
        prot: ProtectionFlags,
        flags: MapFlags,
        fd: Option<i64>,
        offset: u64
    ) -> SyscallResult<Address>;

    /// Unmap memory pages
    async fn unmap_memory(&self, addr: Address, length: usize) -> SyscallResult<()>;

    /// Change memory protection
    async fn protect_memory(
        &self,
        addr: Address,
        length: usize,
        prot: ProtectionFlags
    ) -> SyscallResult<()>;

    /// Sync memory mappings
    async fn sync_memory(&self, addr: Address, length: usize, flags: i32) -> SyscallResult<()>;

    /// Extend program break
    async fn extend_break(&self, increment: isize) -> SyscallResult<Address>;

    /// Lock memory pages
    async fn lock_memory(&self, addr: Address, length: usize) -> SyscallResult<()>;

    /// Unlock memory pages
    async fn unlock_memory(&self, addr: Address, length: usize) -> SyscallResult<()>;

    /// Provide memory usage advice
    async fn advise_memory(&self, addr: Address, length: usize, advice: MemoryAdvice) -> SyscallResult<()>;

    /// Create anonymous memory file
    async fn create_memory_file(&self, name: &str, flags: u32) -> SyscallResult<i32>;
}

/// Process memory access interface for cross-process operations
#[async_trait::async_trait]
pub trait CrossProcessMemoryService: Send + Sync {
    /// Read memory from another process
    async fn read_process_memory(&self, pid: Pid, remote_addr: Address, local_buf: &mut [u8]) -> SyscallResult<usize>;

    /// Write memory to another process
    async fn write_process_memory(&self, pid: Pid, remote_addr: Address, local_buf: &[u8]) -> SyscallResult<usize>;
}

/// Future syscall handler implementations will use this service
pub struct MemorySyscallHandler<T: MemoryService> {
    service: T,
}
EOF
    echo "Created service.rs"
fi

echo "Memory Management module initialized successfully!"
echo "Next steps:"
echo "1. Move existing memory.rs, advanced_mmap.rs files to this directory"
echo "2. Update main syscalls/mod.rs to include this module"
echo "3. Implement MemoryService interface for existing code"