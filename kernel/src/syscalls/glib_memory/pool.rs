//! Core memory pool management functions

use super::*;
use core::ffi::size_t;

/// 创建GLib专用内存池
///
/// # 参数
/// * `size` - 每个块的大小
/// * `alignment` - 内存对齐要求
///
/// # 返回值
/// * 成功时返回内存池ID
/// * 失败时返回MemoryError
#[no_mangle]
pub extern "C" fn sys_glib_memory_pool_create(size: usize, alignment: usize) -> MemoryResult<c_int> {
    crate::println!("[glib_memory] 创建内存池: size={}, alignment={}", size, alignment);

    // 验证参数
    if size == 0 || alignment == 0 || !alignment.is_power_of_two() {
        crate::println!("[glib_memory] 无效的参数: size={}, alignment={}", size, alignment);
        return Err(MemoryError::InvalidArgument);
    }

    // 验证大小限制 (最大64MB)
    if size > 64 * 1024 * 1024 {
        crate::println!("[glib_memory] 内存池大小过大: {}", size);
        return Err(MemoryError::InvalidSize);
    }

    // 分配内存池ID
    let pool_id = NEXT_POOL_ID.fetch_add(1, Ordering::SeqCst) as c_int;
    if pool_id < 0 {
        crate::println!("[glib_memory] 内存池ID溢出");
        return Err(MemoryError::OutOfMemory);
    }

    // 创建固定大小分配器
    let allocator = match FixedSizeAllocator::new(size, alignment) {
        Ok(alloc) => alloc,
        Err(_) => {
            crate::println!("[glib_memory] 创建分配器失败");
            return Err(MemoryError::OutOfMemory);
        }
    };

    // 创建内存池信息
    let pool_info = MemoryPoolInfo {
        size,
        alignment,
        allocated_blocks: AtomicUsize::new(0),
        freed_blocks: AtomicUsize::new(0),
        active_blocks: AtomicUsize::new(0),
        created_timestamp: crate::time::get_timestamp() as u64,
    };

    // 注册内存池
    {
        let mut pools = MEMORY_POOLS.lock();
        if pools.contains_key(&pool_id) {
            crate::println!("[glib_memory] 内存池ID冲突: {}", pool_id);
            return Err(MemoryError::PoolExists);
        }
        pools.insert(pool_id, (allocator, pool_info));
    }

    crate::println!("[glib_memory] 成功创建内存池 ID={}", pool_id);
    Ok(pool_id)
}

/// 从内存池分配内存
///
/// # 参数
/// * `pool_id` - 内存池ID
/// * `size` - 要分配的内存大小
///
/// # 返回值
/// * 成功时返回内存指针
/// * 失败时返回MemoryError
#[no_mangle]
pub extern "C" fn sys_glib_memory_pool_alloc(pool_id: c_int, size: usize) -> MemoryResult<*mut c_void> {
    crate::println!("[glib_memory] 从池 {} 分配 {} 字节", pool_id, size);

    // 验证内存池ID
    if pool_id <= 0 {
        crate::println!("[glib_memory] 无效的内存池ID: {}", pool_id);
        return Err(MemoryError::InvalidArgument);
    }

    // 验证请求大小
    if size == 0 {
        return Err(MemoryError::InvalidArgument);
    }

    // 获取内存池
    let (allocator, mut pool_info) = {
        let pools = MEMORY_POOLS.lock();
        match pools.get_mut(&pool_id) {
            Some(pair) => pair,
            None => {
                crate::println!("[glib_memory] 内存池不存在: {}", pool_id);
                return Err(MemoryError::PoolNotFound);
            }
        }
    };

    // 检查内存池是否有足够的空间
    if size > pool_info.size {
        crate::println!("[glib_memory] 请求大小 {} 超过内存池块大小 {}", size, pool_info.size);
        return Err(MemoryError::InvalidSize);
    }

    // 分配内存
    match allocator.allocate() {
        Ok(ptr) => {
            // 更新统计信息
            pool_info.allocated_blocks.fetch_add(1, Ordering::SeqCst);
            pool_info.active_blocks.fetch_add(1, Ordering::SeqCst);

            crate::println!("[glib_memory] 成功分配内存: {:p}, 池ID={}", ptr as *mut c_void, pool_id);
            Ok(ptr as *mut c_void)
        }
        Err(_) => {
            crate::println!("[glib_memory] 内存分配失败");
            Err(MemoryError::OutOfMemory)
        }
    }
}

/// 释放内存到内存池
///
/// # 参数
/// * `pool_id` - 内存池ID
/// * `ptr` - 要释放的内存指针
///
/// # 返回值
/// * 成功时返回()
/// * 失败时返回MemoryError
#[no_mangle]
pub extern "C" fn sys_glib_memory_pool_free(pool_id: c_int, ptr: *mut c_void) -> MemoryResult<()> {
    crate::println!("[glib_memory] 释放内存到池 {}: {:p}", pool_id, ptr);

    // 验证参数
    if pool_id <= 0 || ptr.is_null() {
        crate::println!("[glib_memory] 无效参数: pool_id={}, ptr={:p}", pool_id, ptr);
        return Err(MemoryError::InvalidArgument);
    }

    // 获取内存池
    let (allocator, pool_info) = {
        let pools = MEMORY_POOLS.lock();
        match pools.get_mut(&pool_id) {
            Some(pair) => pair,
            None => {
                crate::println!("[glib_memory] 内存池不存在: {}", pool_id);
                return Err(MemoryError::PoolNotFound);
            }
        }
    };

    // 验证指针是否属于该内存池
    if !allocator.owns_pointer(ptr as *mut u8) {
        crate::println!("[glib_memory] 指针不属于内存池 {}: {:p}", pool_id, ptr);
        return Err(MemoryError::InvalidArgument);
    }

    // 释放内存
    unsafe {
        allocator.deallocate(ptr as *mut u8);
    }

    // 更新统计信息
    pool_info.freed_blocks.fetch_add(1, Ordering::SeqCst);
    pool_info.active_blocks.fetch_sub(1, Ordering::SeqCst);

    crate::println!("[glib_memory] 成功释放内存，池ID={}", pool_id);
    Ok(())
}

/// 获取内存池统计信息
///
/// # 参数
/// * `pool_id` - 内存池ID
/// * `stats` - 用于存储统计信息的缓冲区
///
/// # 返回值
/// * 成功时返回()
/// * 失败时返回MemoryError
#[no_mangle]
pub extern "C" fn sys_glib_memory_pool_stats(pool_id: c_int, stats: *mut MemoryPoolInfo) -> MemoryResult<()> {
    crate::println!("[glib_memory] 获取内存池 {} 统计信息", pool_id);

    // 验证参数
    if pool_id <= 0 || stats.is_null() {
        return Err(MemoryError::InvalidArgument);
    }

    // 获取内存池信息
    let pool_info = {
        let pools = MEMORY_POOLS.lock();
        match pools.get(&pool_id) {
            Some((_, info)) => info.clone(),
            None => {
                crate::println!("[glib_memory] 内存池不存在: {}", pool_id);
                return Err(MemoryError::PoolNotFound);
            }
        }
    };

    // 复制统计信息
    unsafe {
        *stats = MemoryPoolInfo {
            size: pool_info.size,
            alignment: pool_info.alignment,
            allocated_blocks: AtomicUsize::new(pool_info.allocated_blocks.load(Ordering::SeqCst)),
            freed_blocks: AtomicUsize::new(pool_info.freed_blocks.load(Ordering::SeqCst)),
            active_blocks: AtomicUsize::new(pool_info.active_blocks.load(Ordering::SeqCst)),
            created_timestamp: pool_info.created_timestamp,
        };
    }

    crate::println!("[glib_memory] 内存池 {} 统计: 分配={}, 释放={}, 活跃={}",
        pool_id,
        pool_info.allocated_blocks.load(Ordering::SeqCst),
        pool_info.freed_blocks.load(Ordering::SeqCst),
        pool_info.active_blocks.load(Ordering::SeqCst)
    );

    Ok(())
}

/// 销毁内存池
///
/// # 参数
/// * `pool_id` - 要销毁的内存池ID
///
/// # 返回值
/// * 成功时返回()
/// * 失败时返回MemoryError
#[no_mangle]
pub extern "C" fn sys_glib_memory_pool_destroy(pool_id: c_int) -> MemoryResult<()> {
    crate::println!("[glib_memory] 销毁内存池: {}", pool_id);

    // 验证参数
    if pool_id <= 0 {
        return Err(MemoryError::InvalidArgument);
    }

    // 移除内存池
    let active_blocks = {
        let mut pools = MEMORY_POOLS.lock();
        match pools.remove(&pool_id) {
            Some((_, info)) => {
                let active = info.active_blocks.load(Ordering::SeqCst);
                crate::println!("[glib_memory] 内存池 {} 仍有 {} 个活跃块", pool_id, active);
                active
            }
            None => {
                crate::println!("[glib_memory] 内存池不存在: {}", pool_id);
                return Err(MemoryError::PoolNotFound);
            }
        }
    };

    if active_blocks > 0 {
        crate::println!("[glib_memory] 警告：内存池 {} 仍有 {} 个活跃块", pool_id, active_blocks);
        // 仍然继续销毁，因为调用者可能知道这些块的清理
    }

    crate::println!("[glib_memory] 成功销毁内存池: {}", pool_id);
    Ok(())
}

/// 清空所有内存池（用于调试）
///
/// # 返回值
/// * 成功时返回()
/// * 失败时返回MemoryError
#[no_mangle]
pub extern "C" fn sys_glib_memory_pools_cleanup() -> MemoryResult<()> {
    crate::println!("[glib_memory] 清理所有GLib内存池");

    let mut total_pools = 0;
    let mut total_active_blocks = 0;

    {
        let mut pools = MEMORY_POOLS.lock();
        let pool_ids: Vec<c_int> = pools.keys().cloned().collect();

        for pool_id in pool_ids {
            if let Some((_, info)) = pools.get(&pool_id) {
                total_pools += 1;
                total_active_blocks += info.active_blocks.load(Ordering::SeqCst);
            }
            pools.remove(&pool_id);
        }
    }

    crate::println!("[glib_memory] 清理完成: {} 个池, {} 个活跃块", total_pools, total_active_blocks);

    // 重置ID计数器
    NEXT_POOL_ID.store(1, Ordering::SeqCst);

    Ok(())
}

/// 获取所有内存池的摘要信息
///
/// # 参数
/// * `max_pools` - 最大返回的内存池数量
/// * `pool_ids` - 用于存储内存池ID的数组
/// * `pool_count` - 实际返回的内存池数量
///
/// # 返回值
/// * 成功时返回()
/// * 失败时返回MemoryError
#[no_mangle]
pub extern "C" fn sys_glib_memory_pools_list(max_pools: c_int, pool_ids: *mut c_int, pool_count: *mut c_int) -> MemoryResult<()> {
    crate::println!("[glib_memory] 列出内存池信息，最大数量: {}", max_pools);

    // 验证参数
    if max_pools <= 0 || pool_ids.is_null() || pool_count.is_null() {
        return Err(MemoryError::InvalidArgument);
    }

    let pools = MEMORY_POOLS.lock();
    let count = core::cmp::min(pools.len(), max_pools as usize);

    let mut actual_count = 0;
    for (i, pool_id) in pools.keys().take(count).enumerate() {
        unsafe {
            *pool_ids.add(i) = *pool_id;
        }
        actual_count += 1;
    }

    unsafe {
        *pool_count = actual_count as c_int;
    }

    crate::println!("[glib_memory] 列出 {} 个内存池", actual_count);
    Ok(())
}