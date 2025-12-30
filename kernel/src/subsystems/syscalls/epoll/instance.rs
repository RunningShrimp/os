//! Core epoll instance management functions

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::ffi::c_int;
use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use crate::prelude::*;
use crate::subsystems::sync::Mutex;
use crate::subsystems::time;

// Import parent module types
use super::{GLIB_EPOLL_INSTANCES, GLibEpollInstance, NEXT_EPOLL_ID};

// Include sys_close from file_io module
#[path = "../fs/file_io.rs"]
mod file_io;
use file_io::sys_close;

/// Raw epoll event structure (C-compatible)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct EpollEvent {
    pub events: u32,
    pub data: u64,
}

/// Epoll event info with raw u32 events (compatible with Linux API)
#[derive(Debug, Clone)]
pub struct EpollEventInfo {
    /// Event type bitmask (EPOLLIN, EPOLLOUT, etc.)
    pub events: u32,
    /// User data
    pub data: u64,
}

/// Epoll instance
struct EpollInstance {
    /// File descriptors being monitored
    fds: BTreeMap<c_int, EpollEventInfo>,
    /// Event queue
    events: Vec<EpollEvent>,
}

impl EpollInstance {
    fn new() -> Self {
        Self {
            fds: BTreeMap::new(),
            events: Vec::new(),
        }
    }
}

/// Global epoll manager
struct EpollManager {
    instances: BTreeMap<c_int, Arc<Mutex<EpollInstance>>>,
    next_id: AtomicI32,
}

impl EpollManager {
    fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            next_id: AtomicI32::new(0),
        }
    }

    fn create() -> Result<c_int> {
        let mut manager = EPOLL_MANAGER.lock();
        let id = manager.next_id.fetch_add(1, Ordering::SeqCst);
        manager.instances.insert(id, Arc::new(Mutex::new(EpollInstance::new())));
        Ok(id)
    }

    fn add(epfd: c_int, fd: c_int, event: &EpollEventInfo) -> Result<UnifiedError> {
        let manager = EPOLL_MANAGER.lock();
        if let Some(instance) = manager.instances.get(&epfd) {
            let mut inst = instance.lock();
            inst.fds.insert(fd, event.clone());
            Ok(UnifiedError::Other("Success".to_string()))
        } else {
            Err(UnifiedError::NotFound)
        }
    }

    fn modify(epfd: c_int, fd: c_int, event: &EpollEventInfo) -> Result<UnifiedError> {
        let manager = EPOLL_MANAGER.lock();
        if let Some(instance) = manager.instances.get(&epfd) {
            let mut inst = instance.lock();
            inst.fds.insert(fd, event.clone());
            Ok(UnifiedError::Other("Success".to_string()))
        } else {
            Err(UnifiedError::NotFound)
        }
    }

    fn remove(epfd: c_int, fd: c_int) -> Result<UnifiedError> {
        let manager = EPOLL_MANAGER.lock();
        if let Some(instance) = manager.instances.get(&epfd) {
            let mut inst = instance.lock();
            inst.fds.remove(&fd);
            Ok(UnifiedError::Other("Success".to_string()))
        } else {
            Err(UnifiedError::NotFound)
        }
    }

    fn wait(epfd: c_int, _events: &mut [EpollEvent], _timeout: i32) -> i32 {
        let manager = EPOLL_MANAGER.lock();
        if let Some(_instance) = manager.instances.get(&epfd) {
            // For now, just return 0 events (stub implementation)
            // In a real implementation, this would block and wait for events
            0
        } else {
            -1
        }
    }
}

/// Global epoll manager instance
static EPOLL_MANAGER: Lazy<Mutex<EpollManager>> = Lazy::new(|| Mutex::new(EpollManager::new()));

/// 创建GLib专用epoll实例
///
/// # 返回值
/// * 成功时返回epoll文件描述符
/// * 失败时返回EpollError
#[unsafe(no_mangle)]
pub extern "C" fn sys_glib_epoll_create() -> c_int {
    crate::println!("[glib_epoll] 创建GLib专用epoll实例");

    // 创建epoll文件描述符
    let epfd = match EpollManager::create() {
        Ok(fd) => fd,
        Err(_) => {
            crate::println!("[glib_epoll] 创建epoll失败");
            return -19; // ENODEV
        },
    };

    if epfd < 0 {
        crate::println!("[glib_epoll] epoll文件描述符无效: {}", epfd);
        return -19; // ENODEV
    }

    // 创建实例信息
    let instance = GLibEpollInstance {
        epfd,
        source_count: AtomicUsize::new(0),
        max_sources: 1024, // 默认最大1024个事件源
        created_timestamp: time::get_timestamp() as u64,
        total_waits: AtomicUsize::new(0),
        total_events: AtomicUsize::new(0),
    };

    // 注册实例
    {
        let mut instances = GLIB_EPOLL_INSTANCES.lock();
        if instances.contains_key(&epfd) {
            crate::println!("[glib_epoll] epoll实例已存在: {}", epfd);
            // 关闭重复的epoll
            let _ = sys_close(epfd);
            return -17; // EEXIST
        }
        instances.insert(epfd, instance);
    }

    crate::println!("[glib_epoll] 成功创建GLib epoll实例: epfd={}", epfd);
    epfd
}

/// 添加事件源到GLib epoll实例
///
/// # 参数
/// * `epfd` - epoll文件描述符
/// * `fd` - 要监听的文件描述符
/// * `events` - 要监听的事件类型
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[unsafe(no_mangle)]
pub extern "C" fn sys_glib_epoll_add_source(epfd: c_int, fd: c_int, events: u32) -> i32 {
    crate::println!("[glib_epoll] 添加事件源: epfd={}, fd={}, events=0x{:x}", epfd, fd, events);

    // 验证参数
    if epfd < 0 || fd < 0 {
        crate::println!("[glib_epoll] 无效的文件描述符: epfd={}, fd={}", epfd, fd);
        return -22; // EINVAL
    }

    // 验证事件类型
    if events == 0 {
        crate::println!("[glib_epoll] 无效的事件类型: 0x{:x}", events);
        return -22; // EINVAL
    }

    // 检查实例是否存在
    let max_sources = {
        let instances = GLIB_EPOLL_INSTANCES.lock();
        match instances.get(&epfd) {
            Some(instance) => instance.max_sources,
            None => {
                crate::println!("[glib_epoll] epoll实例不存在: {}", epfd);
                return -2; // ENOENT
            },
        }
    };

    // 检查事件源数量限制
    let current_sources = {
        let instances = GLIB_EPOLL_INSTANCES.lock();
        match instances.get(&epfd) {
            Some(instance) => instance.source_count.load(Ordering::SeqCst),
            None => return -2, // ENOENT
        }
    };

    if current_sources >= max_sources {
        crate::println!("[glib_epoll] 事件源数量超过限制: {}/{}", current_sources, max_sources);
        return -38; // ENOSYS
    }

    // 创建epoll事件
    let epoll_event = EpollEventInfo {
        events,
        data: fd as u64, // 使用fd作为data
    };

    // 添加到epoll
    match EpollManager::add(epfd, fd, &epoll_event) {
        Ok(_) => {
            // 更新实例统计
            {
                let mut instances = GLIB_EPOLL_INSTANCES.lock();
                if let Some(instance) = instances.get_mut(&epfd) {
                    instance.source_count.fetch_add(1, Ordering::SeqCst);
                }
            }

            crate::println!(
                "[glib_epoll] 成功添加事件源: epfd={}, fd={}, events=0x{:x}",
                epfd,
                fd,
                events
            );
            0
        },
        Err(e) => {
            crate::println!("[glib_epoll] 添加事件源失败: epfd={}, fd={}, error={:?}", epfd, fd, e);
            -5 // EIO
        },
    }
}

/// 从GLib epoll实例移除事件源
///
/// # 参数
/// * `epfd` - epoll文件描述符
/// * `fd` - 要移除的文件描述符
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[unsafe(no_mangle)]
pub extern "C" fn sys_glib_epoll_remove_source(epfd: c_int, fd: c_int) -> i32 {
    crate::println!("[glib_epoll] 移除事件源: epfd={}, fd={}", epfd, fd);

    // 验证参数
    if epfd < 0 || fd < 0 {
        crate::println!("[glib_epoll] 无效的文件描述符: epfd={}, fd={}", epfd, fd);
        return -22; // EINVAL
    }

    // 检查实例是否存在
    {
        let instances = GLIB_EPOLL_INSTANCES.lock();
        if !instances.contains_key(&epfd) {
            crate::println!("[glib_epoll] epoll实例不存在: {}", epfd);
            return -2; // ENOENT
        }
    }

    // 从epoll移除
    match EpollManager::remove(epfd, fd) {
        Ok(_) => {
            // 更新实例统计
            {
                let mut instances = GLIB_EPOLL_INSTANCES.lock();
                if let Some(instance) = instances.get_mut(&epfd) {
                    instance.source_count.fetch_sub(1, Ordering::SeqCst);
                }
            }

            crate::println!("[glib_epoll] 成功移除事件源: epfd={}, fd={}", epfd, fd);
            0
        },
        Err(e) => {
            crate::println!("[glib_epoll] 移除事件源失败: epfd={}, fd={}, error={:?}", epfd, fd, e);
            -5 // EIO
        },
    }
}

/// 等待GLib epoll事件
///
/// # 参数
/// * `epfd` - epoll文件描述符
/// * `events` - 用于存储事件的缓冲区
/// * `maxevents` - 缓冲区最大事件数
/// * `timeout` - 超时时间（毫秒），-1表示无限等待
///
/// # 返回值
/// * 成功时返回实际获取的事件数量
/// * 失败时返回负数错误码
#[unsafe(no_mangle)]
pub extern "C" fn sys_glib_epoll_wait(
    epfd: c_int,
    events: *mut EpollEvent,
    maxevents: c_int,
    timeout: c_int,
) -> i32 {
    crate::println!(
        "[glib_epoll] 等待事件: epfd={}, maxevents={}, timeout={}",
        epfd,
        maxevents,
        timeout
    );

    // 验证参数
    if epfd < 0 || maxevents <= 0 || events.is_null() {
        crate::println!(
            "[glib_epoll] 无效参数: epfd={}, maxevents={}, events={:p}",
            epfd,
            maxevents,
            events
        );
        return -22; // EINVAL
    }

    // 检查实例是否存在
    {
        let instances = GLIB_EPOLL_INSTANCES.lock();
        if !instances.contains_key(&epfd) {
            crate::println!("[glib_epoll] epoll实例不存在: {}", epfd);
            return -2; // ENOENT
        }
    }

    // 创建事件切片
    let event_slice = unsafe { core::slice::from_raw_parts_mut(events, maxevents as usize) };

    // 等待事件
    let start_time = time::get_timestamp();
    let result = EpollManager::wait(epfd, event_slice, timeout as i32);
    let wait_time = time::get_timestamp() - start_time;

    // 更新统计信息
    {
        let mut instances = GLIB_EPOLL_INSTANCES.lock();
        if let Some(instance) = instances.get_mut(&epfd) {
            instance.total_waits.fetch_add(1, Ordering::SeqCst);
            if result > 0 {
                instance
                    .total_events
                    .fetch_add(result as usize, Ordering::SeqCst);
            }
        }
    }

    if result >= 0 {
        crate::println!("[glib_epoll] 获取事件: {} 个，耗时: {}ms", result, wait_time);

        // 打印事件详情（仅前几个，避免日志过多）
        let print_count = core::cmp::min(result as usize, 5);
        for i in 0..print_count {
            let event = unsafe { &*events.add(i) };
            crate::println!(
                "[glib_epoll] 事件 {}: fd={}, events=0x{:x}",
                i,
                event.data as c_int,
                event.events
            );
        }

        result
    } else {
        crate::println!("[glib_epoll] 等待超时或错误: {}, 耗时: {}ms", result, wait_time);
        result
    }
}

/// 修改事件源的监听事件
///
/// # 参数
/// * `epfd` - epoll文件描述符
/// * `fd` - 文件描述符
/// * `events` - 新的事件类型
/// * `op` - 操作类型 (1=添加, 2=修改, 3=删除)
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[unsafe(no_mangle)]
pub extern "C" fn sys_glib_epoll_mod_source(
    epfd: c_int,
    fd: c_int,
    events: u32,
    op: c_int,
) -> i32 {
    crate::println!(
        "[glib_epoll] 修改事件源: epfd={}, fd={}, events=0x{:x}, op={}",
        epfd,
        fd,
        events,
        op
    );

    // 验证参数
    if epfd < 0 || fd < 0 {
        crate::println!("[glib_epoll] 无效的文件描述符: epfd={}, fd={}", epfd, fd);
        return -22; // EINVAL
    }

    // 检查实例是否存在
    {
        let instances = GLIB_EPOLL_INSTANCES.lock();
        if !instances.contains_key(&epfd) {
            crate::println!("[glib_epoll] epoll实例不存在: {}", epfd);
            return -2; // ENOENT
        }
    }

    // 创建epoll事件
    let epoll_event = EpollEventInfo { events, data: fd as u64 };

    // 执行操作
    match op {
        1 => {
            // 添加事件源
            match EpollManager::add(epfd, fd, &epoll_event) {
                Ok(_) => {
                    // 更新实例统计
                    {
                        let mut instances = GLIB_EPOLL_INSTANCES.lock();
                        if let Some(instance) = instances.get_mut(&epfd) {
                            instance.source_count.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    crate::println!("[glib_epoll] 修改操作: 添加事件源 epfd={}, fd={}", epfd, fd);
                    0
                },
                Err(e) => {
                    crate::println!("[glib_epoll] 添加事件源失败: {:?}", e);
                    return -5; // EIO
                },
            }
        },
        2 => {
            // 修改事件源
            match EpollManager::modify(epfd, fd, &epoll_event) {
                Ok(_) => {
                    crate::println!(
                        "[glib_epoll] 修改操作: 修改事件源 epfd={}, fd={}, events=0x{:x}",
                        epfd,
                        fd,
                        events
                    );
                    0
                },
                Err(e) => {
                    crate::println!("[glib_epoll] 修改事件源失败: {:?}", e);
                    return -5; // EIO
                },
            }
        },
        3 => {
            // 删除事件源
            match EpollManager::remove(epfd, fd) {
                Ok(_) => {
                    // 更新实例统计
                    {
                        let mut instances = GLIB_EPOLL_INSTANCES.lock();
                        if let Some(instance) = instances.get_mut(&epfd) {
                            instance.source_count.fetch_sub(1, Ordering::SeqCst);
                        }
                    }
                    crate::println!("[glib_epoll] 修改操作: 删除事件源 epfd={}, fd={}", epfd, fd);
                    0
                },
                Err(e) => {
                    crate::println!("[glib_epoll] 删除事件源失败: {:?}", e);
                    return -5; // EIO
                },
            }
        },
        _ => {
            crate::println!("[glib_epoll] 无效的操作类型: {}", op);
            return -22; // EINVAL
        }
    }
}

/// 关闭GLib epoll实例
///
/// # 参数
/// * `epfd` - epoll文件描述符
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[unsafe(no_mangle)]
pub extern "C" fn sys_glib_epoll_close(epfd: c_int) -> i32 {
    crate::println!("[glib_epoll] 关闭epoll实例: {}", epfd);

    // 验证参数
    if epfd < 0 {
        crate::println!("[glib_epoll] 无效的epoll文件描述符: {}", epfd);
        return -22; // EINVAL
    }

    // 获取实例信息
    let (total_waits, total_events, created_timestamp) = {
        let mut instances = GLIB_EPOLL_INSTANCES.lock();
        match instances.remove(&epfd) {
            Some(instance) => (
                instance.total_waits.load(Ordering::SeqCst),
                instance.total_events.load(Ordering::SeqCst),
                instance.created_timestamp,
            ),
            None => {
                crate::println!("[glib_epoll] epoll实例不存在: {}", epfd);
                return -2; // ENOENT
            },
        }
    };

    let uptime = time::get_timestamp() as u64 - created_timestamp;

    crate::println!(
        "[glib_epoll] 实例统计: 总等待={}, 总事件={}, 运行时间={}ms",
        total_waits,
        total_events,
        uptime
    );

    // 关闭epoll文件描述符
    let close_result = sys_close(epfd);
    if close_result == 0 {
        crate::println!("[glib_epoll] 成功关闭epoll实例: {}", epfd);
        0
    } else {
        crate::println!("[glib_epoll] 关闭epoll失败: epfd={}, error={}", epfd, close_result);
        -5 // EIO
    }
}

/// 获取GLib epoll实例统计信息
///
/// # 参数
/// * `epfd` - epoll文件描述符
/// * `stats` - 用于存储统计信息的结构体指针
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[unsafe(no_mangle)]
pub extern "C" fn sys_glib_epoll_stats(epfd: c_int, stats: *mut GLibEpollInstance) -> c_int {
    crate::println!("[glib_epoll] 获取epoll实例 {} 统计", epfd);

    // 验证参数
    if epfd < 0 || stats.is_null() {
        return -22; // EINVAL
    }

    // 获取实例信息
    let instance = {
        let instances = GLIB_EPOLL_INSTANCES.lock();
        match instances.get(&epfd) {
            Some(instance) => instance.clone(),
            None => {
                crate::println!("[glib_epoll] epoll实例不存在: {}", epfd);
                return -2; // ENOENT
            },
        }
    };

    // 复制统计信息
    unsafe {
        *stats = GLibEpollInstance {
            epfd: instance.epfd,
            source_count: AtomicUsize::new(instance.source_count.load(Ordering::SeqCst)),
            max_sources: instance.max_sources,
            created_timestamp: instance.created_timestamp,
            total_waits: AtomicUsize::new(instance.total_waits.load(Ordering::SeqCst)),
            total_events: AtomicUsize::new(instance.total_events.load(Ordering::SeqCst)),
        };
    }

    0
}

/// 清空所有GLib epoll实例（用于调试）
#[unsafe(no_mangle)]
pub extern "C" fn sys_glib_epoll_cleanup() -> c_int {
    crate::println!("[glib_epoll] 清理所有GLib epoll实例");

    let mut total_instances = 0;
    let mut total_waits = 0;
    let mut total_events = 0;

    {
        let mut instances = GLIB_EPOLL_INSTANCES.lock();
        let epfd_list: Vec<c_int> = instances.keys().cloned().collect();

        for epfd in epfd_list {
            if let Some(instance) = instances.get(&epfd) {
                total_instances += 1;
                total_waits += instance.total_waits.load(Ordering::SeqCst);
                total_events += instance.total_events.load(Ordering::SeqCst);
            }
            instances.remove(&epfd);

            // 关闭epoll文件描述符
            let _ = sys_close(epfd);
        }
    }

    crate::println!(
        "[glib_epoll] 清理完成: {} 个实例, {} 次等待, {} 个事件",
        total_instances,
        total_waits,
        total_events
    );

    // 重置ID计数器
    NEXT_EPOLL_ID.store(1, Ordering::SeqCst);

    0
}
