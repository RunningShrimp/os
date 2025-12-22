//! Core epoll instance management functions

use super::*;

/// 创建GLib专用epoll实例
///
/// # 返回值
/// * 成功时返回epoll文件描述符
/// * 失败时返回EpollError
#[no_mangle]
pub extern "C" fn sys_glib_epoll_create() -> EpollResult<c_int> {
    crate::println!("[glib_epoll] 创建GLib专用epoll实例");

    // 创建epoll文件描述符
    let epfd = match EpollManager::create() {
        Ok(fd) => fd,
        Err(_) => {
            crate::println!("[glib_epoll] 创建epoll失败");
            return Err(EpollError::DeviceError);
        }
    };

    if epfd < 0 {
        crate::println!("[glib_epoll] epoll文件描述符无效: {}", epfd);
        return Err(EpollError::DeviceError);
    }

    // 创建实例信息
    let instance = GLibEpollInstance {
        epfd,
        source_count: AtomicUsize::new(0),
        max_sources: 1024, // 默认最大1024个事件源
        created_timestamp: crate::subsystems::time::get_timestamp() as u64,
        total_waits: AtomicUsize::new(0),
        total_events: AtomicUsize::new(0),
    };

    // 注册实例
    {
        let mut instances = GLIB_EPOLL_INSTANCES.lock();
        if instances.contains_key(&epfd) {
            crate::println!("[glib_epoll] epoll实例已存在: {}", epfd);
            // 关闭重复的epoll
            crate::syscalls::close(epfd);
            return Err(EpollError::AlreadyExists);
        }
        instances.insert(epfd, instance);
    }

    crate::println!("[glib_epoll] 成功创建GLib epoll实例: epfd={}", epfd);
    Ok(epfd)
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
#[no_mangle]
pub extern "C" fn sys_glib_epoll_add_source(epfd: c_int, fd: c_int, events: u32) -> SyscallResult {
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
            }
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
        return -28; // ENOSPC
    }

    // 创建epoll事件
    let mut epoll_event = EpollEvent {
        events,
        data: fd as u64, // 使用fd作为data
        ..Default::default()
    };

    // 添加到epoll
    match EpollManager::add(epfd, fd, &epoll_event) {
        Ok(()) => {
            // 更新实例统计
            {
                let instances = GLIB_EPOLL_INSTANCES.lock();
                if let Some(instance) = instances.get_mut(&epfd) {
                    instance.source_count.fetch_add(1, Ordering::SeqCst);
                }
            }

            crate::println!("[glib_epoll] 成功添加事件源: epfd={}, fd={}, events=0x{:x}", epfd, fd, events);
            0
        }
        Err(e) => {
            crate::println!("[glib_epoll] 添加事件源失败: epfd={}, fd={}, error={:?}", epfd, fd, e);
            -1
        }
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
#[no_mangle]
pub extern "C" fn sys_glib_epoll_remove_source(epfd: c_int, fd: c_int) -> SyscallResult {
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
        Ok(()) => {
            // 更新实例统计
            {
                let instances = GLIB_EPOLL_INSTANCES.lock();
                if let Some(instance) = instances.get_mut(&epfd) {
                    instance.source_count.fetch_sub(1, Ordering::SeqCst);
                }
            }

            crate::println!("[glib_epoll] 成功移除事件源: epfd={}, fd={}", epfd, fd);
            0
        }
        Err(e) => {
            crate::println!("[glib_epoll] 移除事件源失败: epfd={}, fd={}, error={:?}", epfd, fd, e);
            -1
        }
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
#[no_mangle]
pub extern "C" fn sys_glib_epoll_wait(
    epfd: c_int,
    events: *mut EpollEvent,
    maxevents: c_int,
    timeout: c_int,
) -> SyscallResult {
    crate::println!("[glib_epoll] 等待事件: epfd={}, maxevents={}, timeout={}", epfd, maxevents, timeout);

    // 验证参数
    if epfd < 0 || maxevents <= 0 || events.is_null() {
        crate::println!("[glib_epoll] 无效参数: epfd={}, maxevents={}, events={:p}", epfd, maxevents, events);
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
    let event_slice = unsafe {
        core::slice::from_raw_parts_mut(events, maxevents as usize)
    };

    // 等待事件
    let start_time = crate::subsystems::time::get_timestamp();
    let result = EpollManager::wait(epfd, event_slice, timeout as i32);
    let wait_time = crate::subsystems::time::get_timestamp() - start_time;

    // 更新统计信息
    {
        let instances = GLIB_EPOLL_INSTANCES.lock();
        if let Some(instance) = instances.get_mut(&epfd) {
            instance.total_waits.fetch_add(1, Ordering::SeqCst);
            if result > 0 {
                instance.total_events.fetch_add(result as usize, Ordering::SeqCst);
            }
        }
    }

    if result >= 0 {
        crate::println!("[glib_epoll] 获取事件: {} 个，耗时: {}ms", result, wait_time);

        // 打印事件详情（仅前几个，避免日志过多）
        let print_count = core::cmp::min(result as usize, 5);
        for i in 0..print_count {
            let event = unsafe { &*events.add(i) };
            crate::println!("[glib_epoll] 事件 {}: fd={}, events=0x{:x}",
                i, event.data as c_int, event.events);
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
#[no_mangle]
pub extern "C" fn sys_glib_epoll_mod_source(
    epfd: c_int,
    fd: c_int,
    events: u32,
    op: c_int,
) -> SyscallResult {
    crate::println!("[glib_epoll] 修改事件源: epfd={}, fd={}, events=0x{:x}, op={}", epfd, fd, events, op);

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
    let epoll_event = EpollEvent {
        events,
        data: fd as u64,
        ..Default::default()
    };

    // 执行操作
    let result = match op {
        1 => {
            // 添加事件源
            match EpollManager::add(epfd, fd, &epoll_event) {
                Ok(()) => {
                    // 更新实例统计
                    {
                        let instances = GLIB_EPOLL_INSTANCES.lock();
                        if let Some(instance) = instances.get_mut(&epfd) {
                            instance.source_count.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    crate::println!("[glib_epoll] 修改操作: 添加事件源 epfd={}, fd={}", epfd, fd);
                    0
                }
                Err(e) => {
                    crate::println!("[glib_epoll] 添加事件源失败: {:?}", e);
                    -1
                }
            }
        }
        2 => {
            // 修改事件源
            match EpollManager::modify(epfd, fd, &epoll_event) {
                Ok(()) => {
                    crate::println!("[glib_epoll] 修改操作: 修改事件源 epfd={}, fd={}, events=0x{:x}", epfd, fd, events);
                    0
                }
                Err(e) => {
                    crate::println!("[glib_epoll] 修改事件源失败: {:?}", e);
                    -1
                }
            }
        }
        3 => {
            // 删除事件源
            match EpollManager::remove(epfd, fd) {
                Ok(()) => {
                    // 更新实例统计
                    {
                        let instances = GLIB_EPOLL_INSTANCES.lock();
                        if let Some(instance) = instances.get_mut(&epfd) {
                            instance.source_count.fetch_sub(1, Ordering::SeqCst);
                        }
                    }
                    crate::println!("[glib_epoll] 修改操作: 删除事件源 epfd={}, fd={}", epfd, fd);
                    0
                }
                Err(e) => {
                    crate::println!("[glib_epoll] 删除事件源失败: {:?}", e);
                    -1
                }
            }
        }
        _ => {
            crate::println!("[glib_epoll] 无效的操作类型: {}", op);
            -22; // EINVAL
        }
    };

    result
}

/// 关闭GLib epoll实例
///
/// # 参数
/// * `epfd` - epoll文件描述符
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_epoll_close(epfd: c_int) -> SyscallResult {
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
            }
        }
    };

    let uptime = crate::subsystems::time::get_timestamp() as u64 - created_timestamp;

    crate::println!("[glib_epoll] 实例统计: 总等待={}, 总事件={}, 运行时间={}ms",
        total_waits, total_events, uptime);

    // 关闭epoll文件描述符
    match crate::syscalls::close(epfd) {
        0 => {
            crate::println!("[glib_epoll] 成功关闭epoll实例: {}", epfd);
            0
        }
        err => {
            crate::println!("[glib_epoll] 关闭epoll失败: epfd={}, error={}", epfd, err);
            err
        }
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
#[no_mangle]
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
            }
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
#[no_mangle]
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
            crate::syscalls::close(epfd);
        }
    }

    crate::println!("[glib_epoll] 清理完成: {} 个实例, {} 次等待, {} 个事件",
        total_instances, total_waits, total_events);

    // 重置ID计数器
    NEXT_EPOLL_ID.store(1, Ordering::SeqCst);

    0
}