//! Physical memory management for xv6-rust
//! Provides page frame allocation using a free list (like xv6) and bitmap allocator

use crate::sync::Mutex;
use crate::println;
use core::ptr;
use crate::mm::buddy::OptimizedBuddyAllocator;

static BUDDY: Mutex<OptimizedBuddyAllocator> = Mutex::new(OptimizedBuddyAllocator::new());
use core::sync::atomic::{AtomicUsize, Ordering};
extern crate alloc;

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
    #[cfg(feature = "link_phys_end")]
    static _phys_end: u8;
}

/// Get heap start address
pub fn heap_start() -> usize {
    unsafe { &_heap_start as *const u8 as usize }
}

/// Get heap end address  
pub fn heap_end() -> usize {
    unsafe { &_heap_end as *const u8 as usize }
}

static PHYS_END_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

pub fn phys_end() -> usize {
    #[cfg(feature = "link_phys_end")]
    unsafe { &_phys_end as *const u8 as usize }
    #[cfg(not(feature = "link_phys_end"))]
    {
        let v = PHYS_END_OVERRIDE.load(Ordering::SeqCst);
        if v != 0 { v } else { heap_end() }
    }
}

pub fn set_phys_end(end: usize) {
    PHYS_END_OVERRIDE.store(end, Ordering::SeqCst);
}

// ============================================================================
// Dynamic MMIO regions
// ============================================================================

use alloc::vec::Vec;
static DYNAMIC_MMIO: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
static DYNAMIC_MMIO_STRONG: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
static DYNAMIC_MMIO_WC: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
static MMIO_STATS: Mutex<Vec<MmioStat>> = Mutex::new(Vec::new());
static MMIO_USAGE: Mutex<Usage> = Mutex::new(Usage { used: 0, total: 0, covered: 0, left: 0 });

/// Add a dynamic MMIO region
pub fn add_mmio_region(base: usize, size: usize) {
    let mut regions = DYNAMIC_MMIO.lock();
    regions.push((base, size));
}

pub fn add_mmio_region_strong(base: usize, size: usize) {
    let mut regions = DYNAMIC_MMIO_STRONG.lock();
    regions.push((base, size));
}

pub fn add_mmio_region_wc(base: usize, size: usize) {
    let mut regions = DYNAMIC_MMIO_WC.lock();
    regions.push((base, size));
}

// ============================================================================
// Free List Page Allocator (xv6 style)
// ============================================================================

/// A free page in the free list
#[repr(C)]
struct FreeNode {
    next: *mut FreeNode,
}

/// Compressed page entry
struct CompressedPage {
    compressed_data: alloc::vec::Vec<u8>,
    original_addr: usize,
}

/// Free list based page allocator
struct FreeListAllocator {
    free_list: *mut FreeNode,
    free_count: usize,
    total_pages: usize,
    compressed_pages: alloc::collections::BTreeMap<usize, CompressedPage>, // Map from original address to compressed data
}

// Safety: Protected by mutex
unsafe impl Send for FreeListAllocator {}

impl FreeListAllocator {
    const fn new() -> Self {
        Self {
            free_list: ptr::null_mut(),
            free_count: 0,
            total_pages: 0,
            compressed_pages: alloc::collections::BTreeMap::new(),
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
        
        // Memory pressure handling: enable compression when free pages < 10%
        if self.free_count < self.total_pages / 10 {
            // TODO: Implement slab_shrink in optimized_slab module
            // let freed = crate::mm::optimized_slab::slab_shrink();
            crate::println!("[mm] pressure: free={} total={}", self.free_count, self.total_pages);
            
            // Trigger memory compression for inactive pages
            // This compresses pages that haven't been accessed recently to free up more memory
            if crate::mm::compress::is_compression_enabled() {
                let compressed_count = self.compress_inactive_pages();
                if compressed_count > 0 {
                    crate::println!("[mm] compression: compressed {} pages, freed {} pages", compressed_count, compressed_count);
                }
            }
        }
        
        // Try to reuse a compressed page if available
        if let Some((addr, compressed)) = self.compressed_pages.iter().next() {
            let addr = *addr;
            let compressed_data = compressed.compressed_data.clone();
            
            // Decompress the page
            if let Some(decompressed) = unsafe { crate::mm::compress::decompress_page(&compressed_data) } {
                if decompressed.len() == PAGE_SIZE {
                    // Remove from compressed pages
                    self.compressed_pages.remove(&addr);
                    
                    // Write decompressed data to page
                    let page_ptr = addr as *mut u8;
                    unsafe {
                        ptr::copy_nonoverlapping(decompressed.as_ptr(), page_ptr, PAGE_SIZE);
                    }
                    
                    return page_ptr;
                }
            }
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

    /// Compress inactive pages when under memory pressure
    /// Returns the number of pages compressed
    fn compress_inactive_pages(&mut self) -> usize {
        if !crate::mm::compress::is_compression_enabled() {
            return 0;
        }
        
        // For demonstration, we'll compress a few pages from the free list
        // In a real implementation, this would track page access and compress inactive pages
        let target_compress = (self.free_count / 10).min(10); // Compress up to 10% of free pages or 10 pages max
        let mut compressed_count = 0;
        
        // Take some pages from free list and compress them
        for _ in 0..target_compress {
            let node = self.free_list;
            if node.is_null() {
                break;
            }
            
            unsafe {
                self.free_list = (*node).next;
            }
            self.free_count -= 1;
            
            let page_ptr = node as *mut u8;
            let page_addr = page_ptr as usize;
            
            // Compress the page
            if let Some(compressed_data) = unsafe { crate::mm::compress::compress_page(page_ptr, PAGE_SIZE) } {
                // Check if compression actually saved space
                if compressed_data.len() < PAGE_SIZE {
                    // Store compressed page
                    self.compressed_pages.insert(page_addr, CompressedPage {
                        compressed_data,
                        original_addr: page_addr,
                    });
                    compressed_count += 1;
                } else {
                    // Compression didn't help, put page back
                    unsafe {
                        self.free_page(page_ptr);
                    }
                }
            } else {
                // Compression failed, put page back
                unsafe {
                    self.free_page(page_ptr);
                }
            }
        }
        
        compressed_count
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
    // Split heap into slab region (first 25%) and buddy region (remaining 75%)
    let total_size = end - start;
    let slab_size = total_size / 4;
    let buddy_size = total_size - slab_size;
    unsafe {
        crate::mm::allocator::init(start, slab_size, start + slab_size, buddy_size, PAGE_SIZE);
        // Initialize buddy allocator for multi-page allocations
        BUDDY.lock().init(start + slab_size, end, PAGE_SIZE);
    }

    // Enable memory compression if memory is limited
    // Compression is enabled by default for systems with < 64MB RAM
    let total_memory_mb = total_size / (1024 * 1024);
    if total_memory_mb < 64 {
        crate::mm::compress::enable_compression();
        crate::println!("[mm] Memory compression enabled (total RAM: {} MB)", total_memory_mb);
    }

    // Buddy allocator statistics
    let stats = BUDDY.lock().stats();
    crate::println!("[mm] buddy: allocated={}, freed={}, frag={}", stats.allocated, stats.freed, stats.fragmentation);
    // TODO: Implement slab_stats and slab_shrink in optimized_slab module
    // let slab = crate::mm::optimized_slab::slab_stats();
    // crate::println!("[mm] slab: used={} allocated={}", slab.used, slab.allocated);
    // let freed = crate::mm::optimized_slab::slab_shrink();
    // crate::println!("[mm] slab: shrink freed {} empty slabs", freed);
    
    // Huge page allocator statistics
    let hugepage = crate::mm::allocator::get_global_allocator();
    let hugepage_sizes = hugepage.get_hugepage_sizes();
    if !hugepage_sizes.is_empty() {
        crate::println!("[mm] hugepage: supported sizes: {:?}", hugepage_sizes);
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
    if count == 0 { return ptr::null_mut(); }
    if count == 1 { return kalloc(); }
    use core::alloc::Layout;
    let layout = Layout::from_size_align(count * PAGE_SIZE, PAGE_SIZE).unwrap();
    let addr = BUDDY.lock().alloc(layout);
    if !addr.is_null() {
        unsafe { ptr::write_bytes(addr as *mut u8, 0, count * PAGE_SIZE); }
        return addr as *mut u8;
    }
    ptr::null_mut()
}

pub fn mmio_regions() -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    #[cfg(target_arch = "riscv64")]
    {
        regions.push((0x1000_0000, 0x0001_0000)); // UART
        regions.push((0x0200_0000, 0x000C_0000)); // CLINT
        regions.push((0x0C00_0000, 0x0040_0000)); // PLIC
    }
    #[cfg(target_arch = "aarch64")]
    {
        regions.push((0x0900_0000, 0x0002_0000)); // PL011 UART
        regions.push((0x0800_0000, 0x0020_0000)); // GIC (approx)
    }
    
    // Add dynamic regions
    let dynamic = DYNAMIC_MMIO.lock();
    regions.extend_from_slice(&dynamic);
    
    regions
}

pub fn mmio_regions_strong() -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    #[cfg(target_arch = "aarch64")]
    {
        regions.push((0x0800_0000, 0x0020_0000));
    }
    let dynamic = DYNAMIC_MMIO_STRONG.lock();
    regions.extend_from_slice(&dynamic);
    regions
}

pub fn mmio_regions_wc() -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let dynamic = DYNAMIC_MMIO_WC.lock();
    regions.extend_from_slice(&dynamic);
    regions
}

#[derive(Clone, Copy)]
struct MmioStat { base: usize, size: usize, priority: u8, hits: u64, ewma_q16: u64, cooldown_until: u64, last_tick: u64, sample_acc: u64 }

#[derive(Clone, Copy)]
struct Usage { used: usize, total: usize, covered: u64, left: u64 }

#[derive(Clone, Copy)]
struct MmioCfg { decay_interval_ticks: u64, decay_num: u64, decay_den: u64, threshold_q16: u64, cooldown_ticks: u64, sample_div: u64 }
static MMIO_CFG: Mutex<MmioCfg> = Mutex::new(MmioCfg { decay_interval_ticks: 100, decay_num: 7, decay_den: 8, threshold_q16: 200u64 << 16, cooldown_ticks: 1000, sample_div: 1 });

pub fn mmio_stats_init() {
    let mut v = MMIO_STATS.lock();
    v.clear();
    let now = crate::time::get_ticks();
    for (b, s) in mmio_regions_strong() { v.push(MmioStat { base: b, size: s, priority: 1, hits: 0, ewma_q16: 0, cooldown_until: 0, last_tick: now, sample_acc: 0 }); }
    for (b, s) in mmio_regions() { v.push(MmioStat { base: b, size: s, priority: 0, hits: 0, ewma_q16: 0, cooldown_until: 0, last_tick: now, sample_acc: 0 }); }
}

pub fn mmio_hit(addr: usize) {
    let mut v = MMIO_STATS.lock();
    let cfg = MMIO_CFG.lock().clone();
    for e in v.iter_mut() {
        if addr >= e.base && addr < e.base.saturating_add(e.size) {
            e.sample_acc = e.sample_acc.saturating_add(1);
            let div = if cfg.sample_div == 0 { 1 } else { cfg.sample_div };
            if e.sample_acc % div == 0 { e.hits = e.hits.saturating_add(1); }
            break;
        }
    }
}

pub fn mmio_stats_take() -> Vec<(usize, usize, u8, u64)> {
    let v = MMIO_STATS.lock();
    v.iter().map(|e| (e.base, e.size, e.priority, e.ewma_q16)).collect()
}

pub fn mmio_stats_periodic(current_tick: u64) {
    let cfg = MMIO_CFG.lock().clone();
    let mut v = MMIO_STATS.lock();
    if cfg.decay_interval_ticks == 0 { return; }
    if current_tick % cfg.decay_interval_ticks != 0 { return; }
    let mut max_q16 = 0u64;
    let mut max_idx = None::<usize>;
    for (i, e) in v.iter_mut().enumerate() {
        let ticks = current_tick.saturating_sub(e.last_tick);
        if ticks >= cfg.decay_interval_ticks {
            e.ewma_q16 = (e.ewma_q16.saturating_mul(cfg.decay_num) / cfg.decay_den).saturating_add(e.hits << 16);
            e.hits = 0;
            e.last_tick = current_tick;
        }
        if current_tick >= e.cooldown_until {
            if e.ewma_q16 > max_q16 { max_q16 = e.ewma_q16; max_idx = Some(i); }
        }
    }
    if let Some(_i) = max_idx {
        if max_q16 >= cfg.threshold_q16 {
            #[cfg(target_arch = "x86_64")]
            {
                drop(v);
                crate::mm::vm::refresh_mtrr_hot();
                let mut v2 = MMIO_STATS.lock();
                let now = current_tick;
                for e in v2.iter_mut() { e.cooldown_until = now.saturating_add(cfg.cooldown_ticks); }
            }
        }
    }
}

pub fn mmio_cooldown_all(current_tick: u64) {
    let cfg = MMIO_CFG.lock().clone();
    let mut v = MMIO_STATS.lock();
    for e in v.iter_mut() { e.cooldown_until = current_tick.saturating_add(cfg.cooldown_ticks); }
}

pub fn mmio_record_mtrr_usage(used: usize, total: usize, covered: u64, left: u64) {
    let mut u = MMIO_USAGE.lock();
    *u = Usage { used, total, covered, left };
}

pub fn mmio_last_usage() -> (usize, usize, u64, u64) {
    let u = MMIO_USAGE.lock();
    (u.used, u.total, u.covered, u.left)
}

pub fn mmio_cfg_set(decay_interval_ticks: u64, decay_num: u64, decay_den: u64, threshold_hits: u64, cooldown_ticks: u64, sample_div: u64) {
    let mut cfg = MMIO_CFG.lock();
    cfg.decay_interval_ticks = decay_interval_ticks;
    cfg.decay_num = if decay_num == 0 { 1 } else { decay_num };
    cfg.decay_den = if decay_den == 0 { 1 } else { decay_den };
    cfg.threshold_q16 = threshold_hits << 16;
    cfg.cooldown_ticks = cooldown_ticks;
    cfg.sample_div = if sample_div == 0 { 1 } else { sample_div };
}

pub fn mmio_cfg_update(decay_interval_ticks: Option<u64>, decay_num: Option<u64>, decay_den: Option<u64>, threshold_hits: Option<u64>, cooldown_ticks: Option<u64>, sample_div: Option<u64>) {
    let mut cfg = MMIO_CFG.lock();
    if let Some(v) = decay_interval_ticks { cfg.decay_interval_ticks = v; }
    if let Some(v) = decay_num { cfg.decay_num = if v == 0 { 1 } else { v }; }
    if let Some(v) = decay_den { cfg.decay_den = if v == 0 { 1 } else { v }; }
    if let Some(v) = threshold_hits { cfg.threshold_q16 = v << 16; }
    if let Some(v) = cooldown_ticks { cfg.cooldown_ticks = v; }
    if let Some(v) = sample_div { cfg.sample_div = if v == 0 { 1 } else { v }; }
}

#[inline]
pub fn mmio_read8(addr: *const u8) -> u8 {
    mmio_hit(addr as usize);
    unsafe { core::ptr::read_volatile(addr) }
}
#[inline]
pub fn mmio_read16(addr: *const u16) -> u16 {
    mmio_hit(addr as usize);
    unsafe { core::ptr::read_volatile(addr) }
}
#[inline]
pub fn mmio_read32(addr: *const u32) -> u32 {
    mmio_hit(addr as usize);
    unsafe { core::ptr::read_volatile(addr) }
}
#[inline]
pub fn mmio_read64(addr: *const u64) -> u64 {
    mmio_hit(addr as usize);
    unsafe { core::ptr::read_volatile(addr) }
}
#[inline]
pub fn mmio_write8(addr: *mut u8, val: u8) {
    mmio_hit(addr as usize);
    unsafe { core::ptr::write_volatile(addr, val) }
}
#[inline]
pub fn mmio_write16(addr: *mut u16, val: u16) {
    mmio_hit(addr as usize);
    unsafe { core::ptr::write_volatile(addr, val) }
}
#[inline]
pub fn mmio_write32(addr: *mut u32, val: u32) {
    mmio_hit(addr as usize);
    unsafe { core::ptr::write_volatile(addr, val) }
}
#[inline]
pub fn mmio_write64(addr: *mut u64, val: u64) {
    mmio_hit(addr as usize);
    unsafe { core::ptr::write_volatile(addr, val) }
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
