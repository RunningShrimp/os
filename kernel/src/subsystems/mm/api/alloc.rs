//! mm模块内存分配公共接口
//!
//! 提供基本的内存分配和释放功能（作为 mm 的唯一对外入口，内部走每CPU快速路径 + 全局回退）。

use super::AllocError;
use core::alloc::Layout;
use core::ptr::{self, null_mut};

use crate::subsystems::mm::percpu_allocator::current_cpu_allocator;

/// Allocate memory block
///
/// # Contract
/// * Returned memory must be aligned as specified
/// * Return clear error when allocation fails
/// * Allocated memory must be initialized to zero
/// * Must support zero size allocation
pub fn allocate(size: usize, align: usize) -> Result<*mut u8, AllocError> {
    // 零大小分配：返回空指针（调用方不得解引用）
    if size == 0 {
        return Ok(null_mut());
    }

    if align == 0 || !align.is_power_of_two() {
        return Err(AllocError::InvalidAlignment);
    }

    let layout = Layout::from_size_align(size, align).map_err(|_| AllocError::InvalidAlignment)?;

    unsafe {
        // 快路径：当前 CPU 的本地分配器（内部有小对象 per-CPU 缓存，超过阈值自动回退全局）
        let ptr = current_cpu_allocator().alloc(layout);
        if ptr.is_null() {
            return Err(AllocError::OutOfMemory);
        }

        // 按约定清零
        ptr::write_bytes(ptr, 0, size);
        Ok(ptr)
    }
}

/// Deallocate memory block
///
/// # Contract
/// * Can only deallocate memory previously allocated
/// * Memory cannot be accessed after deallocation
/// * Must handle double free
/// * Must support zero size deallocation
pub fn deallocate(ptr: *mut u8, size: usize) -> Result<(), AllocError> {
    // 零大小或空指针释放视为 no-op
    if ptr.is_null() || size == 0 {
        return Ok(());
    }

    // 无法获知原始对齐，使用机器字对齐；调用方需与 allocate 保持一致
    let layout = Layout::from_size_align(size, core::mem::align_of::<usize>())
        .map_err(|_| AllocError::InvalidAlignment)?;

    unsafe {
        current_cpu_allocator().dealloc(ptr, layout);
    }
    Ok(())
}

/// Reallocate memory block
///
/// # Contract
/// * Try to expand at original address（这里使用 allocate+copy 语义化实现）
/// * Allocate new memory and copy data when expansion not possible
/// * Original memory is automatically freed
/// * Must handle zero size reallocation
pub fn reallocate(ptr: *mut u8, old_size: usize, new_size: usize, align: usize) -> Result<*mut u8, AllocError> {
    // new_size == 0 等价于 free + 返回空指针
    if new_size == 0 {
        if !ptr.is_null() && old_size > 0 {
            let _ = deallocate(ptr, old_size);
        }
        return Ok(null_mut());
    }

    // 等价于首次分配
    if ptr.is_null() || old_size == 0 {
        return allocate(new_size, align);
    }

    let new_ptr = allocate(new_size, align)?;

    unsafe {
        let copy_size = core::cmp::min(old_size, new_size);
        ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
        let _ = deallocate(ptr, old_size);
    }

    Ok(new_ptr)
}

/// Allocate memory with Layout
pub fn allocate_layout(layout: Layout) -> Result<*mut u8, AllocError> {
    allocate(layout.size(), layout.align())
}

/// Deallocate memory with Layout
pub fn deallocate_layout(ptr: *mut u8, layout: Layout) -> Result<(), AllocError> {
    deallocate(ptr, layout.size())
}

