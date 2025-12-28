use alloc::vec::Vec;
use alloc::collections::{VecDeque, HashMap};
use alloc::sync::Arc;
use spin::Mutex;
use nos_api::{Result, Error};

pub struct ZeroCopyBuffer {
    pub physical_addr: usize,
    pub virtual_addr: usize,
    pub size: usize,
    pub flags: BufferFlags,
}

pub struct BufferFlags {
    pub read_only: bool,
    pub write_only: bool,
    pub dma_mapped: bool,
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

pub struct ZeroCopyPacket {
    pub header: PacketHeader,
    pub buffer: ZeroCopyBuffer,
    pub metadata: PacketMetadata,
}

pub struct PacketHeader {
    pub src_addr: u32,
    pub dst_addr: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub length: u16,
    pub checksum: u16,
}

pub struct PacketMetadata {
    pub timestamp: u64,
    pub queue_depth: u32,
    pub priority: u8,
    pub hops: u8,
}

#[derive(Debug)]
pub struct ZeroCopySocket {
    pub fd: i32,
    pub socket_type: SocketType,
    pub recv_queue: Arc<Mutex<VecDeque<ZeroCopyPacket>>>,
    pub send_queue: Arc<Mutex<VecDeque<ZeroCopyPacket>>>,
    pub state: SocketState,
    pub config: ZeroCopyConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream,
    Datagram,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Unconnected,
    Connecting,
    Connected,
    Listening,
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
