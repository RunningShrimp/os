//! Common system call utilities
//!
//! This module provides common utilities for system calls.

use alloc::string::{String, ToString};
use nos_api::Result;
use nos_api::fmt_utils::format;

/// Get current timestamp in microseconds
pub fn get_timestamp() -> u64 {
    // TODO: Implement actual timestamp logic
    0
}

/// Validate user pointer
/// 
/// Checks if a user-space pointer is valid and safe to access.
/// This function ensures the pointer is not null and within valid user address range.
pub fn validate_user_ptr<T: core::fmt::Debug>(ptr: *const T) -> Result<()> {
    if ptr.is_null() {
        return Err(nos_api::Error::InvalidArgument("Null pointer provided".to_string()));
    }

    let addr = ptr as usize;

    // Basic validation: ensure pointer is not in kernel space
    // User space typically starts at 0 and goes up to some limit
    const KERNEL_BASE: usize = 0xFFFF_8000_0000_0000;
    if addr >= KERNEL_BASE {
        return Err(nos_api::Error::InvalidArgument("Pointer in kernel space".to_string()));
    }

    // Ensure proper alignment for type T
    let align = core::mem::align_of::<T>();
    if !addr.is_multiple_of(align) {
        return Err(nos_api::Error::InvalidArgument(format!("Pointer not properly aligned for type (required: {})", align)));
    }

    sys_trace!("validate_user_ptr passed: ptr={:?}, addr=0x{:x}", ptr, addr);
    Ok(())
}

/// Validate user buffer
///
/// Validates that a user-space buffer pointer and size are safe to access.
/// Checks for null pointer, proper alignment, and size constraints.
pub fn validate_user_buffer(ptr: *const u8, size: usize) -> Result<()> {
    // Validate the pointer itself
    validate_user_ptr(ptr)?;

    // Check size constraints
    if size == 0 {
        return Err(nos_api::Error::InvalidArgument("Buffer size cannot be zero".to_string()));
    }

    // Check for potential overflow when adding size to pointer address
    let addr = ptr as usize;
    const MAX_USER_ADDRESS: usize = 0x0000_7FFF_FFFF_FFFF;
    if addr.checked_add(size).is_none_or(|end| end > MAX_USER_ADDRESS) {
        return Err(nos_api::Error::InvalidArgument("Buffer would overflow user address space".to_string()));
    }

    sys_trace!("validate_user_buffer passed: ptr={:?}, size={}", ptr, size);
    Ok(())
}

/// Copy data from user space
///
/// Copies data from a user-space pointer to a kernel-space reference.
/// This is a stub implementation that logs the operation and validates the source pointer.
pub fn copy_from_user<T: core::fmt::Debug>(_dst: &mut T, src: *const T) -> Result<()> {
    // Validate source pointer
    validate_user_ptr(src)?;

    // In a real kernel implementation, this would perform the actual copy
    // from user space with proper fault handling. For now, we trace the operation
    // and return success since this is a stub implementation.
    sys_trace!("copy_from_user: src={:?}", src);

    // NOTE: Actual copy implementation would use unsafe block with proper error handling:
    // unsafe {
    //     core::ptr::copy_nonoverlapping(src, dst, 1);
    // }

    Ok(())
}

/// Copy data to user space
///
/// Copies data from a kernel-space reference to a user-space pointer.
/// This is a stub implementation that logs the operation and validates the destination pointer.
pub fn copy_to_user<T: core::fmt::Debug>(dst: *mut T, _src: &T) -> Result<()> {
    // Validate destination pointer (allow mutable pointer)
    validate_user_ptr(dst as *const T)?;
    
    // In a real kernel implementation, this would perform the actual copy
    // to user space with proper fault handling. For now, we trace the operation
    // and return success since this is a stub implementation.
    sys_trace!("copy_to_user: dst={:?}", dst);
    
    // NOTE: Actual copy implementation would use unsafe block with proper error handling:
    // unsafe {
    //     core::ptr::copy_nonoverlapping(src, dst, 1);
    // }
    
    Ok(())
}

/// Copy string from user space
///
/// Copies a null-terminated string from user space with a maximum length limit.
/// This is a stub implementation that validates the pointer and returns an empty string.
pub fn copy_string_from_user(ptr: *const u8, max_len: usize) -> Result<String> {
    // Validate the pointer and buffer size
    validate_user_buffer(ptr, max_len)?;
    
    // In a real kernel implementation, this would read the string from user space
    // with proper fault handling. For now, we trace the operation and return an empty string.
    sys_trace!("copy_string_from_user: ptr={:?}, max_len={}", ptr, max_len);
    
    // NOTE: Actual implementation would read bytes from user space until null terminator
    // or max_len is reached, handling page faults appropriately.
    //
    // unsafe {
    //     let mut bytes = Vec::with_capacity(max_len);
    //     for i in 0..max_len {
    //         let byte = *ptr.add(i);
    //         if byte == 0 {
    //             break;
    //         }
    //         bytes.push(byte);
    //     }
    //     String::from_utf8(bytes).map_err(|_| nos_api::Error::InvalidInput("Invalid UTF-8".to_string()))
    // }
    
    Ok(String::new())
}