//! Async I/O context management functions

use super::*;
use core::ffi::c_char;

/// 创建异步I/O上下文
///
/// # 参数
/// * `name` - 上下文名称
/// * `max_operations` - 最大并发操作数
///
/// # 返回值
/// * 成功时返回上下文ID
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_async_context_create(
    name: *const c_char,
    max_operations: usize,
) -> SyscallResult {
    crate::println!("[glib_async] 创建异步I/O上下文: max_ops={}", max_operations);

    // 验证参数
    if name.is_null() {
        crate::println!("[glib_async] 上下文名称为空");
        return -22; // EINVAL
    }

    if max_operations == 0 {
        crate::println!("[glib_async] 最大操作数不能为0");
        return -22; // EINVAL
    }

    // 验证最大操作数限制（最多10000个并发操作）
    if max_operations > 10000 {
        crate::println!("[glib_async] 最大操作数超过限制: {}", max_operations);
        return -22; // EINVAL
    }

    // 读取上下文名称
    let context_name = unsafe {
        let len = (0..).find(|&i| *name.add(i) == 0).unwrap_or(255);
        core::str::from_utf8(core::slice::from_raw_parts(name as *const u8, len))
            .unwrap_or("invalid")
            .to_string()
    };

    if context_name.is_empty() {
        crate::println!("[glib_async] 上下文名称无效");
        return -22; // EINVAL
    }

    // 分配上下文ID
    let context_id = NEXT_CONTEXT_ID.fetch_add(1, Ordering::SeqCst) as u64;
    if context_id == 0 {
        crate::println!("[glib_async] 上下文ID溢出");
        return -12; // ENOMEM
    }

    // 创建上下文信息
    let context_info = AsyncIOContext {
        context_id,
        name: context_name.clone(),
        max_operations,
        active_operations: AtomicUsize::new(0),
        created_timestamp: crate::subsystems::time::get_timestamp() as u64,
        total_operations: AtomicUsize::new(0),
        successful_operations: AtomicUsize::new(0),
        failed_operations: AtomicUsize::new(0),
    };

    // 注册上下文
    {
        let mut contexts = ASYNC_CONTEXTS.lock();
        contexts.insert(context_id, context_info);
    }

    crate::println!("[glib_async] 成功创建异步I/O上下文: {} (ID={})", context_name, context_id);
    context_id as SyscallResult
}

/// 获取异步上下文统计信息
///
/// # 参数
/// * `context_id` - 上下文ID
/// * `total_ops` - 用于存储总操作数的指针
/// * `active_ops` - 用于存储活跃操作数的指针
/// * `successful_ops` - 用于存储成功操作数的指针
/// * `failed_ops` - 用于存储失败操作数的指针
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_async_context_stats(
    context_id: u64,
    total_ops: *mut usize,
    active_ops: *mut usize,
    successful_ops: *mut usize,
    failed_ops: *mut usize,
) -> SyscallResult {
    crate::println!("[glib_async] 获取上下文统计: {}", context_id);

    // 验证参数
    if context_id == 0 || total_ops.is_null() || active_ops.is_null()
        || successful_ops.is_null() || failed_ops.is_null() {
        crate::println!("[glib_async] 无效参数");
        return -22; // EINVAL
    }

    // 获取上下文统计
    let context_info = {
        let contexts = ASYNC_CONTEXTS.lock();
        match contexts.get(&context_id) {
            Some(info) => info.clone(),
            None => {
                crate::println!("[glib_async] 异步上下文不存在: {}", context_id);
                return -2; // ENOENT
            }
        }
    };

    // 填充统计信息
    unsafe {
        *total_ops = context_info.total_operations.load(Ordering::SeqCst);
        *active_ops = context_info.active_operations.load(Ordering::SeqCst);
        *successful_ops = context_info.successful_operations.load(Ordering::SeqCst);
        *failed_ops = context_info.failed_operations.load(Ordering::SeqCst);
    }

    crate::println!("[glib_async] 上下文统计: ID={}, total={}, active={}, success={}, failed={}",
        context_id,
        context_info.total_operations.load(Ordering::SeqCst),
        context_info.active_operations.load(Ordering::SeqCst),
        context_info.successful_operations.load(Ordering::SeqCst),
        context_info.failed_operations.load(Ordering::SeqCst));
    0
}

/// 销毁异步I/O上下文
///
/// # 参数
/// * `context_id` - 上下文ID
///
/// # 返回值
/// * 成功时返回0
/// * 失败时返回负数错误码
#[no_mangle]
pub extern "C" fn sys_glib_async_context_destroy(context_id: u64) -> SyscallResult {
    crate::println!("[glib_async] 销毁异步上下文: {}", context_id);

    // 验证参数
    if context_id == 0 {
        crate::println!("[glib_async] 无效的上下文ID: {}", context_id);
        return -22; // EINVAL
    }

    // 获取上下文统计
    let (total_ops, successful_ops, failed_ops, active_ops) = {
        let mut contexts = ASYNC_CONTEXTS.lock();
        match contexts.remove(&context_id) {
            Some(context_info) => {
                let active = context_info.active_operations.load(Ordering::SeqCst);
                if active > 0 {
                    crate::println!("[glib_async] 警告：上下文仍有 {} 个活跃操作", active);
                }
                (
                    context_info.total_operations.load(Ordering::SeqCst),
                    context_info.successful_operations.load(Ordering::SeqCst),
                    context_info.failed_operations.load(Ordering::SeqCst),
                    active,
                )
            }
            None => {
                crate::println!("[glib_async] 异步上下文不存在: {}", context_id);
                return -2; // ENOENT
            }
        }
    };

    crate::println!("[glib_async] 上下文销毁完成: ID={}, total={}, success={}, failed={}, active={}",
        context_id, total_ops, successful_ops, failed_ops, active_ops);
    0
}