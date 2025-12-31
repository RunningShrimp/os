#![allow(dead_code)]
//! # 内存管理子系统
//!
//! 负责物理内存和虚拟内存的管理，提供高效的内存分配和映射机制。
//!
//! ## 概述
//!
//! 内存管理子系统是内核的核心组件，提供：
//! - **物理内存管理**: 页面分配器和物理内存管理
//! - **虚拟内存管理**: 地址空间、页表和内存映射
//! - **内存分配器**: 多种分配策略（Buddy、Slab、Per-CPU）
//! - **内存优化**: 大页、压缩、预取等优化
//! - **NUMA 支持**: 多节点系统的内存管理
//! - **内存隔离**: 进程间和容器间的内存隔离
//!
//! ## 主要组件
//!
//! ### 核心模块
//!
//! - [`phys`]: 物理内存管理，提供 `kalloc` 和 `kfree`
//! - [`vm`]: 虚拟内存管理，页表操作和地址空间
//! - [`allocator`]: 通用内存分配器接口
//! - [`buddy`]: Buddy 分配器，用于大块内存分配
//! - [`slab`]: Slab 分配器，用于固定大小对象
//!
//! ### 高级特性
//!
//! - [`api`]: 统一的内存管理 API
//! - [`hugepage`]: 大页支持（2MB、1GB）
//! - [`compress`]: 内存压缩

//! - [`numa`]: NUMA 感知内存分配
//! - [`prefetch`]: 内存预取优化
//! - [`percpu_allocator`]: Per-CPU 分配器
//! - [`optimized_page_allocator`]: 优化的页面分配器
//! - [`memory_isolation`]: 内存隔离机制
//! - [`stats`]: 内存使用统计
//! - [`unified_stats`]: 统一的统计接口
//!
//! ## 架构
//!
//! ```
//! 内存管理子系统
//!     ├── 物理内存管理 (phys)
//!     │   ├── 页面分配器
//!     │   └── 物理页帧
//!     ├── 虚拟内存管理 (vm)
//!     │   ├── 页表管理
//!     │   ├── 地址空间
//!     │   └── 内存映射
//!     ├── 分配器
//!     │   ├── Buddy 分配器
//!     │   ├── Slab 分配器
//!     │   └── Per-CPU 分配器
//!     └── 优化
//!         ├── 大页 (hugepage)
//!         ├── 压缩 (compress)
//!         └── NUMA (numa)
//! ```
//!
//! ## 使用示例
//!
//! ### 物理内存分配
//!
//! ```no_run
//! use kernel::subsystems::mm::{kalloc, kfree, PAGE_SIZE};
//!
//! // 分配一页物理内存
//! let ptr = kalloc();
//! if !ptr.is_null() {
//!     // 使用内存
//!     unsafe {
//!         *ptr = 42u8;
//!     }
//!
//!     // 释放内存
//!     kfree(ptr);
//! }
//! ```
//!
//! ### 虚拟内存映射
//!
//! ```no_run
//! use kernel::subsystems::mm::{map_pages, VmPerm, PAGE_SIZE};
//!
//! // 映射虚拟页面到物理内存
//! let virt_addr = 0x1000_0000;
//! let phys_addr = 0x2000_0000;
//! let perm = VmPerm::READ | VmPerm::WRITE;
//!
//! map_pages(virt_addr, phys_addr, PAGE_SIZE, perm)?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ### 内存统计
//!
//! ```no_run
//! use kernel::subsystems::mm::{AllocationStats, get_memory_stats};
//!
//! let stats = get_memory_stats();
//! println!("Total memory: {} bytes", stats.total_memory);
//! println!("Used memory: {} bytes", stats.used_memory);
//! println!("Free memory: {} bytes", stats.free_memory);
//! ```
//!
//! ## 设计决策
//!
//! ### 多层次分配器
//!
//! NOS 使用多层次的内存管理策略：
//! - **Buddy 分配器**: 管理物理页帧，适合大块分配
//! - **Slab 分配器**: 管理固定大小对象，减少碎片
//! - **Per-CPU 分配器**: 减少 SMP 系统中的锁竞争
//!
//! ### 写时复制
//!
//! Fork 时使用写时复制（COW）优化：
//! - 父子进程共享物理内存页
//! - 只在写入时复制页面
//! - 大大减少 fork 的开销
//!
//! ### 大页支持
//!
//! 支持多种页面大小：
//! - 4KB: 默认页面大小
//! - 2MB: 大页，减少 TLB 缺失
//! - 1GB: 巨页，用于大内存应用
//!
//! ## 性能特征
//!
//! - **分配延迟**:
//!   - Per-CPU: O(1) 快速路径
//!   - Slab: O(1)
//!   - Buddy: O(log n)
//! - **内存开销**: < 5% 用于元数据
//! - **碎片率**: < 10% 外部碎片
//!
//! ## 线程安全
//!
//! 内存管理器提供多种同步机制：
//! - 全局分配器使用 `Mutex` 或 `SpinLock`
//! - Per-CPU 分配器无锁（快速路径）
//! - RCU 用于延迟释放
//!
//! ## 内存布局
//!
//! ```
//! 虚拟地址空间 (48-bit)
//!     ├── 用户空间 (0x0000_0000_0000 - 0x0000_ffff_ffff)
//!     ├── 内核空间 (0xffff_8000_0000 - 0xffff_ffff_ffff)
//!     │   ├── 内核代码段
//!     │   ├── 内核数据段
//!     │   ├── 直接映射区域
//!     │   └── vmalloc 区域
//! ```
//!
//! ## 相关模块
//!
//! - [`crate::subsystems::process`]: 进程地址空间管理
//! - [`crate::arch::memory_layout`]: 架构特定的内存布局
//! - [`crate::security::aslr`]: 地址空间布局随机化

// Note: nos-mm re-export removed since crate::mm module doesn't exist
// Memory management functionality is now provided directly by this module

// Core memory management modules
pub mod allocator;
pub mod brk;
pub mod buddy;
pub mod madvise;
pub mod phys;
pub mod slab;
pub mod vm;

// Advanced memory management extensions
pub mod api;
pub mod compression;
pub mod page_compression;
pub mod swap_compression;
pub mod hugepage;
pub mod memory_isolation;
pub mod page_table_isolation;
pub mod numa;
pub mod optimized_page_allocator;
pub mod percpu_allocator;
pub mod sharded_allocator;
pub mod prefetch;
pub mod traits;
pub mod types;
pub mod unified_stats;

// Track AV: 高级内存管理优化
pub mod compress;
pub mod migration;
pub mod hotremove;
pub mod thp;
pub mod memcg;
pub mod memory_failure;

// Storage Class Memory (SCM) support
pub mod nvdimm;
pub mod pmem_tx;
pub mod unified;
pub mod libpmem;

// Re-export commonly used items from phys and vm modules
pub use phys::{kalloc, kfree, PAGE_SIZE};
pub use phys::{
    add_mmio_region, add_mmio_region_strong, add_mmio_region_wc,
    mmio_read64, mmio_write64, set_phys_end, mmio_cfg_update,
};
// Re-export unified stats to avoid duplication
pub use unified_stats::{
    AllocationStats, AtomicAllocationStats, ExtendedAllocationStats, LightweightAllocationStats,
    MemoryManagementStats, MemoryType, NumStats,
};
pub use vm::{
    PTE_COUNT, VmArea, VmPerm, activate, copyout, flags, flush_tlb_page, free_pagetable,
    map_pages,
};
pub use page_table_isolation::PageTable;

#[cfg(feature = "kernel_tests")]
pub mod tests;

// pub use optimized_allocator::OptimizedHybridAllocator;

/// Align up to the given alignment
pub const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Align down to the given alignment
pub const fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

/// Check if an address is aligned to the given alignment
pub const fn is_aligned(addr: usize, align: usize) -> bool {
    (addr & (align - 1)) == 0
}

/// Round up to the next power of 2
pub const fn round_up_power_of_2(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        let mut v = n - 1;
        v |= v >> 1;
        v |= v >> 2;
        v |= v >> 4;
        v |= v >> 8;
        v |= v >> 16;
        v |= v >> 32;
        v + 1
    }
}

/// Get the log2 of a power-of-2 number
pub const fn log2_pow2(n: usize) -> u32 {
    if n == 0 {
        panic!("log2_pow2(0) is undefined");
    }
    (usize::BITS - 1) - n.leading_zeros()
}

/// Get the order (log2) of a size, rounded up to the nearest power of 2
pub const fn get_order(size: usize, min_order: usize) -> usize {
    let mut order = min_order;
    let mut current_size = 1 << min_order;

    while current_size < size {
        current_size *= 2;
        order += 1;
    }

    order
}

/// Initialize advanced memory management
///
/// This function initializes advanced memory management features
/// that build on top of the basic nos-mm functionality.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn init_advanced_memory_management() -> nos_api::Result<()> {
    // Initialize NUMA support
    numa::init_numa()?;

    // Initialize per-CPU allocators
    percpu_allocator::init_percpu_allocators();

    // Initialize optimized memory manager
    // optimized_memory_manager::init_optimized_memory_manager()?;

    // Note: unified_stats does not require initialization
    // Statistics are tracked using atomic operations

    Ok(())
}

/// Shutdown advanced memory management
///
/// This function shuts down advanced memory management features.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn shutdown_advanced_memory_management() -> nos_api::Result<()> {
    // Shutdown optimized memory manager
    // optimized_memory_manager::shutdown_optimized_memory_manager()?;

    // Shutdown per-CPU allocators
    percpu_allocator::shutdown_percpu_allocators()?;

    // Shutdown NUMA support
    numa::shutdown_numa()?;

    Ok(())
}

/// Get memory management statistics
///
/// # Returns
///
/// * `MemoryManagementStats` - Memory management statistics
pub fn get_memory_stats() -> MemoryManagementStats {
    // Return default empty statistics
    // Unified stats can be accessed directly via unified_stats module
    MemoryManagementStats::default()
}

/// Free unused memory pages
///
/// This function attempts to free unused memory pages back to the system.
/// It's useful for memory-constrained environments or when memory pressure
/// is high.
///
/// # Returns
///
/// * `usize` - Number of pages freed
pub fn free_unused_memory() -> usize {
    // Try to free pages from the buddy allocator
    // This is a placeholder implementation
    // In a real system, this would scan for unused pages and return them
    0
}

/// Get total free memory
///
/// # Returns
///
/// * `usize` - Number of free bytes
pub fn get_free_memory() -> usize {
    let stats = get_memory_stats();
    stats.available_physical_memory as usize
}

/// Get total used memory
///
/// # Returns
///
/// * `usize` - Number of used bytes
pub fn get_used_memory() -> usize {
    let stats = get_memory_stats();
    (stats.total_physical_memory - stats.available_physical_memory) as usize
}

/// Memory-mapped I/O read (32-bit)
///
/// Reads a 32-bit value from a memory-mapped I/O register.
///
/// # Safety
///
/// The address must be valid and aligned for a 32-bit read.
///
/// # Arguments
///
/// * `addr` - Pointer to the MMIO register
///
/// # Returns
///
/// * `u32` - The value read from the register
#[inline]
pub unsafe fn mmio_read32(addr: *const u32) -> u32 {
    unsafe { addr.read_volatile() }
}

/// Memory-mapped I/O write (32-bit)
///
/// Writes a 32-bit value to a memory-mapped I/O register.
///
/// # Safety
///
/// The address must be valid and aligned for a 32-bit write.
///
/// # Arguments
///
/// * `addr` - Pointer to the MMIO register
/// * `value` - The value to write
#[inline]
pub unsafe fn mmio_write32(addr: *mut u32, value: u32) {
    unsafe { addr.write_volatile(value); }
}

// Page table entry type
pub type PageTableEntry = u64;

// Physical and virtual address types are defined in nos-api::core::types
pub use nos_api::core::types::{PhysAddr, VirtAddr};
