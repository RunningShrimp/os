//! # Zero-Copy I/O for High-Performance Networking
//!
//! This module provides zero-copy I/O primitives for network operations,
//! eliminating unnecessary memory copies and improving throughput.
//!
//! ## Overview
//!
//! Traditional network I/O involves multiple memory copies:
//! 1. Network card → kernel buffer
//! 2. Kernel buffer → user buffer
//! 3. User buffer → kernel buffer (for sending)
//! 4. Kernel buffer → network card
//!
//! Zero-copy I/O eliminates these copies by:
//! - Using DMA to transfer data directly between network hardware and user memory
//! - Passing buffer references instead of copying data
//! - Sharing memory mappings between kernel and userspace
//!
//! ## Key Components
//!
//! - [`ZeroCopyBuffer`]: A buffer that can be shared without copying
//! - [`ZeroCopyPacket`]: Network packet with zero-copy metadata
//! - [`ZeroCopySocket`]: Socket type supporting zero-copy operations
//!
//! ## Usage Example
//!
//! ```no_run
//! use kernel::network::zero_copy_io::{ZeroCopySocket, SocketType};
//!
//! // Create a zero-copy socket
//! let socket = ZeroCopySocket::new(SocketType::Stream)?;
//!
//! // Send data without copying
//! let buffer = allocate_buffer(4096);
//! socket.send_zero_copy(buffer)?;
//!
//! // Receive data without copying
//! let received = socket.recv_zero_copy()?;
//! process_packet(&received)?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ## Performance Benefits
//!
//! - **Reduced CPU usage**: No memcpy operations
//! - **Lower latency**: Fewer memory operations
//! - **Higher throughput**: Cache-friendly access patterns
//! - **Better scalability**: Less memory bandwidth contention
//!
//! ## Security Considerations
//!
//! Zero-copy I/O requires careful security management:
//! - Buffer pinning prevents memory from being swapped
//! - DMA mapping exposes physical memory to devices
//! - Access control must be enforced on shared buffers
//!
//! ## Limitations
//!
//! - Requires hardware support (DMA, IOMMU)
//! - Buffer alignment requirements
//! - Complex memory management
//! - Not suitable for all workloads

use alloc::vec::Vec;
use alloc::collections::{VecDeque, HashMap};
use alloc::sync::Arc;
use spin::Mutex;
use nos_api::{Result, Error};

/// A zero-copy buffer that can be shared between kernel and userspace
/// or between kernel and hardware without memory copying.
///
/// # Fields
///
/// * `physical_addr` - Physical address of the buffer (for DMA operations)
/// * `virtual_addr` - Virtual address for CPU access
/// * `size` - Buffer size in bytes
/// * `flags` - Buffer flags (read-only, DMA-mapped, pinned, etc.)
///
/// # Example
///
/// ```no_run
/// use kernel::network::zero_copy_io::ZeroCopyBuffer;
///
/// let buffer = ZeroCopyBuffer {
///     physical_addr: 0x1000000,
///     virtual_addr: 0x7f0000000,
///     size: 4096,
///     flags: BufferFlags::default(),
/// };
/// ```
pub struct ZeroCopyBuffer {
    /// Physical address of the buffer
    pub physical_addr: usize,
    /// Virtual address of the buffer
    pub virtual_addr: usize,
    /// Buffer size in bytes
    pub size: usize,
    /// Buffer access and mapping flags
    pub flags: BufferFlags,
}

/// Flags describing zero-copy buffer properties and restrictions.
///
/// # Flags
///
/// * `read_only` - Buffer is read-only (writes will cause page faults)
/// * `write_only` - Buffer is write-only (reads will return garbage)
/// * `dma_mapped` - Buffer is mapped for DMA access
/// * `pinned` - Buffer is pinned in physical memory (won't be swapped)
///
/// # Example
///
/// ```
/// use kernel::network::zero_copy_io::BufferFlags;
///
/// let flags = BufferFlags {
///     read_only: false,
///     write_only: false,
///     dma_mapped: true,
///     pinned: true,
///     ..Default::default()
/// };
/// ```
pub struct BufferFlags {
    /// Buffer is read-only
    pub read_only: bool,
    /// Buffer is write-only
    pub write_only: bool,
    /// Buffer is mapped for DMA
    pub dma_mapped: bool,
    /// Buffer is pinned in physical memory
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

/// A zero-copy network packet containing header, data buffer, and metadata.
///
/// This structure represents a complete network packet that can be passed
/// through the network stack without copying the payload data.
///
/// # Components
///
/// * `header` - Network protocol header (IP, TCP, UDP, etc.)
/// * `buffer` - Zero-copy payload data
/// * `metadata` - Packet metadata (timestamp, priority, etc.)
///
/// # Example
///
/// ```no_run
/// use kernel::network::zero_copy_io::{ZeroCopyPacket, PacketHeader, ZeroCopyBuffer, PacketMetadata};
///
/// let packet = ZeroCopyPacket {
///     header: PacketHeader::default(),
///     buffer: ZeroCopyBuffer::default(),
///     metadata: PacketMetadata::default(),
/// };
/// ```
pub struct ZeroCopyPacket {
    /// Network protocol header
    pub header: PacketHeader,
    /// Zero-copy payload data
    pub buffer: ZeroCopyBuffer,
    /// Packet metadata
    pub metadata: PacketMetadata,
}

/// Network packet header containing addressing and protocol information.
///
/// This structure contains the standard fields found in network protocol headers
/// like IP addresses, ports, protocol type, and checksums.
///
/// # Fields
///
/// * `src_addr` - Source IP address
/// * `dst_addr` - Destination IP address
/// * `src_port` - Source port (for TCP/UDP)
/// * `dst_port` - Destination port (for TCP/UDP)
/// * `protocol` - Protocol number (6=TCP, 17=UDP, etc.)
/// * `length` - Packet payload length
/// * `checksum` - Packet checksum for integrity verification
///
/// # Example
///
/// ```
/// use kernel::network::zero_copy_io::PacketHeader;
///
/// let header = PacketHeader {
///     src_addr: 0xC0A80101, // 192.168.1.1
///     dst_addr: 0xC0A80102, // 192.168.1.2
///     src_port: 8080,
///     dst_port: 80,
///     protocol: 6, // TCP
///     length: 1024,
///     checksum: 0x1234,
/// };
/// ```
pub struct PacketHeader {
    /// Source IP address
    pub src_addr: u32,
    /// Destination IP address
    pub dst_addr: u32,
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Protocol number
    pub protocol: u8,
    /// Payload length
    pub length: u16,
    /// Checksum
    pub checksum: u16,
}

/// Packet metadata containing performance and routing information.
///
/// This structure tracks metadata about the packet's journey through
/// the network stack, including timing, queue state, and routing info.
///
/// # Fields
///
/// * `timestamp` - Packet creation/receipt timestamp (nanoseconds)
/// * `queue_depth` - Queue depth when packet was enqueued
/// * `priority` - Packet priority (0-255, higher = more important)
/// * `hops` - Number of hops the packet has taken
///
/// # Example
///
/// ```
/// use kernel::network::zero_copy_io::PacketMetadata;
///
/// let metadata = PacketMetadata {
///     timestamp: 1234567890,
///     queue_depth: 10,
///     priority: 128,
///     hops: 3,
/// };
/// ```
pub struct PacketMetadata {
    /// Packet timestamp in nanoseconds
    pub timestamp: u64,
    /// Queue depth at enqueue time
    pub queue_depth: u32,
    /// Packet priority (0-255)
    pub priority: u8,
    /// Number of hops taken
    pub hops: u8,
}

/// A zero-copy socket that supports zero-copy send and receive operations.
///
/// This socket type provides high-performance network I/O by eliminating
/// memory copies when sending and receiving data.
///
/// # Fields
///
/// * `fd` - File descriptor for the socket
/// * `socket_type` - Socket type (Stream, Datagram, Raw)
/// * `recv_queue` - Receive queue (lock-protected)
/// * `send_queue` - Send queue (lock-protected)
/// * `state` - Current socket state
/// * `config` - Zero-copy configuration options
///
/// # Example
///
/// ```no_run
/// use kernel::network::zero_copy_io::{ZeroCopySocket, SocketType};
/// use alloc::sync::Arc;
/// use spin::Mutex;
/// use alloc::collections::VecDeque;
///
/// let socket = ZeroCopySocket {
///     fd: 3,
///     socket_type: SocketType::Stream,
///     recv_queue: Arc::new(Mutex::new(VecDeque::new())),
///     send_queue: Arc::new(Mutex::new(VecDeque::new())),
///     state: SocketState::Connected,
///     config: ZeroCopyConfig::default(),
/// };
/// ```
#[derive(Debug)]
pub struct ZeroCopySocket {
    /// File descriptor
    pub fd: i32,
    /// Socket type
    pub socket_type: SocketType,
    /// Receive queue
    pub recv_queue: Arc<Mutex<VecDeque<ZeroCopyPacket>>>,
    /// Send queue
    pub send_queue: Arc<Mutex<VecDeque<ZeroCopyPacket>>>,
    /// Socket state
    pub state: SocketState,
    /// Zero-copy configuration
    pub config: ZeroCopyConfig,
}

/// Socket type specifying the communication semantics.
///
/// # Variants
///
/// * `Stream` - Stream socket (TCP-like, reliable, connection-oriented)
/// * `Datagram` - Datagram socket (UDP-like, unreliable, connectionless)
/// * `Raw` - Raw socket (access to underlying protocol)
///
/// # Example
///
/// ```
/// use kernel::network::zero_copy_io::SocketType;
///
/// let tcp_socket = SocketType::Stream;
/// let udp_socket = SocketType::Datagram;
/// let icmp_socket = SocketType::Raw;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// Stream socket (TCP)
    Stream,
    /// Datagram socket (UDP)
    Datagram,
    /// Raw socket
    Raw,
}

/// Socket state machine states.
///
/// # Variants
///
/// * `Unconnected` - Socket not connected
/// * `Connecting` - Socket is connecting
/// * `Connected` - Socket is connected
/// * `Listening` - Socket is listening for connections
/// * `Closed` - Socket is closed
///
/// # Example
///
/// ```
/// use kernel::network::zero_copy_io::SocketState;
///
/// let state = SocketState::Connected;
/// assert_eq!(state, SocketState::Connected);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    /// Not connected
    Unconnected,
    /// Currently connecting
    Connecting,
    /// Connected
    Connected,
    /// Listening for connections
    Listening,
    /// Closed
    Closed,
}

#[derive(Debug, Clone)]
pub struct ZeroCopyConfig {
    pub enable_zero_copy_send: bool,
    pub enable_zero_copy_recv: bool,
    pub max_packet_size: usize,
    pub buffer_pool_size: usize,
    pub enable_dma: bool,
}

impl Default for ZeroCopyConfig {
    fn default() -> Self {
        Self {
            enable_zero_copy_send: true,
            enable_zero_copy_recv: true,
            max_packet_size: 65536,
            buffer_pool_size: 1024,
            enable_dma: false,
        }
    }
}

pub struct ZeroCopyBufferPool {
    available_buffers: Arc<Mutex<VecDeque<ZeroCopyBuffer>>>,
    used_buffers: Arc<Mutex<VecDeque<ZeroCopyBuffer>>>,
    max_buffers: usize,
    buffer_size: usize,
}

impl ZeroCopyBufferPool {
    pub fn new(count: usize, size: usize) -> Result<Self> {
        Ok(Self {
            available_buffers: Arc::new(Mutex::new(VecDeque::with_capacity(count))),
            used_buffers: Arc::new(Mutex::new(VecDeque::with_capacity(count))),
            max_buffers: count,
            buffer_size: size,
        })
    }

    pub fn allocate(&self) -> Result<ZeroCopyBuffer> {
        let mut available = self.available_buffers.lock();
        if let Some(buffer) = available.pop_front() {
            let mut used = self.used_buffers.lock();
            used.push_back(buffer.clone());
            Ok(buffer)
        } else {
            Err(Error::ResourceExhausted("No available buffers".to_string()))
        }
    }

    pub fn free(&self, buffer: ZeroCopyBuffer) {
        let mut used = self.used_buffers.lock();
        if let Some(buf) = used.iter().position(|b| b.physical_addr == buffer.physical_addr) {
            used.remove(buf);
            let mut available = self.available_buffers.lock();
            available.push_back(buffer);
        }
    }

    pub fn available_count(&self) -> usize {
        self.available_buffers.lock().len()
    }

    pub fn used_count(&self) -> usize {
        self.used_buffers.lock().len()
    }
}

pub struct ZeroCopyNetworkManager {
    pub buffer_pool: Arc<ZeroCopyBufferPool>,
    pub sockets: Arc<Mutex<HashMap<i32, ZeroCopySocket>>>,
    pub dma_engine: Option<DmaEngine>,
    pub stats: Arc<Mutex<ZeroCopyStats>>,
    pub next_fd: Arc<Mutex<i32>>,
}

pub struct DmaEngine {
    pub channel_id: u32,
    pub state: DmaState,
    pub transfer_queue: Arc<Mutex<Vec<DmaTransfer>>>,
}

pub enum DmaState {
    Idle,
    Transferring,
    Paused,
    Error,
}

pub struct DmaTransfer {
    pub transfer_id: u32,
    pub src_addr: usize,
    pub dst_addr: usize,
    pub size: usize,
    pub direction: DmaDirection,
    pub status: DmaTransferStatus,
}

pub enum DmaDirection {
    MemoryToDevice,
    DeviceToMemory,
    MemoryToMemory,
}

pub enum DmaTransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Default, Clone)]
pub struct ZeroCopyStats {
    pub zero_copy_recv_count: u64,
    pub zero_copy_send_count: u64,
    pub fallback_copy_count: u64,
    pub zero_copy_bytes: u64,
    pub avg_transfer_time: u64,
    pub allocation_failures: u64,
    pub dma_transfers: u64,
}

impl ZeroCopyNetworkManager {
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
            sockets: Arc::new(Mutex::new(HashMap::new())),
            dma_engine,
            stats: Arc::new(Mutex::new(ZeroCopyStats::default())),
            next_fd: Arc::new(Mutex::new(0)),
        })
    }

    pub fn create_socket(&self, socket_type: SocketType, config: ZeroCopyConfig) -> Result<i32> {
        let mut sockets = self.sockets.lock();
        let mut next_fd = self.next_fd.lock();
        let fd = *next_fd;
        *next_fd += 1;

        let socket = ZeroCopySocket {
            fd,
            socket_type,
            recv_queue: Arc::new(Mutex::new(VecDeque::new())),
            send_queue: Arc::new(Mutex::new(VecDeque::new())),
            state: SocketState::Unconnected,
            config,
        };

        sockets.insert(fd, socket);
        Ok(fd)
    }

    pub fn send_zero_copy(&self, fd: i32, buffer: ZeroCopyBuffer) -> Result<usize> {
        let mut sockets = self.sockets.lock();
        if let Some(socket) = sockets.get_mut(&fd) {
            if !socket.config.enable_zero_copy_send {
                drop(sockets);
                return self.send_fallback(fd, buffer);
            }

            let packet = ZeroCopyPacket {
                header: PacketHeader {
                    src_addr: 0,
                    dst_addr: 0,
                    src_port: 0,
                    dst_port: 0,
                    protocol: match socket.socket_type {
                        SocketType::Stream => 6,
                        SocketType::Datagram => 17,
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

            let mut send_queue = socket.send_queue.lock();
            send_queue.push_back(packet);

            let mut stats = self.stats.lock();
            stats.zero_copy_send_count += 1;
            stats.zero_copy_bytes += buffer.size as u64;

            if let Some(ref dma_engine) = self.dma_engine {
                self.schedule_dma_transfer(dma_engine, &buffer, DmaDirection::MemoryToDevice)?;
                stats.dma_transfers += 1;
            }

            Ok(buffer.size)
        } else {
            Err(Error::NotFound("Socket not found".to_string()))
        }
    }

    pub fn recv_zero_copy(&self, fd: i32) -> Result<ZeroCopyPacket> {
        let sockets = self.sockets.lock();
        if let Some(socket) = sockets.get(&fd) {
            if !socket.config.enable_zero_copy_recv {
                drop(sockets);
                return self.recv_fallback(fd);
            }

            let mut recv_queue = socket.recv_queue.lock();
            if let Some(packet) = recv_queue.pop_front() {
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

    fn send_fallback(&self, fd: i32, buffer: ZeroCopyBuffer) -> Result<usize> {
        let mut stats = self.stats.lock();
        stats.fallback_copy_count += 1;
        Ok(buffer.size)
    }

    fn recv_fallback(&self, fd: i32) -> Result<ZeroCopyPacket> {
        let mut stats = self.stats.lock();
        stats.fallback_copy_count += 1;
        Err(Error::NotImplemented("Fallback receive".to_string()))
    }

    fn schedule_dma_transfer(&self, engine: &DmaEngine, buffer: &ZeroCopyBuffer, direction: DmaDirection) -> Result<()> {
        let mut queue = engine.transfer_queue.lock();
        queue.push(DmaTransfer {
            transfer_id: queue.len() as u32,
            src_addr: buffer.physical_addr,
            dst_addr: buffer.virtual_addr,
            size: buffer.size,
            direction,
            status: DmaTransferStatus::Pending,
        });
        Ok(())
    }

    fn get_timestamp(&self) -> u64 {
        0
    }

    pub fn close_socket(&self, fd: i32) -> Result<()> {
        let mut sockets = self.sockets.lock();
        if sockets.remove(&fd).is_some() {
            Ok(())
        } else {
            Err(Error::NotFound("Socket not found".to_string()))
        }
    }

    pub fn get_stats(&self) -> ZeroCopyStats {
        self.stats.lock().clone()
    }
}
