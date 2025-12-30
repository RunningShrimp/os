//! GLib异步I/O系统调用
//!
//! 为GLib的GIO异步I/O提供内核级支持，包括：
//! - 异步文件读写操作
//! - 异步网络通信
//! - 异步操作队列管理
//! - 回调和完成通知
//! - 超时和取消机制

extern crate alloc;

use alloc::collections::BTreeMap;
use crate::prelude::*;
use core::{
    ffi::{c_int, c_void},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::subsystems::sync::Mutex;

/// 异步操作状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncOperationStatus {
    /// 操作已提交但尚未开始
    Submitted = 0,
    /// 操作正在进行中
    InProgress = 1,
    /// 操作已成功完成
    Completed = 2,
    /// 操作因错误而失败
    Failed = 3,
    /// 操作已被取消
    Cancelled = 4,
}

/// 异步操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncOperationType {
    /// 读操作
    Read = 0,
    /// 写操作
    Write = 1,
    /// 连接操作
    Connect = 2,
    /// 接受操作
    Accept = 3,
    /// 发送操作
    Send = 4,
    /// 接收操作
    Receive = 5,
}

/// 异步操作信息
#[derive(Debug)]
pub struct AsyncOperationInfo {
    /// 操作ID
    pub operation_id: u64,
    /// 操作类型
    pub operation_type: AsyncOperationType,
    /// 文件描述符
    pub fd: c_int,
    /// 缓冲区指针
    pub buffer: *mut c_void,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 操作状态
    pub status: AsyncOperationStatus,
    /// 已完成的字节数
    pub bytes_completed: AtomicUsize,
    /// 错误码
    pub error_code: c_int,
    /// 用户数据指针
    pub user_data: *mut c_void,
    /// 回调函数指针
    pub callback: *mut c_void,
    /// 超时时间（毫秒）
    pub timeout: u32,
    /// 创建时间戳
    pub created_timestamp: u64,
    /// 完成时间戳
    pub completed_timestamp: u64,
}

// SAFETY: AsyncOperationInfo is safe to send between threads because:
// 1. The raw pointers (buffer, user_data, callback) are only used within the kernel context
// 2. Access to these pointers is synchronized through the ASYNC_OPERATIONS Mutex
// 3. The struct is only accessed in a thread-safe manner through the registry
unsafe impl Send for AsyncOperationInfo {}

// SAFETY: AsyncOperationInfo is safe to share between threads because:
// 1. All mutable access is protected by the ASYNC_OPERATIONS Mutex
// 2. AtomicUsize fields provide their own synchronization
// 3. Raw pointers are only accessed through the mutex-protected registry
unsafe impl Sync for AsyncOperationInfo {}

/// 异步I/O上下文
#[derive(Debug)]
pub struct AsyncIOContext {
    /// 上下文ID
    pub context_id: u64,
    /// 上下文名称
    pub name: String,
    /// 最大并发操作数
    pub max_operations: usize,
    /// 当前活跃操作数
    pub active_operations: AtomicUsize,
    /// 创建时间戳
    pub created_timestamp: u64,
    /// 总操作数统计
    pub total_operations: AtomicUsize,
    /// 成功操作数统计
    pub successful_operations: AtomicUsize,
    /// 失败操作数统计
    pub failed_operations: AtomicUsize,
}

// SAFETY: AsyncIOContext is safe to send between threads because:
// 1. All fields are either immutable (context_id, name, max_operations, created_timestamp)
// 2. Or are atomic types (active_operations, total_operations, successful_operations, failed_operations)
// 3. Access is synchronized through the ASYNC_CONTEXTS Mutex
unsafe impl Send for AsyncIOContext {}

// SAFETY: AsyncIOContext is safe to share between threads because:
// 1. All mutable access is protected by the ASYNC_CONTEXTS Mutex
// 2. Atomic fields provide their own synchronization
unsafe impl Sync for AsyncIOContext {}

/// 全局异步操作注册表
static ASYNC_OPERATIONS: Mutex<BTreeMap<u64, AsyncOperationInfo>> = Mutex::new(BTreeMap::new());

/// 全局异步I/O上下文注册表
static ASYNC_CONTEXTS: Mutex<BTreeMap<u64, AsyncIOContext>> = Mutex::new(BTreeMap::new());

/// 下一个可用的操作ID
static NEXT_OPERATION_ID: AtomicUsize = AtomicUsize::new(1);

/// 下一个可用的上下文ID
static NEXT_CONTEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// GLib异步I/O管理器单例
pub static mut GLIB_ASYNC_MANAGER: () = ();

/// 获取GLib异步I/O管理器引用
pub fn get_glib_async_manager() -> &'static mut dyn manager::GAsyncManager {
    unsafe { &mut GLIB_ASYNC_MANAGER }
}

pub mod context;
pub mod manager;
pub mod operation;
