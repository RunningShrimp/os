//! GLib事件循环集成系统调用

extern crate alloc;
//!
//! 为GLib的主事件循环提供高性能的epoll支持，包括：
//! - GLib专用epoll实例创建
//! - 高效的事件源添加和管理
//! - 批量事件等待和处理
//! - 超时和定时器支持

use crate::syscalls::SyscallResult;
use crate::subsystems::sync::Mutex;
use crate::fs::epoll::{EpollManager, EpollEvent, EPOLLIN, EPOLLOUT, EPOLLERR, EPOLLHUP};
use alloc::collections::BTreeMap;
use core::ffi::{c_int, c_void};
use core::sync::atomic::{AtomicUsize, Ordering};

/// GLib epoll error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollError {
    InvalidArgument,
    NotFound,
    OutOfSpace,
    DeviceError,
    AlreadyExists,
}

pub type EpollResult<T> = Result<T, EpollError>;

/// GLib专用的epoll实例信息
#[derive(Debug, Clone)]
pub struct GLibEpollInstance {
    /// epoll文件描述符
    pub epfd: c_int,
    /// 事件源数量
    pub source_count: AtomicUsize,
    /// 最大事件源数量
    pub max_sources: usize,
    /// 创建时间戳
    pub created_timestamp: u64,
    /// 总等待次数
    pub total_waits: AtomicUsize,
    /// 总事件数
    pub total_events: AtomicUsize,
}

/// 全局GLib epoll实例注册表
static GLIB_EPOLL_INSTANCES: Mutex<BTreeMap<c_int, GLibEpollInstance>> =
    Mutex::new(BTreeMap::new());

/// 下一个可用的epoll实例ID
static NEXT_EPOLL_ID: AtomicUsize = AtomicUsize::new(1);

/// GLib事件循环管理器单例
pub static mut GLIB_EPOLL_MANAGER: () = ();

/// 获取GLib事件循环管理器引用
pub fn get_glib_epoll_manager() -> &'static dyn super::manager::GLibEpollManager {
    unsafe { &GLIB_EPOLL_MANAGER }
}

pub mod instance;
pub mod manager;