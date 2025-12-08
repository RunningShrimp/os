//! GLib async manager trait and implementation

use super::*;

/// GLib异步I/O管理器特征
pub trait GAsyncManager {
    /// 创建异步I/O上下文
    fn create_context(&mut self, name: &str, max_operations: usize) -> Result<u64, c_int>;

    /// 提交异步读操作
    fn submit_read(&mut self, context_id: u64, fd: c_int, buffer: *mut c_void, size: usize,
                   offset: i64, callback: *mut c_void, user_data: *mut c_void, timeout: u32) -> Result<u64, c_int>;

    /// 提交异步写操作
    fn submit_write(&mut self, context_id: u64, fd: c_int, buffer: *const c_void, size: usize,
                    offset: i64, callback: *mut c_void, user_data: *mut c_void, timeout: u32) -> Result<u64, c_int>;

    /// 取消异步操作
    fn cancel_operation(&mut self, operation_id: u64) -> Result<(), c_int>;

    /// 查询操作状态
    fn query_operation(&self, operation_id: u64) -> Result<(AsyncOperationStatus, usize, c_int), c_int>;

    /// 等待操作完成
    fn wait_operation(&self, operation_id: u64, timeout: u32) -> Result<c_int, c_int>;

    /// 获取上下文统计
    fn get_context_stats(&self, context_id: u64) -> Result<(usize, usize, usize, usize), c_int>;

    /// 销毁上下文
    fn destroy_context(&mut self, context_id: u64) -> Result<(), c_int>;
}

impl Default for GAsyncManager {
    fn default() -> Self {
        Self
    }
}

impl GAsyncManager for () {
    fn create_context(&mut self, name: &str, max_operations: usize) -> Result<u64, c_int> {
        let result = super::context::sys_glib_async_context_create(
            name.as_ptr() as *const core::ffi::c_char,
            max_operations,
        );
        if result > 0 {
            Ok(result as u64)
        } else {
            Err(result)
        }
    }

    fn submit_read(&mut self, context_id: u64, fd: c_int, buffer: *mut c_void, size: usize,
                   offset: i64, callback: *mut c_void, user_data: *mut c_void, timeout: u32) -> Result<u64, c_int> {
        let result = super::operation::sys_glib_async_read(
            context_id, fd, buffer, size, offset,
            callback, user_data, timeout,
        );
        if result > 0 {
            Ok(result as u64)
        } else {
            Err(result)
        }
    }

    fn submit_write(&mut self, context_id: u64, fd: c_int, buffer: *const c_void, size: usize,
                    offset: i64, callback: *mut c_void, user_data: *mut c_void, timeout: u32) -> Result<u64, c_int> {
        let result = super::operation::sys_glib_async_write(
            context_id, fd, buffer, size, offset,
            callback, user_data, timeout,
        );
        if result > 0 {
            Ok(result as u64)
        } else {
            Err(result)
        }
    }

    fn cancel_operation(&mut self, operation_id: u64) -> Result<(), c_int> {
        let result = super::operation::sys_glib_async_cancel(operation_id);
        if result == 0 {
            Ok(())
        } else {
            Err(result)
        }
    }

    fn query_operation(&self, operation_id: u64) -> Result<(AsyncOperationStatus, usize, c_int), c_int> {
        let mut status = AsyncOperationStatus::Submitted;
        let mut bytes_completed = 0usize;
        let mut error_code = 0i32;

        let result = super::operation::sys_glib_async_query(
            operation_id,
            &mut status as *mut AsyncOperationStatus,
            &mut bytes_completed as *mut usize,
            &mut error_code as *mut c_int,
        );

        if result == 0 {
            Ok((status, bytes_completed, error_code))
        } else {
            Err(result)
        }
    }

    fn wait_operation(&self, operation_id: u64, timeout: u32) -> Result<c_int, c_int> {
        let result = super::operation::sys_glib_async_wait(operation_id, timeout);
        if result >= 0 {
            Ok(result)
        } else {
            Err(result)
        }
    }

    fn get_context_stats(&self, context_id: u64) -> Result<(usize, usize, usize, usize), c_int> {
        let mut total_ops = 0usize;
        let mut active_ops = 0usize;
        let mut successful_ops = 0usize;
        let mut failed_ops = 0usize;

        let result = super::context::sys_glib_async_context_stats(
            context_id,
            &mut total_ops as *mut usize,
            &mut active_ops as *mut usize,
            &mut successful_ops as *mut usize,
            &mut failed_ops as *mut usize,
        );

        if result == 0 {
            Ok((total_ops, active_ops, successful_ops, failed_ops))
        } else {
            Err(result)
        }
    }

    fn destroy_context(&mut self, context_id: u64) -> Result<(), c_int> {
        let result = super::context::sys_glib_async_context_destroy(context_id);
        if result == 0 {
            Ok(())
        } else {
            Err(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        // 测试上下文创建
        let context_id = super::context::sys_glib_async_context_create(
            b"test-context\0".as_ptr() as *const core::ffi::c_char,
            100,
        );
        assert!(context_id > 0);

        // 清理
        super::context::sys_glib_async_context_destroy(context_id as u64);
    }

    #[test]
    fn test_async_read() {
        // 创建上下文
        let context_id = super::context::sys_glib_async_context_create(
            b"test-context\0".as_ptr() as *const core::ffi::c_char,
            100,
        );
        assert!(context_id > 0);

        // 提交异步读操作
        let buffer = [0u8; 1024];
        let operation_id = super::operation::sys_glib_async_read(
            context_id as u64,
            1, // 假设的文件描述符
            buffer.as_ptr() as *mut c_void,
            buffer.len(),
            -1, // 当前位置
            core::ptr::null_mut(), // 无回调
            core::ptr::null_mut(), // 无用户数据
            5000, // 5秒超时
        );
        assert!(operation_id > 0);

        // 查询操作状态
        let mut status = AsyncOperationStatus::Submitted;
        let mut bytes_completed = 0usize;
        let mut error_code = 0i32;
        let result = super::operation::sys_glib_async_query(
            operation_id as u64,
            &mut status as *mut AsyncOperationStatus,
            &mut bytes_completed as *mut usize,
            &mut error_code as *mut c_int,
        );
        assert_eq!(result, 0);

        // 取消操作
        let result = super::operation::sys_glib_async_cancel(operation_id as u64);
        assert_eq!(result, 0);

        // 清理
        super::context::sys_glib_async_context_destroy(context_id as u64);
    }

    #[test]
    fn test_context_stats() {
        // 创建上下文
        let context_id = super::context::sys_glib_async_context_create(
            b"stats-test\0".as_ptr() as *const core::ffi::c_char,
            10,
        );
        assert!(context_id > 0);

        // 提交一些操作
        let buffer = [0u8; 256];
        let op1 = super::operation::sys_glib_async_read(
            context_id as u64, 1, buffer.as_ptr() as *mut c_void, 256,
            -1, core::ptr::null_mut(), core::ptr::null_mut(), 1000,
        );
        assert!(op1 > 0);

        let op2 = super::operation::sys_glib_async_write(
            context_id as u64, 2, buffer.as_ptr() as *const c_void, 256,
            -1, core::ptr::null_mut(), core::ptr::null_mut(), 1000,
        );
        assert!(op2 > 0);

        // 获取统计信息
        let mut total_ops = 0usize;
        let mut active_ops = 0usize;
        let mut successful_ops = 0usize;
        let mut failed_ops = 0usize;
        let result = super::context::sys_glib_async_context_stats(
            context_id as u64,
            &mut total_ops as *mut usize,
            &mut active_ops as *mut usize,
            &mut successful_ops as *mut usize,
            &mut failed_ops as *mut usize,
        );
        assert_eq!(result, 0);
        assert_eq!(total_ops, 2);
        assert_eq!(active_ops, 2);

        // 取消操作
        super::operation::sys_glib_async_cancel(op1 as u64);
        super::operation::sys_glib_async_cancel(op2 as u64);

        // 清理
        super::context::sys_glib_async_context_destroy(context_id as u64);
    }

    #[test]
    fn test_cleanup() {
        // 确保清理不会崩溃
        super::operation::sys_glib_async_cleanup();
    }
}