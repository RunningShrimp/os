//! GLib内存管理扩展系统调用

extern crate alloc;
//!
//! 为GLib提供高性能的内存管理支持，包括：
//! - 专用内存池创建和管理
//! - 快速内存分配和释放
//! - 内存池统计和调试信息
//! - 线程安全的内存操作

use crate::syscalls::SyscallResult;
use crate::alloc::allocator::FixedSizeAllocator;
use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use core::ffi::{c_int, c_void};
use core::sync::atomic::{AtomicUsize, Ordering};

/// GLib memory error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    InvalidArgument,
    OutOfMemory,
    PoolNotFound,
    PoolExists,
    PoolFull,
    InvalidSize,
    AlignmentError,
}

pub type MemoryResult<T> = Result<T, MemoryError>;

/// 内存池信息
#[derive(Debug, Clone)]
pub struct MemoryPoolInfo {
    /// 内存池大小
    pub size: usize,
    /// 对齐要求
    pub alignment: usize,
    /// 分配的块数量
    pub allocated_blocks: AtomicUsize,
    /// 释放的块数量
    pub freed_blocks: AtomicUsize,
    /// 当前活跃块数量
    pub active_blocks: AtomicUsize,
    /// 创建时间戳
    pub created_timestamp: u64,
}

/// 全局内存池注册表
static MEMORY_POOLS: Mutex<BTreeMap<c_int, (FixedSizeAllocator, MemoryPoolInfo)>> =
    Mutex::new(BTreeMap::new());

/// 下一个可用的内存池ID
static NEXT_POOL_ID: AtomicUsize = AtomicUsize::new(1);

/// GLib内存管理器单例
pub static mut GLIB_MEMORY_MANAGER: () = ();

/// 获取GLib内存管理器引用
pub fn get_glib_memory_manager() -> &'static dyn super::allocator::GLibMemoryAllocator {
    unsafe { &GLIB_MEMORY_MANAGER }
}

pub mod pool;
pub mod allocator;
pub mod adapter;