//! Physical memory management for xv6-rust
//! Provides page frame allocation using a free list (like xv6) and bitmap allocator

use crate::sync::Mutex;
use core::ptr;

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;
pub const PAGE_SHIFT: usize = 12;

/// Align address down to page boundary
#[inline]
pub const fn page_round_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

/// Align address up to page boundary
#[inline]
pub const fn page_round_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Physical address to page number
#[inline]
pub const fn addr_to_pfn(addr: usize) -> usize {
    addr >> PAGE_SHIFT
}

/// Page number to physical address
#[inline]
pub const fn pfn_to_addr(pfn: usize) -> usize {
    pfn << PAGE_SHIFT
}

// ============================================================================
// Memory layout symbols from linker script
// ============================================================================

unsafe extern "C" {
    static _heap_start: u8;
    static _heap_end: u8;
    static _stack_top: u8;
}

/// Get heap start address
pub fn heap_start() -> usize {
    unsafe { &_heap_start as *const u8 as usize }
}

/// Get heap end address  
pub fn heap_end() -> usize {
    unsafe { &_heap_end as *const u8 as usize }
}

// ============================================================================
// Free List Page Allocator (xv6 style)
// ============================================================================

/// A free page in the free list
#[repr(C)]
struct FreeNode {
    next: *mut FreeNode,
}

/// Free list based page allocator
struct FreeListAllocator {
    free_list: *mut FreeNode,
    free_count: usize,
    total_pages: usize,
}

// Safety: Protected by mutex
unsafe impl Send for FreeListAllocator {}

impl FreeListAllocator {
    const fn new() -> Self {
        Self {
            free_list: ptr::null_mut(),
            free_count: 0,
            total_pages: 0,
        }
    }

    /// Initialize the allocator with a memory range
    unsafe fn init(&mut self, start: usize, end: usize) {
        let start = page_round_up(start);
        let end = page_round_down(end);
        
        self.free_list = ptr::null_mut();
        self.free_count = 0;
        self.total_pages = (end - start) / PAGE_SIZE;

        // Add all pages to free list
        let mut addr = start;
        while addr + PAGE_SIZE <= end {
            self.free_page(addr as *mut u8);
            addr += PAGE_SIZE;
        }
    }

    /// Allocate a single page, returns null on failure
    fn alloc_page(&mut self) -> *mut u8 {
        let node = self.free_list;
        if node.is_null() {
            return ptr::null_mut();
        }

        // Remove from free list
        unsafe {
            self.free_list = (*node).next;
        }
        self.free_count -= 1;

        // Zero the page
        let page = node as *mut u8;
        unsafe {
            ptr::write_bytes(page, 0, PAGE_SIZE);
        }
        page
    }

    /// Free a single page
    unsafe fn free_page(&mut self, page: *mut u8) {
        if page.is_null() {
            return;
        }

        // Zero the page for security
        ptr::write_bytes(page, 0, PAGE_SIZE);

        // Add to front of free list
        let node = page as *mut FreeNode;
        (*node).next = self.free_list;
        self.free_list = node;
        self.free_count += 1;
    }

    /// Get number of free pages
    fn free_pages(&self) -> usize {
        self.free_count
    }

    /// Get total managed pages
    fn total_pages(&self) -> usize {
        self.total_pages
    }
}

// ============================================================================
// Global Page Allocator
// ============================================================================

static PAGE_ALLOCATOR: Mutex<FreeListAllocator> = Mutex::new(FreeListAllocator::new());

/// Initialize physical memory management
pub fn init() {
    let start = heap_start();
    let end = heap_end();
    
    unsafe {
        PAGE_ALLOCATOR.lock().init(start, end);
    }
    
    // Initialize the kernel heap allocator
    unsafe {
        crate::alloc::init(start, end);
    }

    let alloc = PAGE_ALLOCATOR.lock();
    crate::println!(
        "mm: {} pages free, {} total ({} KB)",
        alloc.free_pages(),
        alloc.total_pages(),
        alloc.free_pages() * PAGE_SIZE / 1024
    );
}

/// Allocate a single physical page (4KB)
/// Returns null pointer on failure
pub fn kalloc() -> *mut u8 {
    PAGE_ALLOCATOR.lock().alloc_page()
}

/// Free a physical page
/// # Safety
/// The page must have been allocated by kalloc
pub unsafe fn kfree(page: *mut u8) {
    if page.is_null() {
        return;
    }
    
    // Validate alignment
    let addr = page as usize;
    if addr % PAGE_SIZE != 0 {
        panic!("kfree: unaligned page {:p}", page);
    }

    unsafe {
        PAGE_ALLOCATOR.lock().free_page(page);
    }
}

/// Allocate multiple contiguous pages
/// Returns null pointer on failure
pub fn kalloc_pages(count: usize) -> *mut u8 {
    if count == 0 {
        return ptr::null_mut();
    }
    if count == 1 {
        return kalloc();
    }

    // For multiple pages, we need a different strategy
    // Simple approach: allocate from end of heap and track separately
    // TODO: Implement buddy allocator for efficient multi-page allocation
    
    // For now, just allocate pages and hope they're contiguous
    // This is a simplification - real implementation needs buddy system
    let first = kalloc();
    if first.is_null() {
        return ptr::null_mut();
    }
    
    for i in 1..count {
        let page = kalloc();
        if page.is_null() || page as usize != first as usize + i * PAGE_SIZE {
            // Not contiguous or failed, free what we got
            for j in 0..i {
                unsafe {
                    kfree((first as usize + j * PAGE_SIZE) as *mut u8);
                }
            }
            return ptr::null_mut();
        }
    }
    
    first
}

/// Get memory statistics
pub fn mem_stats() -> (usize, usize) {
    let alloc = PAGE_ALLOCATOR.lock();
    (alloc.free_pages(), alloc.total_pages())
}

// ============================================================================
// Physical Address wrapper
// ============================================================================

/// A physical address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub const fn page_offset(self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    pub const fn page_number(self) -> usize {
        self.0 >> PAGE_SHIFT
    }

    pub const fn is_page_aligned(self) -> bool {
        self.page_offset() == 0
    }

    pub const fn page_round_up(self) -> Self {
        Self(page_round_up(self.0))
    }

    pub const fn page_round_down(self) -> Self {
        Self(page_round_down(self.0))
    }
}

impl From<usize> for PhysAddr {
    fn from(addr: usize) -> Self {
        Self(addr)
    }
}

impl From<PhysAddr> for usize {
    fn from(addr: PhysAddr) -> Self {
        addr.0
    }
}

// ============================================================================
// Virtual Address wrapper
// ============================================================================

/// A virtual address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(pub usize);

impl VirtAddr {
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub const fn page_offset(self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    pub const fn page_number(self) -> usize {
        self.0 >> PAGE_SHIFT
    }

    pub const fn is_page_aligned(self) -> bool {
        self.page_offset() == 0
    }

    pub const fn page_round_up(self) -> Self {
        Self(page_round_up(self.0))
    }

    pub const fn page_round_down(self) -> Self {
        Self(page_round_down(self.0))
    }

    /// Get page table indices for this virtual address (for 4-level paging)
    #[cfg(target_arch = "x86_64")]
    pub const fn page_table_indices(self) -> [usize; 4] {
        [
            (self.0 >> 39) & 0x1FF, // PML4
            (self.0 >> 30) & 0x1FF, // PDPT
            (self.0 >> 21) & 0x1FF, // PD
            (self.0 >> 12) & 0x1FF, // PT
        ]
    }

    /// Get page table indices for Sv39 (RISC-V)
    #[cfg(target_arch = "riscv64")]
    pub const fn page_table_indices(self) -> [usize; 3] {
        [
            (self.0 >> 30) & 0x1FF, // VPN[2]
            (self.0 >> 21) & 0x1FF, // VPN[1]
            (self.0 >> 12) & 0x1FF, // VPN[0]
        ]
    }

    /// Get page table indices for AArch64 4KB granule
    #[cfg(target_arch = "aarch64")]
    pub const fn page_table_indices(self) -> [usize; 4] {
        [
            (self.0 >> 39) & 0x1FF, // L0
            (self.0 >> 30) & 0x1FF, // L1
            (self.0 >> 21) & 0x1FF, // L2
            (self.0 >> 12) & 0x1FF, // L3
        ]
    }
}

impl From<usize> for VirtAddr {
    fn from(addr: usize) -> Self {
        Self(addr)
    }
}

impl From<VirtAddr> for usize {
    fn from(addr: VirtAddr) -> Self {
        addr.0
    }
}

// ============================================================================
// Memory copy utilities
// ============================================================================

/// Copy data between kernel memory regions
/// # Safety
/// Both src and dst must be valid for len bytes
pub unsafe fn memmove(dst: *mut u8, src: *const u8, len: usize) {
    if dst < src as *mut u8 {
        unsafe {
            ptr::copy(src, dst, len);
        }
    } else {
        unsafe {
            ptr::copy(src, dst, len);
        }
    }
}

/// Set memory to a value
/// # Safety
/// dst must be valid for len bytes
pub unsafe fn memset(dst: *mut u8, val: u8, len: usize) {
    unsafe {
        ptr::write_bytes(dst, val, len);
    }
}

/// Compare memory regions
/// # Safety
/// Both pointers must be valid for len bytes
pub unsafe fn memcmp(s1: *const u8, s2: *const u8, len: usize) -> i32 {
    for i in 0..len {
        let a = unsafe { *s1.add(i) };
        let b = unsafe { *s2.add(i) };
        if a != b {
            return (a as i32) - (b as i32);
        }
    }
    0
}