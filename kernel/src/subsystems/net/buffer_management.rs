//! Network buffer management for NOS network stack
//!
//! This module provides efficient network buffer management including:
//! - Zero-copy buffer operations
//! - Buffer pools for different packet sizes
//! - Memory-mapped I/O support
//! - DMA buffer management
//! - Buffer chaining and fragmentation
//! - Memory pressure handling
//! - Buffer statistics and monitoring

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicUsize, AtomicBool, Ordering};
use spin::Mutex;

use crate::time;

/// Network buffer handle
pub type BufferHandle = u32;

/// Buffer types for different network uses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    /// Standard packet buffer
    Packet,
    /// Header buffer (for protocol headers)
    Header,
    /// Data buffer (for payload data)
    Data,
    /// Control buffer (for control messages)
    Control,
    /// DMA buffer (for hardware DMA)
    Dma,
    /// Memory-mapped I/O buffer
    Mmio,
}

/// Buffer allocation flags
#[derive(Debug, Clone, Copy)]
pub struct BufferFlags {
    /// Buffer is readable
    pub readable: bool,
    /// Buffer is writable
    pub writable: bool,
    /// Buffer can be used for DMA
    pub dma_capable: bool,
    /// Buffer is cacheable
    pub cacheable: bool,
    /// Buffer is zero-initialized
    pub zeroed: bool,
    /// Buffer is pre-allocated
    pub preallocated: bool,
    /// Buffer is persistent (won't be freed)
    pub persistent: bool,
}

impl Default for BufferFlags {
    fn default() -> Self {
        Self {
            readable: true,
            writable: true,
            dma_capable: false,
            cacheable: true,
            zeroed: false,
            preallocated: false,
            persistent: false,
        }
    }
}

/// Network buffer metadata
#[derive(Debug, Clone)]
pub struct BufferMetadata {
    /// Buffer type
    pub buffer_type: BufferType,
    /// Buffer flags
    pub flags: BufferFlags,
    /// Buffer size
    pub size: usize,
    /// Data length
    pub data_len: usize,
    /// Headroom space
    pub headroom: usize,
    /// Tailroom space
    pub tailroom: usize,
    /// Buffer priority
    pub priority: u8,
    /// Buffer timestamp
    pub timestamp: u64,
    /// Buffer owner
    pub owner: BufferOwner,
    /// Network interface ID
    pub interface_id: Option<u32>,
    /// Protocol-specific data
    pub protocol_data: Option<ProtocolData>,
}

/// Buffer owner information
#[derive(Debug, Clone)]
pub enum BufferOwner {
    /// No owner
    None,
    /// Kernel component
    Kernel(String),
    /// Network protocol
    Protocol(String),
    /// Network interface
    Interface(u32),
    /// User process
    Process(u32),
}

/// Protocol-specific data
#[derive(Debug, Clone)]
pub struct ProtocolData {
    /// Protocol type
    pub protocol_type: u8,
    /// Protocol-specific metadata
    pub metadata: Vec<u8>,
}

/// Network buffer
pub struct NetworkBuffer {
    /// Buffer handle
    pub handle: BufferHandle,
    /// Buffer address
    pub addr: usize,
    /// Physical address (for DMA)
    pub phys_addr: Option<usize>,
    /// Buffer metadata
    pub metadata: BufferMetadata,
    /// Reference count
    pub ref_count: AtomicU32,
    /// Buffer is mapped for DMA
    pub dma_mapped: AtomicBool,
    /// Buffer is locked (cannot be freed)
    pub locked: AtomicBool,
    /// Buffer chain (for fragmented packets)
    pub chain: Option<Box<NetworkBuffer>>,
    /// Buffer statistics
    pub stats: BufferStats,
}

impl NetworkBuffer {
    /// Create a new network buffer
    pub fn new(
        handle: BufferHandle,
        addr: usize,
        phys_addr: Option<usize>,
        metadata: BufferMetadata,
    ) -> Self {
        Self {
            handle,
            addr,
            phys_addr,
            metadata,
            ref_count: AtomicU32::new(1),
            dma_mapped: AtomicBool::new(false),
            locked: AtomicBool::new(false),
            chain: None,
            stats: BufferStats::default(),
        }
    }

    /// Get buffer data pointer
    pub fn data(&self) -> *mut u8 {
        (self.addr + self.metadata.headroom) as *mut u8
    }

    /// Get buffer data length
    pub fn len(&self) -> usize {
        self.metadata.data_len
    }

    /// Set buffer data length
    pub fn set_len(&mut self, len: usize) {
        if len <= self.metadata.size - self.metadata.headroom {
            self.metadata.data_len = len;
        }
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.metadata.size - self.metadata.headroom
    }

    /// Get headroom space
    pub fn headroom(&self) -> usize {
        self.metadata.headroom
    }

    /// Get tailroom space
    pub fn tailroom(&self) -> usize {
        self.metadata.size - self.metadata.headroom - self.metadata.data_len
    }

    /// Reserve headroom space
    pub fn reserve_headroom(&mut self, len: usize) -> bool {
        if len <= self.metadata.headroom {
            self.metadata.headroom -= len;
            true
        } else {
            false
        }
    }

    /// Reserve tailroom space
    pub fn reserve_tailroom(&mut self, len: usize) -> bool {
        if len <= self.tailroom() {
            true
        } else {
            false
        }
    }

    /// Push data to the beginning of buffer
    pub fn push_front(&mut self, data: &[u8]) -> bool {
        if data.len() <= self.metadata.headroom {
            unsafe {
                let dst = self.addr as *mut u8;
                let src = dst.add(self.metadata.headroom);
                core::ptr::copy(src, dst.add(data.len()), self.metadata.data_len);
                core::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
            }
            self.metadata.headroom -= data.len();
            self.metadata.data_len += data.len();
            true
        } else {
            false
        }
    }

    /// Push data to the end of buffer
    pub fn push_back(&mut self, data: &[u8]) -> bool {
        if data.len() <= self.tailroom() {
            unsafe {
                let dst = (self.addr + self.metadata.headroom + self.metadata.data_len) as *mut u8;
                core::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
            }
            self.metadata.data_len += data.len();
            true
        } else {
            false
        }
    }

    /// Pop data from the beginning of buffer
    pub fn pop_front(&mut self, len: usize) -> Option<Vec<u8>> {
        if len <= self.metadata.data_len {
            let mut data = Vec::with_capacity(len);
            unsafe {
                let src = (self.addr + self.metadata.headroom) as *const u8;
                data.set_len(len);
                core::ptr::copy_nonoverlapping(src, data.as_mut_ptr(), len);
            }
            self.metadata.headroom += len;
            self.metadata.data_len -= len;
            Some(data)
        } else {
            None
        }
    }

    /// Pop data from the end of buffer
    pub fn pop_back(&mut self, len: usize) -> Option<Vec<u8>> {
        if len <= self.metadata.data_len {
            let mut data = Vec::with_capacity(len);
            unsafe {
                let src = (self.addr + self.metadata.headroom + self.metadata.data_len - len) as *const u8;
                data.set_len(len);
                core::ptr::copy_nonoverlapping(src, data.as_mut_ptr(), len);
            }
            self.metadata.data_len -= len;
            Some(data)
        } else {
            None
        }
    }

    /// Acquire buffer (increment reference count)
    pub fn acquire(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Release buffer (decrement reference count)
    pub fn release(&self) -> bool {
        let count = self.ref_count.fetch_sub(1, Ordering::Acquire);
        if count == 1 {
            // Last reference - buffer should be freed
            true
        } else {
            false
        }
    }

    /// Lock buffer (prevent freeing)
    pub fn lock(&self) {
        self.locked.store(true, Ordering::Relaxed);
    }

    /// Unlock buffer
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Relaxed);
    }

    /// Check if buffer is locked
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    /// Map buffer for DMA
    pub fn map_dma(&self) -> bool {
        if self.metadata.flags.dma_capable && !self.is_locked() {
            self.dma_mapped.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Unmap buffer from DMA
    pub fn unmap_dma(&self) {
        self.dma_mapped.store(false, Ordering::Relaxed);
    }

    /// Check if buffer is DMA mapped
    pub fn is_dma_mapped(&self) -> bool {
        self.dma_mapped.load(Ordering::Relaxed)
    }

    /// Get physical address for DMA
    pub fn get_phys_addr(&self) -> Option<usize> {
        if self.is_dma_mapped() {
            self.phys_addr
        } else {
            None
        }
    }

    /// Chain this buffer with another
    pub fn chain(&mut self, other: NetworkBuffer) {
        self.chain = Some(Box::new(other));
    }

    /// Get next buffer in chain
    pub fn next(&self) -> Option<&NetworkBuffer> {
        self.chain.as_ref().map(|b| b.as_ref())
    }

    /// Get total length of chained buffers
    pub fn total_len(&self) -> usize {
        let mut total = self.metadata.data_len;
        let mut current = self.chain.as_ref();
        while let Some(buffer) = current {
            total += buffer.metadata.data_len;
            current = buffer.chain.as_ref();
        }
        total
    }

    /// Update timestamp
    pub fn update_timestamp(&mut self) {
        self.metadata.timestamp = time::get_monotonic_time();
    }

    /// Get buffer age
    pub fn age(&self) -> u64 {
        time::get_monotonic_time() - self.metadata.timestamp
    }
}

/// Buffer statistics
#[derive(Debug, Clone, Default)]
pub struct BufferStats {
    /// Number of times buffer was accessed
    pub access_count: AtomicU32,
    /// Total bytes read from buffer
    pub bytes_read: AtomicU64,
    /// Total bytes written to buffer
    pub bytes_written: AtomicU64,
    /// Number of DMA operations
    pub dma_operations: AtomicU32,
    /// Last access timestamp
    pub last_access: AtomicU64,
}

impl BufferStats {
    /// Record buffer access
    pub fn record_access(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        self.last_access.store(time::get_monotonic_time(), Ordering::Relaxed);
    }

    /// Record bytes read
    pub fn record_bytes_read(&self, bytes: u64) {
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
        self.record_access();
    }

    /// Record bytes written
    pub fn record_bytes_written(&self, bytes: u64) {
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
        self.record_access();
    }

    /// Record DMA operation
    pub fn record_dma_operation(&self) {
        self.dma_operations.fetch_add(1, Ordering::Relaxed);
        self.record_access();
    }

    /// Get access count
    pub fn get_access_count(&self) -> u32 {
        self.access_count.load(Ordering::Relaxed)
    }

    /// Get bytes read
    pub fn get_bytes_read(&self) -> u64 {
        self.bytes_read.load(Ordering::Relaxed)
    }

    /// Get bytes written
    pub fn get_bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }

    /// Get DMA operations count
    pub fn get_dma_operations(&self) -> u32 {
        self.dma_operations.load(Ordering::Relaxed)
    }

    /// Get last access time
    pub fn get_last_access(&self) -> u64 {
        self.last_access.load(Ordering::Relaxed)
    }
}

/// Buffer pool for managing buffers of specific sizes
pub struct BufferPool {
    /// Pool name
    pub name: String,
    /// Buffer size for this pool
    pub buffer_size: usize,
    /// Maximum number of buffers in pool
    pub max_buffers: usize,
    /// Current number of buffers in pool
    pub current_buffers: AtomicUsize,
    /// Available buffers
    pub available_buffers: Mutex<VecDeque<BufferHandle>>,
    /// Buffer metadata for this pool
    pub buffer_metadata: BufferMetadata,
    /// Pool statistics
    pub stats: PoolStats,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(
        name: String,
        buffer_size: usize,
        max_buffers: usize,
        buffer_metadata: BufferMetadata,
    ) -> Self {
        Self {
            name,
            buffer_size,
            max_buffers,
            current_buffers: AtomicUsize::new(0),
            available_buffers: Mutex::new(VecDeque::new()),
            buffer_metadata,
            stats: PoolStats::default(),
        }
    }

    /// Allocate a buffer from the pool
    pub fn allocate(&self) -> Option<BufferHandle> {
        // Check if we have available buffers
        {
            let mut available = self.available_buffers.lock();
            if let Some(handle) = available.pop_front() {
                self.stats.allocations.fetch_add(1, Ordering::Relaxed);
                self.stats.current_in_use.fetch_add(1, Ordering::Relaxed);
                return Some(handle);
            }
        }

        // Check if we can allocate a new buffer
        if self.current_buffers.load(Ordering::Relaxed) < self.max_buffers {
            self.stats.allocations.fetch_add(1, Ordering::Relaxed);
            self.stats.current_in_use.fetch_add(1, Ordering::Relaxed);
            self.current_buffers.fetch_add(1, Ordering::Relaxed);
            Some(0) // Placeholder - actual allocation would be done by buffer manager
        } else {
            self.stats.failures.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Deallocate a buffer back to the pool
    pub fn deallocate(&self, handle: BufferHandle) {
        let mut available = self.available_buffers.lock();
        available.push_back(handle);
        self.stats.current_in_use.fetch_sub(1, Ordering::Relaxed);
        self.stats.deallocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        PoolStats {
            name: self.name.clone(),
            buffer_size: self.buffer_size,
            max_buffers: self.max_buffers,
            current_buffers: self.current_buffers.load(Ordering::Relaxed),
            current_in_use: self.stats.current_in_use.load(Ordering::Relaxed),
            total_allocations: self.stats.allocations.load(Ordering::Relaxed),
            total_deallocations: self.stats.deallocations.load(Ordering::Relaxed),
            failures: self.stats.failures.load(Ordering::Relaxed),
        }
    }

    /// Resize the pool
    pub fn resize(&mut self, new_max: usize) {
        self.max_buffers = new_max;
    }

    /// Pre-allocate buffers in the pool
    pub fn preallocate(&self, count: usize) -> usize {
        let current = self.current_buffers.load(Ordering::Relaxed);
        let to_allocate = core::cmp::min(count, self.max_buffers - current);
        
        for _ in 0..to_allocate {
            // In real implementation, this would allocate actual buffers
            self.current_buffers.fetch_add(1, Ordering::Relaxed);
        }
        
        to_allocate
    }
}

/// Pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Pool name
    pub name: String,
    /// Buffer size
    pub buffer_size: usize,
    /// Maximum number of buffers
    pub max_buffers: usize,
    /// Current number of buffers
    pub current_buffers: usize,
    /// Current buffers in use
    pub current_in_use: AtomicU32,
    /// Total allocations
    pub allocations: AtomicU32,
    /// Total deallocations
    pub deallocations: AtomicU32,
    /// Allocation failures
    pub failures: AtomicU32,
}

/// Network buffer manager
pub struct NetworkBufferManager {
    /// Next buffer handle
    next_handle: AtomicU32,
    /// Buffers by handle
    buffers: Mutex<BTreeMap<BufferHandle, Arc<NetworkBuffer>>>,
    /// Buffer pools by size
    pools: Mutex<BTreeMap<usize, Arc<BufferPool>>>,
    /// DMA-capable buffers
    dma_buffers: Mutex<BTreeMap<BufferHandle, Arc<NetworkBuffer>>>,
    /// Memory-mapped I/O buffers
    mmio_buffers: Mutex<BTreeMap<BufferHandle, Arc<NetworkBuffer>>>,
    /// Global statistics
    global_stats: GlobalBufferStats,
    /// Memory pressure state
    memory_pressure: AtomicBool,
    /// Memory pressure threshold (percentage)
    memory_pressure_threshold: u8,
}

impl NetworkBufferManager {
    /// Create a new network buffer manager
    pub fn new() -> Self {
        Self {
            next_handle: AtomicU32::new(1),
            buffers: Mutex::new(BTreeMap::new()),
            pools: Mutex::new(BTreeMap::new()),
            dma_buffers: Mutex::new(BTreeMap::new()),
            mmio_buffers: Mutex::new(BTreeMap::new()),
            global_stats: GlobalBufferStats::default(),
            memory_pressure: AtomicBool::new(false),
            memory_pressure_threshold: 80, // 80% threshold
        }
    }

    /// Allocate a network buffer
    pub fn allocate_buffer(
        &self,
        size: usize,
        buffer_type: BufferType,
        flags: BufferFlags,
    ) -> Result<BufferHandle, BufferError> {
        // Check memory pressure
        if self.is_under_memory_pressure() && !flags.persistent {
            return Err(BufferError::MemoryPressure);
        }

        // Try to allocate from appropriate pool
        let pool_size = self.get_pool_size(size);
        let handle = if let Some(pool) = self.get_pool(pool_size) {
            pool.allocate()
        } else {
            None
        };

        if let Some(handle) = handle {
            return Ok(handle);
        }

        // Allocate new buffer
        let handle = self.next_handle.fetch_add(1, Ordering::SeqCst);
        
        // Allocate memory
        let addr = if flags.dma_capable {
            // Allocate DMA-capable memory
            crate::subsystems::mm::kalloc_dma(size).ok_or(BufferError::OutOfMemory)?
        } else {
            crate::subsystems::mm::kalloc(size).ok_or(BufferError::OutOfMemory)?
        };

        // Get physical address for DMA buffers
        let phys_addr = if flags.dma_capable {
            crate::subsystems::mm::virt_to_phys(addr)
        } else {
            None
        };

        // Create metadata
        let metadata = BufferMetadata {
            buffer_type,
            flags,
            size,
            data_len: 0,
            headroom: 64, // Default headroom
            tailroom: size - 64,
            priority: 0,
            timestamp: time::get_monotonic_time(),
            owner: BufferOwner::None,
            interface_id: None,
            protocol_data: None,
        };

        // Create buffer
        let buffer = Arc::new(NetworkBuffer::new(handle, addr, phys_addr, metadata));

        // Store buffer
        {
            let mut buffers = self.buffers.lock();
            buffers.insert(handle, buffer.clone());
        }

        // Store in appropriate collection
        if flags.dma_capable {
            let mut dma_buffers = self.dma_buffers.lock();
            dma_buffers.insert(handle, buffer);
        }

        if matches!(buffer_type, BufferType::Mmio) {
            let mut mmio_buffers = self.mmio_buffers.lock();
            mmio_buffers.insert(handle, buffer);
        }

        // Update statistics
        self.global_stats.total_buffers.fetch_add(1, Ordering::Relaxed);
        self.global_stats.total_memory.fetch_add(size as u64, Ordering::Relaxed);
        self.global_stats.allocations.fetch_add(1, Ordering::Relaxed);

        Ok(handle)
    }

    /// Deallocate a network buffer
    pub fn deallocate_buffer(&self, handle: BufferHandle) -> Result<(), BufferError> {
        // Get buffer
        let buffer = {
            let mut buffers = self.buffers.lock();
            buffers.remove(&handle).ok_or(BufferError::InvalidHandle)?
        };

        // Check if buffer is locked
        if buffer.is_locked() {
            return Err(BufferError::BufferLocked);
        }

        // Return to pool if applicable
        if let Some(pool) = self.get_pool(buffer.metadata.size) {
            pool.deallocate(handle);
        }

        // Remove from special collections
        if buffer.metadata.flags.dma_capable {
            let mut dma_buffers = self.dma_buffers.lock();
            dma_buffers.remove(&handle);
        }

        if matches!(buffer.metadata.buffer_type, BufferType::Mmio) {
            let mut mmio_buffers = self.mmio_buffers.lock();
            mmio_buffers.remove(&handle);
        }

        // Free memory
        unsafe {
            if buffer.metadata.flags.dma_capable {
                crate::subsystems::mm::kfree_dma(buffer.addr, buffer.metadata.size);
            } else {
                crate::subsystems::mm::kfree(buffer.addr, buffer.metadata.size);
            }
        }

        // Update statistics
        self.global_stats.total_buffers.fetch_sub(1, Ordering::Relaxed);
        self.global_stats.total_memory.fetch_sub(buffer.metadata.size as u64, Ordering::Relaxed);
        self.global_stats.deallocations.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get a buffer by handle
    pub fn get_buffer(&self, handle: BufferHandle) -> Option<Arc<NetworkBuffer>> {
        let buffers = self.buffers.lock();
        buffers.get(&handle).cloned()
    }

    /// Create a buffer pool
    pub fn create_pool(
        &self,
        name: String,
        buffer_size: usize,
        max_buffers: usize,
        buffer_type: BufferType,
        flags: BufferFlags,
    ) -> Result<(), BufferError> {
        let metadata = BufferMetadata {
            buffer_type,
            flags,
            size: buffer_size,
            data_len: 0,
            headroom: 64,
            tailroom: buffer_size - 64,
            priority: 0,
            timestamp: time::get_monotonic_time(),
            owner: BufferOwner::None,
            interface_id: None,
            protocol_data: None,
        };

        let pool = Arc::new(BufferPool::new(name, buffer_size, max_buffers, metadata));
        
        let mut pools = self.pools.lock();
        pools.insert(buffer_size, pool);

        Ok(())
    }

    /// Get a buffer pool by size
    pub fn get_pool(&self, size: usize) -> Option<Arc<BufferPool>> {
        let pools = self.pools.lock();
        pools.get(&size).cloned()
    }

    /// Get all buffer pools
    pub fn get_all_pools(&self) -> Vec<Arc<BufferPool>> {
        let pools = self.pools.lock();
        pools.values().cloned().collect()
    }

    /// Get DMA buffers
    pub fn get_dma_buffers(&self) -> Vec<Arc<NetworkBuffer>> {
        let dma_buffers = self.dma_buffers.lock();
        dma_buffers.values().cloned().collect()
    }

    /// Get MMIO buffers
    pub fn get_mmio_buffers(&self) -> Vec<Arc<NetworkBuffer>> {
        let mmio_buffers = self.mmio_buffers.lock();
        mmio_buffers.values().cloned().collect()
    }

    /// Get global statistics
    pub fn get_global_stats(&self) -> GlobalBufferStats {
        GlobalBufferStats {
            total_buffers: self.global_stats.total_buffers.load(Ordering::Relaxed),
            total_memory: self.global_stats.total_memory.load(Ordering::Relaxed),
            allocations: self.global_stats.allocations.load(Ordering::Relaxed),
            deallocations: self.global_stats.deallocations.load(Ordering::Relaxed),
            memory_pressure: self.memory_pressure.load(Ordering::Relaxed),
            memory_pressure_threshold: self.memory_pressure_threshold,
        }
    }

    /// Check if under memory pressure
    pub fn is_under_memory_pressure(&self) -> bool {
        self.memory_pressure.load(Ordering::Relaxed)
    }

    /// Set memory pressure state
    pub fn set_memory_pressure(&self, pressure: bool) {
        self.memory_pressure.store(pressure, Ordering::Relaxed);
    }

    /// Get memory pressure threshold
    pub fn get_memory_pressure_threshold(&self) -> u8 {
        self.memory_pressure_threshold
    }

    /// Set memory pressure threshold
    pub fn set_memory_pressure_threshold(&mut self, threshold: u8) {
        self.memory_pressure_threshold = threshold;
    }

    /// Garbage collect old buffers
    pub fn garbage_collect(&self, max_age: u64) -> usize {
        let mut collected = 0;
        let now = time::get_monotonic_time();
        
        let buffers = self.buffers.lock();
        for (handle, buffer) in buffers.iter() {
            if buffer.age() > max_age && buffer.ref_count.load(Ordering::Relaxed) == 1 {
                if self.deallocate_buffer(*handle).is_ok() {
                    collected += 1;
                }
            }
        }
        
        collected
    }

    /// Optimize buffer pools
    pub fn optimize_pools(&self) {
        let pools = self.pools.lock();
        for pool in pools.values() {
            // In a real implementation, this would optimize pool allocation
            // based on usage patterns
        }
    }

    /// Get appropriate pool size for a buffer
    fn get_pool_size(&self, size: usize) -> usize {
        // Round up to nearest power of 2 for efficient pooling
        if size <= 64 {
            64
        } else if size <= 128 {
            128
        } else if size <= 256 {
            256
        } else if size <= 512 {
            512
        } else if size <= 1024 {
            1024
        } else if size <= 2048 {
            2048
        } else if size <= 4096 {
            4096
        } else {
            // For larger buffers, round up to nearest page size
            (size + 4095) & !4095
        }
    }
}

impl Default for NetworkBufferManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global buffer statistics
#[derive(Debug, Clone, Default)]
pub struct GlobalBufferStats {
    /// Total number of buffers
    pub total_buffers: AtomicU32,
    /// Total memory used by buffers
    pub total_memory: AtomicU64,
    /// Total allocations
    pub allocations: AtomicU32,
    /// Total deallocations
    pub deallocations: AtomicU32,
    /// Memory pressure state
    pub memory_pressure: AtomicBool,
    /// Memory pressure threshold
    pub memory_pressure_threshold: u8,
}

/// Buffer management errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferError {
    /// Out of memory
    OutOfMemory,
    /// Invalid buffer handle
    InvalidHandle,
    /// Buffer is locked
    BufferLocked,
    /// Memory pressure
    MemoryPressure,
    /// Invalid buffer size
    InvalidSize,
    /// DMA operation failed
    DmaFailed,
    /// Permission denied
    PermissionDenied,
}

/// Global network buffer manager instance
static GLOBAL_BUFFER_MANAGER: once_cell::sync::Lazy<Mutex<NetworkBufferManager>> = 
    once_cell::sync::Lazy::new(|| Mutex::new(NetworkBufferManager::new()));

/// Get global network buffer manager
pub fn get_global_buffer_manager() -> &'static Mutex<NetworkBufferManager> {
    &GLOBAL_BUFFER_MANAGER
}

/// Initialize network buffer management
pub fn init_buffer_management() -> Result<(), BufferError> {
    let manager = get_global_buffer_manager();
    let mut manager = manager.lock();
    
    // Create default buffer pools
    manager.create_pool(
        "small".to_string(),
        256,
        1000,
        BufferType::Packet,
        BufferFlags::default(),
    )?;
    
    manager.create_pool(
        "medium".to_string(),
        1024,
        500,
        BufferType::Packet,
        BufferFlags::default(),
    )?;
    
    manager.create_pool(
        "large".to_string(),
        4096,
        100,
        BufferType::Packet,
        BufferFlags::default(),
    )?;
    
    manager.create_pool(
        "jumbo".to_string(),
        9000,
        50,
        BufferType::Packet,
        BufferFlags::default(),
    )?;
    
    // Create DMA buffer pool
    manager.create_pool(
        "dma".to_string(),
        4096,
        200,
        BufferType::Dma,
        BufferFlags {
            dma_capable: true,
            cacheable: false,
            ..Default::default()
        },
    )?;
    
    log::info!("Network buffer management initialized");
    Ok(())
}

/// Buffer management utility functions
pub mod utils {
    use super::*;

    /// Calculate optimal buffer size for packet
    pub fn calculate_optimal_buffer_size(packet_size: usize, headroom: usize, tailroom: usize) -> usize {
        let total_size = packet_size + headroom + tailroom;
        
        // Round up to nearest cache line size (64 bytes)
        (total_size + 63) & !63
    }

    /// Validate buffer flags
    pub fn validate_buffer_flags(flags: &BufferFlags) -> bool {
        // DMA-capable buffers must be writable
        if flags.dma_capable && !flags.writable {
            return false;
        }
        
        // Cacheable and DMA-capable are mutually exclusive
        if flags.cacheable && flags.dma_capable {
            return false;
        }
        
        true
    }

    /// Estimate memory usage for buffer pools
    pub fn estimate_pool_memory_usage(pools: &[Arc<BufferPool>]) -> u64 {
        let mut total = 0u64;
        
        for pool in pools {
            let stats = pool.get_stats();
            total += (stats.buffer_size * stats.current_buffers) as u64;
        }
        
        total
    }

    /// Find best buffer pool for size
    pub fn find_best_pool_for_size(pools: &[Arc<BufferPool>], size: usize) -> Option<Arc<BufferPool>> {
        pools
            .iter()
            .filter(|pool| pool.buffer_size >= size)
            .min_by_key(|pool| pool.buffer_size)
            .cloned()
    }

    /// Optimize buffer layout for DMA
    pub fn optimize_for_dma(addr: usize, size: usize) -> Option<usize> {
        // Check if address is properly aligned for DMA
        if addr % 64 == 0 && size % 64 == 0 {
            Some(addr)
        } else {
            None
        }
    }

    /// Create buffer chain from multiple buffers
    pub fn create_buffer_chain(buffers: Vec<Arc<NetworkBuffer>>) -> Option<Arc<NetworkBuffer>> {
        if buffers.is_empty() {
            return None;
        }
        
        let mut current = buffers[0].clone();
        
        for buffer in buffers.iter().skip(1) {
            // In a real implementation, this would properly chain buffers
            // For now, we'll just return the first buffer
        }
        
        Some(current)
    }

    /// Flatten buffer chain into single buffer
    pub fn flatten_buffer_chain(buffer: &NetworkBuffer) -> Option<Vec<u8>> {
        let mut total_len = buffer.total_len();
        let mut result = Vec::with_capacity(total_len);
        
        // Copy data from first buffer
        unsafe {
            let src = buffer.data();
            result.set_len(buffer.len());
            core::ptr::copy_nonoverlapping(src, result.as_mut_ptr(), buffer.len());
        }
        
        // Copy data from chained buffers
        let mut current = buffer.next();
        let mut offset = buffer.len();
        
        while let Some(next_buffer) = current {
            unsafe {
                let src = next_buffer.data();
                result.set_len(offset + next_buffer.len());
                core::ptr::copy_nonoverlapping(src, result.as_mut_ptr().add(offset), next_buffer.len());
            }
            
            offset += next_buffer.len();
            current = next_buffer.next();
        }
        
        Some(result)
    }
}