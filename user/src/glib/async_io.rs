//! GLib异步I/O系统模块
//!
//! 提供与GLib GIO兼容的异步I/O功能，包括：
//! - 异步上下文管理
//! - 异步文件读写操作
//! - 异步网络通信
//! - 回调和完成通知
//! - 超时和取消机制
//! - 流和输入输出流



extern crate alloc;

use crate::glib::{gboolean, gpointer, error::GError, GObject, g_malloc, g_malloc0, g_free};
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ffi::c_int;
use core::ptr;
use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};


/// 异步操作状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncOperationState {
    /// 操作已提交
    Submitted = 0,
    /// 正在进行
    InProgress = 1,
    /// 已完成
    Completed = 2,
    /// 因错误失败
    Failed = 3,
    /// 已取消
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
    /// 查询操作
    Query = 6,
}

/// 异步操作结果
#[derive(Debug, Clone)]
pub struct AsyncResult {
    pub operation_id: u64,
    pub state: AsyncOperationState,
    pub bytes_transferred: usize,
    pub error_code: c_int,
    pub user_data: gpointer,
}

/// 异步回调函数类型
pub type AsyncReadyCallback = unsafe extern "C" fn(*mut GObject, *mut AsyncResult, gpointer);

/// 异步操作信息
#[derive(Debug)]
pub struct AsyncOperation {
    pub operation_id: u64,
    pub operation_type: AsyncOperationType,
    pub state: AsyncOperationState,
    pub fd: c_int,
    pub buffer: *mut c_void,
    pub buffer_size: usize,
    pub bytes_transferred: usize,
    pub error_code: c_int,
    pub callback: AsyncReadyCallback,
    pub user_data: gpointer,
    pub source_object: *mut GObject,
    pub timeout_ms: u32,
    pub created_timestamp: u64,
    pub completed_timestamp: u64,
}

/// 异步I/O上下文
#[derive(Debug)]
pub struct AsyncIOContext {
    pub context_id: u64,
    pub name: String,
    pub max_operations: usize,
    pub active_operations: BTreeMap<u64, *mut AsyncOperation>,
    pub next_operation_id: AtomicUsize,
    pub created_timestamp: u64,
}

/// 异步输入流基础接口
#[derive(Debug)]
pub struct GInputStream {
    pub parent_instance: GObject,
    pub async_context: *mut AsyncIOContext,
}

/// 异步输出流基础接口
#[derive(Debug)]
pub struct GOutputStream {
    pub parent_instance: GObject,
    pub async_context: *mut AsyncIOContext,
}

/// 异步结果结构
#[derive(Debug)]
pub struct GAsyncResult {
    pub source_object: *mut GObject,
    pub user_data: gpointer,
    pub operation_id: u64,
}

/// 文件输入流
#[derive(Debug)]
pub struct GFileInputStream {
    pub parent_instance: GInputStream,
    pub file_path: String,
    pub file_descriptor: c_int,
}

/// 文件输出流
#[derive(Debug)]
pub struct GFileOutputStream {
    pub parent_instance: GOutputStream,
    pub file_path: String,
    pub file_descriptor: c_int,
}

/// 套接字输入流
#[derive(Debug)]
pub struct GSocketInputStream {
    pub parent_instance: GInputStream,
    pub socket_fd: c_int,
}

/// 套接字输出流
#[derive(Debug)]
pub struct GSocketOutputStream {
    pub parent_instance: GOutputStream,
    pub socket_fd: c_int,
}

/// 缓冲输入流
#[derive(Debug)]
pub struct GBufferedInputStream {
    pub parent_instance: GInputStream,
    pub base_stream: *mut GInputStream,
    pub buffer: *mut u8,
    pub buffer_size: usize,
    pub buffer_pos: usize,
    pub buffer_end: usize,
}

/// 缓冲输出流
#[derive(Debug)]
pub struct GBufferedOutputStream {
    pub parent_instance: GOutputStream,
    pub base_stream: *mut GOutputStream,
    pub buffer: *mut u8,
    pub buffer_size: usize,
    pub buffer_pos: usize,
}

/// 简单的自旋锁实现（适用于no_std环境）
pub struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> Self {
        SpinLock {
            locked: AtomicBool::new(false),
        }
    }

    pub unsafe fn lock(&self) {
        while self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            // 自旋等待锁释放
        }
    }

    pub unsafe fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

/// 异步I/O上下文注册表
static mut ASYNC_CONTEXTS: BTreeMap<u64, *mut AsyncIOContext> = BTreeMap::new();
static mut ASYNC_CONTEXTS_LOCK: SpinLock = SpinLock::new();

/// 获取ASYNC_CONTEXTS的原始指针
unsafe fn get_async_contexts_ptr() -> *mut BTreeMap<u64, *mut AsyncIOContext> {
    core::ptr::addr_of_mut!(ASYNC_CONTEXTS)
}

/// 获取ASYNC_CONTEXTS_LOCK的原始指针
unsafe fn get_async_contexts_lock_ptr() -> *mut SpinLock {
    core::ptr::addr_of_mut!(ASYNC_CONTEXTS_LOCK)
}
static mut NEXT_ASYNC_CONTEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// 获取NEXT_ASYNC_CONTEXT_ID的原始指针
unsafe fn get_next_async_context_id_ptr() -> *mut AtomicUsize {
    core::ptr::addr_of_mut!(NEXT_ASYNC_CONTEXT_ID)
}

/// 初始化异步I/O系统
pub fn init() -> Result<(), GError> {
    glib_println!("[glib_async_io] 初始化异步I/O系统");

    unsafe {
        let id_ptr = get_next_async_context_id_ptr();
        (*id_ptr).store(1, Ordering::SeqCst);
    }

    glib_println!("[glib_async_io] 异步I/O系统初始化完成");
    Ok(())
}

/// 创建异步I/O上下文
pub fn g_async_context_new(name: &str, max_operations: usize) -> *mut AsyncIOContext {
    if name.is_empty() || max_operations == 0 {
        return ptr::null_mut();
    }

    unsafe {
        let id_ptr = get_next_async_context_id_ptr();
        let context_id = (*id_ptr).fetch_add(1, Ordering::SeqCst) as u64;
        let context = g_malloc0(core::mem::size_of::<AsyncIOContext>()) as *mut AsyncIOContext;
        if context.is_null() {
            return ptr::null_mut();
        }

        (*context).context_id = context_id;
        (*context).name = name.to_string();
        (*context).max_operations = max_operations;
        (*context).active_operations = BTreeMap::new();
        (*context).next_operation_id = AtomicUsize::new(1);
        (*context).created_timestamp = 0; // 临时使用0，避免编译错误

        // 注册到内核
        let result = crate::syscall(syscall_number::GLIB_ASYNC_CONTEXT_CREATE, [
            name.as_ptr() as usize,
            max_operations,
            0, 0, 0, // 补充空参数到5个
        ]);

        if result <= 0 {
            glib_println!("[glib_async_io] 创建内核异步上下文失败");
            g_free(context as gpointer);
            return ptr::null_mut();
        }

        // 加锁保护异步上下文注册表
        unsafe {
            let lock_ptr = get_async_contexts_lock_ptr();
            let contexts_ptr = get_async_contexts_ptr();
            (*lock_ptr).lock();
            (*contexts_ptr).insert(context_id, context);
            (*lock_ptr).unlock();
        }

        glib_println!("[glib_async_io] 创建异步上下文: {} (ID={})", name, context_id);
        context
    }
}

/// 销毁异步I/O上下文
pub fn g_async_context_destroy(context: *mut AsyncIOContext) {
    if context.is_null() {
        return;
    }

    unsafe {
        // 取消所有活跃操作
        for (_, operation_ptr) in (*context).active_operations.iter() {
            let operation = *operation_ptr;
            if !operation.is_null() {
                g_async_operation_cancel(operation);
            }
        }

        // 销毁上下文
        let result = crate::syscall(syscall_number::GLIB_ASYNC_CONTEXT_DESTROY, [
            (*context).context_id as usize,
            0, 0, 0, 0, // 补充空参数到5个
        ]);

        if result == 0 {
            glib_println!("[glib_async_io] 异步上下文销毁成功: {}", (*context).name);
        } else {
            glib_println!("[glib_async_io] 异步上下文销毁失败: {}", (*context).name);
        }

        // 加锁保护异步上下文注册表
        unsafe {
            let lock_ptr = get_async_contexts_lock_ptr();
            let contexts_ptr = get_async_contexts_ptr();
            (*lock_ptr).lock();
            (*contexts_ptr).remove(&(*context).context_id);
            (*lock_ptr).unlock();
        }

        // 释放内存
        g_free(context as gpointer);
    }
}

/// 创建异步操作
fn create_async_operation(
    context: *mut AsyncIOContext,
    operation_type: AsyncOperationType,
    fd: c_int,
    buffer: *mut c_void,
    buffer_size: usize,
    callback: AsyncReadyCallback,
    user_data: gpointer,
    source_object: *mut GObject,
    timeout_ms: u32,
) -> *mut AsyncOperation {
    if context.is_null() || buffer.is_null() || buffer_size == 0 {
        return ptr::null_mut();
    }

    unsafe {
        // 检查操作数量限制
        if (*context).active_operations.len() >= (*context).max_operations {
            glib_println!("[glib_async_io] 异步操作数量超过限制");
            return ptr::null_mut();
        }

        let operation_id = (*context).next_operation_id.fetch_add(1, Ordering::SeqCst) as u64;
        let operation = g_malloc0(core::mem::size_of::<AsyncOperation>()) as *mut AsyncOperation;
        if operation.is_null() {
            return ptr::null_mut();
        }

        (*operation).operation_id = operation_id;
        (*operation).operation_type = operation_type;
        (*operation).state = AsyncOperationState::Submitted;
        (*operation).fd = fd;
        (*operation).buffer = buffer;
        (*operation).buffer_size = buffer_size;
        (*operation).bytes_transferred = 0;
        (*operation).error_code = 0;
        (*operation).callback = callback;
        (*operation).user_data = user_data;
        (*operation).source_object = source_object;
        (*operation).timeout_ms = timeout_ms;
        (*operation).created_timestamp = 0; // 临时使用0，避免编译错误
        (*operation).completed_timestamp = 0;

        // 添加到活跃操作
        (*context).active_operations.insert(operation_id, operation);

        glib_println!("[glib_async_io] 创建异步操作: ID={}, type={:?}", operation_id, operation_type);
        operation
    }
}

/// 异步读取操作
pub fn g_async_read(
    input_stream: *mut GInputStream,
    buffer: *mut u8,
    count: usize,
    _io_priority: i32,
    _cancellable: *mut GCancellable,
    callback: AsyncReadyCallback,
    user_data: gpointer,
) -> u64 {
    if input_stream.is_null() || buffer.is_null() || count == 0 {
        return 0;
    }

    unsafe {
        let context = (*input_stream).async_context;
        if context.is_null() {
            return 0;
        }

        let operation = create_async_operation(
            context,
            AsyncOperationType::Read,
            i32::MAX, // 文件描述符（使用最大值表示假设的描述符）
            buffer as *mut c_void,
            count,
            callback,
            user_data,
            ptr::null_mut(), // source_object
            5000, // 5秒超时
        );

        if operation.is_null() {
            return 0;
        }

        // 提交到内核
        let result = crate::syscall(syscall_number::GLIB_ASYNC_READ, [
            (*context).context_id as usize,
            usize::MAX, // 文件描述符（使用最大值表示假设的描述符）
            buffer as usize,
            count,
            5000, // timeout
        ]) as u64;

        if result > 0 {
            (*operation).state = AsyncOperationState::InProgress;
            glib_println!("[glib_async_io] 异步读取已提交: ID={}, bytes={}", result, count);
            result
        } else {
            // 从活跃操作中移除
            (*context).active_operations.remove(&(*operation).operation_id);
            g_free(operation as gpointer);
            0
        }
    }
}

/// 异步写入操作
pub fn g_async_write(
    output_stream: *mut GOutputStream,
    buffer: *const u8,
    count: usize,
    _io_priority: i32,
    _cancellable: *mut GCancellable,
    callback: AsyncReadyCallback,
    user_data: gpointer,
) -> u64 {
    if output_stream.is_null() || buffer.is_null() || count == 0 {
        return 0;
    }

    unsafe {
        let context = (*output_stream).async_context;
        if context.is_null() {
            return 0;
        }

        let operation = create_async_operation(
            context,
            AsyncOperationType::Write,
            i32::MAX, // 文件描述符（使用最大值表示假设的描述符）
            buffer as *mut c_void,
            count,
            callback,
            user_data,
            ptr::null_mut(),
            5000,
        );

        if operation.is_null() {
            return 0;
        }

        // 提交到内核
        let result = crate::syscall(syscall_number::GLIB_ASYNC_WRITE, [
            (*context).context_id as usize,
            usize::MAX, // 文件描述符（使用最大值表示假设的描述符）
            buffer as usize,
            count,
            5000, // timeout
        ]) as u64;

        if result > 0 {
            (*operation).state = AsyncOperationState::InProgress;
            glib_println!("[glib_async_io] 异步写入已提交: ID={}, bytes={}", result, count);
            result
        } else {
            (*context).active_operations.remove(&(*operation).operation_id);
            g_free(operation as gpointer);
            0
        }
    }
}

/// 取消异步操作
pub fn g_async_operation_cancel(operation: *mut AsyncOperation) -> gboolean {
    if operation.is_null() {
        return 0;
    }

    unsafe {
        // 如果已经完成或已取消，返回false
        if matches!((*operation).state, AsyncOperationState::Completed | AsyncOperationState::Cancelled) {
            return 0;
        }

        // 调用内核取消
        let result = crate::syscall(syscall_number::GLIB_ASYNC_CANCEL, [
            (*operation).operation_id as usize,
            0, 0, 0, 0, // 补充空参数
        ]);

        if result == 0 {
            (*operation).state = AsyncOperationState::Cancelled;
            (*operation).error_code = -125; // ECANCELED
            (*operation).completed_timestamp = 0; // 临时使用0，避免编译错误

            glib_println!("[glib_async_io] 异步操作已取消: ID={}", (*operation).operation_id);
            1 // true
        } else {
            0 // false
        }
    }
}

/// 查询异步操作状态
pub fn g_async_operation_query(operation: *mut AsyncOperation) -> AsyncResult {
    if operation.is_null() {
        return AsyncResult {
            operation_id: 0,
            state: AsyncOperationState::Failed,
            bytes_transferred: 0,
            error_code: -22, // EINVAL
            user_data: ptr::null_mut(),
        };
    }

    unsafe {
        // 调用内核查询
        let mut state = AsyncOperationState::Submitted;
        let mut bytes_transferred = 0usize;
        let mut error_code = 0i32;

        let result = crate::syscall(syscall_number::GLIB_ASYNC_QUERY, [
            (*operation).operation_id as usize,
            &mut state as *mut AsyncOperationState as usize,
            &mut bytes_transferred as *mut usize as usize,
            &mut error_code as *mut i32 as usize,
            0, // 补充第5个参数
        ]);

        if result == 0 {
            // 更新本地状态
            (*operation).state = state;
            (*operation).bytes_transferred = bytes_transferred;
            (*operation).error_code = error_code;

            if state == AsyncOperationState::Completed {
                (*operation).completed_timestamp = 0; // 临时使用0，避免编译错误
            }
        }

        AsyncResult {
            operation_id: (*operation).operation_id,
            state,
            bytes_transferred,
            error_code,
            user_data: (*operation).user_data,
        }
    }
}

/// 等待异步操作完成
pub fn g_async_operation_wait(operation: *mut AsyncOperation, timeout_ms: u32) -> gboolean {
    if operation.is_null() {
        return 0;
    }

    unsafe {
        let result = crate::syscall(syscall_number::GLIB_ASYNC_WAIT, [
            (*operation).operation_id as usize,
            timeout_ms as usize,
            0, 0, 0
        ]);

        if result == 0 {
            // 更新操作状态
            let query_result = g_async_operation_query(operation);
            if query_result.state == AsyncOperationState::Completed {
                1 // true
            } else {
                0 // false
            }
        } else {
            0 // false - 超时或错误
        }
    }
}

/// 创建文件输入流
pub fn g_file_input_stream_new(file_path: &str) -> *mut GFileInputStream {
    if file_path.is_empty() {
        return ptr::null_mut();
    }

    unsafe {
        let stream = g_malloc0(core::mem::size_of::<GFileInputStream>()) as *mut GFileInputStream;
        if stream.is_null() {
            return ptr::null_mut();
        }

        // 创建异步上下文
        let async_context = g_async_context_new("file-input", 64);
        if async_context.is_null() {
            g_free(stream as gpointer);
            return ptr::null_mut();
        }

        (*stream).parent_instance.async_context = async_context;
        (*stream).file_path = file_path.to_string();
        (*stream).file_descriptor = -1; // 将在open时设置

        glib_println!("[glib_async_io] 创建文件输入流: {}", file_path);
        stream
    }
}

/// 创建文件输出流
pub fn g_file_output_stream_new(file_path: &str) -> *mut GFileOutputStream {
    if file_path.is_empty() {
        return ptr::null_mut();
    }

    unsafe {
        let stream = g_malloc0(core::mem::size_of::<GFileOutputStream>()) as *mut GFileOutputStream;
        if stream.is_null() {
            return ptr::null_mut();
        }

        let async_context = g_async_context_new("file-output", 64);
        if async_context.is_null() {
            g_free(stream as gpointer);
            return ptr::null_mut();
        }

        (*stream).parent_instance.async_context = async_context;
        (*stream).file_path = file_path.to_string();
        (*stream).file_descriptor = -1;

        glib_println!("[glib_async_io] 创建文件输出流: {}", file_path);
        stream
    }
}

/// 创建套接字输入流
pub fn g_socket_input_stream_new(socket_fd: c_int) -> *mut GSocketInputStream {
    if socket_fd < 0 {
        return ptr::null_mut();
    }

    unsafe {
        let stream = g_malloc0(core::mem::size_of::<GSocketInputStream>()) as *mut GSocketInputStream;
        if stream.is_null() {
            return ptr::null_mut();
        }

        let async_context = g_async_context_new("socket-input", 128);
        if async_context.is_null() {
            g_free(stream as gpointer);
            return ptr::null_mut();
        }

        (*stream).parent_instance.async_context = async_context;
        (*stream).socket_fd = socket_fd;

        glib_println!("[glib_async_io] 创建套接字输入流: fd={}", socket_fd);
        stream
    }
}

/// 创建套接字输出流
pub fn g_socket_output_stream_new(socket_fd: c_int) -> *mut GSocketOutputStream {
    if socket_fd < 0 {
        return ptr::null_mut();
    }

    unsafe {
        let stream = g_malloc0(core::mem::size_of::<GSocketOutputStream>()) as *mut GSocketOutputStream;
        if stream.is_null() {
            return ptr::null_mut();
        }

        let async_context = g_async_context_new("socket-output", 128);
        if async_context.is_null() {
            g_free(stream as gpointer);
            return ptr::null_mut();
        }

        (*stream).parent_instance.async_context = async_context;
        (*stream).socket_fd = socket_fd;

        glib_println!("[glib_async_io] 创建套接字输出流: fd={}", socket_fd);
        stream
    }
}

/// 创建缓冲输入流
pub fn g_buffered_input_stream_new(base_stream: *mut GInputStream) -> *mut GBufferedInputStream {
    if base_stream.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let stream = g_malloc0(core::mem::size_of::<GBufferedInputStream>()) as *mut GBufferedInputStream;
        if stream.is_null() {
            return ptr::null_mut();
        }

        let buffer = g_malloc(8192) as *mut u8; // 8KB缓冲区
        if buffer.is_null() {
            g_free(stream as gpointer);
            return ptr::null_mut();
        }

        (*stream).parent_instance.async_context = (*base_stream).async_context;
        (*stream).base_stream = base_stream;
        (*stream).buffer = buffer;
        (*stream).buffer_size = 8192;
        (*stream).buffer_pos = 0;
        (*stream).buffer_end = 0;

        glib_println!("[glib_async_io] 创建缓冲输入流: buffer_size=8192");
        stream
    }
}

/// 异步操作回调处理
pub fn handle_async_operation_complete(operation_id: u64, bytes_transferred: usize, error_code: c_int) {
    unsafe {
        // 查找操作
        let mut found_operation = None;

        // 加锁保护异步上下文注册表
        let lock_ptr = get_async_contexts_lock_ptr();
        let contexts_ptr = get_async_contexts_ptr();
        (*lock_ptr).lock();
        for (_, context_ptr) in (*contexts_ptr).iter() {
            if let Some(operation) = (**context_ptr).active_operations.get(&operation_id) {
                found_operation = Some(*operation);
                break;
            }
        }
        (*lock_ptr).unlock();

        if let Some(operation) = found_operation {
            // 更新操作状态
            (*operation).bytes_transferred = bytes_transferred;
            (*operation).error_code = error_code;
            (*operation).completed_timestamp = 0; // 临时使用0，避免编译错误

            if error_code == 0 {
                (*operation).state = AsyncOperationState::Completed;
            } else {
                (*operation).state = AsyncOperationState::Failed;
            }

            // 创建结果对象
            let result = g_malloc0(core::mem::size_of::<GAsyncResult>()) as *mut GAsyncResult;
            if !result.is_null() {
                (*result).source_object = (*operation).source_object;
                (*result).user_data = (*operation).user_data;
                (*result).operation_id = operation_id;

                // 调用回调函数
                let callback = (*operation).callback;
                // 检查函数指针是否为null（使用比较方式而不是is_null方法）
                if callback as *const () != ptr::null() {
                    // 创建AsyncResult而不是GAsyncResult，因为回调函数期望AsyncResult类型
                    let async_result = g_malloc0(core::mem::size_of::<AsyncResult>()) as *mut AsyncResult;
                    if !async_result.is_null() {
                        (*async_result).operation_id = operation_id;
                        (*async_result).state = (*operation).state;
                        (*async_result).bytes_transferred = bytes_transferred;
                        (*async_result).error_code = error_code;
                        (*async_result).user_data = (*operation).user_data;
                        
                        callback((*operation).source_object, async_result, (*operation).user_data);
                        g_free(async_result as gpointer);
                    }
                }

                g_free(result as gpointer);
            }

            glib_println!("[glib_async_io] 异步操作完成: ID={}, bytes={}, error={}",
                operation_id, bytes_transferred, error_code);
        }
    }
}

/// 取消令牌类型（简化实现）
#[derive(Debug)]
pub struct GCancellable {
    pub is_cancelled: AtomicBool,
}

/// 创建取消令牌
pub fn g_cancellable_new() -> *mut GCancellable {
    unsafe {
        let cancellable = g_malloc0(core::mem::size_of::<GCancellable>()) as *mut GCancellable;
        if !cancellable.is_null() {
            (*cancellable).is_cancelled = AtomicBool::new(false);
        }
        cancellable
    }
}

/// 取消操作
pub fn g_cancellable_cancel(cancellable: *mut GCancellable) {
    if !cancellable.is_null() {
        unsafe {
            (*cancellable).is_cancelled.store(true, Ordering::SeqCst);
        }
    }
}

/// 检查是否已取消
pub fn g_cancellable_is_cancelled(cancellable: *mut GCancellable) -> gboolean {
    if cancellable.is_null() {
        return 0;
    }

    unsafe {
        if (*cancellable).is_cancelled.load(Ordering::SeqCst) {
            1 // true
        } else {
            0 // false
        }
    }
}

/// 清理异步I/O系统
pub fn cleanup() {
    glib_println!("[glib_async_io] 清理异步I/O系统");

    unsafe {
        // 清理所有异步上下文
        // 加锁保护异步上下文注册表
        let (_context_ids, context_ptrs) = {
            let lock_ptr = get_async_contexts_lock_ptr();
            let contexts_ptr = get_async_contexts_ptr();
            (*lock_ptr).lock();
            let ids: Vec<u64> = (*contexts_ptr).keys().cloned().collect();
            let ptrs: Vec<*mut AsyncIOContext> = ids.iter()
                .filter_map(|id| (*contexts_ptr).get(id).copied())
                .collect();
            (*lock_ptr).unlock();
            (ids, ptrs)
        };
        
        for context_ptr in context_ptrs {
            g_async_context_destroy(context_ptr);
        }

        // 加锁保护异步上下文注册表
        let lock_ptr = get_async_contexts_lock_ptr();
        let contexts_ptr = get_async_contexts_ptr();
        (*lock_ptr).lock();
        (*contexts_ptr).clear();
        (*lock_ptr).unlock();

        // 清理内核中的异步操作
        crate::syscall(syscall_number::GLIB_ASYNC_CLEANUP, [0, 0, 0, 0, 0]);
    }

    glib_println!("[glib_async_io] 异步I/O系统清理完成");
}

/// 异步I/O测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_context_creation() {
        init().unwrap();

        let context = g_async_context_new("test-context", 16);
        assert!(!context.is_null());

        unsafe {
            assert_eq!((*context).max_operations, 16);
            assert_eq!((*context).active_operations.len(), 0);
        }

        g_async_context_destroy(context);

        cleanup();
    }

    #[test]
    fn test_cancellable() {
        let cancellable = g_cancellable_new();
        assert!(!cancellable.is_null());

        assert_eq!(g_cancellable_is_cancelled(cancellable), 0);

        g_cancellable_cancel(cancellable);
        assert_eq!(g_cancellable_is_cancelled(cancellable), 1);

        // 需要手动释放，简化实现中没有引用计数
    }

    #[test]
    fn test_file_stream_creation() {
        init().unwrap();

        let input_stream = g_file_input_stream_new("/tmp/test.txt");
        assert!(!input_stream.is_null());

        let output_stream = g_file_output_stream_new("/tmp/test.txt");
        assert!(!output_stream.is_null());

        // 清理（简化实现）
        unsafe {
            g_async_context_destroy((*input_stream).parent_instance.async_context);
            g_async_context_destroy((*output_stream).parent_instance.async_context);
            g_free(input_stream as gpointer);
            g_free(output_stream as gpointer);
        }

        cleanup();
    }

    #[test]
    fn test_socket_stream_creation() {
        init().unwrap();

        let input_stream = g_socket_input_stream_new(3); // 假设的fd
        assert!(!input_stream.is_null());

        let output_stream = g_socket_output_stream_new(4); // 假设的fd
        assert!(!output_stream.is_null());

        // 清理（简化实现）
        unsafe {
            g_async_context_destroy((*input_stream).parent_instance.async_context);
            g_async_context_destroy((*output_stream).parent_instance.async_context);
            g_free(input_stream as gpointer);
            g_free(output_stream as gpointer);
        }

        cleanup();
    }
}

// 系统调用号映射
mod syscall_number {
    pub const GLIB_ASYNC_CONTEXT_CREATE: usize = 1030;
    pub const GLIB_ASYNC_READ: usize = 1031;
    pub const GLIB_ASYNC_WRITE: usize = 1032;
    pub const GLIB_ASYNC_CANCEL: usize = 1033;
    pub const GLIB_ASYNC_QUERY: usize = 1034;
    pub const GLIB_ASYNC_WAIT: usize = 1035;
    pub const GLIB_ASYNC_CONTEXT_DESTROY: usize = 1038;
    pub const GLIB_ASYNC_CLEANUP: usize = 1039;
}