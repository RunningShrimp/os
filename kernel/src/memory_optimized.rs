//! 优化的内存分配器
//!
//! 本模块实现了一个高性能的内存分配器，专门设计来减少内存碎片化，
//! 提高分配和释放效率，并提供详细的性能统计。

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::alloc::Layout;
use core::ptr::NonNull;

use crate::sync::Mutex;
use crate::mm::PAGE_SIZE;

// ============================================================================
// 常量定义
// ============================================================================

/// 小对象阈值（使用slab分配器）
pub const SMALL_OBJECT_THRESHOLD: usize = 2048;

/// 中等对象阈值（使用中等大小的buddy分配器）
pub const MEDIUM_OBJECT_THRESHOLD: usize = 32768;

/// 大对象阈值（使用大对象buddy分配器）
pub const LARGE_OBJECT_THRESHOLD: usize = 1048576; // 1MB

/// 内存池大小
pub const MEMORY_POOL_SIZE: usize = 64 * 1024 * 1024; // 64MB

/// 最小分配大小
pub const MIN_ALLOC_SIZE: usize = 8;

/// 最大分配大小
pub const MAX_ALLOC_SIZE: usize = 16 * 1024 * 1024; // 16MB

/// 对齐大小
pub const ALIGNMENT: usize = 8;

// ============================================================================
// 内存块类型
// ============================================================================

/// 内存块类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Free,
    Small,      // 小对象（<= 2KB）
    Medium,     // 中等对象（<= 32KB）
    Large,      // 大对象（<= 1MB）
    Huge,       // 巨大对象（> 1MB）
}

/// 内存块状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockState {
    Free,
    Allocated,
    Split,
    Merged,
}

// ============================================================================
// 内存块结构
// ============================================================================

/// 内存块头信息
#[derive(Debug)]
#[repr(C)]
pub struct BlockHeader {
    /// 块大小（包括头部）
    pub size: usize,
    /// 块类型
    pub block_type: BlockType,
    /// 块状态
    pub state: BlockState,
    /// 前一个块（用于合并空闲块）
    pub prev: Option<NonNull<BlockHeader>>,
    /// 后一个块（用于合并空闲块）
    pub next: Option<NonNull<BlockHeader>>,
    /// 分配时间戳（用于调试）
    pub alloc_timestamp: u64,
    /// 分配进程ID（用于调试）
    pub alloc_pid: u32,
}

impl BlockHeader {
    /// 创建新的块头
    pub fn new(size: usize, block_type: BlockType) -> Self {
        Self {
            size,
            block_type,
            state: BlockState::Free,
            prev: None,
            next: None,
            alloc_timestamp: 0,
            alloc_pid: 0,
        }
    }
    
    /// 获取数据部分指针
    pub fn data_ptr(&self) -> *mut u8 {
        unsafe {
            (self as *const BlockHeader as *mut u8).add(core::mem::size_of::<BlockHeader>())
        }
    }
    
    /// 从数据指针获取块头
    pub fn from_data_ptr(ptr: *mut u8) -> *mut BlockHeader {
        unsafe {
            (ptr as *mut BlockHeader).sub(core::mem::size_of::<BlockHeader>())
        }
    }
    
    /// 检查块是否空闲
    pub fn is_free(&self) -> bool {
        self.state == BlockState::Free
    }
    
    /// 标记为已分配
    pub fn mark_allocated(&mut self, pid: u32) {
        self.state = BlockState::Allocated;
        self.alloc_timestamp = crate::time::get_timestamp();
        self.alloc_pid = pid;
    }
    
    /// 标记为空闲
    pub fn mark_free(&mut self) {
        self.state = BlockState::Free;
        self.alloc_timestamp = 0;
        self.alloc_pid = 0;
    }
    
    /// 获取对齐后的大小
    pub fn aligned_size(&self) -> usize {
        (self.size + ALIGNMENT - 1) & !(ALIGNMENT - 1)
    }
}

// ============================================================================
// 内存池
// ============================================================================

/// 内存池
pub struct MemoryPool {
    /// 池起始地址
    pub start_addr: usize,
    /// 池大小
    pub size: usize,
    /// 空闲块列表
    pub free_blocks: Mutex<Vec<NonNull<BlockHeader>>>,
    /// 已分配块列表（用于调试）
    pub allocated_blocks: Mutex<Vec<NonNull<BlockHeader>>>,
    /// 分配统计
    pub alloc_count: AtomicUsize,
    /// 释放统计
    pub free_count: AtomicUsize,
    /// 当前使用量
    pub used_bytes: AtomicUsize,
    /// 峰值使用量
    pub peak_used_bytes: AtomicUsize,
    /// 碎片化统计
    pub fragmentation_count: AtomicUsize,
}

impl MemoryPool {
    /// 创建新的内存池
    pub fn new(size: usize) -> Self {
        // 分配内存池
        let pool_memory = unsafe {
            let layout = Layout::from_size_align(size, PAGE_SIZE).unwrap();
            alloc::alloc::alloc(layout)
        };
        
        let pool_addr = pool_memory.as_ptr() as usize;
        
        Self {
            start_addr: pool_addr,
            size,
            free_blocks: Mutex::new(Vec::new()),
            allocated_blocks: Mutex::new(Vec::new()),
            alloc_count: AtomicUsize::new(0),
            free_count: AtomicUsize::new(0),
            used_bytes: AtomicUsize::new(0),
            peak_used_bytes: AtomicUsize::new(0),
            fragmentation_count: AtomicUsize::new(0),
        }
    }
    
    /// 初始化内存池
    pub fn initialize(&mut self) {
        // 创建初始空闲块
        let initial_block = unsafe {
            let block_ptr = self.start_addr as *mut BlockHeader;
            (*block_ptr) = BlockHeader::new(self.size, BlockType::Free);
            NonNull::new_unchecked(block_ptr)
        };
        
        let mut free_blocks = self.free_blocks.lock();
        free_blocks.push(initial_block);
        drop(free_blocks);
        
        crate::println!("[memory_pool] Initialized with {} bytes", self.size);
    }
    
    /// 分配内存
    pub fn allocate(&self, size: usize) -> *mut u8 {
        // 对齐大小
        let aligned_size = (size + ALIGNMENT - 1) & !(ALIGNMENT - 1);
        
        // 检查最小和最大大小
        if aligned_size < MIN_ALLOC_SIZE || aligned_size > MAX_ALLOC_SIZE {
            return core::ptr::null_mut();
        }
        
        let mut free_blocks = self.free_blocks.lock();
        
        // 查找合适的空闲块
        for (i, &block_header) in free_blocks.iter().enumerate() {
            let block = unsafe { block_header.as_ref() };
            
            if block.is_free() && block.size >= aligned_size {
                // 检查是否需要分割块
                if block.size > aligned_size + core::mem::size_of::<BlockHeader>() + MIN_ALLOC_SIZE {
                    // 分割块
                    let remaining_size = block.size - aligned_size;
                    let new_block_addr = block.data_ptr() as usize + aligned_size;
                    let new_block = unsafe {
                        let block_ptr = new_block_addr as *mut BlockHeader;
                        (*block_ptr) = BlockHeader::new(remaining_size, BlockType::Free);
                        NonNull::new_unchecked(block_ptr)
                    };
                    
                    // 更新原块
                    let mut block = unsafe { &mut *(block_header.as_ptr() as *mut BlockHeader) };
                    block.size = aligned_size;
                    block.mark_allocated(0); // 使用默认PID
                    
                    // 添加新块到空闲列表
                    free_blocks.push(new_block);
                    
                    // 更新统计
                    self.alloc_count.fetch_add(1, Ordering::SeqCst);
                    let current_used = self.used_bytes.fetch_add(aligned_size, Ordering::SeqCst) + aligned_size;
                    let peak_used = self.peak_used_bytes.load(Ordering::SeqCst);
                    if current_used > peak_used {
                        self.peak_used_bytes.store(current_used, Ordering::SeqCst);
                    }
                    
                    drop(free_blocks);
                    return block.data_ptr();
                } else {
                    // 直接分配整个块
                    let mut block = unsafe { &mut *(block_header.as_ptr() as *mut BlockHeader) };
                    block.mark_allocated(0); // 使用默认PID
                    
                    // 从空闲列表移除
                    free_blocks.remove(i);
                    
                    // 更新统计
                    self.alloc_count.fetch_add(1, Ordering::SeqCst);
                    let current_used = self.used_bytes.fetch_add(aligned_size, Ordering::SeqCst) + aligned_size;
                    let peak_used = self.peak_used_bytes.load(Ordering::SeqCst);
                    if current_used > peak_used {
                        self.peak_used_bytes.store(current_used, Ordering::SeqCst);
                    }
                    
                    drop(free_blocks);
                    return block.data_ptr();
                }
            }
        }
        
        drop(free_blocks);
        core::ptr::null_mut()
    }
    
    /// 释放内存
    pub unsafe fn deallocate(&self, ptr: *mut u8, _size: usize) {
        if ptr.is_null() {
            return;
        }
        
        // 获取块头
        let block_header = BlockHeader::from_data_ptr(ptr);
        let block = &mut *block_header;
        
        // 检查是否已分配
        if !block.is_free() {
            // 标记为空闲
            block.mark_free();
            
            // 尝试与相邻块合并
            self.try_merge(block_header);
            
            // 添加到空闲列表
            let mut free_blocks = self.free_blocks.lock();
            free_blocks.push(NonNull::new_unchecked(block_header));
            drop(free_blocks);
            
            // 更新统计
            self.free_count.fetch_add(1, Ordering::SeqCst);
            self.used_bytes.fetch_sub(block.aligned_size(), Ordering::SeqCst);
            
            // 从已分配列表移除
            let mut allocated_blocks = self.allocated_blocks.lock();
            allocated_blocks.retain(|&b| b.as_ptr() != block_header);
            drop(allocated_blocks);
        }
    }
    
    /// 尝试合并相邻的空闲块
    unsafe fn try_merge(&self, block: *mut BlockHeader) {
        let current_block = &mut *block;
        
        // 检查前一个块
        if let Some(prev) = current_block.prev {
            let prev_block = &mut *prev.as_ptr();
            if prev_block.is_free() {
                // 合并前一个块
                let merged_size = current_block.size + prev_block.size;
                let merged_block = prev_block;
                merged_block.size = merged_size;
                
                // 从空闲列表移除前一个块
                let mut free_blocks = self.free_blocks.lock();
                free_blocks.retain(|&b| b.as_ptr() != prev.as_ptr());
                drop(free_blocks);
                
                // 更新碎片化统计
                if current_block.size < SMALL_OBJECT_THRESHOLD || prev_block.size < SMALL_OBJECT_THRESHOLD {
                    self.fragmentation_count.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
        
        // 检查后一个块
        if let Some(next) = current_block.next {
            let next_block = &mut *next.as_ptr();
            if next_block.is_free() {
                // 合并后一个块
                let merged_size = current_block.size + next_block.size;
                let merged_block = current_block;
                merged_block.size = merged_size;
                
                // 从空闲列表移除后一个块
                let mut free_blocks = self.free_blocks.lock();
                free_blocks.retain(|&b| b.as_ptr() != next.as_ptr());
                drop(free_blocks);
                
                // 更新碎片化统计
                if current_block.size < SMALL_OBJECT_THRESHOLD || next_block.size < SMALL_OBJECT_THRESHOLD {
                    self.fragmentation_count.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }
    
    /// 获取内存池统计信息
    pub fn get_stats(&self) -> MemoryPoolStats {
        MemoryPoolStats {
            total_size: self.size,
            used_bytes: self.used_bytes.load(Ordering::SeqCst),
            peak_used_bytes: self.peak_used_bytes.load(Ordering::SeqCst),
            alloc_count: self.alloc_count.load(Ordering::SeqCst),
            free_count: self.free_count.load(Ordering::SeqCst),
            fragmentation_count: self.fragmentation_count.load(Ordering::SeqCst),
            free_blocks: self.free_blocks.lock().len(),
            allocated_blocks: self.allocated_blocks.lock().len(),
        }
    }
    
    /// 获取内存池详细信息
    pub fn get_pool_info(&self) -> PoolInfo {
        let free_blocks_len = self.free_blocks.lock().len();
        let allocated_blocks_len = self.allocated_blocks.lock().len();
        let total_blocks = free_blocks_len + allocated_blocks_len;
        
        let used_bytes = self.used_bytes.load(Ordering::SeqCst);
        let free_bytes = self.size - used_bytes;
        
        // 计算碎片化比率
        let fragmentation_ratio = if total_blocks > 0 {
            // 简化的碎片化计算：基于空闲块数量与总块数的比例
            (free_blocks_len as f32) / (total_blocks as f32)
        } else {
            0.0
        };
        
        PoolInfo {
            total_blocks,
            free_blocks: free_blocks_len,
            allocated_blocks: allocated_blocks_len,
            total_bytes: self.size,
            free_bytes,
            allocated_bytes: used_bytes,
            fragmentation_ratio,
        }
    }
    
    /// 执行碎片整理
    pub fn defragment(&self) {
        crate::println!("[memory_pool] Starting defragmentation");
        
        let mut free_blocks = self.free_blocks.lock();
        
        // 按地址排序空闲块
        free_blocks.sort_by(|a, b| {
            let block_a = unsafe { a.as_ref() };
            let block_b = unsafe { b.as_ref() };
            block_a.data_ptr() as usize - block_b.data_ptr() as usize
        });
        
        // 合并相邻的空闲块
        let mut i = 0;
        while i < free_blocks.len() - 1 {
            let current_block = unsafe { &mut *free_blocks[i].as_ptr() };
            let next_block = unsafe { &mut *free_blocks[i + 1].as_ptr() };
            
            // 检查是否相邻
            let current_end = current_block.data_ptr() as usize + current_block.size;
            let next_start = next_block.data_ptr() as usize;
            
            if current_end == next_start {
                // 合并块
                let merged_size = current_block.size + next_block.size;
                current_block.size = merged_size;
                current_block.mark_free();
                
                // 移除下一个块
                free_blocks.remove(i + 1);
                
                // 更新碎片化统计
                if current_block.size < SMALL_OBJECT_THRESHOLD || next_block.size < SMALL_OBJECT_THRESHOLD {
                    self.fragmentation_count.fetch_add(1, Ordering::SeqCst);
                }
            } else {
                i += 1;
            }
        }
        
        drop(free_blocks);
        crate::println!("[memory_pool] Defragmentation completed");
    }
    
    /// 尝试重新分配内存
    pub fn try_reallocate(&self, ptr: *mut u8, new_size: usize) -> Option<*mut u8> {
        if ptr.is_null() {
            return Some(self.allocate(new_size));
        }
        
        // 获取块头
        let block_header = BlockHeader::from_data_ptr(ptr);
        let block = unsafe { block_header.as_ref() };
        
        // 检查是否已分配
        if block.is_free() {
            return None;
        }
        
        // 如果新大小小于等于当前大小，直接返回原指针
        if new_size <= block.size {
            return Some(ptr);
        }
        
        // 检查是否可以扩展
        let block_end = block_header as usize + block.size;
        let mut free_blocks = self.free_blocks.lock();
        
        // 查找相邻的空闲块
        for (i, &free_block) in free_blocks.iter().enumerate() {
            let free_block_ptr = free_block.as_ptr();
            
            // 检查是否紧邻在当前块之后
            if free_block_ptr as usize == block_end {
                let free_block = unsafe { free_block.as_ref() };
                let total_size = block.size + free_block.size;
                
                // 如果合并后的大小足够，执行合并
                if total_size >= new_size {
                    // 移除空闲块
                    free_blocks.remove(i);
                    drop(free_blocks);
                    
                    // 扩展当前块
                    let mut block = unsafe { &mut *block_header };
                    block.size = total_size;
                    
                    return Some(ptr);
                }
                break;
            }
        }
        
        None
    }
}

// ============================================================================
// 内存池统计信息
// ============================================================================

/// 内存池统计信息
#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    /// 总大小
    pub total_size: usize,
    /// 已使用字节数
    pub used_bytes: usize,
    /// 峰值使用字节数
    pub peak_used_bytes: usize,
    /// 分配次数
    pub alloc_count: usize,
    /// 释放次数
    pub free_count: usize,
    /// 碎片化次数
    pub fragmentation_count: usize,
    /// 空闲块数量
    pub free_blocks: usize,
    /// 已分配块数量
    pub allocated_blocks: usize,
}

impl MemoryPoolStats {
    /// 获取内存使用率
    pub fn usage_ratio(&self) -> f64 {
        if self.total_size == 0 {
            0.0
        } else {
            self.used_bytes as f64 / self.total_size as f64
        }
    }
    
    /// 获取碎片化率
    pub fn fragmentation_ratio(&self) -> f64 {
        if self.alloc_count == 0 {
            0.0
        } else {
            self.fragmentation_count as f64 / self.alloc_count as f64
        }
    }
}

// ============================================================================
// 优化的内存分配器
// ============================================================================

/// 优化的内存分配器
pub struct OptimizedMemoryAllocator {
    /// 小对象内存池
    small_pool: MemoryPool,
    /// 中等对象内存池
    medium_pool: MemoryPool,
    /// 大对象内存池
    large_pool: MemoryPool,
    /// 分配统计
    pub stats: Mutex<AllocatorStats>,
}

/// 分配器统计信息
#[derive(Debug, Default, Clone)]
pub struct AllocatorStats {
    /// 总分配次数
    pub total_allocations: AtomicU64,
    /// 总释放次数
    pub total_deallocations: AtomicU64,
    /// 总分配字节数
    pub total_allocated_bytes: AtomicU64,
    /// 总释放字节数
    pub total_freed_bytes: AtomicU64,
    /// 当前分配字节数
    pub current_allocated_bytes: AtomicU64,
    /// 峰值分配字节数
    pub peak_allocated_bytes: AtomicU64,
    /// 失败的分配次数
    pub failed_allocations: AtomicU64,
}

impl OptimizedMemoryAllocator {
    /// 创建新的优化内存分配器
    pub fn new() -> Self {
        // 创建不同大小的内存池
        let small_pool = MemoryPool::new(16 * 1024 * 1024);  // 16MB for small objects
        let medium_pool = MemoryPool::new(32 * 1024 * 1024); // 32MB for medium objects
        let large_pool = MemoryPool::new(16 * 1024 * 1024);  // 16MB for large objects
        
        Self {
            small_pool,
            medium_pool,
            large_pool,
            stats: Mutex::new(AllocatorStats::default()),
        }
    }
    
    /// 初始化分配器
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        // 初始化所有内存池
        self.small_pool.initialize();
        self.medium_pool.initialize();
        self.large_pool.initialize();
        
        crate::println!("[optimized_allocator] Initialized with tiered memory pools");
        Ok(())
    }
    
    /// 分配内存
    pub fn allocate(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        
        // 更新统计
        self.stats.lock().total_allocations.fetch_add(1, Ordering::SeqCst);
        
        // 根据大小选择合适的内存池
        let ptr = if size <= SMALL_OBJECT_THRESHOLD {
            self.small_pool.allocate(size)
        } else if size <= MEDIUM_OBJECT_THRESHOLD {
            self.medium_pool.allocate(size)
        } else if size <= LARGE_OBJECT_THRESHOLD {
            self.large_pool.allocate(size)
        } else {
            // 超大对象，直接从系统分配
            unsafe {
                let ptr = alloc::alloc::alloc(layout);
                if !ptr.is_null() {
                    let mut stats = self.stats.lock();
                    stats.total_allocated_bytes.fetch_add(size as u64, Ordering::SeqCst);
                    stats.current_allocated_bytes.fetch_add(size as u64, Ordering::SeqCst);
                    
                    let current = stats.current_allocated_bytes.load(Ordering::SeqCst);
                    let peak = stats.peak_allocated_bytes.load(Ordering::SeqCst);
                    if current > peak {
                        stats.peak_allocated_bytes.store(current, Ordering::SeqCst);
                    }
                }
                ptr
            }
        };
        
        if ptr.is_null() {
            self.stats.lock().failed_allocations.fetch_add(1, Ordering::SeqCst);
        }
        
        ptr
    }
    
    /// 释放内存
    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }
        
        let size = layout.size();
        
        // 更新统计
        self.stats.lock().total_deallocations.fetch_add(1, Ordering::SeqCst);
        self.stats.lock().total_freed_bytes.fetch_add(size as u64, Ordering::SeqCst);
        self.stats.lock().current_allocated_bytes.fetch_sub(size as u64, Ordering::SeqCst);
        
        // 根据大小选择合适的内存池
        if size <= SMALL_OBJECT_THRESHOLD {
            self.small_pool.deallocate(ptr, 0);
        } else if size <= MEDIUM_OBJECT_THRESHOLD {
            self.medium_pool.deallocate(ptr, 0);
        } else if size <= LARGE_OBJECT_THRESHOLD {
            self.large_pool.deallocate(ptr, 0);
        } else {
            // 超大对象，直接释放到系统
            alloc::alloc::dealloc(ptr, layout);
        }
    }
    
    /// 获取分配器统计信息
    pub fn get_stats(&self) -> AllocatorStats {
        self.stats.lock().clone()
    }
    
    /// 获取内存池统计信息
    pub fn get_pool_stats(&self) -> (MemoryPoolStats, MemoryPoolStats, MemoryPoolStats) {
        (
            self.small_pool.get_stats(),
            self.medium_pool.get_stats(),
            self.large_pool.get_stats(),
        )
    }
    
    /// 获取所有内存池的统计信息
    pub fn get_all_pool_stats(&self) -> (MemoryPoolStats, MemoryPoolStats, MemoryPoolStats) {
        (
            self.small_pool.get_stats(),
            self.medium_pool.get_stats(),
            self.large_pool.get_stats(),
        )
    }
    
    /// 获取内存池详细信息
    pub fn get_pool_info(&self, pool_type: u32) -> PoolInfo {
        match pool_type {
            0 => self.small_pool.get_pool_info(),
            1 => self.medium_pool.get_pool_info(),
            2 => self.large_pool.get_pool_info(),
            3 => {
                // 返回所有池的汇总信息
                let small_info = self.small_pool.get_pool_info();
                let medium_info = self.medium_pool.get_pool_info();
                let large_info = self.large_pool.get_pool_info();
                
                PoolInfo {
                    total_blocks: small_info.total_blocks + medium_info.total_blocks + large_info.total_blocks,
                    free_blocks: small_info.free_blocks + medium_info.free_blocks + large_info.free_blocks,
                    allocated_blocks: small_info.allocated_blocks + medium_info.allocated_blocks + large_info.allocated_blocks,
                    total_bytes: small_info.total_bytes + medium_info.total_bytes + large_info.total_bytes,
                    free_bytes: small_info.free_bytes + medium_info.free_bytes + large_info.free_bytes,
                    allocated_bytes: small_info.allocated_bytes + medium_info.allocated_bytes + large_info.allocated_bytes,
                    fragmentation_ratio: {
                        let total_frag = small_info.fragmentation_ratio + medium_info.fragmentation_ratio + large_info.fragmentation_ratio;
                        if total_frag > 0.0 { total_frag / 3.0 } else { 0.0 }
                    },
                }
            },
            _ => PoolInfo::default(),
        }
    }
    
    /// 执行碎片整理
    pub fn defragment(&mut self) {
        crate::println!("[optimized_allocator] Starting global defragmentation");
        
        self.small_pool.defragment();
        self.medium_pool.defragment();
        self.large_pool.defragment();
        
        crate::println!("[optimized_allocator] Global defragmentation completed");
    }
    
    /// 执行所有内存池的碎片整理
    pub fn defragment_all_pools(&mut self) {
        self.defragment();
    }
    
    /// 执行特定进程的内存碎片整理
    pub fn defragment_process_memory(&mut self, _pid: u32) {
        // 在当前实现中，我们执行全局碎片整理
        // 在更复杂的实现中，可以只整理特定进程使用的内存
        self.defragment();
    }
    
    /// 重新分配内存
    pub fn reallocate(&mut self, ptr: *mut u8, new_layout: Layout) -> *mut u8 {
        let new_size = new_layout.size();
        
        // 如果指针为空，直接分配新内存
        if ptr.is_null() {
            return self.allocate(new_layout);
        }
        
        // 如果新大小为0，释放内存并返回空指针
        if new_size == 0 {
            // 这里需要原始大小，但我们没有保存，所以只能假设
            // 在实际实现中，应该有一个机制来跟踪原始大小
            unsafe {
                self.deallocate(ptr, new_layout);
            }
            return core::ptr::null_mut();
        }
        
        // 对于小对象，尝试在原块上扩展
        if new_size <= SMALL_OBJECT_THRESHOLD {
            if let Some(new_ptr) = self.small_pool.try_reallocate(ptr, new_size) {
                return new_ptr;
            }
        }
        
        // 对于无法在原块上扩展的情况，分配新内存并复制数据
        let new_ptr = self.allocate(new_layout);
        if !new_ptr.is_null() && !ptr.is_null() {
            // 复制数据，这里需要知道原始大小
            // 在实际实现中，应该有一个机制来跟踪原始大小
            // 这里我们假设原始大小和新大小中的较小值
            let copy_size = new_size; // 简化实现
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
                self.deallocate(ptr, new_layout);
            }
        }
        
        new_ptr
    }
    
    /// 获取内存使用情况
    pub fn get_memory_usage(&self) -> MemoryUsage {
        let small_stats = self.small_pool.get_stats();
        let medium_stats = self.medium_pool.get_stats();
        let large_stats = self.large_pool.get_stats();
        let stats = self.get_stats();
        
        let total_used = small_stats.used_bytes + medium_stats.used_bytes + large_stats.used_bytes;
        let total_size = small_stats.total_size + medium_stats.total_size + large_stats.total_size;
        
        MemoryUsage {
            total_size,
            used_bytes: total_used,
            free_bytes: total_size - total_used,
            usage_ratio: if total_size > 0 { total_used as f64 / total_size as f64 } else { 0.0 },
            fragmentation_ratio: (small_stats.fragmentation_ratio + medium_stats.fragmentation_ratio + large_stats.fragmentation_ratio) / 3.0,
            total_allocations: stats.total_allocations.load(Ordering::SeqCst),
            total_deallocations: stats.total_deallocations.load(Ordering::SeqCst),
            failed_allocations: stats.failed_allocations.load(Ordering::SeqCst),
        }
    }
}

/// 内存使用情况
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// 总大小
    pub total_size: usize,
    /// 已使用字节数
    pub used_bytes: usize,
    /// 空闲字节数
    pub free_bytes: usize,
    /// 使用率
    pub usage_ratio: f64,
    /// 碎片化率
    pub fragmentation_ratio: f64,
    /// 总分配次数
    pub total_allocations: u64,
    /// 总释放次数
    pub total_deallocations: u64,
    /// 失败的分配次数
    pub failed_allocations: u64,
}

/// 全局优化内存分配器
static mut GLOBAL_OPTIMIZED_ALLOCATOR: Option<OptimizedMemoryAllocator> = None;
static ALLOCATOR_LOCK: Mutex<()> = Mutex::new(());

/// 内存池信息结构
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub total_blocks: usize,
    pub free_blocks: usize,
    pub allocated_blocks: usize,
    pub total_bytes: usize,
    pub free_bytes: usize,
    pub allocated_bytes: usize,
    pub fragmentation_ratio: f32,
}

impl Default for PoolInfo {
    fn default() -> Self {
        Self {
            total_blocks: 0,
            free_blocks: 0,
            allocated_blocks: 0,
            total_bytes: 0,
            free_bytes: 0,
            allocated_bytes: 0,
            fragmentation_ratio: 0.0,
        }
    }
}

/// 获取全局优化内存分配器
pub fn get_global_optimized_allocator() -> &'static mut OptimizedMemoryAllocator {
    unsafe {
        if GLOBAL_OPTIMIZED_ALLOCATOR.is_none() {
            GLOBAL_OPTIMIZED_ALLOCATOR = Some(OptimizedMemoryAllocator::new());
        }
        GLOBAL_OPTIMIZED_ALLOCATOR.as_mut().unwrap()
    }
}

/// 初始化优化内存分配器
pub fn init_optimized_allocator() -> Result<(), &'static str> {
    let _lock = ALLOCATOR_LOCK.lock();
    let allocator = get_global_optimized_allocator();
    allocator.initialize()
}

// ============================================================================
// 公共接口函数 - 用于系统调用
// ============================================================================

/// 使用优化分配器分配内存
pub fn allocate_optimized(layout: Layout, _pid: u32) -> *mut u8 {
    let allocator = get_global_optimized_allocator();
    allocator.allocate(layout)
}

/// 使用优化分配器释放内存
pub fn deallocate_optimized(ptr: *mut u8, layout: Layout, _pid: u32) {
    let allocator = get_global_optimized_allocator();
    unsafe {
        allocator.deallocate(ptr, layout);
    }
}

/// 获取分配器统计信息
pub fn get_allocator_stats() -> AllocatorStats {
    let allocator = get_global_optimized_allocator();
    allocator.get_stats()
}

/// 获取内存池信息
pub fn get_pool_info(pool_type: u32) -> PoolInfo {
    let allocator = get_global_optimized_allocator();
    allocator.get_pool_info(pool_type)
}

/// 执行内存碎片整理
pub fn defragment_memory() {
    let allocator = get_global_optimized_allocator();
    allocator.defragment();
}

/// 获取内存使用情况
pub fn get_memory_usage() -> MemoryUsage {
    let allocator = get_global_optimized_allocator();
    allocator.get_memory_usage()
}

/// 使用优化分配器重新分配内存
pub fn reallocate_optimized(ptr: *mut u8, new_layout: Layout, _pid: u32) -> *mut u8 {
    let allocator = get_global_optimized_allocator();
    allocator.reallocate(ptr, new_layout)
}