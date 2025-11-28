//! Memory Management Service for hybrid architecture
//! Separates memory management functionality from kernel core

use crate::mm::{
    kalloc, kfree, kalloc_pages, PAGE_SIZE, 
    VirtAddr, PhysAddr, page_round_up, page_round_down,
};
use crate::services::{service_register, ServiceInfo};
use crate::sync::Mutex;

// ============================================================================
// Memory Management Service State
// ============================================================================

/// Memory service endpoint (IPC channel)
pub const MEMORY_SERVICE_ENDPOINT: usize = 0x1000;

/// Memory statistics
pub struct MemoryStats {
    pub free_pages: usize,
    pub total_pages: usize,
    pub used_slabs: usize,
    pub free_slabs: usize,
}

static MEMORY_STATS: Mutex<MemoryStats> = Mutex::new(MemoryStats {
    free_pages: 0,
    total_pages: 0,
    used_slabs: 0,
    free_slabs: 0,
});

// ============================================================================
// Public API
// ============================================================================

/// Initialize memory management service
pub fn init() {
    // Register memory management service
    service_register(
        "memory_manager",
        "Memory management service for physical and virtual memory",
        MEMORY_SERVICE_ENDPOINT
    );
    
    // Initialize memory statistics
    let (free, total) = crate::mm::mem_stats();
    let mut stats = MEMORY_STATS.lock();
    stats.free_pages = free;
    stats.total_pages = total;
    stats.used_slabs = 0;
    stats.free_slabs = 0;
    
    crate::println!("services/memory: initialized");
}

/// Allocate a single page
pub fn mem_alloc_page() -> *mut u8 {
    let page = kalloc();
    
    // Update statistics
    let mut stats = MEMORY_STATS.lock();
    if !page.is_null() {
        stats.free_pages = stats.free_pages.saturating_sub(1);
    }
    
    page
}

/// Free a single page
pub unsafe fn mem_free_page(page: *mut u8) {
    if page.is_null() {
        return;
    }
    
    // Update statistics
    let mut stats = MEMORY_STATS.lock();
    stats.free_pages += 1;
    
    kfree(page);
}

/// Allocate multiple contiguous pages
pub fn mem_alloc_pages(count: usize) -> *mut u8 {
    let addr = kalloc_pages(count);
    
    // Update statistics
    let mut stats = MEMORY_STATS.lock();
    if !addr.is_null() {
        stats.free_pages = stats.free_pages.saturating_sub(count);
    }
    
    addr
}

/// Free multiple contiguous pages
pub unsafe fn mem_free_pages(addr: *mut u8, count: usize) {
    if addr.is_null() {
        return;
    }
    
    // TODO: Need to implement buddy system free
    // For now, let's keep using the old free mechanism
    
    // Update statistics
    let mut stats = MEMORY_STATS.lock();
    stats.free_pages += count;
}

/// Get memory statistics
pub fn mem_get_stats() -> MemoryStats {
    let stats = MEMORY_STATS.lock();
    *stats
}

/// Align address down to page boundary
pub fn mem_page_round_down(addr: usize) -> usize {
    page_round_down(addr)
}

/// Align address up to page boundary
pub fn mem_page_round_up(addr: usize) -> usize {
    page_round_up(addr)
}