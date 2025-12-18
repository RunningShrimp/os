//! 统一内存管理模块
//! 
//! 本模块整合了内核中的混合分配器和nos-memory-management crate中的分配器实现，
//! 提供统一的内存管理接口。

// Re-export from nos-memory-management crate
pub use nos_memory_management as memory;

// Re-export commonly used types
pub use memory::{
    allocator::{buddy::OptimizedBuddyAllocator, slab::OptimizedSlabAllocator},
    traits::{UnifiedAllocator, AllocatorWithStats, AllocatorStats},
};

// Re-export kernel memory management
pub use crate::mm::{
    vm::{PageTable, map_pages, flags},
    phys::{init_memory, free_pages},
};

/// 统一内存分配器
pub struct UnifiedMemoryAllocator {
    buddy: memory::allocator::buddy::OptimizedBuddyAllocator,
    slab: memory::allocator::slab::OptimizedSlabAllocator,
    stats: AllocatorStats,
}

impl UnifiedMemoryAllocator {
    /// 创建新的统一内存分配器
    pub fn new() -> Self {
        Self {
            buddy: memory::allocator::buddy::OptimizedBuddyAllocator::new(),
            slab: memory::allocator::slab::OptimizedSlabAllocator::new(),
            stats: AllocatorStats::default(),
        }
    }
    
    /// 初始化内存分配器
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        // 初始化buddy分配器
        self.buddy.initialize()
            .map_err(|_| "Failed to initialize buddy allocator")?;
        
        // 初始化slab分配器
        self.slab.initialize()
            .map_err(|_| "Failed to initialize slab allocator")?;
        
        crate::println!("[memory] Unified allocator initialized");
        Ok(())
    }
    
    /// 分配内存
    pub fn allocate(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        // 根据大小选择分配器
        if layout.size() <= 2048 {
            // 小对象使用slab分配器
            let ptr = self.slab.allocate(layout);
            if !ptr.is_null() {
                self.stats.allocated += layout.size();
                return ptr;
            }
        }
        
        // 大对象使用buddy分配器
        let ptr = self.buddy.allocate(layout);
        if !ptr.is_null() {
            self.stats.allocated += layout.size();
        }
        ptr
    }
    
    /// 释放内存
    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: core::alloc::Layout) {
        // 根据大小选择释放策略
        if layout.size() <= 2048 {
            // 小对象使用slab分配器
            self.slab.deallocate(ptr, layout);
        } else {
            // 大对象使用buddy分配器
            self.buddy.deallocate(ptr, layout);
        }
        self.stats.freed += layout.size();
    }
    
    /// 获取分配器统计信息
    pub fn get_stats(&self) -> &AllocatorStats {
        &self.stats
    }
}

/// 全局统一内存分配器
static mut GLOBAL_ALLOCATOR: Option<UnifiedMemoryAllocator> = None;
static ALLOCATOR_LOCK: crate::sync::Mutex<()> = crate::sync::Mutex::new(());

/// 获取全局内存分配器
pub fn get_global_allocator() -> &'static mut UnifiedMemoryAllocator {
    unsafe {
        if GLOBAL_ALLOCATOR.is_none() {
            GLOBAL_ALLOCATOR = Some(UnifiedMemoryAllocator::new());
        }
        GLOBAL_ALLOCATOR.as_mut().unwrap()
    }
}

/// 初始化全局内存分配器
pub fn init_global_allocator() -> Result<(), &'static str> {
    let _lock = ALLOCATOR_LOCK.lock();
    let allocator = get_global_allocator();
    allocator.initialize()
}

/// 分配内存（全局接口）
pub fn kalloc(layout: core::alloc::Layout) -> *mut u8 {
    let allocator = get_global_allocator();
    allocator.allocate(layout)
}

/// 释放内存（全局接口）
pub unsafe fn kfree(ptr: *mut u8, layout: core::alloc::Layout) {
    let allocator = get_global_allocator();
    allocator.deallocate(ptr, layout);
}

/// 获取内存统计信息
pub fn get_memory_stats() -> AllocatorStats {
    let allocator = get_global_allocator();
    allocator.get_stats().clone()
}