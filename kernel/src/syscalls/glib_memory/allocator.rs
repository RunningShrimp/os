//! GLib memory allocator trait and implementation

use super::*;

/// 内存分配器特征
pub trait GLibMemoryAllocator {
    /// 创建新的内存池
    fn create_pool(&mut self, size: usize, alignment: usize) -> Result<c_int, c_int>;

    /// 从内存池分配
    fn allocate_from_pool(&self, pool_id: c_int, size: usize) -> Result<*mut c_void, ()>;

    /// 释放到内存池
    fn deallocate_to_pool(&self, pool_id: c_int, ptr: *mut c_void) -> Result<(), ()>;

    /// 销毁内存池
    fn destroy_pool(&mut self, pool_id: c_int) -> Result<(), ()>;

    /// 获取内存池统计
    fn get_pool_stats(&self, pool_id: c_int) -> Result<MemoryPoolInfo, ()>;
}

impl Default for GLibMemoryAllocator {
    fn default() -> Self {
        Self
    }
}

impl GLibMemoryAllocator for () {
    fn create_pool(&mut self, size: usize, alignment: usize) -> Result<c_int, c_int> {
        let result = super::pool::sys_glib_memory_pool_create(size, alignment);
        if result >= 0 {
            Ok(result)
        } else {
            Err(result)
        }
    }

    fn allocate_from_pool(&self, pool_id: c_int, size: usize) -> Result<*mut c_void, ()> {
        let ptr = super::pool::sys_glib_memory_pool_alloc(pool_id, size);
        if !ptr.is_null() {
            Ok(ptr)
        } else {
            Err(())
        }
    }

    fn deallocate_to_pool(&self, pool_id: c_int, ptr: *mut c_void) -> Result<(), ()> {
        super::pool::sys_glib_memory_pool_free(pool_id, ptr);
        Ok(())
    }

    fn destroy_pool(&mut self, pool_id: c_int) -> Result<(), ()> {
        let result = super::pool::sys_glib_memory_pool_destroy(pool_id);
        if result == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    fn get_pool_stats(&self, pool_id: c_int) -> Result<MemoryPoolInfo, ()> {
        let mut stats = MemoryPoolInfo {
            size: 0,
            alignment: 0,
            allocated_blocks: AtomicUsize::new(0),
            freed_blocks: AtomicUsize::new(0),
            active_blocks: AtomicUsize::new(0),
            created_timestamp: 0,
        };

        let result = super::pool::sys_glib_memory_pool_stats(pool_id, &mut stats as *mut MemoryPoolInfo);
        if result == 0 {
            Ok(stats)
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_creation() {
        // 测试内存池创建
        let pool_id = super::pool::sys_glib_memory_pool_create(1024, 8);
        assert!(pool_id > 0);

        // 清理
        super::pool::sys_glib_memory_pool_destroy(pool_id);
    }

    #[test]
    fn test_memory_allocation() {
        // 创建内存池
        let pool_id = super::pool::sys_glib_memory_pool_create(1024, 8);
        assert!(pool_id > 0);

        // 分配内存
        let ptr = super::pool::sys_glib_memory_pool_alloc(pool_id, 512);
        assert!(!ptr.is_null());

        // 释放内存
        super::pool::sys_glib_memory_pool_free(pool_id, ptr);

        // 销毁内存池
        super::pool::sys_glib_memory_pool_destroy(pool_id);
    }

    #[test]
    fn test_memory_pool_stats() {
        // 创建内存池
        let pool_id = super::pool::sys_glib_memory_pool_create(1024, 8);
        assert!(pool_id > 0);

        // 获取统计信息
        let mut stats = MemoryPoolInfo {
            size: 0,
            alignment: 0,
            allocated_blocks: AtomicUsize::new(0),
            freed_blocks: AtomicUsize::new(0),
            active_blocks: AtomicUsize::new(0),
            created_timestamp: 0,
        };

        let result = super::pool::sys_glib_memory_pool_stats(pool_id, &mut stats as *mut MemoryPoolInfo);
        assert_eq!(result, 0);
        assert_eq!(stats.size, 1024);
        assert_eq!(stats.alignment, 8);

        // 分配和释放一些内存
        let ptr1 = super::pool::sys_glib_memory_pool_alloc(pool_id, 512);
        let ptr2 = super::pool::sys_glib_memory_pool_alloc(pool_id, 256);

        // 获取更新后的统计
        let result = super::pool::sys_glib_memory_pool_stats(pool_id, &mut stats as *mut MemoryPoolInfo);
        assert_eq!(result, 0);
        assert_eq!(stats.allocated_blocks.load(Ordering::SeqCst), 2);
        assert_eq!(stats.active_blocks.load(Ordering::SeqCst), 2);

        // 释放内存
        super::pool::sys_glib_memory_pool_free(pool_id, ptr1);
        super::pool::sys_glib_memory_pool_free(pool_id, ptr2);

        // 销毁内存池
        super::pool::sys_glib_memory_pool_destroy(pool_id);
    }
}