//! GLib内存管理模块
//!
//! 提供与GLib兼容的内存管理功能，包括：
//! - g_malloc/g_free 标准内存分配
//! - g_slice 高性能切片分配器
//! - g_realloc 内存重新分配
//! - g_malloc0 清零分配
//! - 内存统计和调试
//! - 内存对齐优化
//! - 泄漏检测

#![no_std]

extern crate alloc;

use crate::glib::{types::*, error::GError, get_state_mut};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};


/// 内存统计信息
#[derive(Debug, Default)]
pub struct MemoryStats {
    pub total_allocations: AtomicUsize,
    pub total_deallocations: AtomicUsize,
    pub current_allocated_bytes: AtomicUsize,
    pub peak_allocated_bytes: AtomicUsize,
    pub fragmentation_count: AtomicUsize,
}

/// 内存池信息
#[derive(Debug)]
pub struct MemoryPool {
    pub pool_id: i32,
    pub block_size: usize,
    pub alignment: usize,
    pub allocated_blocks: AtomicUsize,
    pub free_blocks: AtomicUsize,
}

/// 切片分配器
#[derive(Debug)]
pub struct SliceAllocator {
    pub small_pool: Option<MemoryPool>,   // 小对象池 (< 256 bytes)
    pub medium_pool: Option<MemoryPool>,  // 中等对象池 (256-2048 bytes)
    pub large_pool: Option<MemoryPool>,   // 大对象池 (> 2048 bytes)
}

static mut SLICE_ALLOCATOR: Option<SliceAllocator> = None;
static MEMORY_STATS: MemoryStats = MemoryStats {
    total_allocations: AtomicUsize::new(0),
    total_deallocations: AtomicUsize::new(0),
    current_allocated_bytes: AtomicUsize::new(0),
    peak_allocated_bytes: AtomicUsize::new(0),
    fragmentation_count: AtomicUsize::new(0),
};

/// 初始化内存管理模块
pub fn init() -> Result<(), GError> {
    glib_println!("[glib_memory] 初始化内存管理模块");

    unsafe {
        SLICE_ALLOCATOR = Some(SliceAllocator {
            small_pool: None,
            medium_pool: None,
            large_pool: None,
        });
    }

    // 创建默认内存池
    create_memory_pools()?;

    glib_println!("[glib_memory] 内存管理模块初始化完成");
    Ok(())
}

/// 创建默认内存池
fn create_memory_pools() -> Result<(), GError> {
    // 创建小对象内存池
    let small_pool_id = unsafe {
        crate::syscall(syscall_number::GLibMemoryPoolCreate, [
            64,    // 64字节块
            8,     // 8字节对齐
            0, 0, 0, 0
        ]) as i32
    };

    if small_pool_id <= 0 {
        return Err(GError::new(crate::glib::error::domains::G_FILE_ERROR,
                              crate::glib::error::file_errors::G_FILE_ERROR_FAILED,
                              "Failed to create small memory pool"));
    }

    // 创建中等对象内存池
    let medium_pool_id = unsafe {
        crate::syscall(syscall_number::GLibMemoryPoolCreate, [
            512,   // 512字节块
            16,    // 16字节对齐
            0, 0, 0, 0
        ]) as i32
    };

    if medium_pool_id <= 0 {
        return Err(GError::new(crate::glib::error::domains::G_FILE_ERROR,
                              crate::glib::error::file_errors::G_FILE_ERROR_FAILED,
                              "Failed to create medium memory pool"));
    }

    // 更新切片分配器
    unsafe {
        if let Some(ref mut slice_alloc) = SLICE_ALLOCATOR {
            slice_alloc.small_pool = Some(MemoryPool {
                pool_id: small_pool_id,
                block_size: 64,
                alignment: 8,
                allocated_blocks: AtomicUsize::new(0),
                free_blocks: AtomicUsize::new(0),
            });

            slice_alloc.medium_pool = Some(MemoryPool {
                pool_id: medium_pool_id,
                block_size: 512,
                alignment: 16,
                allocated_blocks: AtomicUsize::new(0),
                free_blocks: AtomicUsize::new(0),
            });
        }
    }

    glib_println!("[glib_memory] 默认内存池创建完成: 小={}, 中={}",
        small_pool_id, medium_pool_id);
    Ok(())
}

/// 标准内存分配 (g_malloc)
pub fn g_malloc(size: usize) -> gpointer {
    if size == 0 {
        return ptr::null_mut();
    }

    // 对齐内存大小
    let aligned_size = align_size(size);

    // 尝试使用切片分配器
    let ptr = try_slice_alloc(aligned_size);

    if !ptr.is_null() {
        update_alloc_stats(aligned_size);
        return ptr;
    }

    // 回退到系统分配器
    let layout = Layout::from_size_align(aligned_size, G_MEM_ALIGN)
        .unwrap_or_else(|_| Layout::from_size_align(aligned_size, 8).unwrap());

    unsafe {
        let alloc_ptr = alloc::alloc::alloc(layout);
        if alloc_ptr.is_null() {
            glib_println!("[glib_memory] 内存分配失败: {} bytes", aligned_size);
            return ptr::null_mut();
        }
        update_alloc_stats(aligned_size);
        alloc_ptr as gpointer
    }
}

/// 清零内存分配 (g_malloc0)
pub fn g_malloc0(size: usize) -> gpointer {
    if size == 0 {
        return ptr::null_mut();
    }

    let aligned_size = align_size(size);

    // 尝试使用切片分配器
    let ptr = try_slice_alloc(aligned_size);

    if !ptr.is_null() {
        // 清零内存
        unsafe {
            ptr::write_bytes(ptr as *mut u8, 0, aligned_size);
        }
        update_alloc_stats(aligned_size);
        return ptr;
    }

    // 使用系统分配器并清零
    let layout = Layout::from_size_align(aligned_size, G_MEM_ALIGN)
        .unwrap_or_else(|_| Layout::from_size_align(aligned_size, 8).unwrap());

    unsafe {
        let alloc_ptr = alloc::alloc::alloc_zeroed(layout);
        if alloc_ptr.is_null() {
            glib_println!("[glib_memory] 清零内存分配失败: {} bytes", aligned_size);
            return ptr::null_mut();
        }
        update_alloc_stats(aligned_size);
        alloc_ptr as gpointer
    }
}

/// 重新分配内存 (g_realloc)
pub fn g_realloc(mem: gpointer, size: usize) -> gpointer {
    if size == 0 {
        g_free(mem);
        return ptr::null_mut();
    }

    if mem.is_null() {
        return g_malloc(size);
    }

    let aligned_size = align_size(size);

    // 如果新大小较小，可能直接使用现有内存
    let old_size = get_allocated_size(mem);
    if old_size >= aligned_size {
        return mem;
    }

    // 分配新内存
    let new_ptr = g_malloc(aligned_size);
    if new_ptr.is_null() {
        return ptr::null_mut();
    }

    // 复制旧数据
    unsafe {
        let copy_size = core::cmp::min(old_size, aligned_size);
        ptr::copy_nonoverlapping(mem as *const u8, new_ptr as *mut u8, copy_size);
    }

    // 释放旧内存
    g_free(mem);

    new_ptr
}

/// 释放内存 (g_free)
pub fn g_free(mem: gpointer) {
    if mem.is_null() {
        return;
    }

    let size = get_allocated_size(mem);
    if size > 0 {
        // 尝试使用切片释放
        if try_slice_free(mem, size) {
            update_free_stats(size);
            return;
        }
    }

    // 系统释放
    unsafe {
        let layout = Layout::from_size_align(size, G_MEM_ALIGN)
            .unwrap_or_else(|_| Layout::from_size_align(size, 8).unwrap());
        alloc::alloc::dealloc(mem as *mut u8, layout);
    }

    update_free_stats(size);
}

/// 切片分配 (g_slice_alloc)
pub fn g_slice_alloc(block_size: usize) -> gpointer {
    if block_size == 0 {
        return ptr::null_mut();
    }

    let aligned_size = align_size(block_size);

    // 选择合适的内存池
    let pool_id = select_slice_pool(aligned_size);
    if pool_id > 0 {
        let ptr = unsafe {
            crate::syscall(syscall_number::GLibMemoryPoolAlloc, [
                pool_id as usize,
                aligned_size,
                0, 0, 0, 0
            ]) as gpointer
        };

        if !ptr.is_null() {
            update_alloc_stats(aligned_size);
            return ptr;
        }
    }

    // 回退到标准分配
    g_malloc(aligned_size)
}

/// 切片释放 (g_slice_free1)
pub unsafe fn g_slice_free1(block_size: usize, mem_block: gpointer) {
    if mem_block.is_null() || block_size == 0 {
        return;
    }

    let aligned_size = align_size(block_size);

    // 尝试释放到内存池
    let pool_id = select_slice_pool(aligned_size);
    if pool_id > 0 {
        crate::syscall(syscall_number::GLibMemoryPoolFree, [
            pool_id as usize,
            mem_block as usize,
            0, 0, 0, 0
        ]);
        update_free_stats(aligned_size);
        return;
    }

    // 标准释放
    g_free(mem_block);
}

/// 批量分配 (g_malloc_n)
pub fn g_malloc_n(size: usize, n: usize) -> gpointer {
    if n == 0 || size == 0 {
        return ptr::null_mut();
    }

    // 检查溢出
    if let Some(total_size) = size.checked_mul(n) {
        g_malloc(total_size)
    } else {
        glib_println!("[glib_memory] 批量分配大小溢出: {} * {}", size, n);
        ptr::null_mut()
    }
}

/// 批量清零分配 (g_malloc0_n)
pub fn g_malloc0_n(size: usize, n: usize) -> gpointer {
    if n == 0 || size == 0 {
        return ptr::null_mut();
    }

    // 检查溢出
    if let Some(total_size) = size.checked_mul(n) {
        g_malloc0(total_size)
    } else {
        glib_println!("[glib_memory] 批量清零分配大小溢出: {} * {}", size, n);
        ptr::null_mut()
    }
}

/// 内存对齐
fn align_size(size: usize) -> usize {
    (size + G_MEM_ALIGN - 1) & !(G_MEM_ALIGN - 1)
}

/// 尝试切片分配
fn try_slice_alloc(size: usize) -> gpointer {
    let pool_id = select_slice_pool(size);
    if pool_id <= 0 {
        return ptr::null_mut();
    }

    unsafe {
        crate::syscall(syscall_number::GLibMemoryPoolAlloc, [
            pool_id as usize,
            size,
            0, 0, 0, 0
        ]) as gpointer
    }
}

/// 尝试切片释放
fn try_slice_free(mem: gpointer, size: usize) -> bool {
    let pool_id = select_slice_pool(size);
    if pool_id <= 0 {
        return false;
    }

    unsafe {
        crate::syscall(syscall_number::GLibMemoryPoolFree, [
            pool_id as usize,
            mem as usize,
            0, 0, 0, 0
        ]) == 0
    }
}

/// 选择切片池
fn select_slice_pool(size: usize) -> i32 {
    unsafe {
        if let Some(ref slice_alloc) = SLICE_ALLOCATOR {
            if size <= 256 {
                slice_alloc.small_pool.as_ref().map(|p| p.pool_id).unwrap_or(0)
            } else if size <= 2048 {
                slice_alloc.medium_pool.as_ref().map(|p| p.pool_id).unwrap_or(0)
            } else {
                slice_alloc.large_pool.as_ref().map(|p| p.pool_id).unwrap_or(0)
            }
        } else {
            0
        }
    }
}

/// 获取分配的内存大小
fn get_allocated_size(mem: gpointer) -> usize {
    // 简化实现：返回固定大小或使用元数据
    // 在实际实现中，应该有更复杂的元数据管理
    if mem.is_null() {
        0
    } else {
        // 暂时返回默认大小
        64
    }
}

/// 更新分配统计
fn update_alloc_stats(size: usize) {
    MEMORY_STATS.total_allocations.fetch_add(1, Ordering::SeqCst);
    let current = MEMORY_STATS.current_allocated_bytes.fetch_add(size, Ordering::SeqCst) + size;

    // 更新峰值
    let mut peak = MEMORY_STATS.peak_allocated_bytes.load(Ordering::SeqCst);
    while current > peak {
        match MEMORY_STATS.peak_allocated_bytes.compare_exchange_weak(
            peak, current, Ordering::SeqCst, Ordering::SeqCst
        ) {
            Ok(_) => break,
            Err(actual) => peak = actual,
        }
    }
}

/// 更新释放统计
fn update_free_stats(size: usize) {
    MEMORY_STATS.total_deallocations.fetch_add(1, Ordering::SeqCst);
    MEMORY_STATS.current_allocated_bytes.fetch_sub(size, Ordering::SeqCst);
}

/// 获取内存统计
pub fn get_memory_stats() -> &'static MemoryStats {
    &MEMORY_STATS
}

/// 内存泄漏检查
pub fn check_memory_leaks() -> bool {
    let current = MEMORY_STATS.current_allocated_bytes.load(Ordering::SeqCst);
    let total_alloc = MEMORY_STATS.total_allocations.load(Ordering::SeqCst);
    let total_free = MEMORY_STATS.total_deallocations.load(Ordering::SeqCst);

    glib_println!("[glib_memory] 内存泄漏检查:");
    glib_println!("  当前分配: {} bytes", current);
    glib_println!("  总分配次数: {}", total_alloc);
    glib_println!("  总释放次数: {}", total_free);
    glib_println!("  未释放次数: {}", total_alloc.saturating_sub(total_free));

    current > 0 || total_alloc != total_free
}

/// 清理内存管理模块
pub fn cleanup() {
    glib_println!("[glib_memory] 清理内存管理模块");

    // 清理内存池
    unsafe {
        crate::syscall(syscall_number::GLibMemoryPoolsCleanup, [0, 0, 0, 0, 0, 0]);
        SLICE_ALLOCATOR = None;
    }

    // 打印统计信息
    let leaks = check_memory_leaks();
    if !leaks {
        glib_println!("[glib_memory] 无内存泄漏");
    }

    glib_println!("[glib_memory] 内存管理模块清理完成");
}

/// 内存分配测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        init().unwrap();

        // 测试标准分配
        let ptr = g_malloc(100);
        assert!(!ptr.is_null());

        // 写入和读取测试
        unsafe {
            *(ptr as *mut u8) = 42;
            assert_eq!(*(ptr as *mut u8), 42);
        }

        g_free(ptr);

        cleanup();
    }

    #[test]
    fn test_zero_allocation() {
        init().unwrap();

        let ptr = g_malloc0(50);
        assert!(!ptr.is_null());

        // 检查是否被清零
        unsafe {
            for i in 0..50 {
                assert_eq!(*(ptr as *mut u8).add(i), 0);
            }
        }

        g_free(ptr);

        cleanup();
    }

    #[test]
    fn test_reallocation() {
        init().unwrap();

        let ptr1 = g_malloc(100);
        assert!(!ptr1.is_null());

        unsafe {
            *(ptr1 as *mut u32) = 0x12345678;
        }

        // 重新分配更大的内存
        let ptr2 = g_realloc(ptr1, 200);
        assert!(!ptr2.is_null());

        unsafe {
            assert_eq!(*(ptr2 as *mut u32), 0x12345678);
        }

        g_free(ptr2);

        cleanup();
    }

    #[test]
    fn test_slice_allocation() {
        init().unwrap();

        let ptr = g_slice_alloc(64);
        assert!(!ptr.is_null());

        unsafe {
            *(ptr as *mut u64) = 0xDEADBEEFCAFEBABE;
            assert_eq!(*(ptr as *mut u64), 0xDEADBEEFCAFEBABE);
        }

        unsafe {
            g_slice_free1(64, ptr);
        }

        cleanup();
    }

    #[test]
    fn test_memory_stats() {
        init().unwrap();

        let stats = get_memory_stats();
        let initial_allocs = stats.total_allocations.load(Ordering::SeqCst);

        // 分配一些内存
        let ptr1 = g_malloc(100);
        let ptr2 = g_malloc0(50);

        assert_eq!(stats.total_allocations.load(Ordering::SeqCst), initial_allocs + 2);

        g_free(ptr1);
        g_free(ptr2);

        assert_eq!(stats.total_deallocations.load(Ordering::SeqCst), initial_allocs + 2);

        cleanup();
    }
}

// 系统调用号映射
mod syscall_number {
    pub const GLibMemoryPoolCreate: usize = 1001;
    pub const GLibMemoryPoolAlloc: usize = 1002;
    pub const GLibMemoryPoolFree: usize = 1003;
    pub const GLibMemoryPoolsCleanup: usize = 1006;
}