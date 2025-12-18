//! mm模块内存分配公共接口
//! 
//! 提供基本的内存分配和释放功能

use super::AllocError;
use core::alloc::Layout;

/// Allocate memory block
/// 
/// # Contract
/// * Returned memory must be aligned as specified
/// * Return clear error when allocation fails
/// * Allocated memory must be initialized to zero
/// * Must support zero size allocation
pub fn allocate(size: usize, align: usize) -> Result<*mut u8, AllocError> {
    // TODO: Implement this function
    Err(AllocError::OutOfMemory)
}

/// Deallocate memory block
/// 
/// # Contract
/// * Can only deallocate memory previously allocated
/// * Memory cannot be accessed after deallocation
/// * Must handle double free
/// * Must support zero size deallocation
pub fn deallocate(ptr: *mut u8, size: usize) -> Result<(), AllocError> {
    // TODO: Implement this function
    Ok(())
}

/// Reallocate memory block
/// 
/// # Contract
/// * Try to expand at original address
/// * Allocate new memory and copy data when expansion not possible
/// * Original memory is automatically freed
/// * Must handle zero size reallocation
pub fn reallocate(ptr: *mut u8, old_size: usize, new_size: usize, align: usize) -> Result<*mut u8, AllocError> {
    // TODO: Implement this function
    Err(AllocError::OutOfMemory)
}

/// Allocate memory with Layout
pub fn allocate_layout(layout: Layout) -> Result<*mut u8, AllocError> {
    allocate(layout.size(), layout.align())
}

/// Deallocate memory with Layout
pub fn deallocate_layout(ptr: *mut u8, layout: Layout) -> Result<(), AllocError> {
    deallocate(ptr, layout.size())
}