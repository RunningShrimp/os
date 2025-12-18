// Memory utility functions for bootloader

/** Copy memory region (memmove equivalent)
 * 
 * # Safety
 * - `dst` must be a valid mutable pointer to a memory region of at least `size` bytes
 * - `src` must be a valid pointer to a memory region of at least `size` bytes
 * - The memory regions must not overlap in a way that would cause data corruption
 */
pub unsafe fn mem_copy(dst: *mut u8, src: *const u8, size: usize) {
    if dst as usize <= src as usize {
        // Forward copy
        for i in 0..size {
            *dst.add(i) = *src.add(i);
        }
    } else {
        // Backward copy (for overlapping regions)
        for i in (0..size).rev() {
            *dst.add(i) = *src.add(i);
        }
    }
}

/** Zero memory region (memset equivalent)
 * 
 * # Safety
 * - `ptr` must be a valid mutable pointer to a memory region of at least `size` bytes
 */
pub unsafe fn mem_zero(ptr: *mut u8, size: usize) {
    for i in 0..size {
        *ptr.add(i) = 0;
    }
}

/** Compare memory regions
 * 
 * # Safety
 * - `a` must be a valid pointer to a memory region of at least `size` bytes
 * - `b` must be a valid pointer to a memory region of at least `size` bytes
 */
pub unsafe fn mem_cmp(
    a: *const u8,
    b: *const u8,
    size: usize,
) -> bool {
    for i in 0..size {
        if *a.add(i) != *b.add(i) {
            return false;
        }
    }
    true
}

/** Find byte in memory
 * 
 * # Safety
 * - `haystack` must be a valid pointer to a memory region of at least `size` bytes
 */
pub unsafe fn mem_find_u8(
    haystack: *const u8,
    needle: u8,
    size: usize,
) -> Option<usize> {
    (0..size).find(|&i| *haystack.add(i) == needle)
}

/** Get word from memory at aligned address
 * 
 * # Safety
 * - `addr` must be a valid, aligned pointer to a u64 value
 */
pub unsafe fn mem_read_u64(addr: *const u64) -> u64 {
    *addr
}

/** Write word to memory at aligned address
 * 
 * # Safety
 * - `addr` must be a valid, aligned pointer to a u64 value
 */
pub unsafe fn mem_write_u64(addr: *mut u64, val: u64) {
    *addr = val;
}

/** Check memory is zero-filled
 * 
 * # Safety
 * - `ptr` must be a valid pointer to a memory region of at least `size` bytes
 */
pub unsafe fn mem_is_zero(ptr: *const u8, size: usize) -> bool {
    for i in 0..size {
        if *ptr.add(i) != 0 {
            return false;
        }
    }
    true
}

/// Align pointer up to boundary
pub const fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

/// Align pointer down to boundary
pub const fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

/// Check if address is aligned
pub const fn is_aligned(addr: u64, align: u64) -> bool {
    (addr & (align - 1)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0x1001, 0x1000), 0x2000);
        assert_eq!(align_up(0x1000, 0x1000), 0x1000);
    }

    #[test]
    fn test_align_down() {
        assert_eq!(align_down(0x1FFF, 0x1000), 0x1000);
        assert_eq!(align_down(0x1000, 0x1000), 0x1000);
    }

    #[test]
    fn test_is_aligned() {
        assert!(is_aligned(0x1000, 0x1000));
        assert!(!is_aligned(0x1001, 0x1000));
    }
}
