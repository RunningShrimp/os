//! Zero-copy network I/O implementation
//!
//! This module provides zero-copy network I/O functionality to improve performance
//! by reducing memory copies between kernel and user space.

use alloc::vec::Vec;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use nos_api::{Result, Error};

/// Zero-copy buffer descriptor
#[derive(Debug, Clone)]
pub struct ZeroCopyBuffer {
    /// Physical address of the buffer
    pub physical_addr: usize,
    /// Virtual address of the buffer
    pub virtual_addr: usize,
    /// Size of the buffer
    pub size: usize,
    /// Buffer flags
    pub flags: BufferFlags,
}

/// Buffer flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferFlags {
    /// Buffer is read-only
    pub read_only: bool,
    /// Buffer is write-only
    pub write_only: bool,
    /// Buffer is mapped for DMA
    pub dma_mapped: bool,
    /// Buffer is pinned in memory
    pub pinned: bool,
}

impl Default for BufferFlags {
    fn default() -> Self {
        Self {
            read_only: false,
            write_only: false,
            dma_mapped: false,
            pinned: false,
        }
    }
}

/// Zero-copy network packet
#[derive(Debug)]
pub struct ZeroCopyPacket {
    /// Packet header
    pub header: PacketHeader,
    /// Zero-copy buffer for packet data
    pub buffer: ZeroCopyBuffer,
    /// Packet metadata
    pub metadata: PacketMetadata,
}

/// Packet header
#[derive(Debug, Clone, Copy)]
pub struct PacketHeader {
    /// Source address
    pub src_addr: u32,
    /// Destination address
    pub dst_addr: u32,
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Protocol
    pub protocol: u8,
    /// Packet length
    pub length: u16,
    /// Checksum
    pub checksum: u16,
}

/// Packet metadata
#[derive(Debug, Clone)]
pub struct PacketMetadata {
    /// Timestamp when packet was received
    pub timestamp: u64,
    /// Queue depth at reception
    pub queue_depth: u32,
    /// Processing priority
    pub priority: u8,
    /// Number of hops
    pub hops: u8,
}

/// Zero-copy socket state
#[derive(Debug)]
pub struct ZeroCopySocket {
    /// Socket file descriptor
    pub fd: i32,
    /// Socket type
    pub socket_type: SocketType,
    /// Receive buffer queue
    pub recv_queue: Arc<Mutex<VecDeque<ZeroCopyPacket>>>,
    /// Send buffer queue
    pub send_queue: Arc<Mutex<VecDeque<ZeroCopyPacket>>>,
    /// Socket state
    pub state: SocketState,
    /// Zero-copy configuration
    pub config: ZeroCopyConfig,
}

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// Stream socket (TCP)
    Stream,
    /// Datagram socket (UDP)
    Datagram,
    /// Raw socket
    Raw,
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    /// Socket is unconnected
    Unconnected,
    /// Socket is connecting
    Connecting,
    /// Socket is connected
    Connected,
    /// Socket is listening
    Listening,
    /// Socket is closed
    Closed,
}

/// Zero-copy configuration
#[derive(Debug, Clone)]
pub struct ZeroCopyConfig {
    /// Enable zero-copy receive
    pub enable_zero_copy_recv: bool,
    /// Enable zero-copy send
    pub enable_zero_copy_send: bool,
    /// Buffer pool size
    pub buffer_pool_size: usize,
    /// Maximum packet size
    pub max_packet_size: usize,
    /// Enable DMA
    pub enable_dma: bool,
    /// Prefetch buffer count
    pub prefetch_count: usize,
}

impl Default for ZeroCopyConfig {
    fn default() -> Self {
        Self {
            enable_zero_copy_recv: true,
            enable_zero_copy_send: true,
            buffer_pool_size: 1024,
            max_packet_size: 65536,
            enable_dma: true,
            prefetch_count: 4,
        }
    }
}

/// Zero-copy buffer pool
#[derive(Debug)]
pub struct ZeroCopyBufferPool {
    /// Available buffers
    pub available_buffers: Arc<Mutex<Vec<ZeroCopyBuffer>>>,
    /// Used buffers
    pub used_buffers: Arc<Mutex<Vec<ZeroCopyBuffer>>>,
    /// Total buffer size
    pub buffer_size: usize,
    /// Number of buffers
    pub buffer_count: usize,
}

impl ZeroCopyBufferPool {
    /// Create a new zero-copy buffer pool
    pub fn new(buffer_count: usize, buffer_size: usize) -> Result<Self> {
        let mut available_buffers = Vec::with_capacity(buffer_count);
        
        // Allocate buffers (in a real implementation, this would allocate physical memory)
        for i in 0..buffer_count {
            let buffer = ZeroCopyBuffer {
                physical_addr: 0x1000000 + i * buffer_size, // Mock physical address
                virtual_addr: 0x8000000 + i * buffer_size,  // Mock virtual address
                size: buffer_size,
                flags: BufferFlags::default(),
            };
            available_buffers.push(buffer);
        }
        
        Ok(Self {
            available_buffers: Arc::new(Mutex::new(available_buffers)),
            used_buffers: Arc::new(Mutex::new(Vec::new())),
            buffer_size,
            buffer_count,
        })
    }
    
    /// Allocate a buffer from the pool
    pub fn allocate(&self) -> Result<ZeroCopyBuffer> {
        let mut available = self.available_buffers.lock();
        if let Some(buffer) = available.pop() {
            let mut used = self.used_buffers.lock();
            used.push(buffer.clone());
            Ok(buffer)
        } else {
            Err(Error::ResourceExhausted("No available buffers in pool".to_string()))
        }
    }
    
    /// Deallocate a buffer back to the pool
    pub fn deallocate(&self, buffer: ZeroCopyBuffer) -> Result<()> {
        let mut used = self.used_buffers.lock();
        if let Some(pos) = used.iter().position(|b| b.physical_addr == buffer.physical_addr) {
            let buffer = used.remove(pos);
            drop(used);
            
            let mut available = self.available_buffers.lock();
            available.push(buffer);
            Ok(())
        } else {
            Err(Error::InvalidArgument("Buffer not from this pool".to_string()))
        }
    }
    
    /// Get the number of available buffers
    pub fn available_count(&self) -> usize {
        self.available_buffers.lock().len()
    }
    
    /// Get the number of used buffers
    pub fn used_count(&self) -> usize {
        self.used_buffers.lock().len()
    }
}

/// Zero-copy network I/O manager
#[derive(Debug)]
pub struct ZeroCopyNetworkManager {
    /// Buffer pool for zero-copy operations
    pub buffer_pool: Arc<ZeroCopyBufferPool>,
    /// Active sockets
    pub sockets: Arc<Mutex<Vec<ZeroCopySocket>>>,
    /// DMA engine for zero-copy transfers
    pub dma_engine: Option<DmaEngine>,
    /// Performance statistics
    pub stats: Arc<Mutex<ZeroCopyStats>>,
}

/// DMA engine for zero-copy transfers
#[derive(Debug)]
pub struct DmaEngine {
    /// DMA channel ID
    pub channel_id: u32,
    /// DMA engine state
    pub state: DmaState,
    /// Transfer queue
    pub transfer_queue: Arc<Mutex<Vec<DmaTransfer>>>,
}

/// DMA state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaState {
    /// DMA engine is idle
    Idle,
    /// DMA engine is transferring
    Transferring,
    /// DMA engine is paused
    Paused,
    /// DMA engine has an error
    Error,
}

/// DMA transfer descriptor
#[derive(Debug, Clone)]
pub struct DmaTransfer {
    /// Transfer ID
    pub transfer_id: u32,
    /// Source address
    pub src_addr: usize,
    /// Destination address
    pub dst_addr: usize,
    /// Transfer size
    pub size: usize,
    /// Transfer direction
    pub direction: DmaDirection,
    /// Transfer status
    pub status: DmaTransferStatus,
}

/// DMA transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// Memory to device
    MemoryToDevice,
    /// Device to memory
    DeviceToMemory,
    /// Memory to memory
    MemoryToMemory,
}

/// DMA transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaTransferStatus {
    /// Transfer is pending
    Pending,
    /// Transfer is in progress
    InProgress,
    /// Transfer completed successfully
    Completed,
    /// Transfer failed
    Failed,
}

/// Zero-copy performance statistics
#[derive(Debug, Default, Clone)]
pub struct ZeroCopyStats {
    /// Number of zero-copy receives
    pub zero_copy_recv_count: u64,
    /// Number of zero-copy sends
    pub zero_copy_send_count: u64,
    /// Number of fallback copies
    pub fallback_copy_count: u64,
    /// Total bytes transferred via zero-copy
    pub zero_copy_bytes: u64,
    /// Average zero-copy transfer time
    pub avg_transfer_time: u64,
    /// Buffer allocation failures
    pub allocation_failures: u64,
    /// DMA transfer count
    pub dma_transfers: u64,
}

impl ZeroCopyNetworkManager {
    /// Create a new zero-copy network manager
    pub fn new(config: ZeroCopyConfig) -> Result<Self> {
        let buffer_pool = Arc::new(ZeroCopyBufferPool::new(
            config.buffer_pool_size,
            config.max_packet_size
        )?);
        
        let dma_engine = if config.enable_dma {
            Some(DmaEngine {
                channel_id: 0,
                state: DmaState::Idle,
                transfer_queue: Arc::new(Mutex::new(Vec::new())),
            })
        } else {
            None
        };
        
        Ok(Self {
            buffer_pool,
            sockets: Arc::new(Mutex::new(Vec::new())),
            dma_engine,
            stats: Arc::new(Mutex::new(ZeroCopyStats::default())),
        })
    }
    
    /// Create a new zero-copy socket
    pub fn create_socket(&self, socket_type: SocketType, config: ZeroCopyConfig) -> Result<i32> {
        let mut sockets = self.sockets.lock();
        let fd = sockets.len() as i32;
        
        let socket = ZeroCopySocket {
            fd,
            socket_type,
            recv_queue: Arc::new(Mutex::new(VecDeque::new())),
            send_queue: Arc::new(Mutex::new(VecDeque::new())),
            state: SocketState::Unconnected,
            config,
        };
        
        sockets.push(socket);
        Ok(fd)
    }
    
    /// Send data using zero-copy
    pub fn send_zero_copy(&self, fd: i32, buffer: ZeroCopyBuffer) -> Result<usize> {
        let sockets = self.sockets.lock();
        if let Some(socket) = sockets.iter().find(|s| s.fd == fd) {
            if !socket.config.enable_zero_copy_send {
                return self.send_fallback(fd, buffer);
            }
            
            // Create packet
            let packet = ZeroCopyPacket {
                header: PacketHeader {
                    src_addr: 0,
                    dst_addr: 0,
                    src_port: 0,
                    dst_port: 0,
                    protocol: match socket.socket_type {
                        SocketType::Stream => 6,  // TCP
                        SocketType::Datagram => 17, // UDP
                        SocketType::Raw => 255,
                    },
                    length: buffer.size as u16,
                    checksum: 0,
                },
                buffer: buffer.clone(),
                metadata: PacketMetadata {
                    timestamp: self.get_timestamp(),
                    queue_depth: socket.recv_queue.lock().len() as u32,
                    priority: 0,
                    hops: 0,
                },
            };
            
            // Add to send queue
            let mut send_queue = socket.send_queue.lock();
            send_queue.push_back(packet);
            
            // Update statistics
            let mut stats = self.stats.lock();
            stats.zero_copy_send_count += 1;
            stats.zero_copy_bytes += buffer.size as u64;
            
            // Use DMA if available
            if let Some(ref dma_engine) = self.dma_engine {
                self.schedule_dma_transfer(dma_engine, &buffer, DmaDirection::MemoryToDevice)?;
                stats.dma_transfers += 1;
            }
            
            Ok(buffer.size)
        } else {
            Err(Error::NotFound("Socket not found".to_string()))
        }
    }
    
    /// Receive data using zero-copy
    pub fn recv_zero_copy(&self, fd: i32) -> Result<ZeroCopyPacket> {
        let sockets = self.sockets.lock();
        if let Some(socket) = sockets.iter().find(|s| s.fd == fd) {
            if !socket.config.enable_zero_copy_recv {
                return self.recv_fallback(fd);
            }
            
            // Try to get packet from receive queue
            let mut recv_queue = socket.recv_queue.lock();
            if let Some(packet) = recv_queue.pop_front() {
                // Update statistics
                let mut stats = self.stats.lock();
                stats.zero_copy_recv_count += 1;
                stats.zero_copy_bytes += packet.buffer.size as u64;
                
                Ok(packet)
            } else {
                Err(Error::ResourceExhausted("No packets available".to_string()))
            }
        } else {
            Err(Error::NotFound("Socket not found".to_string()))
        }
    }
    
    /// Fallback send method using traditional copying
    fn send_fallback(&self, fd: i32, buffer: ZeroCopyBuffer) -> Result<usize> {
        // In a real implementation, this would copy data to kernel buffers
        let mut stats = self.stats.lock();
        stats.fallback_copy_count += 1;
        Ok(buffer.size)
    }
    
    /// Fallback receive method using traditional copying
    fn recv_fallback(&self, fd: i32) -> Result<ZeroCopyPacket> {
        // In a real implementation, this would create a packet and copy data
        let buffer = self.buffer_pool.allocate()?;
        let packet = ZeroCopyPacket {
            header: PacketHeader {
                src_addr: 0,
                dst_addr: 0,
                src_port: 0,
                dst_port: 0,
                protocol: 6,
                length: buffer.size as u16,
                checksum: 0,
            },
            buffer,
            metadata: PacketMetadata {
                timestamp: self.get_timestamp(),
                queue_depth: 0,
                priority: 0,
                hops: 0,
            },
        };
        
        let mut stats = self.stats.lock();
        stats.fallback_copy_count += 1;
        
        Ok(packet)
    }
    
    /// Schedule a DMA transfer
    fn schedule_dma_transfer(&self, dma_engine: &DmaEngine, buffer: &ZeroCopyBuffer, direction: DmaDirection) -> Result<()> {
        let transfer = DmaTransfer {
            transfer_id: self.get_timestamp() as u32,
            src_addr: match direction {
                DmaDirection::MemoryToDevice => buffer.virtual_addr,
                DmaDirection::DeviceToMemory => buffer.physical_addr,
                DmaDirection::MemoryToMemory => buffer.virtual_addr,
            },
            dst_addr: match direction {
                DmaDirection::MemoryToDevice => buffer.physical_addr,
                DmaDirection::DeviceToMemory => buffer.virtual_addr,
                DmaDirection::MemoryToMemory => buffer.virtual_addr + buffer.size,
            },
            size: buffer.size,
            direction,
            status: DmaTransferStatus::Pending,
        };
        
        let mut queue = dma_engine.transfer_queue.lock();
        queue.push(transfer);
        
        Ok(())
    }
    
    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        // In a real implementation, this would get the actual timestamp
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIMESTAMP: AtomicU64 = AtomicU64::new(0);
        TIMESTAMP.fetch_add(1, Ordering::Relaxed)
    }
    
    /// Get performance statistics
    pub fn get_stats(&self) -> ZeroCopyStats {
        self.stats.lock().clone()
    }
    
    /// Reset performance statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = ZeroCopyStats::default();
    }
}

/// Zero-copy network I/O system call handler
pub struct ZeroCopySendHandler {
    manager: Arc<ZeroCopyNetworkManager>,
}

impl ZeroCopySendHandler {
    /// Create a new zero-copy send handler
    pub fn new(manager: Arc<ZeroCopyNetworkManager>) -> Self {
        Self { manager }
    }
}

impl crate::core::SyscallHandler for ZeroCopySendHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ZERO_COPY_SEND
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            return Err(Error::InvalidArgument("Insufficient arguments for zero-copy send".to_string()));
        }
        
        let fd = args[0] as i32;
        let buffer_addr = args[1];
        let buffer_size = args[2];
        
        // Create zero-copy buffer from user address
        let buffer = ZeroCopyBuffer {
            physical_addr: buffer_addr, // In a real implementation, this would translate virtual to physical
            virtual_addr: buffer_addr,
            size: buffer_size,
            flags: BufferFlags::default(),
        };
        
        let bytes_sent = self.manager.send_zero_copy(fd, buffer)?;
        Ok(bytes_sent as isize)
    }
}

/// Zero-copy receive system call handler
pub struct ZeroCopyRecvHandler {
    manager: Arc<ZeroCopyNetworkManager>,
}

impl ZeroCopyRecvHandler {
    /// Create a new zero-copy receive handler
    pub fn new(manager: Arc<ZeroCopyNetworkManager>) -> Self {
        Self { manager }
    }
}

impl crate::core::SyscallHandler for ZeroCopyRecvHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ZERO_COPY_RECV
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 2 {
            return Err(Error::InvalidArgument("Insufficient arguments for zero-copy receive".to_string()));
        }
        
        let fd = args[0] as i32;
        let buffer_addr = args[1];
        
        let packet = self.manager.recv_zero_copy(fd)?;
        
        // In a real implementation, this would map the packet buffer to user space
        // For now, just return the packet size
        Ok(packet.buffer.size as isize)
    }
}