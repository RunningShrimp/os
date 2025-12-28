//! Async operation management functions

use super::*;

/// 提交异步读操作
///
/// # 参数
/// * `context_id` - 上下文ID
/// * `fd` - 文件描述符
/// * `buffer` - 缓冲区指针
/// * `size` - 缓冲区大小
/// * `offset` - 文件偏移量（-1表示当前位置）
/// * `callback` - 完成回调函数指针
/// * `user_data` - 用户数据指针
/// * `timeout` - 超时时间（毫秒）
///
/// # 返回值
/// * 成功时返回操作ID
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_async_read(
    context_id: u64,
    fd: c_int,
    buffer: *mut c_void,
    size: usize,
    offset: i64,
    callback: *mut c_void,
    user_data: *mut c_void,
    timeout: u32,
) -> SyscallResult {
    crate::println!("[glib_async] 提交异步读: context={}, fd={}, size={}, offset={}, timeout={}",
        context_id, fd, size, offset, timeout);

    // 验证参数
    if context_id == 0 || fd < 0 || buffer.is_null() || size == 0 {
        crate::println!("[glib_async] 无效参数: context={}, fd={}, buffer={:p}, size={}",
            context_id, fd, buffer, size);
        return -22; // EINVAL
    }

    // 检查上下文是否存在
    let max_operations = {
        let contexts = ASYNC_CONTEXTS.lock();
        match contexts.get(&context_id) {
            Some(context) => {
                // 检查并发操作限制
                if context.active_operations.load(Ordering::SeqCst) >= context.max_operations {
                    crate::println!("[glib_async] 并发操作数超过限制: {}/{}",
                        context.active_operations.load(Ordering::SeqCst), context.max_operations);
                    return -28; // ENOSPC
                }
                context.max_operations
            }
            None => {
                crate::println!("[glib_async] 异步上下文不存在: {}", context_id);
                return -2; // ENOENT
            }
        }
    };

    // 分配操作ID
    let operation_id = NEXT_OPERATION_ID.fetch_add(1, Ordering::SeqCst) as u64;
    if operation_id == 0 {
        crate::println!("[glib_async] 操作ID溢出");
        return -12; // ENOMEM
    }

    // 创建操作信息
    let operation_info = AsyncOperationInfo {
        operation_id,
        operation_type: AsyncOperationType::Read,
        fd,
        buffer,
        buffer_size: size,
        status: AsyncOperationStatus::Submitted,
        bytes_completed: AtomicUsize::new(0),
        error_code: 0,
        user_data,
        callback,
        timeout,
        created_timestamp: crate::subsystems::time::get_timestamp() as u64,
        completed_timestamp: 0,
    };

    // 注册操作
    {
        let mut operations = ASYNC_OPERATIONS.lock();
        operations.insert(operation_id, operation_info);
    }

    // 更新上下文统计
    {
        let contexts = ASYNC_CONTEXTS.lock();
        if let Some(context) = contexts.get(&context_id) {
            context.active_operations.fetch_add(1, Ordering::SeqCst);
            context.total_operations.fetch_add(1, Ordering::SeqCst);
        }
    }

    crate::println!("[glib_async] 成功提交异步读操作: ID={}, fd={}, size={}",
        operation_id, fd, size);
    operation_id as SyscallResult
}

/// 提交异步写操作
///
/// # 参数
/// * `context_id` - 上下文ID
/// * `fd` - 文件描述符
/// * `buffer` - 缓冲区指针
/// * `size` - 缓冲区大小
/// * `offset` - 文件偏移量（-1表示当前位置）
/// * `callback` - 完成回调函数指针
/// * `user_data` - 用户数据指针
/// * `timeout` - 超时时间（毫秒）
///
/// # 返回值
/// * 成功时返回操作ID
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_async_write(
    context_id: u64,
    fd: c_int,
    buffer: *const c_void,
    size: usize,
    offset: i64,
    callback: *mut c_void,
    user_data: *mut c_void,
    timeout: u32,
) -> SyscallResult {
    crate::println!("[glib_async] 提交异步写: context={}, fd={}, size={}, offset={}, timeout={}",
        context_id, fd, size, offset, timeout);

    // 验证参数
    if context_id == 0 || fd < 0 || buffer.is_null() || size == 0 {
        crate::println!("[glib_async] 无效参数: context={}, fd={}, buffer={:p}, size={}",
            context_id, fd, buffer, size);
        return -22; // EINVAL
    }

    // 检查上下文是否存在
    {
        let contexts = ASYNC_CONTEXTS.lock();
        match contexts.get(&context_id) {
            Some(context) => {
                if context.active_operations.load(Ordering::SeqCst) >= context.max_operations {
                    crate::println!("[glib_async] 并发操作数超过限制");
                    return -28; // ENOSPC
                }
            }
            None => {
                crate::println!("[glib_async] 异步上下文不存在: {}", context_id);
                return -2; // ENOENT
            }
        }
    }

    // 分配操作ID
    let operation_id = NEXT_OPERATION_ID.fetch_add(1, Ordering::SeqCst) as u64;
    if operation_id == 0 {
        crate::println!("[glib_async] 操作ID溢出");
        return -12; // ENOMEM
    }

    // 创建操作信息
    let operation_info = AsyncOperationInfo {
        operation_id,
        operation_type: AsyncOperationType::Write,
        fd,
        buffer: buffer as *mut c_void,
        buffer_size: size,
        status: AsyncOperationStatus::Submitted,
        bytes_completed: AtomicUsize::new(0),
        error_code: 0,
        user_data,
        callback,
        timeout,
        created_timestamp: crate::subsystems::time::get_timestamp() as u64,
        completed_timestamp: 0,
    };

    // 注册操作
    {
        let mut operations = ASYNC_OPERATIONS.lock();
        operations.insert(operation_id, operation_info);
    }

    // 更新上下文统计
    {
        let contexts = ASYNC_CONTEXTS.lock();
        if let Some(context) = contexts.get(&context_id) {
            context.active_operations.fetch_add(1, Ordering::SeqCst);
            context.total_operations.fetch_add(1, Ordering::SeqCst);
        }
    }

    crate::println!("[glib_async] 成功提交异步写操作: ID={}, fd={}, size={}",
        operation_id, fd, size);
    operation_id as SyscallResult
}

/// 取消异步操作
///
/// # 参数
/// * `operation_id` - 操作ID
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_async_cancel(operation_id: u64) -> SyscallResult {
    crate::println!("[glib_async] 取消异步操作: {}", operation_id);

    // 验证参数
    if operation_id == 0 {
        crate::println!("[glib_async] 无效的操作ID: {}", operation_id);
        return -22; // EINVAL
    }

    // 获取操作并更新状态
    let context_id = {
        let mut operations = ASYNC_OPERATIONS.lock();
        match operations.get_mut(&operation_id) {
            Some(operation_info) => {
                match operation_info.status {
                    AsyncOperationStatus::Submitted | AsyncOperationStatus::InProgress => {
                        operation_info.status = AsyncOperationStatus::Cancelled;
                        operation_info.completed_timestamp = crate::subsystems::time::get_timestamp() as u64;
                        operation_info.error_code = -125; // ECANCELED
                        crate::println!("[glib_async] 操作已取消: {}", operation_id);
                    }
                    _ => {
                        crate::println!("[glib_async] 操作已完成，无法取消: {}", operation_id);
                        return -22; // EINVAL
                    }
                }
                0 // 这里需要通过某种方式获取context_id，暂时返回0
            }
            None => {
                crate::println!("[glib_async] 异步操作不存在: {}", operation_id);
                return -2; // ENOENT
            }
        }
    };

    // 更新上下文统计（减少活跃操作数）
    if context_id != 0 {
        let contexts = ASYNC_CONTEXTS.lock();
        if let Some(context) = contexts.get(&context_id) {
            context.active_operations.fetch_sub(1, Ordering::SeqCst);
            context.failed_operations.fetch_add(1, Ordering::SeqCst);
        }
    }

    0
}

/// 查询异步操作状态
///
/// # 参数
/// * `operation_id` - 操作ID
/// * `status` - 用于存储状态的指针
/// * `bytes_completed` - 用于存储已完成字节数的指针
/// * `error_code` - 用于存储错误码的指针
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_async_query(
    operation_id: u64,
    status: *mut AsyncOperationStatus,
    bytes_completed: *mut usize,
    error_code: *mut c_int,
) -> SyscallResult {
    crate::println!("[glib_async] 查询操作状态: {}", operation_id);

    // 验证参数
    if operation_id == 0 || status.is_null() || bytes_completed.is_null() || error_code.is_null() {
        crate::println!("[glib_async] 无效参数: operation={}, status={:p}, bytes={:p}, error={:p}",
            operation_id, status, bytes_completed, error_code);
        return -22; // EINVAL
    }

    // 获取操作信息
    let operation_info = {
        let operations = ASYNC_OPERATIONS.lock();
        match operations.get(&operation_id) {
            Some(info) => info.clone(),
            None => {
                crate::println!("[glib_async] 异步操作不存在: {}", operation_id);
                return -2; // ENOENT
            }
        }
    };

    // 填充返回信息
    unsafe {
        *status = operation_info.status;
        *bytes_completed = operation_info.bytes_completed.load(Ordering::SeqCst);
        *error_code = operation_info.error_code;
    }

    crate::println!("[glib_async] 操作状态查询完成: ID={}, status={:?}, bytes={}, error={}",
        operation_id, operation_info.status,
        operation_info.bytes_completed.load(Ordering::SeqCst), operation_info.error_code);
    0
}

/// 完成异步操作（由内核内部调用）
///
/// # 参数
/// * `operation_id` - 操作ID
/// * `bytes_transferred` - 传输的字节数
/// * `error_code` - 错误码（0表示成功）
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_async_complete(
    operation_id: u64,
    bytes_transferred: usize,
    error_code: c_int,
) -> SyscallResult {
    crate::println!("[glib_async] 完成异步操作: ID={}, bytes={}, error={}",
        operation_id, bytes_transferred, error_code);

    // 验证参数
    if operation_id == 0 {
        crate::println!("[glib_async] 无效的操作ID: {}", operation_id);
        return -22; // EINVAL
    }

    // 更新操作状态
    let (context_id, operation_type) = {
        let mut operations = ASYNC_OPERATIONS.lock();
        match operations.get_mut(&operation_id) {
            Some(operation_info) => {
                operation_info.bytes_completed.store(bytes_transferred, Ordering::SeqCst);
                operation_info.error_code = error_code;
                operation_info.completed_timestamp = crate::subsystems::time::get_timestamp() as u64;

                if error_code == 0 {
                    operation_info.status = AsyncOperationStatus::Completed;
                } else {
                    operation_info.status = AsyncOperationStatus::Failed;
                }

                (0, operation_info.operation_type) // context_id需要从其他地方获取，暂时返回0
            }
            None => {
                crate::println!("[glib_async] 异步操作不存在: {}", operation_id);
                return -2; // ENOENT
            }
        }
    };

    // 更新上下文统计
    if context_id != 0 {
        let contexts = ASYNC_CONTEXTS.lock();
        if let Some(context) = contexts.get(&context_id) {
            context.active_operations.fetch_sub(1, Ordering::SeqCst);
            if error_code == 0 {
                context.successful_operations.fetch_add(1, Ordering::SeqCst);
            } else {
                context.failed_operations.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    crate::println!("[glib_async] 异步操作完成处理完成: ID={}, type={:?}",
        operation_id, operation_type);
    0
}

/// 等待异步操作完成
///
/// # 参数
/// * `operation_id` - 操作ID
/// * `timeout` - 等待超时时间（毫秒，0表示无限等待）
///
/// # 返回值
/// * 成功时返回0
/// * 超时时返回-62 (ETIMEDOUT)
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_async_wait(operation_id: u64, timeout: u32) -> SyscallResult {
    crate::println!("[glib_async] 等待异步操作完成: ID={}, timeout={}", operation_id, timeout);

    // 验证参数
    if operation_id == 0 {
        crate::println!("[glib_async] 无效的操作ID: {}", operation_id);
        return -22; // EINVAL
    }

    let start_time = crate::subsystems::time::get_timestamp();
    let timeout_ms = if timeout == 0 { u64::MAX } else { timeout as u64 };

    loop {
        // 检查操作状态
        let (is_completed, error_code) = {
            let operations = ASYNC_OPERATIONS.lock();
            match operations.get(&operation_id) {
                Some(operation_info) => {
                    let is_done = matches!(operation_info.status,
                        AsyncOperationStatus::Completed | AsyncOperationStatus::Failed | AsyncOperationStatus::Cancelled);
                    (is_done, operation_info.error_code)
                }
                None => {
                    crate::println!("[glib_async] 异步操作不存在: {}", operation_id);
                    return -2; // ENOENT
                }
            }
        };

        if is_completed {
            crate::println!("[glib_async] 操作已完成: ID={}", operation_id);
            return error_code;
        }

        // 检查超时
        let elapsed = crate::subsystems::time::get_timestamp() - start_time;
        if elapsed >= timeout_ms {
            crate::println!("[glib_async] 等待超时: ID={}, elapsed={}ms", operation_id, elapsed);
            return -62; // ETIMEDOUT
        }

        // 简单的等待实现（实际应该使用更高效的等待机制）
        // 这里可以使用epoll或其他事件机制来改进
        crate::subsystems::time::sleep(core::time::Duration::from_millis(10));
    }
}

/// 清理所有异步操作（用于调试）
#[no_mangle]
pub extern "C" fn sys_glib_async_cleanup() -> c_int {
    crate::println!("[glib_async] 清理所有GLib异步I/O");

    let mut total_operations = 0;
    let mut total_contexts = 0;
    let mut leaked_operations = 0;

    {
        let operations = ASYNC_OPERATIONS.lock();
        total_operations = operations.len();

        for (_, operation_info) in operations.iter() {
            match operation_info.status {
                AsyncOperationStatus::Submitted | AsyncOperationStatus::InProgress => {
                    leaked_operations += 1;
                    crate::println!("[glib_async] 泄漏警告: 操作 {} 仍在进行", operation_info.operation_id);
                }
                _ => {}
            }
        }
    }

    {
        let contexts = ASYNC_CONTEXTS.lock();
        total_contexts = contexts.len();

        for (_, context_info) in contexts.iter() {
            let active = context_info.active_operations.load(Ordering::SeqCst);
            if active > 0 {
                crate::println!("[glib_async] 泄漏警告: 上下文 {} 仍有 {} 个活跃操作",
                    context_info.name, active);
                leaked_operations += active;
            }
        }
    }

    // 清理注册表
    {
        let mut operations = ASYNC_OPERATIONS.lock();
        operations.clear();
    }

    {
        let mut contexts = ASYNC_CONTEXTS.lock();
        contexts.clear();
    }

    // 重置ID计数器
    NEXT_OPERATION_ID.store(1, Ordering::SeqCst);
    NEXT_CONTEXT_ID.store(1, Ordering::SeqCst);

    crate::println!("[glib_async] 清理完成: {} 个操作, {} 个上下文, {} 个泄漏",
        total_operations, total_contexts, leaked_operations);

    0
}