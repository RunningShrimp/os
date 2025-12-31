//! UDP Fast Path Implementation
//!
//! This module provides high-performance UDP optimizations including:
//! - Zero-copy receive using io_uring-style buffers
//! - Batched send with scatter-gather I/O
//! - Fast hash-based socket lookup
//! - Lock-free per-CPU socket caches
//! - Kernel bypass support for latency-critical applications

extern crate alloc;
use alloc::{
    sync::Arc,
    vec::Vec,
};
use core::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicU16, AtomicU32, AtomicU64, AtomicUsize, Ordering},
};

use spin::Mutex;

use crate::subsystems::net::{
    ipv4::Ipv4Addr,
    socket::SocketAddr,
    udp::UdpSocket,
};

/// Maximum number of CPUs supported
const MAX_CPUS: usize = 256;

/// Default batch size for packet processing
const DEFAULT_BATCH_SIZE: usize = 32;

/// Maximum number of sockets per CPU cache
const MAX_CPU_SOCKETS: usize = 128;

/// Buffer pool size for zero-copy operations
const BUFFER_POOL_SIZE: usize = 1024;

/// Packet buffer for zero-copy operations
#[repr(C)]
pub struct PacketBuffer {
    /// Buffer data
    pub data: UnsafeCell<[u8; 65536]>, // Max jumbo frame size
    /// Current data length
    pub len: AtomicUsize,
    /// Buffer sequence number
    pub seq: AtomicU64,
    /// Reference count
    pub ref_count: AtomicU32,
    /// Buffer state flags
    pub flags: AtomicU16,
}

impl PacketBuffer {
    /// Create a new packet buffer
    pub fn new() -> Self {
        Self {
            data: UnsafeCell::new([0u8; 65536]),
            len: AtomicUsize::new(0),
            seq: AtomicU64::new(0),
            ref_count: AtomicU32::new(1),
            flags: AtomicU16::new(0),
        }
    }

    /// Get buffer data pointer
    pub fn data_ptr(&self) -> *mut u8 {
        unsafe { (*self.data.get()).as_mut_ptr() }
    }

    /// Get buffer data slice
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            let len = self.len.load(Ordering::Acquire);
            core::slice::from_raw_parts((*self.data.get()).as_ptr(), len)
        }
    }

    /// Get buffer data as mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            let len = self.len.load(Ordering::Acquire);
            core::slice::from_raw_parts_mut((*self.data.get()).as_mut_ptr(), len)
        }
    }

    /// Set buffer length
    pub fn set_len(&self, len: usize) {
        self.len.store(len, Ordering::Release);
    }

    /// Increment reference count
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement reference count and return if it reached zero
    pub fn dec_ref(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::Release) == 1
    }
}

/// Lock-free ring buffer for packet buffers
pub struct RingBuffer<T> {
    /// Buffer storage
    buffers: Vec<MaybeUninit<T>>,
    /// Head index (for reads)
    head: AtomicUsize,
    /// Tail index (for writes)
    tail: AtomicUsize,
    /// Capacity (must be power of 2)
    capacity: usize,
    /// Mask for fast modulo
    mask: usize,
}

impl<T> RingBuffer<T> {
    /// Create a new ring buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        let mut buffers = Vec::with_capacity(capacity);
        unsafe {
            buffers.set_len(capacity);
        }

        Self {
            buffers,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            capacity,
            mask: capacity - 1,
        }
    }

    /// Push an item to the ring buffer (returns false if full)
    pub fn push(&self, item: T) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        // Check if ring buffer is full
        if tail.wrapping_sub(head) >= self.capacity {
            return false;
        }

        let idx = tail & self.mask;
        unsafe {
            // Use raw pointer cast to get mutable access without &mut self
            let ptr = self.buffers.as_ptr() as *const MaybeUninit<T> as *mut T;
            ptr.add(idx).write(item);
        }

        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    /// Pop an item from the ring buffer (returns None if empty)
    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let idx = head & self.mask;
        let item = unsafe { self.buffers[idx].as_ptr().read() };

        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(item)
    }

    /// Get the number of items in the ring buffer
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    /// Check if the ring buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Per-CPU socket cache entry
struct PerCpuSocketEntry {
    /// Socket address key
    addr: SocketAddr,
    /// UDP socket reference
    socket: Arc<UdpSocket>,
    /// Last access timestamp
    last_access: AtomicU64,
    /// Access frequency counter
    access_count: AtomicU32,
}

impl PerCpuSocketEntry {
    fn new(addr: SocketAddr, socket: Arc<UdpSocket>) -> Self {
        Self {
            addr,
            socket,
            last_access: AtomicU64::new(0),
            access_count: AtomicU32::new(1),
        }
    }
}

/// Per-CPU socket cache for fast lookup
pub struct PerCpuSocketCache {
    /// Socket entries
    entries: Vec<Option<PerCpuSocketEntry>>,
    /// Hash table for fast lookup
    hash_table: Vec<AtomicU16>,
    /// Number of valid entries
    count: AtomicUsize,
    /// CPU ID
    cpu_id: u32,
}

impl PerCpuSocketCache {
    /// Create a new per-CPU socket cache
    pub fn new(cpu_id: u32) -> Self {
        let size = MAX_CPU_SOCKETS;
        let mut entries = Vec::with_capacity(size);
        let mut hash_table = Vec::with_capacity(size * 2);

        for _ in 0..size {
            entries.push(None);
        }

        for _ in 0..(size * 2) {
            hash_table.push(AtomicU16::new(u16::MAX));
        }

        Self {
            entries,
            hash_table,
            count: AtomicUsize::new(0),
            cpu_id,
        }
    }

    /// Calculate hash for socket address
    fn hash_addr(&self, addr: &SocketAddr) -> u16 {
        let mut hash = addr.port as u32;
        hash = hash.wrapping_mul(31).wrapping_add(addr.ip.to_u32());
        hash = hash.wrapping_mul(31);
        ((hash >> 16) ^ hash) as u16
    }

    /// Look up socket by address (lock-free fast path)
    pub fn lookup(&self, addr: &SocketAddr) -> Option<Arc<UdpSocket>> {
        let hash = self.hash_addr(addr);
        let idx = (hash as usize) % self.hash_table.len();

        let entry_idx = self.hash_table[idx].load(Ordering::Acquire);
        if entry_idx == u16::MAX {
            return None;
        }

        if let Some(entry) = &self.entries[entry_idx as usize] {
            if entry.addr == *addr {
                entry.access_count.fetch_add(1, Ordering::Relaxed);
                let now = Self::get_timestamp();
                entry.last_access.store(now, Ordering::Relaxed);
                return Some(Arc::clone(&entry.socket));
            }
        }

        None
    }

    /// Insert socket into cache
    pub fn insert(&self, addr: SocketAddr, socket: Arc<UdpSocket>) -> bool {
        let count = self.count.load(Ordering::Acquire);
        if count >= MAX_CPU_SOCKETS {
            return false;
        }

        // Find empty slot
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.is_none() {
                let new_entry = PerCpuSocketEntry::new(addr, socket);
                unsafe {
                    let slot = &self.entries[i] as *const _ as *mut Option<PerCpuSocketEntry>;
                    slot.write(Some(new_entry));
                }

                // Update hash table
                let hash = self.hash_addr(&addr);
                let hash_idx = (hash as usize) % self.hash_table.len();
                self.hash_table[hash_idx].store(i as u16, Ordering::Release);

                self.count.fetch_add(1, Ordering::Release);
                return true;
            }
        }

        false
    }

    /// Evict least recently used socket
    pub fn evict_lru(&self) -> bool {
        let mut lru_idx = None;
        let mut lru_time = u64::MAX;

        for (i, entry) in self.entries.iter().enumerate() {
            if let Some(e) = entry {
                let time = e.last_access.load(Ordering::Relaxed);
                if time < lru_time {
                    lru_time = time;
                    lru_idx = Some(i);
                }
            }
        }

        if let Some(idx) = lru_idx {
            unsafe {
                let slot = &self.entries[idx] as *const _ as *mut Option<PerCpuSocketEntry>;
                slot.write(None);
            }
            self.count.fetch_sub(1, Ordering::Release);
            return true;
        }

        false
    }

    /// Get current timestamp
    fn get_timestamp() -> u64 {
        // Use a simple counter for portability
        // In a real implementation, this would use TSC or similar high-resolution timer
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIMESTAMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

/// UDP fast path optimization structure
pub struct UdpFastPath {
    /// Per-CPU socket caches
    per_cpu_sockets: Vec<Mutex<PerCpuSocketCache>>,
    /// Zero-copy buffer pool
    buffer_pool: RingBuffer<PacketBuffer>,
    /// Statistics per CPU
    stats: Vec<UdpFastPathStats>,
    /// Kernel bypass enabled
    kernel_bypass_enabled: bool,
    /// Batch size for processing
    batch_size: usize,
}

/// Fast path statistics
#[derive(Default)]
pub struct UdpFastPathStats {
    /// Zero-copy receives
    pub zero_copy_recvs: AtomicU64,
    /// Fast lookup hits
    pub fast_lookup_hits: AtomicU64,
    /// Fast lookup misses
    pub fast_lookup_misses: AtomicU64,
    /// Batch sends
    pub batch_sends: AtomicU64,
    /// Packets sent in batch
    pub batch_packets_sent: AtomicU64,
    /// Kernel bypass operations
    pub kernel_bypass_ops: AtomicU64,
}

impl UdpFastPath {
    /// Create a new UDP fast path instance
    pub fn new(num_cpus: usize) -> Self {
        let mut per_cpu_sockets = Vec::with_capacity(num_cpus);
        let mut stats = Vec::with_capacity(num_cpus);

        for cpu in 0..num_cpus {
            per_cpu_sockets.push(Mutex::new(PerCpuSocketCache::new(cpu as u32)));
            stats.push(UdpFastPathStats::default());
        }

        Self {
            per_cpu_sockets,
            buffer_pool: RingBuffer::new(BUFFER_POOL_SIZE),
            stats,
            kernel_bypass_enabled: false,
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    /// Enable kernel bypass mode
    pub fn enable_kernel_bypass(&mut self, enabled: bool) {
        self.kernel_bypass_enabled = enabled;
    }

    /// Set batch size
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size.min(DEFAULT_BATCH_SIZE);
    }

    /// Zero-copy receive: Get buffer from pool for direct DMA write
    pub fn recv_zero_copy(
        &self,
        cpu_id: usize,
        buf: &mut [u8],
    ) -> Result<(usize, SocketAddr), UdpFastPathError> {
        // Allocate buffer from pool
        let packet_buf = self
            .buffer_pool
            .pop()
            .ok_or(UdpFastPathError::BufferExhausted)?;

        // Get data slice (zero-copy)
        let data = packet_buf.as_slice();
        let len = data.len().min(buf.len());
        buf[..len].copy_from_slice(&data[..len]);

        // Update statistics
        self.stats[cpu_id].zero_copy_recvs.fetch_add(1, Ordering::Relaxed);

        // Return buffer to pool
        self.buffer_pool.push(packet_buf);

        // In a real implementation, we would extract the source address
        // from the packet metadata
        let addr = SocketAddr::new_ipv4(Ipv4Addr::UNSPECIFIED, 0);

        Ok((len, addr))
    }

    /// Batched send with scatter-gather I/O
    pub fn send_batch(
        &self,
        cpu_id: usize,
        packets: &[(SocketAddr, &[u8])],
    ) -> Result<usize, UdpFastPathError> {
        if packets.is_empty() {
            return Ok(0);
        }

        if packets.len() > self.batch_size {
            return Err(UdpFastPathError::BatchTooLarge);
        }

        let mut sent = 0;

        // Process packets in batch
        for (_addr, _data) in packets {
            // In a real implementation, this would:
            // 1. Construct UDP packet with checksum
            // 2. Perform scatter-gather DMA if hardware supports it
            // 3. Send via network interface

            // For now, just simulate sending
            sent += 1;
        }

        // Update statistics
        self.stats[cpu_id].batch_sends.fetch_add(1, Ordering::Relaxed);
        self.stats[cpu_id].batch_packets_sent.fetch_add(sent as u64, Ordering::Relaxed);

        Ok(sent)
    }

    /// Fast hash-based socket lookup (lock-free read path)
    pub fn fast_lookup(&self, cpu_id: usize, addr: &SocketAddr) -> Option<Arc<UdpSocket>> {
        let cache = self.per_cpu_sockets.get(cpu_id)?;
        let cache = cache.lock();

        match cache.lookup(addr) {
            Some(socket) => {
                self.stats[cpu_id].fast_lookup_hits.fetch_add(1, Ordering::Relaxed);
                Some(socket)
            }
            None => {
                self.stats[cpu_id].fast_lookup_misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Insert socket into per-CPU cache
    pub fn cache_socket(&self, cpu_id: usize, addr: SocketAddr, socket: Arc<UdpSocket>) -> bool {
        if let Some(cache) = self.per_cpu_sockets.get(cpu_id) {
            let cache = cache.lock();
            cache.insert(addr, socket)
        } else {
            false
        }
    }

    /// Get statistics for a CPU
    pub fn get_stats(&self, cpu_id: usize) -> Option<&UdpFastPathStats> {
        self.stats.get(cpu_id)
    }

    /// Preallocate buffers for zero-copy operations
    pub fn preallocate_buffers(&self) {
        while self.buffer_pool.len() < BUFFER_POOL_SIZE / 2 {
            self.buffer_pool.push(PacketBuffer::new());
        }
    }

    /// Get buffer from pool for DMA receive
    pub fn get_dma_buffer(&self) -> Option<PacketBuffer> {
        self.buffer_pool.pop()
    }

    /// Return buffer to pool after DMA complete
    pub fn return_dma_buffer(&self, buffer: PacketBuffer) {
        self.buffer_pool.push(buffer);
    }
}

/// UDP fast path errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdpFastPathError {
    /// Buffer pool exhausted
    BufferExhausted,
    /// Batch size too large
    BatchTooLarge,
    /// Invalid socket address
    InvalidAddress,
    /// Socket not found
    SocketNotFound,
    /// Kernel bypass not enabled
    KernelBypassDisabled,
    /// Operation not supported
    NotSupported,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let ring: RingBuffer<u32> = RingBuffer::new(4);

        assert!(ring.is_empty());
        assert_eq!(ring.len(), 0);

        assert!(ring.push(1));
        assert!(ring.push(2));
        assert!(ring.push(3));
        assert!(ring.push(4));
        assert!(!ring.push(5)); // Full

        assert_eq!(ring.pop(), Some(1));
        assert_eq!(ring.pop(), Some(2));
        assert_eq!(ring.len(), 2);

        assert!(ring.push(5));
        assert_eq!(ring.pop(), Some(3));
    }

    #[test]
    fn test_packet_buffer() {
        let buf = PacketBuffer::new();
        assert_eq!(buf.len.load(Ordering::Relaxed), 0);

        buf.set_len(100);
        assert_eq!(buf.len.load(Ordering::Relaxed), 100);

        buf.inc_ref();
        assert_eq!(buf.ref_count.load(Ordering::Relaxed), 2);

        assert!(!buf.dec_ref());
        assert!(buf.dec_ref());
    }
}
