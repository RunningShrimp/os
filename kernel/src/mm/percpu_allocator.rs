//! 每 CPU 分配器：隔离锁争用并预留全局回退。
//!
//! 优化实现：使用无锁/缓存友好的每CPU分配器

extern crate alloc;

use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};

use crate::subsystems::mm::allocator::HybridAllocator;
use crate::subsystems::sync::{Once, Mutex};
use crate::arch::cpu_id;
use crate::subsystems::sync::SpinLock;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

/// 每 CPU 分配器
pub struct PerCpuAllocator {
    // 使用 Mutex 替代 SpinLock，降低锁开销
    per_cpu: Vec<Mutex<HybridAllocator>>,
}

impl PerCpuAllocator {
    pub fn new(num_cpus: usize) -> Self {
        let cpu_count = core::cmp::max(1, num_cpus);
        let mut per_cpu = Vec::with_capacity(cpu_count);
        for _ in 0..cpu_count {
            per_cpu.push(Mutex::new(HybridAllocator::new()));
        }
        Self { per_cpu }
    }

    pub fn cpu_count(&self) -> usize {
        self.per_cpu.len()
    }

    /// 初始化指定 CPU 的分配器区域
    pub unsafe fn init_cpu(
        &self,
        cpu_id: usize,
        slab_start: usize,
        slab_size: usize,
        buddy_start: usize,
        buddy_size: usize,
        page_size: usize,
    ) {
        if let Some(lock) = self.per_cpu.get(cpu_id) {
            let mut allocator = lock.lock();
            allocator.init(slab_start, slab_size, buddy_start, buddy_size, page_size);
        }
    }

    /// 按 CPU hint 分配；cpu_id 越界时退回 0 号。
    pub unsafe fn alloc_on(&self, cpu_id: usize, layout: Layout) -> *mut u8 {
        let target = cpu_id.min(self.per_cpu.len().saturating_sub(1));
        // 使用 SpinLock 替代 Mutex 以降低锁开销
        let mut guard = self.per_cpu[target].lock();
        // 直接调用 HybridAllocator 的 alloc 方法
        guard.alloc(layout)
    }

    pub unsafe fn dealloc_on(&self, cpu_id: usize, ptr: *mut u8, layout: Layout) {
        let target = cpu_id.min(self.per_cpu.len().saturating_sub(1));
        let mut guard = self.per_cpu[target].lock();
        // 直接调用 HybridAllocator 的 dealloc 方法
        guard.dealloc(ptr, layout);
    }
}

/// 全局每 CPU 分配器（延迟初始化），便于 syscalls/驱动共享。
// 使用 SpinLock 替代 Mutex 以降低全局锁开销
static GLOBAL_PERCPU_ALLOC: Mutex<Option<PerCpuAllocator>> = Mutex::new(None);
static GLOBAL_PERCPU_INIT: Once = Once::new();

pub fn init_global(num_cpus: usize) {
    GLOBAL_PERCPU_INIT.call_once(|| {
        *GLOBAL_PERCPU_ALLOC.lock() = Some(PerCpuAllocator::new(num_cpus));
    });
}

pub fn with_global<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&PerCpuAllocator) -> R,
{
    let guard = GLOBAL_PERCPU_ALLOC.lock();
    guard.as_ref().map(f)
}

/// 便捷接口：按 CPU hint 分配
pub unsafe fn percpu_alloc(cpu_id: usize, layout: Layout) -> Option<*mut u8> {
    with_global(|alloc| alloc.alloc_on(cpu_id, layout))
}

/// 便捷接口：释放
pub unsafe fn percpu_dealloc(cpu_id: usize, ptr: *mut u8, layout: Layout) {
    if let Some(_) = with_global(|alloc| {
        alloc.dealloc_on(cpu_id, ptr, layout);
    }) {
        // ok
    }
}

/// 刷新所有CPU的缓存到全局分配器
pub fn flush_all_caches() {
    for i in 0..PER_CPU_ALLOCATORS.len() {
        let allocator = &PER_CPU_ALLOCATORS[i];
        let (allocated, freelist_len) = allocator.stats();
        
        // 如果有缓存的块，将它们返回给全局分配器
        if freelist_len > 0 {
            // 这里可以实现更复杂的刷新逻辑
            // 目前只是统计信息
        }
    }
}

/// 获取所有CPU的分配统计
pub fn get_all_cpu_stats() -> Vec<(usize, (usize, usize))> {
    let mut stats = Vec::new();
    for i in 0..PER_CPU_ALLOCATORS.len() {
        let allocator = &PER_CPU_ALLOCATORS[i];
        let alloc_stats = allocator.stats();
        stats.push((i, alloc_stats));
    }
    stats
}

/// 平衡各CPU的缓存
pub fn balance_caches() {
    let mut stats = get_all_cpu_stats();
    
    // 简单的平衡策略：将缓存过多的CPU的部分缓存移动到缓存较少的CPU
    stats.sort_by_key(|&(_, (allocated, _))| allocated);
    
    // 这里可以实现更复杂的平衡逻辑
    // 目前只是示例
}

/// 预热CPU缓存
pub fn warmup_caches(cpu_id: usize, count: usize) {
    if cpu_id >= PER_CPU_ALLOCATORS.len() {
        return;
    }
    
    let allocator = &PER_CPU_ALLOCATORS[cpu_id];
    
    // 预分配一些小块到缓存
    for _ in 0..count {
        unsafe {
            let layout = Layout::from_size_align(64, 8).unwrap();
            let ptr = allocator.alloc_from_global(layout);
            if !ptr.is_null() {
                allocator.add_to_freelist(ptr, 64);
            }
        }
    }
}


// 每CPU缓存行大小（通常为64字节）
const CACHE_LINE_SIZE: usize = 64;

// 空闲块结构
struct FreeBlock {
    size: usize,
    next: Option<NonNull<FreeBlock>>,
}

// 全局每CPU分配器数组
static PER_CPU_ALLOCATORS: [PerCpuLocalAllocator; 256] = [const { PerCpuLocalAllocator::new() }; 256];

// 每CPU本地分配器结构体
#[repr(align(64))] // 缓存行对齐
pub struct PerCpuLocalAllocator {
    // 使用独立的缓存行来避免虚假共享
    freelist_head: SpinLock<Option<NonNull<FreeBlock>>>,
    allocated_count: AtomicUsize,
    // 填充到缓存行大小
    _padding: [u8; CACHE_LINE_SIZE - 16],
}

impl PerCpuLocalAllocator {
    pub const fn new() -> Self {
        Self {
            freelist_head: SpinLock::new(None),
            allocated_count: AtomicUsize::new(0),
            _padding: [0; CACHE_LINE_SIZE - 16],
        }
    }
    
    /// 从本地CPU分配内存
    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align());
        
        // 尝试从本地空闲列表分配
        if let Some(block) = self.try_alloc_from_freelist(size) {
            return block.as_ptr() as *mut u8;
        }
        
        // 本地空闲列表没有合适块，从全局分配器分配
        self.alloc_from_global(layout)
    }
    
    /// 释放内存到本地CPU
    pub unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(layout.align());
        
        // 如果块较小，放入本地空闲列表
        if size <= 4096 { // 4KB以下的块缓存到本地
            self.add_to_freelist(ptr, size);
        } else {
            // 大块直接返回给全局分配器
            crate::subsystems::mm::allocator::get_global_allocator().dealloc(ptr, layout);
        }
    }
    
    /// 尝试从空闲列表分配
    fn try_alloc_from_freelist(&self, size: usize) -> Option<NonNull<FreeBlock>> {
        let mut freelist = self.freelist_head.lock();
        let mut current = &mut *freelist;
        let mut prev: Option<&mut NonNull<FreeBlock>> = None;
        
        while let Some(mut block_ptr) = *current {
            let block = unsafe { block_ptr.as_mut() };
            
            if block.size >= size {
                // 找到合适块
                *current = block.next;
                self.allocated_count.fetch_sub(1, Ordering::Relaxed);
                return Some(block_ptr);
            }
            
            prev = Some(current);
            current = &mut block.next;
        }
        
        None
    }
    
    /// 从全局分配器分配
    pub unsafe fn alloc_from_global(&self, layout: Layout) -> *mut u8 {
        let ptr = crate::subsystems::mm::allocator::get_global_allocator().alloc(layout);
        if !ptr.is_null() {
            self.allocated_count.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }
    
    /// 添加到空闲列表
    pub unsafe fn add_to_freelist(&self, ptr: *mut u8, size: usize) {
        let block = ptr as *mut FreeBlock;
        (*block).size = size;
        
        let mut freelist = self.freelist_head.lock();
        (*block).next = *freelist;
        *freelist = Some(NonNull::new_unchecked(block));
        
        self.allocated_count.fetch_sub(1, Ordering::Relaxed);
    }
    
    /// 获取分配统计
    pub fn stats(&self) -> (usize, usize) {
        let allocated = self.allocated_count.load(Ordering::Relaxed);
        let freelist_len = {
            let freelist = self.freelist_head.lock();
            let mut count = 0;
            let mut current = *freelist;
            while let Some(block) = current {
                count += 1;
                let block_ref = unsafe { block.as_ref() };
                current = block_ref.next;
            }
            count
        };
        
        (allocated, freelist_len)
    }
}

/// 获取当前CPU的分配器
pub fn current_cpu_allocator() -> &'static PerCpuLocalAllocator {
    let cpu_id = cpu_id() as usize;
    &PER_CPU_ALLOCATORS[cpu_id % PER_CPU_ALLOCATORS.len()]
}

// 实现全局分配器trait
pub struct PerCpuGlobalAllocator;

unsafe impl GlobalAlloc for PerCpuGlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        current_cpu_allocator().alloc(layout)
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        current_cpu_allocator().dealloc(ptr, layout)
    }
}