//! GLib memory allocator adapter
//!
//! Provides an adapter layer that wraps the unified allocator
//! for use by GLib memory management functions.
//!
//! **Note**: This module uses the unified memory adapter from libc module
//! to avoid code duplication. Both libc and glib use the same underlying
//! allocator implementation.

use core::ffi::c_void;

// Re-export from libc memory adapter to avoid duplication
pub use crate::libc::memory_adapter::{
    LibcMemoryAdapter,
    get_libc_adapter,
    libc_malloc,
    libc_free,
    libc_calloc,
    libc_realloc,
};

/// GLib memory allocator adapter (type alias for unified adapter)
/// 
/// This is a type alias for LibcMemoryAdapter to maintain API compatibility.
/// Both libc and glib use the same underlying allocator.
pub type GLibMemoryAdapter = LibcMemoryAdapter;

/// Get GLib memory adapter instance
/// 
/// Returns the same adapter instance as libc (unified implementation)
pub fn get_glib_adapter() -> &'static GLibMemoryAdapter {
    // Use the same adapter instance as libc
    get_libc_adapter()
}

/// GLib-compatible malloc function using unified allocator
/// 
/// This is an alias for libc_malloc to avoid code duplication.
/// Both libc and glib use the same underlying allocator.
pub fn glib_malloc(size: usize) -> *mut c_void {
    libc_malloc(size)
}

/// GLib-compatible free function using unified allocator
/// 
/// This is an alias for libc_free to avoid code duplication.
pub fn glib_free(ptr: *mut c_void) {
    libc_free(ptr)
}

/// GLib-compatible calloc function using unified allocator
/// 
/// This is an alias for libc_calloc to avoid code duplication.
pub fn glib_calloc(nmemb: usize, size: usize) -> *mut c_void {
    libc_calloc(nmemb, size)
}

/// GLib-compatible realloc function using unified allocator
/// 
/// This is an alias for libc_realloc to avoid code duplication.
pub fn glib_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    libc_realloc(ptr, size)
}

