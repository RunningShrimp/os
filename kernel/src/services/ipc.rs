//! High-Performance IPC Service
//!
//! This module provides a high-performance IPC service that builds on top of
//! the basic IPC primitives (`crate::ipc`) and microkernel IPC (`crate::microkernel::ipc`).
//!
//! Features:
//! - Zero-copy message passing
//! - Batch operations
//! - Asynchronous messaging
//! - Lock-free queues
//! - Memory pools
//!
//! **Architecture**:
//! - Uses `crate::ipc` for POSIX-compliant IPC
//! - Uses `crate::microkernel::ipc` for service communication
//! - Provides unified high-level API for applications

extern crate alloc;
use alloc::boxed::Box;

use crate::types::stubs::{ServiceId, Message, MessageType, send_message, receive_message,
                          ServiceInfo, InterfaceVersion, ServiceCategory, get_service_registry};
use crate::microkernel::ipc::MessageQueue;
use crate::reliability::errno::{EINVAL, ENOENT, EEXIST, ENOMEM, EIO, EAGAIN, ENODEV};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicPtr, Ordering};
use core::ptr::NonNull;
use core::cell::UnsafeCell;

/// IPC消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcMessageType {
    Data,           // 数据消息
    Control,        // 控制消息
    Request,        // 请求消息
    Response,       // 响应消息
    Event,          // 事件消息
    Stream,         // 流消息
    Batch,          // 批量消息
}

/// IPC通信模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcMode {
    Synchronous,    // 同步模式
    Asynchronous,   // 异步模式
    Streaming,      // 流式模式
    ZeroCopy,       // 零拷贝模式
    Batch,          // 批量模式
}

/// IPC传输方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IpcTransport {
    MessageQueue,   // 消息队列
    SharedMemory,   // 共享内存
    LockFreeQueue,  // 无锁队列
    MemoryPool,     // 内存池
    Pipe,           // 管道
    Socket,         // 套接字
    EventFd,        // 事件文件描述符
    Signal,         // 信号
}

/// 共享内存区域
#[derive(Debug)]
pub struct SharedMemoryRegion {
    /// 区域ID
    pub id: u64,
    /// 起始地址
    pub start_addr: usize,
    /// 区域大小
    pub size: usize,
    /// 读写权限
    pub permissions: u32,
    /// 引用计数
    pub ref_count: AtomicU64,
}

impl SharedMemoryRegion {
    /// 创建新的共享内存区域
    pub fn new(id: u64, size: usize, permissions: u32) -> Result<Self, &'static str> {
        // 在实际实现中，这里会调用内存管理服务分配物理内存
        // 这里简化处理，假设内存已经分配
        Ok(Self {
            id,
            start_addr: 0, // 实际应该是分配的地址
            size,
            permissions,
            ref_count: AtomicU64::new(1),
        })
    }

    /// 获取数据指针
    pub fn as_ptr(&self) -> *mut u8 {
        self.start_addr as *mut u8
    }

    /// 增加引用
    pub fn add_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }

    /// 减少引用
    pub fn release(&self) -> u64 {
        self.ref_count.fetch_sub(1, Ordering::SeqCst)
    }
}

/// 无锁MPSC队列节点
struct LockFreeNode<T> {
    data: T,
    next: AtomicPtr<LockFreeNode<T>>,
}

/// 无锁MPSC队列
pub struct LockFreeQueue<T> {
    head: AtomicPtr<LockFreeNode<T>>,
    tail: UnsafeCell<*mut LockFreeNode<T>>,
}

impl<T> LockFreeQueue<T> {
    /// 创建新的无锁队列
    pub fn new() -> Self {
        let dummy = Box::into_raw(Box::new(LockFreeNode {
            data: unsafe { core::mem::MaybeUninit::uninit().assume_init() },
            next: AtomicPtr::new(core::ptr::null_mut()),
        }));

        Self {
            head: AtomicPtr::new(dummy),
            tail: UnsafeCell::new(dummy),
        }
    }

    /// 推入元素
    pub fn push(&self, data: T) {
        let new_node = Box::into_raw(Box::new(LockFreeNode {
            data,
            next: AtomicPtr::new(core::ptr::null_mut()),
        }));

        let prev_tail = self.head.swap(new_node, Ordering::AcqRel);
        unsafe {
            (*prev_tail).next.store(new_node, Ordering::Release);
        }
    }

    /// 弹出元素
    pub fn pop(&self) -> Option<T> {
        unsafe {
            let head = *self.tail.get();
            let next = (*head).next.load(Ordering::Acquire);

            if next.is_null() {
                return None;
            }

            *self.tail.get() = next;
            let _node = Box::from_raw(head);
            Some(core::ptr::read(&(*next).data))
        }
    }
}

unsafe impl<T: Send> Send for LockFreeQueue<T> {}
unsafe impl<T: Send> Sync for LockFreeQueue<T> {}

/// 内存池块
#[derive(Debug)]
struct MemoryPoolBlock {
    data: NonNull<u8>,
    size: usize,
    in_use: bool,
}

/// 高性能内存池
pub struct MemoryPool {
    /// 内存块列表
    blocks: Vec<MemoryPoolBlock>,
    /// 空闲块索引
    free_blocks: Vec<usize>,
    /// 块大小
    block_size: usize,
    /// 总容量
    capacity: usize,
    /// 已使用量
    used: AtomicUsize,
}

impl MemoryPool {
    /// 创建新的内存池
    pub fn new(block_size: usize, capacity: usize) -> Result<Self, &'static str> {
        let mut blocks = Vec::with_capacity(capacity);

        // 预分配内存块
        for _ in 0..capacity {
            // 在实际实现中，这里会从内存管理服务分配内存
            // 这里简化处理
            blocks.push(MemoryPoolBlock {
                data: NonNull::dangling(), // 实际应该是分配的地址
                size: block_size,
                in_use: false,
            });
        }

        let free_blocks: Vec<usize> = (0..capacity).collect();

        Ok(Self {
            blocks,
            free_blocks,
            block_size,
            capacity,
            used: AtomicUsize::new(0),
        })
    }

    /// 分配内存块
    pub fn allocate(&mut self) -> Option<usize> {
        if let Some(index) = self.free_blocks.pop() {
            self.blocks[index].in_use = true;
            self.used.fetch_add(1, Ordering::Relaxed);
            Some(index)
        } else {
            None
        }
    }

    /// 释放内存块
    pub fn deallocate(&mut self, index: usize) -> Result<(), &'static str> {
        if index >= self.blocks.len() {
            return Err("Invalid block index");
        }

        if !self.blocks[index].in_use {
            return Err("Block already free");
        }

        self.blocks[index].in_use = false;
        self.free_blocks.push(index);
        self.used.fetch_sub(1, Ordering::Relaxed);
        Ok(())
    }

    /// 获取内存块指针
    pub fn get_block_ptr(&self, index: usize) -> Option<*mut u8> {
        if index < self.blocks.len() && self.blocks[index].in_use {
            Some(self.blocks[index].data.as_ptr())
        } else {
            None
        }
    }

    /// 获取使用统计
    pub fn get_usage(&self) -> (usize, usize) {
        (self.used.load(Ordering::Relaxed), self.capacity)
    }
}

/// 批量操作缓冲区
pub struct BatchBuffer<T> {
    /// 缓冲区数据
    buffer: Vec<T>,
    /// 当前大小
    size: usize,
    /// 最大容量
    capacity: usize,
}

impl<T> BatchBuffer<T> {
    /// 创建新的批量缓冲区
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            size: 0,
            capacity,
        }
    }

    /// 添加元素
    pub fn push(&mut self, item: T) -> bool {
        if self.size < self.capacity {
            self.buffer.push(item);
            self.size += 1;
            true
        } else {
            false
        }
    }

    /// 清空缓冲区
    pub fn flush(&mut self) -> Vec<T> {
        self.size = 0;
        core::mem::replace(&mut self.buffer, Vec::with_capacity(self.capacity))
    }

    /// 是否已满
    pub fn is_full(&self) -> bool {
        self.size >= self.capacity
    }

    /// 当前大小
    pub fn len(&self) -> usize {
        self.size
    }
}

/// 高性能IPC消息
#[derive(Debug, Clone)]
pub struct IpcMessage {
    /// 消息ID
    pub id: u64,
    /// 发送者ID
    pub sender_id: u64,
    /// 接收者ID
    pub receiver_id: u64,
    /// 消息类型
    pub message_type: IpcMessageType,
    /// 优先级
    pub priority: u8,
    /// 传输方式
    pub transport: IpcTransport,
    /// 通信模式
    pub mode: IpcMode,
    /// 数据指针（零拷贝）
    pub data_ptr: Option<usize>,
    /// 数据长度
    pub data_len: usize,
    /// 实际数据（用于非零拷贝）
    pub data: Option<Vec<u8>>,
    /// 共享内存区域ID（用于共享内存传输）
    pub shared_memory_id: Option<u64>,
    /// 内存池块索引（用于内存池传输）
    pub memory_pool_index: Option<usize>,
    /// 时间戳
    pub timestamp: u64,
    /// 超时时间
    pub timeout: Option<u64>,
    /// 消息标志
    pub flags: u32,
    /// 批次ID（用于批量操作）
    pub batch_id: Option<u64>,
}

impl IpcMessage {
    /// 创建新的IPC消息
    pub fn new(sender_id: u64, receiver_id: u64, message_type: IpcMessageType) -> Self {
        static NEXT_MESSAGE_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: NEXT_MESSAGE_ID.fetch_add(1, Ordering::SeqCst),
            sender_id,
            receiver_id,
            message_type,
            priority: 0,
            transport: IpcTransport::MessageQueue,
            mode: IpcMode::Synchronous,
            data_ptr: None,
            data_len: 0,
            data: None,
            shared_memory_id: None,
            memory_pool_index: None,
            timestamp: get_current_time_ns(),
            timeout: None,
            flags: 0,
            batch_id: None,
        }
    }

    /// 设置消息优先级
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// 设置传输方式
    pub fn with_transport(mut self, transport: IpcTransport) -> Self {
        self.transport = transport;
        self
    }

    /// 设置通信模式
    pub fn with_mode(mut self, mode: IpcMode) -> Self {
        self.mode = mode;
        self
    }

    /// 设置零拷贝数据
    pub fn with_zero_copy_data(mut self, data_ptr: usize, data_len: usize) -> Self {
        self.data_ptr = Some(data_ptr);
        self.data_len = data_len;
        self.data = None;
        self.mode = IpcMode::ZeroCopy;
        self
    }

    /// 设置普通数据
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data_len = data.len();
        self.data = Some(data);
        self.data_ptr = None;
        self
    }

    /// 设置共享内存数据
    pub fn with_shared_memory(mut self, shared_memory_id: u64, offset: usize, len: usize) -> Self {
        self.shared_memory_id = Some(shared_memory_id);
        self.data_ptr = Some(offset);
        self.data_len = len;
        self.data = None;
        self.transport = IpcTransport::SharedMemory;
        self
    }

    /// 设置内存池数据
    pub fn with_memory_pool(mut self, pool_index: usize, len: usize) -> Self {
        self.memory_pool_index = Some(pool_index);
        self.data_len = len;
        self.data = None;
        self.transport = IpcTransport::MemoryPool;
        self
    }

    /// 设置超时
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// 设置批次ID
    pub fn with_batch_id(mut self, batch_id: u64) -> Self {
        self.batch_id = Some(batch_id);
        self.mode = IpcMode::Batch;
        self
    }

    /// 获取消息大小
    pub fn size(&self) -> usize {
        core::mem::size_of::<Self>() + self.data_len
    }

    /// 是否为零拷贝消息
    pub fn is_zero_copy(&self) -> bool {
        self.data_ptr.is_some() || self.shared_memory_id.is_some() || self.memory_pool_index.is_some()
    }

    /// 是否为高性能传输（共享内存、内存池、无锁队列）
    pub fn is_high_performance(&self) -> bool {
        matches!(self.transport,
            IpcTransport::SharedMemory |
            IpcTransport::MemoryPool |
            IpcTransport::LockFreeQueue
        )
    }

    /// 是否需要特殊内存管理
    pub fn requires_memory_management(&self) -> bool {
        self.shared_memory_id.is_some() || self.memory_pool_index.is_some()
    }
}

/// IPC通信通道
pub struct IpcChannel {
    /// 通道ID
    pub id: u64,
    /// 通道名称
    pub name: String,
    /// 通道类型
    pub channel_type: IpcChannelType,
    /// 发送队列（传统模式）
    pub send_queue: Arc<Mutex<Vec<IpcMessage>>>,
    /// 接收队列（传统模式）
    pub recv_queue: Arc<Mutex<Vec<IpcMessage>>>,
    /// 无锁队列（高性能模式）
    pub lock_free_queue: Arc<LockFreeQueue<IpcMessage>>,
    /// 共享内存区域
    pub shared_memory: Option<Arc<SharedMemoryRegion>>,
    /// 内存池
    pub memory_pool: Option<Arc<Mutex<MemoryPool>>>,
    /// 批量缓冲区
    pub batch_buffer: Arc<Mutex<BatchBuffer<IpcMessage>>>,
    /// 最大消息数
    pub max_messages: usize,
    /// 当前消息数
    pub current_count: AtomicUsize,
    /// 通道标志
    pub flags: u32,
    /// 默认传输方式
    pub default_transport: IpcTransport,
    /// 统计信息
    pub stats: Arc<Mutex<IpcChannelStats>>,
}

/// IPC通道类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcChannelType {
    PointToPoint,    // 点对点
    PublishSubscribe, // 发布订阅
    RequestReply,    // 请求响应
    Broadcast,       // 广播
    Pipeline,        // 管道
    Stream,          // 流式
}

/// IPC通道统计信息
#[derive(Debug)]
pub struct IpcChannelStats {
    /// 发送消息数
    pub messages_sent: AtomicU64,
    /// 接收消息数
    pub messages_received: AtomicU64,
    /// 发送字节数
    pub bytes_sent: AtomicU64,
    /// 接收字节数
    pub bytes_received: AtomicU64,
    /// 错误次数
    pub errors: AtomicU64,
    /// 超时次数
    pub timeouts: AtomicU64,
    /// 平均延迟（纳秒）
    pub avg_latency_ns: AtomicU64,
    /// 最大延迟（纳秒）
    pub max_latency_ns: AtomicU64,
}

impl IpcChannelStats {
    /// 创建新的统计信息
    pub const fn new() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            timeouts: AtomicU64::new(0),
            avg_latency_ns: AtomicU64::new(0),
            max_latency_ns: AtomicU64::new(0),
        }
    }

    /// 更新发送统计
    pub fn update_send_stats(&self, bytes: u64, latency_ns: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);

        // 更新延迟统计
        let current_avg = self.avg_latency_ns.load(Ordering::Relaxed);
        let sent_count = self.messages_sent.load(Ordering::Relaxed);
        let new_avg = (current_avg * (sent_count - 1) + latency_ns) / sent_count;
        self.avg_latency_ns.store(new_avg, Ordering::Relaxed);

        let current_max = self.max_latency_ns.load(Ordering::Relaxed);
        if latency_ns > current_max {
            self.max_latency_ns.store(latency_ns, Ordering::Relaxed);
        }
    }

    /// 更新接收统计
    pub fn update_recv_stats(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }
}

impl IpcChannel {
    /// 创建新的IPC通道
    pub fn new(id: u64, name: String, channel_type: IpcChannelType, max_messages: usize) -> Self {
        Self {
            id,
            name,
            channel_type,
            send_queue: Arc::new(Mutex::new(Vec::new())),
            recv_queue: Arc::new(Mutex::new(Vec::new())),
            lock_free_queue: Arc::new(LockFreeQueue::new()),
            shared_memory: None,
            memory_pool: None,
            batch_buffer: Arc::new(Mutex::new(BatchBuffer::new(max_messages / 4))),
            max_messages,
            current_count: AtomicUsize::new(0),
            flags: 0,
            default_transport: IpcTransport::MessageQueue,
            stats: Arc::new(Mutex::new(IpcChannelStats::new())),
        }
    }

    /// 创建带共享内存的IPC通道
    pub fn with_shared_memory(mut self, shared_memory: Arc<SharedMemoryRegion>) -> Self {
        self.shared_memory = Some(shared_memory);
        self.default_transport = IpcTransport::SharedMemory;
        self
    }

    /// 创建带内存池的IPC通道
    pub fn with_memory_pool(mut self, memory_pool: Arc<Mutex<MemoryPool>>) -> Self {
        self.memory_pool = Some(memory_pool);
        self.default_transport = IpcTransport::MemoryPool;
        self
    }

    /// 设置默认传输方式
    pub fn with_default_transport(mut self, transport: IpcTransport) -> Self {
        self.default_transport = transport;
        self
    }

    /// 发送消息（优化版本）
    pub fn send(&self, mut message: IpcMessage) -> Result<(), i32> {
        // 检查通道容量
        if self.current_count.load(Ordering::SeqCst) >= self.max_messages {
            return Err(EAGAIN);
        }

        let start_time = get_current_time_ns();
        let message_size = message.size() as u64;  // Store the size before message is moved

        // 根据传输方式优化发送策略
        match message.transport {
            IpcTransport::LockFreeQueue => {
                // 使用无锁队列进行高性能发送
                self.lock_free_queue.push(message);
            }
            IpcTransport::SharedMemory => {
                // 共享内存传输验证
                if message.shared_memory_id.is_none() {
                    // 如果没有设置共享内存，回退到默认传输方式
                    message.transport = self.default_transport;
                    return self.send(message);
                }

                // 共享内存模式下，只需要发送元数据
                let mut send_queue = self.send_queue.lock();
                send_queue.push(message);
            }
            IpcTransport::MemoryPool => {
                // 内存池传输验证
                if message.memory_pool_index.is_none() {
                    // 如果没有设置内存池，回退到默认传输方式
                    message.transport = self.default_transport;
                    return self.send(message);
                }

                let mut send_queue = self.send_queue.lock();
                send_queue.push(message);
            }
            _ => {
                // 传统队列方式
                match self.channel_type {
                    IpcChannelType::PointToPoint => {
                        let mut send_queue = self.send_queue.lock();

                        // 按优先级插入消息
                        let insert_pos = send_queue.iter().position(|m| m.priority < message.priority)
                            .unwrap_or(send_queue.len());

                        send_queue.insert(insert_pos, message);
                    }
                    IpcChannelType::Broadcast => {
                        // 广播模式：消息放入接收队列
                        let mut recv_queue = self.recv_queue.lock();
                        recv_queue.push(message);
                    }
                    IpcChannelType::PublishSubscribe => {
                        // 发布订阅模式：类似广播
                        let mut recv_queue = self.recv_queue.lock();
                        recv_queue.push(message);
                    }
                    _ => {
                        let mut send_queue = self.send_queue.lock();
                        send_queue.push(message);
                    }
                }
            }
        }

        self.current_count.fetch_add(1, Ordering::Relaxed);

        // 更新统计信息
        let latency = get_current_time_ns() - start_time;
        {
            let stats = self.stats.lock();
            stats.update_send_stats(message_size, latency);
        }

        Ok(())
    }

    /// 高性能发送（使用默认传输优化）
    pub fn send_optimized(&self, sender_id: u64, receiver_id: u64, data: Vec<u8>) -> Result<(), i32> {
        let message = IpcMessage::new(sender_id, receiver_id, IpcMessageType::Data)
            .with_transport(self.default_transport)
            .with_data(data);

        self.send(message)
    }

    /// 零拷贝发送
    pub fn send_zero_copy(&self, sender_id: u64, receiver_id: u64, data_ptr: usize, data_len: usize) -> Result<(), i32> {
        let message = IpcMessage::new(sender_id, receiver_id, IpcMessageType::Data)
            .with_zero_copy_data(data_ptr, data_len);

        self.send(message)
    }

    /// 接收消息（优化版本）
    pub fn receive(&self, receiver_id: u64) -> Result<IpcMessage, i32> {
        let _start_time = get_current_time_ns();

        // 优先尝试无锁队列
        if let Some(message) = self.lock_free_queue.pop() {
            self.current_count.fetch_sub(1, Ordering::Relaxed);

            // 更新统计信息
            {
                let stats = self.stats.lock();
                stats.update_recv_stats(message.size() as u64);
            }

            return Ok(message);
        }

        // 传统队列方式
        let message = match self.channel_type {
            IpcChannelType::PointToPoint => {
                let mut send_queue = self.send_queue.lock();

                // 查找发送给特定接收者的消息
                let pos = send_queue.iter().position(|m| m.receiver_id == receiver_id || m.receiver_id == 0);

                if let Some(index) = pos {
                    Some(send_queue.remove(index))
                } else {
                    None
                }
            }
            IpcChannelType::Broadcast | IpcChannelType::PublishSubscribe => {
                let mut recv_queue = self.recv_queue.lock();
                recv_queue.pop()
            }
            _ => {
                let mut recv_queue = self.recv_queue.lock();
                recv_queue.pop()
            }
        };

        if let Some(msg) = message {
            self.current_count.fetch_sub(1, Ordering::Relaxed);

            // 更新统计信息
            {
                let stats = self.stats.lock();
                stats.update_recv_stats(msg.size() as u64);
            }

            Ok(msg)
        } else {
            Err(EAGAIN)
        }
    }

    /// 高性能接收（批量模式）
    pub fn receive_batch_optimized(&self, receiver_id: u64, max_count: usize) -> Vec<IpcMessage> {
        let mut messages = Vec::with_capacity(max_count);

        // 首先从无锁队列尽可能多地获取消息
        for _ in 0..max_count {
            match self.lock_free_queue.pop() {
                Some(msg) => {
                    if msg.receiver_id == receiver_id || msg.receiver_id == 0 {
                        messages.push(msg);
                        self.current_count.fetch_sub(1, Ordering::Relaxed);
                    }
                }
                None => break,
            }
        }

        // 如果还需要更多消息，从传统队列获取
        if messages.len() < max_count {
            let remaining = max_count - messages.len();
            messages.extend(self.receive_batch(receiver_id, remaining));
        }

        // 更新统计信息
        if !messages.is_empty() {
            // 计算批量接收的总字节数，用于批量统计优化
            let total_bytes: u64 = messages.iter().map(|m| m.size() as u64).sum();
            {
                let stats = self.stats.lock();
                // 批量更新统计信息：先更新总字节数，再逐个更新消息计数
                // 这样可以减少原子操作的次数，提高性能
                stats.bytes_received.fetch_add(total_bytes, Ordering::Relaxed);
                // 然后更新每个消息的计数
                for msg in &messages {
                    stats.messages_received.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        messages
    }

    /// 异步接收（非阻塞）
    pub fn try_receive(&self, receiver_id: u64) -> Option<IpcMessage> {
        match self.receive(receiver_id) {
            Ok(msg) => Some(msg),
            Err(_) => None,
        }
    }

    /// 带超时的接收
    pub fn receive_timeout(&self, receiver_id: u64, timeout_ns: u64) -> Result<IpcMessage, i32> {
        let start = get_current_time_ns();

        loop {
            match self.try_receive(receiver_id) {
                Some(msg) => return Ok(msg),
                None => {
                    if get_current_time_ns() - start > timeout_ns {
                        return Err(EAGAIN);
                    }
                    // 在实际实现中，这里应该使用更高效的等待机制
                    // 这里简化处理，直接继续循环
                }
            }
        }
    }

    /// 批量发送消息（优化版本）
    pub fn send_batch(&self, messages: Vec<IpcMessage>) -> Result<usize, i32> {
        if messages.is_empty() {
            return Ok(0);
        }

        // 检查批量缓冲区是否可以优化
        if messages.len() <= 4 && self.default_transport == IpcTransport::LockFreeQueue {
            // 小批量消息使用无锁队列逐个发送
            let mut sent_count = 0;
            for message in messages {
                match self.send(message) {
                    Ok(()) => sent_count += 1,
                    Err(_) => break,
                }
            }
            return Ok(sent_count);
        }

        // 大批量消息优化处理
        let start_time = get_current_time_ns();
        let mut sent_count = 0;
        let batch_id = {
            static NEXT_BATCH_ID: AtomicU64 = AtomicU64::new(1);
            NEXT_BATCH_ID.fetch_add(1, Ordering::SeqCst)
        };

        // 预先检查容量
        if self.current_count.load(Ordering::SeqCst) + messages.len() > self.max_messages {
            return Err(EAGAIN);
        }

        for mut message in messages {
            // 设置批次ID
            message.batch_id = Some(batch_id);

            // 根据传输方式优化批量发送
            match message.transport {
                IpcTransport::LockFreeQueue => {
                    self.lock_free_queue.push(message);
                    sent_count += 1;
                }
                _ => {
                    match self.send(message) {
                        Ok(()) => sent_count += 1,
                        Err(_) => break,
                    }
                }
            }
        }

        // 批量更新计数器
        self.current_count.fetch_add(sent_count, Ordering::Relaxed);

        // 更新统计信息
        let total_latency = get_current_time_ns() - start_time;
        {
            let stats = self.stats.lock();
            for _ in 0..sent_count {
                stats.update_send_stats(0, total_latency / sent_count as u64);
            }
        }

        Ok(sent_count)
    }

    /// 智能批量发送（自动优化传输方式）
    pub fn send_batch_smart(&self, messages: Vec<IpcMessage>) -> Result<usize, i32> {
        if messages.is_empty() {
            return Ok(0);
        }

        // 分析消息特征，选择最优传输方式
        let total_size: usize = messages.iter().map(|m| m.size()).sum();
        let avg_size = total_size / messages.len();

        // 根据消息大小和数量选择传输策略
        let optimized_messages: Vec<IpcMessage> = messages.into_iter().map(|mut msg| {
            if avg_size > 4096 {
                // 大消息使用共享内存（如果可用）
                if self.shared_memory.is_some() {
                    msg.transport = IpcTransport::SharedMemory;
                }
            } else if msg.data_len > 0 && msg.data_len <= 256 {
                // 小消息使用内存池（如果可用）
                if self.memory_pool.is_some() {
                    msg.transport = IpcTransport::MemoryPool;
                }
            } else {
                // 默认使用无锁队列
                msg.transport = IpcTransport::LockFreeQueue;
            }
            msg
        }).collect();

        self.send_batch(optimized_messages)
    }

    /// 批量接收消息
    pub fn receive_batch(&self, receiver_id: u64, max_count: usize) -> Vec<IpcMessage> {
        let mut messages = Vec::new();

        for _ in 0..max_count {
            match self.receive(receiver_id) {
                Ok(msg) => messages.push(msg),
                Err(_) => break,
            }
        }

        messages
    }

    /// 获取通道统计信息
    pub fn get_stats(&self) -> IpcChannelStatsSnapshot {
        let stats = self.stats.lock();
        IpcChannelStatsSnapshot {
            messages_sent: stats.messages_sent.load(Ordering::Relaxed),
            messages_received: stats.messages_received.load(Ordering::Relaxed),
            bytes_sent: stats.bytes_sent.load(Ordering::Relaxed),
            bytes_received: stats.bytes_received.load(Ordering::Relaxed),
            errors: stats.errors.load(Ordering::Relaxed),
            timeouts: stats.timeouts.load(Ordering::Relaxed),
            avg_latency_ns: stats.avg_latency_ns.load(Ordering::Relaxed),
            max_latency_ns: stats.max_latency_ns.load(Ordering::Relaxed),
        }
    }

    /// 获取当前消息数量
    pub fn count(&self) -> usize {
        self.current_count.load(Ordering::Relaxed)
    }

    /// 是否已满
    pub fn is_full(&self) -> bool {
        self.current_count.load(Ordering::Relaxed) >= self.max_messages
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.current_count.load(Ordering::Relaxed) == 0
    }
}

/// IPC通道统计信息快照
#[derive(Debug, Clone)]
pub struct IpcChannelStatsSnapshot {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub avg_latency_ns: u64,
    pub max_latency_ns: u64,
}

/// IPC服务统计信息
#[derive(Debug)]
pub struct IpcServiceStats {
    /// 总通道数
    pub total_channels: AtomicUsize,
    /// 活跃通道数
    pub active_channels: AtomicUsize,
    /// 总消息数
    pub total_messages: AtomicU64,
    /// 零拷贝消息数
    pub zero_copy_messages: AtomicU64,
    /// 批量操作数
    pub batch_operations: AtomicU64,
    /// 平均消息大小
    pub avg_message_size: AtomicU64,
}

impl IpcServiceStats {
    /// 创建新的统计信息
    pub const fn new() -> Self {
        Self {
            total_channels: AtomicUsize::new(0),
            active_channels: AtomicUsize::new(0),
            total_messages: AtomicU64::new(0),
            zero_copy_messages: AtomicU64::new(0),
            batch_operations: AtomicU64::new(0),
            avg_message_size: AtomicU64::new(0),
        }
    }
}

/// 高性能IPC服务（优化版）
pub struct IpcService {
    /// 服务ID
    service_id: ServiceId,
    /// 消息队列
    message_queue: Arc<Mutex<MessageQueue>>,
    /// IPC通道
    channels: Arc<Mutex<BTreeMap<u64, IpcChannel>>>,
    /// 通道名称索引
    channel_name_index: Arc<Mutex<BTreeMap<String, u64>>>,
    /// 通道ID生成器
    next_channel_id: AtomicU64,
    /// 共享内存区域管理
    shared_memory_regions: Arc<Mutex<BTreeMap<u64, Arc<SharedMemoryRegion>>>>,
    /// 内存池管理
    memory_pools: Arc<Mutex<BTreeMap<usize, Arc<Mutex<MemoryPool>>>>>,
    /// 性能配置
    config: IpcServiceConfig,
    /// 服务统计信息
    stats: Arc<Mutex<IpcServiceStats>>,
}

/// IPC服务配置
#[derive(Debug, Clone)]
pub struct IpcServiceConfig {
    /// 默认通道容量
    pub default_channel_capacity: usize,
    /// 默认传输方式
    pub default_transport: IpcTransport,
    /// 是否启用批量优化
    pub enable_batch_optimization: bool,
    /// 是否启用零拷贝优化
    pub enable_zero_copy: bool,
    /// 批量操作阈值
    pub batch_threshold: usize,
    /// 性能监控级别
    pub performance_monitoring_level: u8,
}

impl Default for IpcServiceConfig {
    fn default() -> Self {
        Self {
            default_channel_capacity: 1024,
            default_transport: IpcTransport::LockFreeQueue,
            enable_batch_optimization: true,
            enable_zero_copy: true,
            batch_threshold: 4,
            performance_monitoring_level: 1,
        }
    }
}

impl IpcService {
    /// 创建新的IPC服务
    pub fn new() -> Result<Self, &'static str> {
        Self::new_with_config(IpcServiceConfig::default())
    }

    /// 使用配置创建新的IPC服务
    pub fn new_with_config(config: IpcServiceConfig) -> Result<Self, &'static str> {
        let service_id: ServiceId = 4; // 固定ID用于IPC服务 (ServiceId is u64)
        let message_queue = Arc::new(Mutex::new(MessageQueue::new(
            service_id,
            service_id,
            config.default_channel_capacity,
            config.default_channel_capacity,
        )));

        let service = Self {
            service_id,
            message_queue,
            channels: Arc::new(Mutex::new(BTreeMap::new())),
            channel_name_index: Arc::new(Mutex::new(BTreeMap::new())),
            next_channel_id: AtomicU64::new(1),
            shared_memory_regions: Arc::new(Mutex::new(BTreeMap::new())),
            memory_pools: Arc::new(Mutex::new(BTreeMap::new())),
            config,
            stats: Arc::new(Mutex::new(IpcServiceStats::new())),
        };

        Ok(service)
    }

    /// 创建优化的IPC通道
    pub fn create_optimized_channel(&self, name: String, channel_type: IpcChannelType) -> Result<u64, &'static str> {
        self.create_channel_with_transport(name, channel_type, self.config.default_channel_capacity, self.config.default_transport)
    }

    /// 创建带传输方式的IPC通道
    pub fn create_channel_with_transport(&self, name: String, channel_type: IpcChannelType, max_messages: usize, transport: IpcTransport) -> Result<u64, &'static str> {
        let channel_id = self.next_channel_id.fetch_add(1, Ordering::SeqCst);

        let mut channel = IpcChannel::new(channel_id, name.clone(), channel_type, max_messages)
            .with_default_transport(transport);

        // 根据传输方式添加优化特性
        match transport {
            IpcTransport::SharedMemory => {
                // 创建共享内存区域
                let shared_memory = SharedMemoryRegion::new(channel_id, max_messages * 1024, 0o666)?;
                let shared_memory_arc = Arc::new(shared_memory);

                // 注册共享内存区域
                {
                    let mut regions = self.shared_memory_regions.lock();
                    regions.insert(channel_id, shared_memory_arc.clone());
                }

                channel = channel.with_shared_memory(shared_memory_arc);
            }
            IpcTransport::MemoryPool => {
                // 创建内存池
                let memory_pool = MemoryPool::new(256, max_messages)?;
                let memory_pool_arc = Arc::new(Mutex::new(memory_pool));

                // 注册内存池
                {
                    let mut pools = self.memory_pools.lock();
                    pools.insert(256, memory_pool_arc.clone());
                }

                channel = channel.with_memory_pool(memory_pool_arc);
            }
            _ => {}
        }

        let name_clone = name.clone();

        {
            let mut channels = self.channels.lock();
            let mut name_index = self.channel_name_index.lock();

            if name_index.contains_key(&name) {
                return Err("Channel name already exists");
            }

            channels.insert(channel_id, channel);
            name_index.insert(name, channel_id);
        }

        // 更新统计信息
        {
            let stats = self.stats.lock();
            stats.total_channels.fetch_add(1, Ordering::Relaxed);
            stats.active_channels.fetch_add(1, Ordering::Relaxed);
        }

        crate::println!("[ipc] Created optimized channel: {} (ID: {}, Type: {:?}, Transport: {:?})",
                 name_clone, channel_id, channel_type, transport);

        Ok(channel_id)
    }

    /// 创建共享内存区域
    pub fn create_shared_memory(&self, size: usize, permissions: u32) -> Result<u64, &'static str> {
        let region_id = self.next_channel_id.fetch_add(1, Ordering::SeqCst);
        let shared_memory = SharedMemoryRegion::new(region_id, size, permissions)?;
        let shared_memory_arc = Arc::new(shared_memory);

        {
            let mut regions = self.shared_memory_regions.lock();
            regions.insert(region_id, shared_memory_arc);
        }

        crate::println!("[ipc] Created shared memory region: ID {}, size {} bytes", region_id, size);
        Ok(region_id)
    }

    /// 创建内存池
    pub fn create_memory_pool(&self, block_size: usize, capacity: usize) -> Result<usize, &'static str> {
        let memory_pool = MemoryPool::new(block_size, capacity)?;
        let memory_pool_arc = Arc::new(Mutex::new(memory_pool));

        {
            let mut pools = self.memory_pools.lock();
            pools.insert(block_size, memory_pool_arc);
        }

        crate::println!("[ipc] Created memory pool: block_size {}, capacity {}", block_size, capacity);
        Ok(block_size)
    }

    /// 创建IPC通道
    pub fn create_channel(&self, name: String, channel_type: IpcChannelType, max_messages: usize) -> Result<u64, &'static str> {
        let channel_id = self.next_channel_id.fetch_add(1, Ordering::SeqCst);
        let channel = IpcChannel::new(channel_id, name.clone(), channel_type, max_messages);

        let name_clone = name.clone();

        {
            let mut channels = self.channels.lock();
            let mut name_index = self.channel_name_index.lock();

            if name_index.contains_key(&name) {
                return Err("Channel name already exists");
            }

            channels.insert(channel_id, channel);
            name_index.insert(name, channel_id);
        }

        // 更新统计信息
        {
            let stats = self.stats.lock();
            stats.total_channels.fetch_add(1, Ordering::Relaxed);
            stats.active_channels.fetch_add(1, Ordering::Relaxed);
        }

        crate::println!("[ipc] Created channel: {} (ID: {}, Type: {:?})",
                 name_clone, channel_id, channel_type);

        Ok(channel_id)
    }

    /// 删除IPC通道
    pub fn destroy_channel(&self, channel_id: u64) -> Result<(), &'static str> {
        let mut channels = self.channels.lock();
        let mut name_index = self.channel_name_index.lock();

        if let Some(channel) = channels.remove(&channel_id) {
            name_index.remove(&channel.name);

            // 更新统计信息
            {
                let stats = self.stats.lock();
                stats.active_channels.fetch_sub(1, Ordering::Relaxed);
            }

            crate::println!("[ipc] Destroyed channel: {} (ID: {})", channel.name, channel_id);
            Ok(())
        } else {
            Err("Channel not found")
        }
    }

    /// 按名称获取通道
    pub fn get_channel_by_name(&self, name: &str) -> Option<u64> {
        let name_index = self.channel_name_index.lock();
        name_index.get(name).copied()
    }

    /// 按ID获取通道
    pub fn get_channel(&self, channel_id: u64) -> Option<Arc<IpcChannel>> {
        let channels = self.channels.lock();
        // 返回Arc包装的通道引用，这里简化处理
        channels.get(&channel_id).map(|_| {
            // 在实际实现中，需要使用Arc或其他共享引用机制
            Arc::new(IpcChannel::new(channel_id, String::new(), IpcChannelType::PointToPoint, 0))
        })
    }

    /// 发送消息到指定通道
    pub fn send_message(&self, channel_id: u64, message: IpcMessage) -> Result<(), i32> {
        let channels = self.channels.lock();

        if let Some(channel) = channels.get(&channel_id) {
            // 在发送消息前检查属性，避免move后的使用
            let is_zero_copy = message.is_zero_copy();
            let message_size = message.size();

            let result = channel.send(message);

            // 更新统计信息
            {
                let stats = self.stats.lock();
                stats.total_messages.fetch_add(1, Ordering::Relaxed);

                if is_zero_copy {
                    stats.zero_copy_messages.fetch_add(1, Ordering::Relaxed);
                }

                // 更新平均消息大小
                let current_avg = stats.avg_message_size.load(Ordering::Relaxed);
                let total_messages = stats.total_messages.load(Ordering::Relaxed);
                let new_avg = (current_avg * (total_messages - 1) + message_size as u64) / total_messages;
                stats.avg_message_size.store(new_avg, Ordering::Relaxed);
            }

            result
        } else {
            Err(ENOENT)
        }
    }

    /// 从指定通道接收消息
    pub fn receive_message(&self, channel_id: u64, receiver_id: u64) -> Result<IpcMessage, i32> {
        let channels = self.channels.lock();

        if let Some(channel) = channels.get(&channel_id) {
            channel.receive(receiver_id)
        } else {
            Err(ENOENT)
        }
    }

    /// 批量发送消息
    pub fn send_batch(&self, channel_id: u64, messages: Vec<IpcMessage>) -> Result<usize, i32> {
        let channels = self.channels.lock();

        if let Some(channel) = channels.get(&channel_id) {
            let result = channel.send_batch(messages);

            // 更新统计信息
            if let Ok(count) = result {
                let stats = self.stats.lock();
                stats.batch_operations.fetch_add(1, Ordering::Relaxed);
                stats.total_messages.fetch_add(count as u64, Ordering::Relaxed);
            }

            result
        } else {
            Err(ENOENT)
        }
    }

    /// 批量接收消息
    pub fn receive_batch(&self, channel_id: u64, receiver_id: u64, max_count: usize) -> Vec<IpcMessage> {
        let channels = self.channels.lock();

        if let Some(channel) = channels.get(&channel_id) {
            let messages = channel.receive_batch(receiver_id, max_count);

            // 更新统计信息
            {
                let stats = self.stats.lock();
                stats.total_messages.fetch_add(messages.len() as u64, Ordering::Relaxed);
                stats.batch_operations.fetch_add(1, Ordering::Relaxed);
            }

            messages
        } else {
            Vec::new()
        }
    }

    /// 获取所有通道
    pub fn get_all_channels(&self) -> Vec<(u64, String, IpcChannelType)> {
        let channels = self.channels.lock();
        channels.values()
            .map(|c| (c.id, c.name.clone(), c.channel_type))
            .collect()
    }

    /// 获取通道统计信息
    pub fn get_channel_stats(&self, channel_id: u64) -> Option<IpcChannelStatsSnapshot> {
        let channels = self.channels.lock();
        channels.get(&channel_id).map(|c| c.get_stats())
    }

    /// 获取服务统计信息
    pub fn get_stats(&self) -> IpcServiceStatsSnapshot {
        let stats = self.stats.lock();
        IpcServiceStatsSnapshot {
            total_channels: stats.total_channels.load(Ordering::Relaxed),
            active_channels: stats.active_channels.load(Ordering::Relaxed),
            total_messages: stats.total_messages.load(Ordering::Relaxed),
            zero_copy_messages: stats.zero_copy_messages.load(Ordering::Relaxed),
            batch_operations: stats.batch_operations.load(Ordering::Relaxed),
            avg_message_size: stats.avg_message_size.load(Ordering::Relaxed),
        }
    }

    /// 获取服务ID
    pub fn get_service_id(&self) -> ServiceId {
        self.service_id
    }
}

/// IPC服务统计信息快照
#[derive(Debug, Clone)]
pub struct IpcServiceStatsSnapshot {
    pub total_channels: usize,
    pub active_channels: usize,
    pub total_messages: u64,
    pub zero_copy_messages: u64,
    pub batch_operations: u64,
    pub avg_message_size: u64,
}

/// IPC服务管理器
pub struct IpcManager {
    /// IPC服务实例
    service: Arc<IpcService>,
    /// 是否已初始化
    initialized: bool,
}

impl IpcManager {
    /// 创建新的IPC管理器
    pub fn new() -> Result<Self, &'static str> {
        let service = Arc::new(IpcService::new()?);

        Ok(Self {
            service,
            initialized: false,
        })
    }

    /// 初始化IPC服务
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        if self.initialized {
            return Ok(());
        }

        // 注册到服务注册表
        let registry = get_service_registry().ok_or("Service registry not initialized")?;
        let service_info = ServiceInfo::new(
            self.service.get_service_id(),
            "IpcService".to_string(),
            "High-performance IPC communication service".to_string(),
            ServiceCategory::System,
            InterfaceVersion::new(1, 0, 0), // major, minor, patch are u16
            0, // owner_id - kernel owned
        );

        registry.register_service(service_info).map_err(|_| "Failed to register service")?;

        self.initialized = true;
        crate::println!("[ipc] High-performance IPC service initialized");
        Ok(())
    }

    /// 获取IPC服务引用
    pub fn get_service(&self) -> Arc<IpcService> {
        self.service.clone()
    }
}

// 全局IPC管理器实例
static mut IPC_MANAGER: Option<IpcManager> = None;
static IPC_MANAGER_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// 初始化IPC服务
pub fn init() -> Result<(), &'static str> {
    if IPC_MANAGER_INIT.load(core::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }

    unsafe {
        let mut manager = IpcManager::new()?;
        manager.initialize()?;
        IPC_MANAGER = Some(manager);
    }

    IPC_MANAGER_INIT.store(true, core::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// 获取全局IPC服务
pub fn get_ipc_service() -> Option<Arc<IpcService>> {
    unsafe {
        IPC_MANAGER.as_ref().map(|m| m.get_service())
    }
}

/// 获取IPC统计信息
pub fn get_stats() -> Option<IpcServiceStatsSnapshot> {
    let service = get_ipc_service()?;
    Some(service.get_stats())
}

/// 兼容性接口 - 提供给现有IPC代码使用
/// Send message using the legacy interface
pub fn send_message_to_queue(
    queue_id: u64,
    sender_id: u64,
    receiver_id: u64,
    _message_type: u32,
    data: Vec<u8>
) -> Result<(), i32> {
    let service = get_ipc_service().ok_or(ENODEV)?;

    let message = IpcMessage::new(sender_id, receiver_id, IpcMessageType::Data)
        .with_data(data)
        .with_transport(IpcTransport::MessageQueue)
        .with_mode(IpcMode::Synchronous);

    service.send_message(queue_id, message)
}

/// Receive message using the legacy interface
pub fn receive_message_from_queue(
    queue_id: u64,
    receiver_id: u64
) -> Result<IpcMessage, i32> {
    let service = get_ipc_service().ok_or(ENODEV)?;
    service.receive_message(queue_id, receiver_id)
}

/// 性能分析器
pub struct IpcPerformanceAnalyzer {
    /// 采样数据
    samples: Vec<PerformanceSample>,
    /// 最大采样数
    max_samples: usize,
    /// 分析统计
    stats: PerformanceStats,
}

/// 性能采样数据
#[derive(Debug, Clone)]
pub struct PerformanceSample {
    /// 时间戳
    pub timestamp: u64,
    /// 延迟（纳秒）
    pub latency_ns: u64,
    /// 吞吐量（消息/秒）
    pub throughput_mps: u64,
    /// 消息大小
    pub message_size: usize,
    /// 传输方式
    pub transport: IpcTransport,
    /// 操作类型
    pub operation: OperationType,
}

/// 操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    Send,
    Receive,
    BatchSend,
    BatchReceive,
}

/// 性能统计
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// 平均延迟
    pub avg_latency_ns: u64,
    /// 最大延迟
    pub max_latency_ns: u64,
    /// 最小延迟
    pub min_latency_ns: u64,
    /// 平均吞吐量
    pub avg_throughput_mps: u64,
    /// 最大吞吐量
    pub max_throughput_mps: u64,
    /// P50延迟
    pub p50_latency_ns: u64,
    /// P95延迟
    pub p95_latency_ns: u64,
    /// P99延迟
    pub p99_latency_ns: u64,
    /// 传输方式分布
    pub transport_distribution: BTreeMap<IpcTransport, u64>,
}

impl IpcPerformanceAnalyzer {
    /// 创建新的性能分析器
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
            stats: PerformanceStats {
                avg_latency_ns: 0,
                max_latency_ns: 0,
                min_latency_ns: u64::MAX,
                avg_throughput_mps: 0,
                max_throughput_mps: 0,
                p50_latency_ns: 0,
                p95_latency_ns: 0,
                p99_latency_ns: 0,
                transport_distribution: BTreeMap::new(),
            },
        }
    }

    /// 添加性能采样
    pub fn add_sample(&mut self, sample: PerformanceSample) {
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push(sample);
        self.update_stats();
    }

    /// 更新统计信息
    fn update_stats(&mut self) {
        if self.samples.is_empty() {
            return;
        }

        // 计算延迟统计
        let mut latencies: Vec<u64> = self.samples.iter().map(|s| s.latency_ns).collect();
        latencies.sort_unstable();

        self.stats.min_latency_ns = latencies[0];
        self.stats.max_latency_ns = latencies[latencies.len() - 1];
        self.stats.avg_latency_ns = latencies.iter().sum::<u64>() / latencies.len() as u64;

        // 计算百分位数
        let len = latencies.len();
        self.stats.p50_latency_ns = latencies[len * 50 / 100];
        self.stats.p95_latency_ns = latencies[len * 95 / 100];
        self.stats.p99_latency_ns = latencies[len * 99 / 100];

        // 计算吞吐量统计
        let throughputs: Vec<u64> = self.samples.iter().map(|s| s.throughput_mps).collect();
        self.stats.avg_throughput_mps = throughputs.iter().sum::<u64>() / throughputs.len() as u64;
        self.stats.max_throughput_mps = *throughputs.iter().max().unwrap_or(&0);

        // 计算传输方式分布
        self.stats.transport_distribution.clear();
        for sample in &self.samples {
            *self.stats.transport_distribution.entry(sample.transport).or_insert(0) += 1;
        }
    }

    /// 获取性能统计
    pub fn get_stats(&self) -> &PerformanceStats {
        &self.stats
    }

    /// 生成性能报告
    pub fn generate_report(&self) -> String {
        let stats = &self.stats;
        format!(
            "IPC Performance Analysis Report:\n\
             - Average Latency: {} ns\n\
             - P50 Latency: {} ns\n\
             - P95 Latency: {} ns\n\
             - P99 Latency: {} ns\n\
             - Max Latency: {} ns\n\
             - Average Throughput: {} msg/s\n\
             - Max Throughput: {} msg/s\n\
             - Transport Distribution: {:?}\n\
             - Sample Count: {}",
            stats.avg_latency_ns,
            stats.p50_latency_ns,
            stats.p95_latency_ns,
            stats.p99_latency_ns,
            stats.max_latency_ns,
            stats.avg_throughput_mps,
            stats.max_throughput_mps,
            stats.transport_distribution,
            self.samples.len()
        )
    }

    /// 优化建议
    pub fn get_optimization_suggestions(&self) -> Vec<String> {
        let mut suggestions = Vec::new();

        if self.stats.avg_latency_ns > 100_000 { // 100微秒
            suggestions.push("Consider using lock-free queues or shared memory for better latency".to_string());
        }

        if self.stats.p95_latency_ns > self.stats.avg_latency_ns * 3 {
            suggestions.push("High latency variance detected - check for system load or contention".to_string());
        }

        if self.stats.avg_throughput_mps < 100_000 { // 100K msg/s
            suggestions.push("Consider batch operations for better throughput".to_string());
        }

        // 检查传输方式分布
        let lock_free_count = self.stats.transport_distribution.get(&IpcTransport::LockFreeQueue).unwrap_or(&0);
        let total_count: u64 = self.stats.transport_distribution.values().sum();
        if total_count > 0 && *lock_free_count < total_count / 2 {
            suggestions.push("Consider using lock-free queues more frequently for better performance".to_string());
        }

        suggestions
    }
}

/// 获取当前时间（纳秒）
fn get_current_time_ns() -> u64 {
    crate::time::rdtsc() as u64
}

/// 计算吞吐量（消息/秒）
fn calculate_throughput(messages: u64, duration_ns: u64) -> u64 {
    if duration_ns == 0 {
        return 0;
    }
    (messages * 1_000_000_000) / duration_ns
}
