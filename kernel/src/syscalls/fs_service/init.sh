#!/bin/bash
# Filesystem System Calls Module Initialization Script
# 初始化文件系统相关的系统调用模块

set -e

echo "Initializing Filesystem System Calls Module..."

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSCALLS_DIR="$(dirname "$MODULE_DIR")"

# 创建必要的文件结构
echo "Creating module structure..."

# 创建模块声明文件
if [ ! -f "$MODULE_DIR/mod.rs" ]; then
    cat > "$MODULE_DIR/mod.rs" << 'EOF'
//! Filesystem system calls
//!
//! This module provides system call implementations for file operations,
//! directory management, filesystem control, and advanced I/O features.

pub mod fs;       // Core filesystem operations
pub mod file_io;  // File I/O system calls
pub mod zero_copy; // Zero-copy I/O optimizations
pub mod aio;      // Asynchronous I/O (to be migrated)

pub use fs::*;
pub use file_io::*;
EOF
    echo "Created mod.rs"
fi

# 创建文件类型定义
if [ ! -f "$MODULE_DIR/types.rs" ]; then
    cat > "$MODULE_DIR/types.rs" << 'EOF'
//! Common types for filesystem syscalls

use crate::fs::FileMode;
use crate::types::{Fd, Result};

/// File open flags
#[derive(Debug, Clone, Copy)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub truncate: bool,
    pub append: bool,
    pub directory: bool,
    pub nonblock: bool,
    pub cloexec: bool,
}

impl OpenFlags {
    pub fn from_bits(flags: i32) -> Self {
        Self {
            read: (flags & 0x1) != 0,
            write: (flags & 0x2) != 0,
            create: (flags & 0x40) != 0,
            truncate: (flags & 0x200) != 0,
            append: (flags & 0x400) != 0,
            directory: (flags & 0x10000) != 0,
            nonblock: (flags & 0x800) != 0,
            cloexec: (flags & 0x80000) != 0,
        }
    }
}

/// Filesystem operations context
#[derive(Debug)]
pub struct FsContext {
    pub cwd: String,
    pub uid: u32,
    pub gid: u32,
    pub umask: u32,
}
EOF
    echo "Created types.rs"
fi

# 创建服务接口定义 (为未来重构做准备)
if [ ! -f "$MODULE_DIR/service.rs" ]; then
    cat > "$MODULE_DIR/service.rs" << 'EOF'
//! Filesystem service abstraction interface
//!
//! This trait defines the interface for filesystem operations,
//! enabling dependency injection and service-oriented architecture.

use crate::types::{Fd, Result};
use crate::syscall::{SyscallResult, SystemCallHandler};

pub type Path = str;
pub type Buffer = [u8];

/// Filesystem service interface
#[async_trait::async_trait]
pub trait FileSystemService: Send + Sync {
    /// Open or create a file
    async fn open_file(&self, path: &Path, flags: i32, mode: FileMode) -> SyscallResult<Fd>;

    /// Read data from a file
    async fn read_file(&self, fd: Fd, buf: &mut Buffer) -> SyscallResult<usize>;

    /// Write data to a file
    async fn write_file(&self, fd: Fd, buf: &[u8]) -> SyscallResult<usize>;

    /// Close a file descriptor
    async fn close_file(&self, fd: Fd) -> SyscallResult<()>;

    /// Get file status
    async fn get_file_stat(&self, fd: Fd) -> SyscallResult<FileStat>;

    /// Create a directory
    async fn create_directory(&self, path: &Path, mode: FileMode) -> SyscallResult<()>;

    /// Remove a directory
    async fn remove_directory(&self, path: &Path) -> SyscallResult<()>;

    /// Zero-copy file transfer
    async fn send_file(&self, out_fd: Fd, in_fd: Fd, offset: Option<u64>, count: usize) -> SyscallResult<usize>;
}

/// File status information
#[derive(Debug, Clone)]
pub struct FileStat {
    pub size: u64,
    pub mode: FileMode,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
}

/// Future syscall handler implementations will use this service
pub struct FsSyscallHandler<T: FileSystemService> {
    service: T,
}
EOF
    echo "Created service.rs"
fi

echo "Filesystem module initialized successfully!"
echo "Next steps:"
echo "1. Move existing fs.rs, file_io.rs, zero_copy.rs, aio.rs files to this directory"
echo "2. Update main syscalls/mod.rs to include this module"
echo "3. Implement FileSystemService interface for existing code"