#!/bin/bash
# Network System Calls Module Initialization Script
# 初始化网络相关的系统调用模块

set -e

echo "Initializing Network System Calls Module..."

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SYSCALLS_DIR="$(dirname "$MODULE_DIR")"

# 创建必要的文件结构
echo "Creating module structure..."

# 创建模块声明文件
if [ ! -f "$MODULE_DIR/mod.rs" ]; then
    cat > "$MODULE_DIR/mod.rs" << 'EOF'
//! Network system calls
//!
//! This module provides system call implementations for network communications,
//! socket operations, and advanced networking features.

pub mod network;  // Core network socket operations (to be migrated)
pub mod epoll;    // Event polling mechanism (to be migrated)

pub use network::*;
EOF
    echo "Created mod.rs"
fi

echo "Network module initialized successfully!"
echo "Next steps:"
echo "1. Move existing network.rs and epoll-related files to this directory"
echo "2. Update main syscalls/mod.rs to include this module"