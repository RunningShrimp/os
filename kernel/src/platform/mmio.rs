//! Memory-Mapped I/O functions
//!
//! This module provides safe and unsafe functions for reading and writing
//! to memory-mapped I/O addresses. These are volatile operations that
//! should not be optimized away by the compiler.

use core::ptr;

/// Read 32-bit value from MMIO address
///
/// # Safety
///
/// The address must be valid for MMIO access
#[inline]
pub unsafe fn mmio_read32(addr: usize) -> u32 {
    ptr::read_volatile(addr as *const u32)
}

/// Write 32-bit value to MMIO address
///
/// # Safety
///
/// The address must be valid for MMIO access
#[inline]
pub unsafe fn mmio_write32(addr: usize, value: u32) {
    ptr::write_volatile(addr as *mut u32, value)
}

/// Read 16-bit value from MMIO address
///
/// # Safety
///
/// The address must be valid for MMIO access
#[inline]
pub unsafe fn mmio_read16(addr: usize) -> u16 {
    ptr::read_volatile(addr as *const u16)
}

/// Write 16-bit value to MMIO address
///
/// # Safety
///
/// The address must be valid for MMIO access
#[inline]
pub unsafe fn mmio_write16(addr: usize, value: u16) {
    ptr::write_volatile(addr as *mut u16, value)
}

/// Read 8-bit value from MMIO address
///
/// # Safety
///
/// The address must be valid for MMIO access
#[inline]
pub unsafe fn mmio_read8(addr: usize) -> u8 {
    ptr::read_volatile(addr as *const u8)
}

/// Write 8-bit value to MMIO address
///
/// # Safety
///
/// The address must be valid for MMIO access
#[inline]
pub unsafe fn mmio_write8(addr: usize, value: u8) {
    ptr::write_volatile(addr as *mut u8, value)
}

/// Read 64-bit value from MMIO address
///
/// # Safety
///
/// The address must be valid for MMIO access
#[inline]
pub unsafe fn mmio_read64(addr: usize) -> u64 {
    ptr::read_volatile(addr as *const u64)
}

/// Write 64-bit value to MMIO address
///
/// # Safety
///
/// The address must be valid for MMIO access
#[inline]
pub unsafe fn mmio_write64(addr: usize, value: u64) {
    ptr::write_volatile(addr as *mut u64, value)
}
