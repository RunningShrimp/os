#!/bin/bash
# IPC System Calls Module Initialization Script
# 初始化进程间通信相关的系统调用模块

set -e

echo "Initializing IPC System Calls Module..."

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSCALLS_DIR="$(dirname "$MODULE_DIR")"

# 创建必要的文件结构
echo "Creating module structure..."

# 创建模块声明文件
if [ ! -f "$MODULE_DIR/mod.rs" ]; then
    cat > "$MODULE_DIR/mod.rs" << 'EOF'
//! Inter-process communication system calls
//!
//! This module provides system call implementations for various IPC mechanisms,
//! including pipes, message queues, semaphores, and shared memory.

pub mod pipe;   // Pipe-based IPC operations (to be migrated)
pub mod mqueue; // Message queue operations (to be migrated)

pub use pipe::*;
pub use mqueue::*;
EOF
    echo "Created mod.rs"
fi

echo "IPC module initialized successfully!"
echo "Next steps:"
echo "1. Move existing pipe.rs and mqueue.rs files to this directory"
echo "2. Update main syscalls/mod.rs to include this module"