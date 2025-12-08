//! Network packet buffer management
//!
//! This module provides efficient packet buffer management with zero-copy
//! optimizations and proper memory alignment for DMA operations.

extern crate alloc;
use alloc::vec::Vec;
use core::ptr::NonNull;
use core::slice;

/// Maximum packet size (including headers)
pub const MAX_PACKET_SIZE: usize = 1518; // Ethernet MTU + headers

/// Packet buffer for network data
pub struct PacketBuffer {
    /// Raw buffer data
    data: NonNull<u8>,
    /// Buffer capacity
    capacity: usize,
    /// Current data length
    length: usize,
    /// Current read/write offsets
    read_offset: usize,
    write_offset: usize,
}

impl PacketBuffer {
    /// Create a new packet buffer with the given capacity
    pub fn new(capacity: usize) -> Result<Self, PacketError> {
        if capacity > MAX_PACKET_SIZE {
            return Err(PacketError::InvalidSize);
        }

        let layout = alloc::alloc::Layout::from_size_align(
            capacity,
            core::mem::align_of::<u8>()
        ).map_err(|_| PacketError::InvalidSize)?;

        let ptr = unsafe { alloc::alloc::alloc(layout) };
        if ptr.is_null() {
            return Err(PacketError::AllocationFailed);
        }

        Ok(Self {
            data: unsafe { NonNull::new_unchecked(ptr) },
            capacity,
            length: 0,
            read_offset: 0,
            write_offset: 0,
        })
    }

    /// Create a packet buffer from existing data
    pub fn from_bytes(data: &[u8]) -> Result<Self, PacketError> {
        let mut buffer = Self::new(data.len())?;
        buffer.write_bytes(data);
        Ok(buffer)
    }

    /// Get the raw data pointer
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Get the mutable data pointer
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_ptr()
    }

    /// Get the current data length
    pub fn len(&self) -> usize {
        self.length
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get remaining space in buffer
    pub fn remaining(&self) -> usize {
        self.capacity - self.write_offset
    }

    /// Write bytes to the buffer
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<usize, PacketError> {
        let available = self.remaining();
        let to_write = core::cmp::min(data.len(), available);

        if to_write == 0 {
            return Ok(0);
        }

        unsafe {
            let dst = self.data.as_ptr().add(self.write_offset);
            core::ptr::copy_nonoverlapping(data.as_ptr(), dst, to_write);
        }

        self.write_offset += to_write;
        self.length = core::cmp::max(self.length, self.write_offset);

        Ok(to_write)
    }

    /// Read bytes from the buffer
    pub fn read_bytes(&mut self, buf: &mut [u8]) -> Result<usize, PacketError> {
        let available = self.length - self.read_offset;
        let to_read = core::cmp::min(buf.len(), available);

        if to_read == 0 {
            return Ok(0);
        }

        unsafe {
            let src = self.data.as_ptr().add(self.read_offset);
            core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), to_read);
        }

        self.read_offset += to_read;

        Ok(to_read)
    }

    /// Get a slice view of the data
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self.data.as_ptr(), self.length)
        }
    }

    /// Get a mutable slice view of the data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.data.as_ptr() as *mut u8, self.length)
        }
    }

    /// Reset the buffer (clear all data)
    pub fn reset(&mut self) {
        self.length = 0;
        self.read_offset = 0;
        self.write_offset = 0;
    }

    /// Zero-copy: Get reference to internal buffer for direct DMA access
    /// This allows network devices to write directly to the buffer
    pub fn as_dma_buffer(&mut self) -> (*mut u8, usize) {
        (self.data.as_ptr() as *mut u8, self.capacity)
    }

    /// Zero-copy: Set length after DMA write
    /// This is used when data is written directly by hardware
    pub unsafe fn set_length(&mut self, length: usize) {
        if length <= self.capacity {
            self.length = length;
            self.write_offset = length;
        }
    }

    /// Zero-copy: Prepare buffer for DMA read
    /// Returns the DMA address and length
    pub fn prepare_for_dma_read(&mut self) -> (*mut u8, usize) {
        let offset = self.read_offset;
        let remaining = self.length - offset;
        (unsafe { self.data.as_ptr().add(offset) as *mut u8 }, remaining)
    }

    /// Reserve space at the beginning of the buffer for headers
    pub fn reserve_header_space(&mut self, header_size: usize) -> Result<(), PacketError> {
        if header_size > self.capacity {
            return Err(PacketError::InvalidSize);
        }

        // Shift existing data to make room for header
        if self.length > 0 {
            unsafe {
                let src = self.data.as_ptr();
                let dst = self.data.as_ptr().add(header_size);
                core::ptr::copy(src, dst, self.length);
            }
        }

        self.write_offset += header_size;
        self.length += header_size;
        self.read_offset = 0;

        Ok(())
    }

    /// Trim the buffer to the specified length
    pub fn trim(&mut self, new_len: usize) {
        if new_len <= self.length {
            self.length = new_len;
            if self.read_offset > new_len {
                self.read_offset = new_len;
            }
            if self.write_offset > new_len {
                self.write_offset = new_len;
            }
        }
    }
}

impl Drop for PacketBuffer {
    fn drop(&mut self) {
        unsafe {
            let layout = alloc::alloc::Layout::from_size_align(
                self.capacity,
                core::mem::align_of::<u8>()
            ).unwrap();
            alloc::alloc::dealloc(self.data.as_ptr(), layout);
        }
    }
}

/// Network packet with metadata
pub struct Packet {
    /// Packet data buffer
    buffer: PacketBuffer,
    /// Packet type
    packet_type: PacketType,
    /// Source interface ID
    interface_id: Option<u32>,
    /// Timestamp when packet was received
    timestamp: u64,
    /// Human-friendly protocol identifier (e.g. "TCP", "UDP")
    pub protocol: alloc::string::String,
    /// Source IP address (string form)
    pub src_ip: alloc::string::String,
    /// Destination IP address (string form)
    pub dst_ip: alloc::string::String,
    /// Source port (if applicable)
    pub src_port: u16,
    /// Destination port (if applicable)
    pub dst_port: u16,
    /// Simplified TCP flags representation (e.g. "SYN,ACK")
    pub tcp_flags: alloc::string::String,
    /// Convenience payload copy of current packet data
    pub payload: Vec<u8>,
    /// Cached size of packet (in bytes)
    pub size: usize,
}

impl Packet {
    /// Create a new empty packet
    pub fn new(packet_type: PacketType) -> Result<Self, PacketError> {
        let buffer = PacketBuffer::new(MAX_PACKET_SIZE)?;
        Ok(Self {
            buffer,
            packet_type,
            interface_id: None,
            timestamp: 0,
            protocol: match packet_type {
                PacketType::Ethernet => alloc::string::String::from("ETHERNET"),
                PacketType::Arp => alloc::string::String::from("ARP"),
                PacketType::Ipv4 => alloc::string::String::from("IPv4"),
                PacketType::Icmp => alloc::string::String::from("ICMP"),
                PacketType::Udp => alloc::string::String::from("UDP"),
                PacketType::Tcp => alloc::string::String::from("TCP"),
                PacketType::Raw => alloc::string::String::from("RAW"),
            },
            src_ip: alloc::string::String::from("0.0.0.0"),
            dst_ip: alloc::string::String::from("0.0.0.0"),
            src_port: 0,
            dst_port: 0,
            tcp_flags: alloc::string::String::new(),
            payload: Vec::new(),
            size: 0,
        })
    }

    /// Create a packet from existing data
    pub fn from_bytes(data: &[u8], packet_type: PacketType) -> Result<Self, PacketError> {
        let buffer = PacketBuffer::from_bytes(data)?;
        Ok(Self {
            buffer,
            packet_type,
            interface_id: None,
            timestamp: 0,
            protocol: match packet_type {
                PacketType::Ethernet => alloc::string::String::from("ETHERNET"),
                PacketType::Arp => alloc::string::String::from("ARP"),
                PacketType::Ipv4 => alloc::string::String::from("IPv4"),
                PacketType::Icmp => alloc::string::String::from("ICMP"),
                PacketType::Udp => alloc::string::String::from("UDP"),
                PacketType::Tcp => alloc::string::String::from("TCP"),
                PacketType::Raw => alloc::string::String::from("RAW"),
            },
            // Best-effort defaults — higher-level code should parse protocol-specific headers
            src_ip: alloc::string::String::from("0.0.0.0"),
            dst_ip: alloc::string::String::from("0.0.0.0"),
            src_port: 0,
            dst_port: 0,
            tcp_flags: alloc::string::String::new(),
            payload: data.to_vec(),
            size: data.len(),
        })
    }

    /// Get the packet type
    pub fn packet_type(&self) -> PacketType {
        self.packet_type
    }

    /// Set the packet type
    pub fn set_packet_type(&mut self, packet_type: PacketType) {
        self.packet_type = packet_type;
    }

    /// Get the interface ID
    pub fn interface_id(&self) -> Option<u32> {
        self.interface_id
    }

    /// Set the interface ID
    pub fn set_interface_id(&mut self, interface_id: u32) {
        self.interface_id = Some(interface_id);
    }

    /// Get the packet timestamp
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Set the packet timestamp
    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.timestamp = timestamp;
    }

    /// Get packet data as slice
    pub fn data(&self) -> &[u8] {
        self.buffer.as_slice()
    }

    /// Get packet data as mutable slice
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.buffer.as_mut_slice()
    }

    /// Get packet length
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Get raw buffer pointer
    pub fn as_ptr(&self) -> *const u8 {
        self.buffer.as_ptr()
    }

    /// Get mutable raw buffer pointer
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr()
    }

    /// Reserve space for packet headers
    pub fn reserve_headers(&mut self, header_size: usize) -> Result<(), PacketError> {
        self.buffer.reserve_header_space(header_size)
    }

    /// Append data to packet
    pub fn append(&mut self, data: &[u8]) -> Result<usize, PacketError> {
        self.buffer.write_bytes(data)
            .map(|written| {
                // Keep convenience fields in sync
                let _ = self.payload.extend_from_slice(&data[..written]);
                self.size = self.buffer.len();
                written
            })
    }

    /// Trim packet to specified length
    pub fn trim(&mut self, new_len: usize) {
        self.buffer.trim(new_len);
        // truncate cached payload and size
        if self.payload.len() > new_len {
            self.payload.truncate(new_len);
        }
        self.size = self.buffer.len();
    }

    /// Clone the packet (creates a new buffer with the same data)
    pub fn clone_packet(&self) -> Result<Self, PacketError> {
        let mut p = Self::from_bytes(self.data(), self.packet_type)?;
        // copy metadata convenience fields
        p.protocol = self.protocol.clone();
        p.src_ip = self.src_ip.clone();
        p.dst_ip = self.dst_ip.clone();
        p.src_port = self.src_port;
        p.dst_port = self.dst_port;
        p.tcp_flags = self.tcp_flags.clone();
        // payload and size already set by from_bytes
        Ok(p)
    }
}

impl core::fmt::Debug for Packet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Packet")
            .field("packet_type", &self.packet_type)
            .field("interface_id", &self.interface_id)
            .field("timestamp", &self.timestamp)
            .field("data_length", &self.buffer.len())
            .finish()
    }
}

impl Clone for Packet {
    fn clone(&self) -> Self {
        self.clone_packet().expect("Failed to clone packet")
    }
}

/// Packet type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// Ethernet frame
    Ethernet,
    /// ARP packet
    Arp,
    /// IPv4 packet
    Ipv4,
    /// ICMP packet
    Icmp,
    /// UDP packet
    Udp,
    /// TCP packet
    Tcp,
    /// Raw packet
    Raw,
}

/// Packet buffer pool for efficient memory management
pub struct PacketPool {
    /// Free packet buffers
    free_buffers: Vec<PacketBuffer>,
    /// Total allocated buffers
    total_buffers: usize,
    /// Maximum pool size
    max_size: usize,
}

impl PacketPool {
    /// Create a new packet pool
    pub fn new() -> Self {
        Self {
            free_buffers: Vec::new(),
            total_buffers: 0,
            max_size: 1000, // Configurable maximum pool size
        }
    }

    /// Initialize the packet pool with pre-allocated buffers
    pub fn init(&mut self) {
        // Pre-allocate some buffers for better performance
        for _ in 0..50 {
            if let Ok(buffer) = PacketBuffer::new(MAX_PACKET_SIZE) {
                self.free_buffers.push(buffer);
                self.total_buffers += 1;
            }
        }

        crate::log_info!("Packet pool initialized with {} buffers", self.free_buffers.len());
    }

    /// Allocate a packet buffer from the pool
    pub fn allocate(&mut self) -> Result<PacketBuffer, PacketError> {
        if let Some(mut buffer) = self.free_buffers.pop() {
            buffer.reset(); // Clear any existing data
            Ok(buffer)
        } else if self.total_buffers < self.max_size {
            // Allocate a new buffer if we haven't reached the maximum
            let buffer = PacketBuffer::new(MAX_PACKET_SIZE)?;
            self.total_buffers += 1;
            Ok(buffer)
        } else {
            Err(PacketError::PoolExhausted)
        }
    }

    /// Return a packet buffer to the pool
    pub fn deallocate(&mut self, buffer: PacketBuffer) {
        if self.free_buffers.len() < self.max_size {
            self.free_buffers.push(buffer);
        }
        // If pool is full, buffer will be dropped and memory freed
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            free_buffers: self.free_buffers.len(),
            total_buffers: self.total_buffers,
            max_size: self.max_size,
        }
    }
}

/// Packet pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of free buffers in pool
    pub free_buffers: usize,
    /// Total number of allocated buffers
    pub total_buffers: usize,
    /// Maximum pool size
    pub max_size: usize,
}

/// Packet buffer errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PacketError {
    /// Invalid buffer size
    InvalidSize,
    /// Memory allocation failed
    AllocationFailed,
    /// Packet pool exhausted
    PoolExhausted,
    /// Buffer overflow
    Overflow,
    /// Invalid operation
    InvalidOperation,
}
