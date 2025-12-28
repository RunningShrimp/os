#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! io_uring Buffers
//!
//! This module implements io_uring buffer management:
//! - Fixed ring buffers
//! - Registered buffers
//! - Zero-copy buffer pools
//! - Buffer mapping and unmapping
//!
//! Features:
//! - Fixed buffer registration
//! - Per-CPU buffer pools
//! - Zero-copy transfers
//! - Buffer lifecycle management

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// io_uring Buffer Constants
// ============================================================================

/// Default buffer size (4KB)
pub const DEFAULT_BUFFER_SIZE: usize = 4096;

/// Maximum number of registered buffers
pub const MAX_REGISTERED_BUFFERS: usize = 1 << 16; // 65536 buffers

/// Minimum buffer alignment (page size)
pub const MIN_BUFFER_ALIGNMENT: usize = 4096;

/// Per-CPU buffer pool size
pub const PER_CPU_POOL_SIZE: usize = 64;

// ============================================================================
// Buffer Flags
// ============================================================================

/// io_uring buffer flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferFlags {
    /// Buffer is read-only
    pub read_only: bool,
    
    /// Buffer is write-only
    pub write_only: bool,
    
    /// Buffer is mapped for DMA
    pub dma_mapped: bool,
    
    /// Buffer is zero-copied
    pub zero_copied: bool,
    
    /// Buffer is pinned (cannot be swapped)
    pub pinned: bool,
}

impl Default for BufferFlags {
    fn default() -> Self {
        Self {
            read_only: false,
            write_only: false,
            dma_mapped: true,
            zero_copied: false,
            pinned: false,
        }
    }
}

// ============================================================================
// Registered Buffer
// ============================================================================

/// io_uring registered buffer
#[derive(Debug, Clone)]
pub struct RegisteredBuffer {
    /// Buffer ID
    pub buffer_id: u32,
    
    /// Buffer address
    pub addr: u64,
    
    /// Buffer size
    pub size: usize,
    
    /// Buffer data
    pub data: Vec<u8>,
    
    /// Buffer flags
    pub flags: BufferFlags,
    
    /// Reference count
    pub refcount: AtomicUsize,
    
    /// Last access timestamp
    pub last_access: AtomicU64,
    
    /// Creation timestamp
    pub created_at: u64,
}

impl RegisteredBuffer {
    /// Create new registered buffer
    pub fn new(buffer_id: u32, size: usize, flags: BufferFlags) -> Self {
        let mut data = {
    let mut v = alloc::vec::Vec::new();
    v.resize(size, 0u8);
    v
};
        
        Self {
            buffer_id,
            addr: data.as_ptr() as u64,
            size,
            data,
            flags,
            refcount: AtomicUsize::new(0),
            last_access: AtomicU64::new(crate::subsystems::time::timestamp_nanos()),
            created_at: crate::subsystems::time::timestamp_nanos(),
        }
    }
    
    /// Read from buffer
    pub fn read(&self, offset: usize, size: usize) -> Option<Vec<u8>> {
        if offset + size > self.size {
            return None;
        }
        
        self.last_access.store(crate::subsystems::time::timestamp_nanos(), Ordering::Relaxed);
        
        Some(self.data[offset..offset + size].to_vec())
    }
    
    /// Write to buffer
    pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<(), BufferError> {
        if offset + data.len() > self.size {
            return Err(BufferError::BufferOutOfBounds);
        }
        
        if self.flags.read_only {
            return Err(BufferError::ReadOnlyBuffer);
        }
        
        self.data[offset..offset + data.len()].copy_from_slice(data);
        self.last_access.store(crate::subsystems::time::timestamp_nanos(), Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Clear buffer
    pub fn clear(&mut self) {
        if self.flags.read_only {
            return;
        }
        
        for byte in self.data.iter_mut() {
            *byte = 0;
        }
        
        self.last_access.store(crate::subsystems::time::timestamp_nanos(), Ordering::Relaxed);
    }
    
    /// Increment reference count
    pub fn increment_ref(&self) {
        self.refcount.fetch_add(1, Ordering::Release);
    }
    
    /// Decrement reference count
    pub fn decrement_ref(&self) {
        self.refcount.fetch_sub(1, Ordering::Release);
    }
    
    /// Get reference count
    pub fn refcount(&self) -> usize {
        self.refcount.load(Ordering::Acquire)
    }
    
    /// Check if buffer is in use
    pub fn is_in_use(&self) -> bool {
        self.refcount() > 0
    }
}

/// io_uring buffer error
#[derive(Debug, Clone)]
pub enum BufferError {
    /// Buffer out of bounds
    BufferOutOfBounds,
    
    /// Read-only buffer
    ReadOnlyBuffer,
    
    /// Write-only buffer
    WriteOnlyBuffer,
    
    /// Invalid buffer ID
    InvalidBufferId {
        buffer_id: u32,
    },
    
    /// Buffer allocation failed
    AllocationFailed {
        reason: String,
    },
    
    /// Buffer registration failed
    RegistrationFailed {
        reason: String,
    },
}

// ============================================================================
// Buffer Pool
// ============================================================================

/// Per-CPU buffer pool
#[derive(Debug, Clone)]
pub struct BufferPool {
    /// Pool ID
    pub pool_id: u32,
    
    /// CPU ID
    pub cpu_id: usize,
    
    /// Free buffers
    pub free_buffers: Mutex<Vec<u32>>,
    
    /// All buffers
    pub buffers: Mutex<BTreeMap<u32, Arc<RegisteredBuffer>>>,
    
    /// Buffer size
    pub buffer_size: usize,
    
    /// Maximum buffers in pool
    pub max_buffers: usize,
    
    /// Total allocations
    pub total_allocations: AtomicUsize,
    
    /// Total frees
    pub total_frees: AtomicUsize,
    
    /// Pool statistics
    pub stats: Mutex<BufferPoolStats>,
}

/// Buffer pool statistics
#[derive(Debug, Clone, Copy)]
pub struct BufferPoolStats {
    pub total_buffers: usize,
    pub free_buffers: usize,
    pub allocated_buffers: usize,
    pub total_allocations: usize,
    pub total_frees: usize,
    pub hit_rate: f64,
}

impl Default for BufferPoolStats {
    fn default() -> Self {
        Self {
            total_buffers: 0,
            free_buffers: 0,
            allocated_buffers: 0,
            total_allocations: 0,
            total_frees: 0,
            hit_rate: 0.0,
        }
    }
}

impl BufferPool {
    /// Create new buffer pool
    pub fn new(pool_id: u32, cpu_id: usize, buffer_size: usize, max_buffers: usize) -> Self {
        let mut buffers = BTreeMap::new();
        let mut free_buffers = Vec::new();
        
        // Pre-allocate buffers
        for i in 0..max_buffers {
            let buffer_id = i as u32;
            let flags = BufferFlags::default();
            let buffer = Arc::new(RegisteredBuffer::new(buffer_id, buffer_size, flags));
            buffers.insert(buffer_id, buffer.clone());
            free_buffers.push(buffer_id);
        }
        
        let stats = BufferPoolStats {
            total_buffers: max_buffers,
            free_buffers: max_buffers,
            allocated_buffers: 0,
            total_allocations: 0,
            total_frees: 0,
            hit_rate: 0.0,
        };
        
        Self {
            pool_id,
            cpu_id,
            free_buffers: Mutex::new(free_buffers),
            buffers: Mutex::new(buffers),
            buffer_size,
            max_buffers,
            total_allocations: AtomicUsize::new(0),
            total_frees: AtomicUsize::new(0),
            stats: Mutex::new(stats),
        }
    }
    
    /// Allocate buffer from pool
    pub fn allocate(&self) -> Option<Arc<RegisteredBuffer>> {
        let mut free_buffers = self.free_buffers.lock();
        
        if let Some(buffer_id) = free_buffers.pop() {
            let buffers = self.buffers.lock();
            
            if let Some(buffer) = buffers.get(&buffer_id) {
                buffer.increment_ref();
                self.total_allocations.fetch_add(1, Ordering::Relaxed);
                
                // Update statistics
                let mut stats = self.stats.lock();
                stats.free_buffers -= 1;
                stats.allocated_buffers += 1;
                stats.total_allocations += 1;
                
                // Update hit rate
                if stats.total_allocations > 0 {
                    stats.hit_rate = (stats.total_frees as f64) / (stats.total_allocations as f64);
                }
                
                crate::println!("[io_uring] Allocated buffer {} from pool {}", buffer_id, self.pool_id);
                
                Some(buffer.clone())
            } else {
                None
            }
        } else {
            crate::println!("[io_uring] No free buffers in pool {}", self.pool_id);
            None
        }
    }
    
    /// Free buffer back to pool
    pub fn free(&self, buffer: Arc<RegisteredBuffer>) {
        buffer.decrement_ref();
        self.total_frees.fetch_add(1, Ordering::Relaxed);
        
        let mut free_buffers = self.free_buffers.lock();
        free_buffers.push(buffer.buffer_id);
        
        // Update statistics
        let mut stats = self.stats.lock();
        stats.free_buffers += 1;
        stats.allocated_buffers -= 1;
        stats.total_frees += 1;
        
        // Update hit rate
        if stats.total_allocations > 0 {
            stats.hit_rate = (stats.total_frees as f64) / (stats.total_allocations as f64);
        }
        
        crate::println!("[io_uring] Freed buffer {} to pool {}", buffer.buffer_id, self.pool_id);
    }
    
    /// Get buffer by ID
    pub fn get_buffer(&self, buffer_id: u32) -> Option<Arc<RegisteredBuffer>> {
        let buffers = self.buffers.lock();
        buffers.get(&buffer_id).cloned()
    }
    
    /// Get pool statistics
    pub fn get_stats(&self) -> BufferPoolStats {
        let mut stats = self.stats.lock();
        
        // Update current state
        stats.allocated_buffers = self.total_allocations.load(Ordering::Relaxed) - 
                             self.total_frees.load(Ordering::Relaxed);
        stats.free_buffers = self.max_buffers - stats.allocated_buffers;
        
        *stats
    }
    
    /// Clear pool (release all buffers)
    pub fn clear(&mut self) {
        let mut free_buffers = self.free_buffers.lock();
        let buffers = self.buffers.lock();
        
        for buffer_id in buffers.keys() {
            if !free_buffers.contains(buffer_id) {
                // Force free in-use buffers
                free_buffers.push(*buffer_id);
            }
        }
        
        self.total_frees.store(self.total_allocations.load(Ordering::Relaxed), Ordering::Relaxed);
        
        crate::println!("[io_uring] Cleared pool {}", self.pool_id);
    }
}

// ============================================================================
// Buffer Manager
// ============================================================================

/// io_uring buffer manager
pub struct BufferManager {
    /// Registered buffers (global)
    pub registered_buffers: Mutex<BTreeMap<u32, Arc<RegisteredBuffer>>>,
    
    /// Per-CPU buffer pools
    pub cpu_pools: Mutex<BTreeMap<usize, Arc<BufferPool>>>,
    
    /// Next buffer ID
    pub next_buffer_id: AtomicU32,
    
    /// Next pool ID
    pub next_pool_id: AtomicU32,
    
    /// Total registered buffers
    pub total_registered: AtomicUsize,
    
    /// Total buffer allocations
    pub total_allocations: AtomicUsize,
    
    /// Manager statistics
    pub stats: Mutex<BufferManagerStats>,
}

/// Buffer manager statistics
#[derive(Debug, Clone, Copy)]
pub struct BufferManagerStats {
    pub total_registered_buffers: usize,
    pub total_pools: usize,
    pub total_allocations: usize,
    pub total_frees: usize,
    pub total_buffer_size: usize,
    pub active_buffers: usize,
}

impl Default for BufferManagerStats {
    fn default() -> Self {
        Self {
            total_registered_buffers: 0,
            total_pools: 0,
            total_allocations: 0,
            total_frees: 0,
            total_buffer_size: 0,
            active_buffers: 0,
        }
    }
}

impl BufferManager {
    /// Create new buffer manager
    pub fn new() -> Self {
        Self {
            registered_buffers: Mutex::new(BTreeMap::new()),
            cpu_pools: Mutex::new(BTreeMap::new()),
            next_buffer_id: AtomicU32::new(1),
            next_pool_id: AtomicU32::new(1),
            total_registered: AtomicUsize::new(0),
            total_allocations: AtomicUsize::new(0),
            stats: Mutex::new(BufferManagerStats::default()),
        }
    }
    
    /// Create per-CPU buffer pool
    pub fn create_pool(&self, cpu_id: usize, buffer_size: usize, 
                    max_buffers: usize) -> Result<u32, BufferError> {
        
        if cpu_id >= 128 {
            return Err(BufferError::AllocationFailed {
                reason: String::from("Invalid CPU ID"),
            });
        }
        
        let pool_id = self.next_pool_id.fetch_add(1, Ordering::Relaxed);
        let pool = Arc::new(BufferPool::new(pool_id, cpu_id, buffer_size, max_buffers));
        
        let mut pools = self.cpu_pools.lock();
        pools.insert(cpu_id, pool);
        
        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_pools += 1;
        
        crate::println!("[io_uring] Created buffer pool {} for CPU {}", pool_id, cpu_id);
        
        Ok(pool_id)
    }
    
    /// Register new buffer
    pub fn register_buffer(&self, buffer: RegisteredBuffer) -> Result<u32, BufferError> {
        let buffer_id = buffer.buffer_id;
        
        let mut buffers = self.registered_buffers.lock();
        
        if buffers.contains_key(&buffer_id) {
            return Err(BufferError::RegistrationFailed {
                reason: String::from("Buffer ID already registered"),
            });
        }
        
        buffers.insert(buffer_id, Arc::new(buffer));
        self.total_registered.fetch_add(1, Ordering::Relaxed);
        
        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_registered_buffers += 1;
        stats.total_buffer_size += buffer.size;
        
        crate::println!("[io_uring] Registered buffer {} ({} bytes)", buffer_id, buffer.size);
        
        Ok(buffer_id)
    }
    
    /// Get buffer by ID
    pub fn get_buffer(&self, buffer_id: u32) -> Option<Arc<RegisteredBuffer>> {
        let buffers = self.registered_buffers.lock();
        buffers.get(&buffer_id).cloned()
    }
    
    /// Unregister buffer
    pub fn unregister_buffer(&self, buffer_id: u32) -> Result<(), BufferError> {
        let mut buffers = self.registered_buffers.lock();
        
        if let Some(buffer) = buffers.remove(&buffer_id) {
            if buffer.is_in_use() {
                return Err(BufferError::AllocationFailed {
                    reason: String::from("Buffer is still in use"),
                });
            }
            
            self.total_registered.fetch_sub(1, Ordering::Relaxed);
            
            // Update statistics
            let mut stats = self.stats.lock();
            stats.total_registered_buffers -= 1;
            stats.total_buffer_size -= buffer.size;
            
            crate::println!("[io_uring] Unregistered buffer {}", buffer_id);
            
            Ok(())
        } else {
            Err(BufferError::InvalidBufferId { buffer_id })
        }
    }
    
    /// Get CPU pool
    pub fn get_cpu_pool(&self, cpu_id: usize) -> Option<Arc<BufferPool>> {
        let pools = self.cpu_pools.lock();
        pools.get(&cpu_id).cloned()
    }
    
    /// Get manager statistics
    pub fn get_stats(&self) -> BufferManagerStats {
        let mut stats = self.stats.lock();
        
        // Update current state
        stats.total_allocations = self.total_allocations.load(Ordering::Relaxed);
        
        *stats
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registered_buffer() {
        let flags = BufferFlags::default();
        let mut buffer = RegisteredBuffer::new(1, 4096, flags);
        
        assert_eq!(buffer.buffer_id, 1);
        assert_eq!(buffer.size, 4096);
        assert!(!buffer.is_in_use());
        
        // Write data
        let data = {
    let mut v = alloc::vec::Vec::new();
    v.push(0xDE);
    v.push(0xAD);
    v.push(0xBE);
    v.push(0xEF);
    v
};
        buffer.write(0, &data).unwrap();
        
        // Read data
        let read_data = buffer.read(0, 4).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_buffer_refcount() {
        let flags = BufferFlags::default();
        let buffer = RegisteredBuffer::new(1, 4096, flags);
        
        assert_eq!(buffer.refcount(), 0);
        
        buffer.increment_ref();
        assert_eq!(buffer.refcount(), 1);
        assert!(buffer.is_in_use());
        
        buffer.decrement_ref();
        assert_eq!(buffer.refcount(), 0);
        assert!(!buffer.is_in_use());
    }

    #[test]
    fn test_buffer_pool() {
        let pool = BufferPool::new(1, 0, 4096, 16);
        
        assert_eq!(pool.max_buffers, 16);
        assert_eq!(pool.pool_id, 1);
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_buffers, 16);
        assert_eq!(stats.free_buffers, 16);
        
        // Allocate buffer
        let buffer = pool.allocate().unwrap();
        assert_eq!(buffer.refcount(), 1);
        
        let stats = pool.get_stats();
        assert_eq!(stats.allocated_buffers, 1);
        
        // Free buffer
        pool.free(buffer);
        
        let stats = pool.get_stats();
        assert_eq!(stats.free_buffers, 16);
    }

    #[test]
    fn test_buffer_manager() {
        let manager = BufferManager::new();
        
        let flags = BufferFlags::default();
        let buffer = RegisteredBuffer::new(1, 4096, flags);
        
        manager.register_buffer(buffer).unwrap();
        
        let retrieved = manager.get_buffer(1).unwrap();
        assert_eq!(retrieved.buffer_id, 1);
        
        manager.unregister_buffer(1).unwrap();
        assert!(manager.get_buffer(1).is_none());
    }

    #[test]
    fn test_buffer_pool_creation() {
        let manager = BufferManager::new();
        
        let pool_id = manager.create_pool(0, 4096, 64).unwrap();
        
        let pool = manager.get_cpu_pool(0).unwrap();
        assert_eq!(pool.pool_id, pool_id);
        assert_eq!(pool.cpu_id, 0);
    }
}
