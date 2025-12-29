//! 虚拟内存管理
//!
//! 提供虚拟内存管理的核心功能，包括：
//! - 内存映射 (mmap/munmap)
//! - 内存保护 (mprotect)
//! - 内存锁定 (mlock/munlock)
//! - 地址空间管理
//! - 页表操作
//!
//! # 模块结构
//!
//! - `mmap.rs`: 内存映射相关功能
//! - `protection.rs`: 内存保护相关功能
//! - `lock.rs`: 内存锁定相关功能

extern crate alloc;

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::subsystems::sync::Mutex;
use crate::subsystems::mm::types::*;
use crate::subsystems::mm::page_table_isolation::PageTable;

// 导出子模块
pub mod mmap;
pub mod protection;
pub mod lock;
pub mod arch;

// 重新导出常用类型和函数
pub use mmap::*;
pub use protection::*;
pub use lock::*;

/// 虚拟内存区域
#[derive(Debug, Clone)]
pub struct VmRegion {
    /// 起始虚拟地址
    pub start: VirtAddr,
    /// 结束虚拟地址
    pub end: VirtAddr,
    /// 物理页帧
    pub frames: Vec<PhysFrame>,
    /// 保护标志
    pub flags: MapFlags,
    /// 偏移量（用于文件映射）
    pub offset: usize,
    /// 区域类型
    pub region_type: VmRegionType,
}

/// 虚拟内存区域类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmRegionType {
    /// 匿名映射
    Anonymous,
    /// 文件映射
    File,
    /// 栈
    Stack,
    /// 堆
    Heap,
    /// 代码段
    Code,
    /// 数据段
    Data,
}

/// 虚拟地址空间
#[derive(Debug)]
pub struct VmSpace {
    /// 页表
    pub page_table: Arc<PageTable>,
    /// 内存区域
    pub regions: Mutex<Vec<VmRegion>>,
    /// 可用的起始地址
    pub start_addr: VirtAddr,
    /// 可用的结束地址
    pub end_addr: VirtAddr,
    /// 最后使用的地址（用于分配）
    pub last_addr: AtomicUsize,
}

impl VmSpace {
    /// 创建新的虚拟地址空间
    pub fn new(start_addr: VirtAddr, end_addr: VirtAddr) -> Self {
        Self {
            page_table: Arc::new(PageTable::new()),
            regions: Mutex::new(Vec::new()),
            start_addr,
            end_addr,
            last_addr: AtomicUsize::new(start_addr),
        }
    }

    /// 添加内存区域
    pub fn add_region(&self, region: VmRegion) -> Result<(), MemoryError> {
        let mut regions = self.regions.lock();

        // 检查是否与现有区域重叠
        for existing in regions.iter() {
            if region.start < existing.end && region.end > existing.start {
                return Err(MemoryError::Overlap);
            }
        }

        regions.push(region);
        Ok(())
    }

    /// 查找包含指定地址的区域
    pub fn find_region(&self, addr: VirtAddr) -> Option<VmRegion> {
        let regions = self.regions.lock();
        for region in regions.iter() {
            if addr >= region.start && addr < region.end {
                return Some(region.clone());
            }
        }
        None
    }

    /// 移除内存区域
    pub fn remove_region(&self, start: VirtAddr) -> Result<VmRegion, MemoryError> {
        let mut regions = self.regions.lock();
        let pos = regions.iter().position(|r| r.start == start)
            .ok_or(MemoryError::InvalidAddress)?;

        Ok(regions.remove(pos))
    }

    /// 分配虚拟地址空间
    pub fn allocate(&self, size: usize) -> Result<VirtAddr, MemoryError> {
        let last = self.last_addr.load(Ordering::Acquire);
        let addr = (last + size - 1) / PAGE_SIZE * PAGE_SIZE; // 对齐到页边界

        if addr + size > self.end_addr {
            return Err(MemoryError::OutOfMemory);
        }

        self.last_addr.store(addr + size, Ordering::Release);
        Ok(addr)
    }

    /// 映射物理页到虚拟地址
    pub fn map(&self, virt: VirtAddr, phys: PhysFrame, flags: MapFlags) -> Result<(), MemoryError> {
        self.page_table.map(virt, phys, flags)
    }

    /// 取消映射虚拟地址
    pub fn unmap(&self, virt: VirtAddr) -> Result<(), MemoryError> {
        self.page_table.unmap(virt)
    }

    /// 更改内存保护属性
    pub fn protect(&self, virt: VirtAddr, size: usize, flags: MapFlags) -> Result<(), MemoryError> {
        self.page_table.protect(virt, size, flags)
    }
}

/// 虚拟内存管理器
pub struct VmManager {
    /// 所有虚拟地址空间
    pub spaces: Mutex<BTreeMap<usize, Arc<VmSpace>>>,
    /// 当前地址空间ID
    pub current_space_id: AtomicUsize,
    /// 下一个可用的地址空间ID
    pub next_space_id: AtomicUsize,
}

impl VmManager {
    /// 创建新的虚拟内存管理器
    pub fn new() -> Self {
        Self {
            spaces: Mutex::new(BTreeMap::new()),
            current_space_id: AtomicUsize::new(0),
            next_space_id: AtomicUsize::new(1),
        }
    }

    /// 创建新的地址空间
    pub fn create_space(&self, start_addr: VirtAddr, end_addr: VirtAddr) -> Result<usize, MemoryError> {
        let space_id = self.next_space_id.fetch_add(1, Ordering::AcqRel);
        let space = Arc::new(VmSpace::new(start_addr, end_addr));

        let mut spaces = self.spaces.lock();
        spaces.insert(space_id, space);

        Ok(space_id)
    }

    /// 销毁地址空间
    pub fn destroy_space(&self, space_id: usize) -> Result<(), MemoryError> {
        let mut spaces = self.spaces.lock();
        spaces.remove(&space_id).ok_or(MemoryError::InvalidAddress)?;
        Ok(())
    }

    /// 获取地址空间
    pub fn get_space(&self, space_id: usize) -> Result<Arc<VmSpace>, MemoryError> {
        let spaces = self.spaces.lock();
        spaces.get(&space_id).cloned().ok_or(MemoryError::InvalidAddress)
    }

    /// 切换当前地址空间
    pub fn switch_space(&self, space_id: usize) -> Result<(), MemoryError> {
        let space = self.get_space(space_id)?;

        // 加载页表
        space.page_table.load();

        // 更新当前地址空间ID
        self.current_space_id.store(space_id, Ordering::Release);

        Ok(())
    }

    /// 获取当前地址空间
    pub fn current_space(&self) -> Result<Arc<VmSpace>, MemoryError> {
        self.get_space(self.current_space_id.load(Ordering::Acquire))
    }
}

impl Default for VmManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局虚拟内存管理器
static VM_MANAGER: Mutex<VmManager> = Mutex::new(VmManager::new());

/// 获取全局虚拟内存管理器
pub fn vm_manager() -> &'static Mutex<VmManager> {
    &VM_MANAGER
}

/// 初始化虚拟内存管理器
pub fn init() {
    crate::println!("vm: initializing virtual memory manager");

    let manager = vm_manager().lock();

    crate::println!("vm: virtual memory manager initialized");
}

// ============================================================================
// 错误类型
// ============================================================================

/// 虚拟内存错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmError {
    /// 无效的地址
    InvalidAddress,
    /// 内存不足
    OutOfMemory,
    /// 权限错误
    PermissionDenied,
    /// 区域重叠
    Overlap,
    /// 未对齐
    Alignment,
    /// 其他错误
    Other,
}

impl core::fmt::Display for VmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VmError::InvalidAddress => write!(f, "Invalid address"),
            VmError::OutOfMemory => write!(f, "Out of memory"),
            VmError::PermissionDenied => write!(f, "Permission denied"),
            VmError::Overlap => write!(f, "Memory region overlap"),
            VmError::Alignment => write!(f, "Memory not aligned"),
            VmError::Other => write!(f, "Unknown error"),
        }
    }
}

impl core::error::Error for VmError {}

// ============================================================================
// Missing constants and functions for compatibility
// ============================================================================

/// Page size
pub const PAGE_SIZE: usize = 4096;

/// Page table entry count (stub)
pub const PTE_COUNT: usize = 512;

/// Virtual memory area (stub)
#[derive(Debug, Clone)]
pub struct VmArea {
    pub start: usize,
    pub end: usize,
    pub flags: usize,
}

/// Virtual memory permissions (stub)
#[derive(Debug, Clone, Copy)]
pub struct VmPerm {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

/// Memory flags (stub)
pub mod flags {
    /// Read permission
    pub const READ: usize = 1;
    /// Write permission
    pub const WRITE: usize = 2;
    /// Execute permission
    pub const EXEC: usize = 4;
    /// User accessible
    pub const USER: usize = 8;
}

/// Copy page table (stub)
pub fn copy_pagetable() -> Result<(), crate::subsystems::mm::MemoryError> {
    Ok(())
}

/// Copy from kernel to user (stub)
pub fn copyin(dst: *mut u8, src: &[u8], len: usize) -> Result<(), crate::subsystems::mm::MemoryError> {
    // Stub implementation
    Ok(())
}

/// Copy string from kernel to user (stub)
pub fn copyinstr(dst: *mut u8, src: &[u8], maxlen: usize) -> Result<(usize, bool), crate::subsystems::mm::MemoryError> {
    // Stub implementation
    Ok((0, false))
}

/// Copy from user to kernel (stub)
pub fn copyout(dst: &mut [u8], src: *const u8, len: usize) -> Result<(), crate::subsystems::mm::MemoryError> {
    // Stub implementation
    Ok(())
}

/// Activate virtual memory (stub)
pub fn activate() {
    // Stub implementation
}

/// Free page table (stub)
pub fn free_pagetable() {
    // Stub implementation
}

/// Map pages (stub)
pub fn map_pages(start: usize, size: usize, flags: usize) -> Result<(), crate::subsystems::mm::MemoryError> {
    // Stub implementation
    Ok(())
}

/// Flush TLB page (stub)
pub fn flush_tlb_page(addr: usize) {
    // Stub implementation
}
