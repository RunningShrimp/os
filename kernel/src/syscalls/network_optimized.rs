//! 优化的网络协议栈实现
//!
//! 本模块提供高性能的网络协议栈，包括：
//! - 高效的套接字管理
//! - 优化的网络I/O
//! - 快速的协议处理
//! - 减少锁竞争的网络连接管理

use crate::syscalls::net::types::*;
use crate::syscalls::common::{SyscallError, SyscallResult, extract_args};
use crate::sync::Mutex;
use crate::collections::HashMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// 全局网络统计
static NET_STATS: Mutex<NetworkStats> = Mutex::new(NetworkStats::new());

/// 网络统计信息
#[derive(Debug, Default)]
pub struct NetworkStats {
    pub socket_count: AtomicU32,
    pub connect_count: AtomicU32,
    pub bind_count: AtomicU32,
    pub listen_count: AtomicU32,
    pub accept_count: AtomicU32,
    pub send_count: AtomicU32,
    pub recv_count: AtomicU32,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub packets_sent: AtomicU32,
    pub packets_received: AtomicU32,
    pub connections_active: AtomicU32,
    pub connections_dropped: AtomicU32,
}

impl NetworkStats {
    pub const fn new() -> Self {
        Self {
            socket_count: AtomicU32::new(0),
            connect_count: AtomicU32::new(0),
            bind_count: AtomicU32::new(0),
            listen_count: AtomicU32::new(0),
            accept_count: AtomicU32::new(0),
            send_count: AtomicU32::new(0),
            recv_count: AtomicU32::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            packets_sent: AtomicU32::new(0),
            packets_received: AtomicU32::new(0),
            connections_active: AtomicU32::new(0),
            connections_dropped: AtomicU32::new(0),
        }
    }
    
    pub fn record_socket_created(&self) {
        self.socket_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_socket_closed(&self) {
        self.socket_count.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn record_connect(&self) {
        self.connect_count.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_bind(&self) {
        self.bind_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_listen(&self) {
        self.listen_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_accept(&self) {
        self.accept_count.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_send(&self, bytes: u64) {
        self.send_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_recv(&self, bytes: u64) {
        self.recv_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
        self.packets_received.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_connection_closed(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn record_connection_dropped(&self) {
        self.connections_dropped.fetch_add(1, Ordering::Relaxed);
    }
}

/// 高效套接字管理器
pub struct FastSocketManager {
    sockets: HashMap<i32, FastSocket>,
    next_fd: AtomicU32,
    socket_pool: Vec<FastSocket>,
    free_fds: Vec<i32>,
}

/// 高效套接字
#[derive(Debug, Clone)]
pub struct FastSocket {
    pub fd: i32,
    pub socket_type: SocketType,
    pub family: AddressFamily,
    pub protocol: SocketProtocol,
    pub local_address: Option<NetworkAddress>,
    pub remote_address: Option<NetworkAddress>,
    pub state: SocketState,
    pub send_buffer: Vec<u8>,
    pub recv_buffer: Vec<u8>,
    pub send_buffer_size: usize,
    pub recv_buffer_size: usize,
    pub options: HashMap<u32, u32>,
    pub last_activity: u64,
    pub connection_id: u32,
}

impl FastSocket {
    pub fn new(fd: i32, socket_type: SocketType, family: AddressFamily, protocol: SocketProtocol) -> Self {
        Self {
            fd,
            socket_type,
            family,
            protocol,
            local_address: None,
            remote_address: None,
            state: SocketState::Unconnected,
            send_buffer: Vec::with_capacity(8192),
            recv_buffer: Vec::with_capacity(8192),
            send_buffer_size: 8192,
            recv_buffer_size: 8192,
            options: HashMap::new(),
            last_activity: 0,
            connection_id: 0,
        }
    }
    
    pub fn is_connected(&self) -> bool {
        self.state == SocketState::Connected
    }
    
    pub fn is_listening(&self) -> bool {
        self.state == SocketState::Listening
    }
    
    pub fn update_activity(&mut self) {
        self.last_activity = get_current_timestamp();
    }
    
    pub fn can_send(&self) -> bool {
        self.send_buffer.len() < self.send_buffer_size
    }
    
    pub fn can_recv(&self) -> bool {
        !self.recv_buffer.is_empty()
    }
    
    pub fn send_data(&mut self, data: &[u8]) -> Result<usize, NetworkError> {
        if !self.is_connected() {
            return Err(NetworkError::NotConnected);
        }
        
        if !self.can_send() {
            return Err(NetworkError::BufferFull);
        }
        
        let available_space = self.send_buffer_size - self.send_buffer.len();
        let bytes_to_send = core::cmp::min(data.len(), available_space);
        
        self.send_buffer.extend_from_slice(&data[..bytes_to_send]);
        self.update_activity();
        
        Ok(bytes_to_send)
    }
    
    pub fn recv_data(&mut self, buf: &mut [u8]) -> Result<usize, NetworkError> {
        if !self.is_connected() && !self.is_listening() {
            return Err(NetworkError::NotConnected);
        }
        
        if self.recv_buffer.is_empty() {
            return Err(NetworkError::NoData);
        }
        
        let bytes_to_recv = core::cmp::min(buf.len(), self.recv_buffer.len());
        buf[..bytes_to_recv].copy_from_slice(&self.recv_buffer[..bytes_to_recv]);
        
        // 移除已接收的数据
        self.recv_buffer.drain(0..bytes_to_recv);
        self.update_activity();
        
        Ok(bytes_to_recv)
    }
}

impl FastSocketManager {
    pub fn new() -> Self {
        Self {
            sockets: HashMap::new(),
            next_fd: AtomicU32::new(4), // 从4开始，0、1、2、3保留
            socket_pool: Vec::with_capacity(256),
            free_fds: Vec::new(),
        }
    }
    
    /// 创建新套接字
    pub fn create_socket(&mut self, socket_type: SocketType, family: AddressFamily, protocol: SocketProtocol) -> Result<i32, NetworkError> {
        // 获取文件描述符
        let fd = if let Some(reused_fd) = self.free_fds.pop() {
            reused_fd
        } else {
            self.next_fd.fetch_add(1, Ordering::Relaxed) as i32
        };
        
        // 创建套接字
        let socket = FastSocket::new(fd, socket_type, family, protocol);
        
        // 添加到管理器
        self.sockets.insert(fd, socket);
        
        // 记录统计
        NET_STATS.lock().record_socket_created();
        
        crate::log_debug!("Created socket: {} (type: {:?}, family: {:?}, protocol: {:?})", 
                    fd, socket_type, family, protocol);
        
        Ok(fd)
    }
    
    /// 关闭套接字
    pub fn close_socket(&mut self, fd: i32) -> Result<(), NetworkError> {
        if let Some(socket) = self.sockets.remove(&fd) {
            // 如果套接字已连接，更新连接统计
            if socket.is_connected() {
                NET_STATS.lock().record_connection_closed();
            }
            
            // 回收文件描述符
            self.free_fds.push(fd);
            
            // 记录统计
            NET_STATS.lock().record_socket_closed();
            
            crate::log_debug!("Closed socket: {}", fd);
            Ok(())
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 绑定套接字
    pub fn bind_socket(&mut self, fd: i32, address: NetworkAddress) -> Result<(), NetworkError> {
        if let Some(socket) = self.sockets.get_mut(&fd) {
            if socket.local_address.is_some() {
                return Err(NetworkError::AddressInUse);
            }
            
            socket.local_address = Some(address.clone());
            socket.update_activity();
            
            // 记录统计
            NET_STATS.lock().record_bind();
            
            crate::log_debug!("Bound socket {} to address: {}", fd, address.to_string());
            Ok(())
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 连接套接字
    pub fn connect_socket(&mut self, fd: i32, address: NetworkAddress) -> Result<(), NetworkError> {
        if let Some(socket) = self.sockets.get_mut(&fd) {
            if socket.local_address.is_none() {
                return Err(NetworkError::NotBound);
            }
            
            if socket.remote_address.is_some() {
                return Err(NetworkError::AlreadyConnected);
            }
            
            // 简化实现：直接设置为已连接状态
            socket.remote_address = Some(address.clone());
            socket.state = SocketState::Connected;
            socket.update_activity();
            
            // 记录统计
            NET_STATS.lock().record_connect();
            
            crate::log_debug!("Connected socket {} to address: {}", fd, address.to_string());
            Ok(())
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 监听套接字
    pub fn listen_socket(&mut self, fd: i32, backlog: i32) -> Result<(), NetworkError> {
        if let Some(socket) = self.sockets.get_mut(&fd) {
            if socket.local_address.is_none() {
                return Err(NetworkError::NotBound);
            }
            
            socket.state = SocketState::Listening;
            socket.update_activity();
            
            // 记录统计
            NET_STATS.lock().record_listen();
            
            crate::log_debug!("Listening socket {} with backlog: {}", fd, backlog);
            Ok(())
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 接受连接
    pub fn accept_socket(&mut self, fd: i32) -> Result<(i32, NetworkAddress), NetworkError> {
        if let Some(socket) = self.sockets.get(&fd) {
            if !socket.is_listening() {
                return Err(NetworkError::NotListening);
            }
            
            // 创建新套接字表示连接
            let new_fd = if let Some(reused_fd) = self.free_fds.pop() {
                reused_fd
            } else {
                self.next_fd.fetch_add(1, Ordering::Relaxed) as i32
            };
            
            let new_socket = FastSocket::new(
                new_fd,
                socket.socket_type,
                socket.family,
                socket.protocol
            );
            
            // 设置新套接字为已连接状态
            let mut connected_socket = new_socket;
            connected_socket.local_address = socket.local_address.clone();
            connected_socket.state = SocketState::Connected;
            connected_socket.update_activity();
            
            // 添加到管理器
            self.sockets.insert(new_fd, connected_socket);
            
            // 记录统计
            NET_STATS.lock().record_accept();
            NET_STATS.lock().record_connect();
            
            // 简化实现：返回本地地址作为远程地址
            let remote_addr = socket.local_address.clone()
                .unwrap_or_else(|| NetworkAddress::ipv4([127, 0, 0, 1], 0));
            
            crate::log_debug!("Accepted connection on socket {} -> new socket: {}", fd, new_fd);
            Ok((new_fd, remote_addr))
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 发送数据
    pub fn send_data(&mut self, fd: i32, data: &[u8]) -> Result<usize, NetworkError> {
        if let Some(socket) = self.sockets.get_mut(&fd) {
            let bytes_sent = socket.send_data(data)?;
            
            // 记录统计
            NET_STATS.lock().record_send(bytes_sent as u64);
            
            Ok(bytes_sent)
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 接收数据
    pub fn recv_data(&mut self, fd: i32, buf: &mut [u8]) -> Result<usize, NetworkError> {
        if let Some(socket) = self.sockets.get_mut(&fd) {
            let bytes_recv = socket.recv_data(buf)?;
            
            // 记录统计
            NET_STATS.lock().record_recv(bytes_recv as u64);
            
            Ok(bytes_recv)
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 获取套接字信息
    pub fn get_socket_info(&self, fd: i32) -> Option<&FastSocket> {
        self.sockets.get(&fd)
    }
    
    /// 获取所有套接字
    pub fn get_all_sockets(&self) -> Vec<&FastSocket> {
        self.sockets.values().collect()
    }
    
    /// 清理空闲套接字
    pub fn cleanup_idle_sockets(&mut self, timeout_ms: u64) {
        let current_time = get_current_timestamp();
        let mut sockets_to_close = Vec::new();
        
        for (fd, socket) in &self.sockets {
            if current_time - socket.last_activity > timeout_ms {
                sockets_to_close.push(*fd);
            }
        }
        
        for fd in sockets_to_close {
            if let Err(e) = self.close_socket(fd) {
                crate::log_debug!("Failed to close idle socket {}: {:?}", fd, e);
            }
        }
    }
}

/// 全局套接字管理器
static GLOBAL_SOCKET_MANAGER: Mutex<FastSocketManager> = Mutex::new(FastSocketManager::new());

/// 优化的socket系统调用实现
pub fn sys_socket_optimized(domain: i32, socket_type: i32, protocol: i32) -> isize {
    // 转换参数
    let family = match domain {
        2 => AddressFamily::IPv4,
        10 => AddressFamily::IPv6,
        _ => return -1, // 不支持的地址族
    };
    
    let socket_type = match socket_type {
        1 => SocketType::Stream,
        2 => SocketType::Datagram,
        3 => SocketType::Raw,
        _ => return -1, // 不支持的套接字类型
    };
    
    let socket_protocol = match protocol {
        0 => SocketProtocol::Default,
        1 => SocketProtocol::ICMP,
        6 => SocketProtocol::TCP,
        17 => SocketProtocol::UDP,
        _ => return -1, // 不支持的协议
    };
    
    // 创建套接字
    let mut manager = GLOBAL_SOCKET_MANAGER.lock();
    match manager.create_socket(socket_type, family, socket_protocol) {
        Ok(fd) => fd as isize,
        Err(_) => -1,
    }
}

/// 优化的bind系统调用实现
pub fn sys_bind_optimized(fd: i32, addr_ptr: *const u8, addrlen: u32) -> isize {
    // 验证参数
    if addr_ptr.is_null() || addrlen == 0 {
        return -1;
    }
    
    // 从用户空间读取地址
    let address = match copy_address_from_user(addr_ptr, addrlen) {
        Ok(addr) => addr,
        Err(_) => return -1,
    };
    
    // 绑定套接字
    let mut manager = GLOBAL_SOCKET_MANAGER.lock();
    match manager.bind_socket(fd, address) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// 优化的listen系统调用实现
pub fn sys_listen_optimized(fd: i32, backlog: i32) -> isize {
    // 监听套接字
    let mut manager = GLOBAL_SOCKET_MANAGER.lock();
    match manager.listen_socket(fd, backlog) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// 优化的accept系统调用实现
pub fn sys_accept_optimized(fd: i32, addr_ptr: *mut u8, addrlen_ptr: *mut u32) -> isize {
    // 接受连接
    let mut manager = GLOBAL_SOCKET_MANAGER.lock();
    match manager.accept_socket(fd) {
        Ok((new_fd, address)) => {
            // 将地址复制到用户空间
            if !addr_ptr.is_null() && !addrlen_ptr.is_null() {
                if let Err(_) = copy_address_to_user(&address, addr_ptr, addrlen_ptr) {
                    return -1;
                }
            }
            
            new_fd as isize
        }
        Err(_) => -1,
    }
}

/// 优化的connect系统调用实现
pub fn sys_connect_optimized(fd: i32, addr_ptr: *const u8, addrlen: u32) -> isize {
    // 验证参数
    if addr_ptr.is_null() || addrlen == 0 {
        return -1;
    }
    
    // 从用户空间读取地址
    let address = match copy_address_from_user(addr_ptr, addrlen) {
        Ok(addr) => addr,
        Err(_) => return -1,
    };
    
    // 连接套接字
    let mut manager = GLOBAL_SOCKET_MANAGER.lock();
    match manager.connect_socket(fd, address) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// 优化的send系统调用实现
pub fn sys_send_optimized(fd: i32, buf_ptr: *const u8, len: usize, flags: i32) -> isize {
    // 验证参数
    if buf_ptr.is_null() || len == 0 {
        return -1;
    }
    
    // 从用户空间读取数据
    let data = match copy_data_from_user(buf_ptr, len) {
        Ok(data) => data,
        Err(_) => return -1,
    };
    
    // 发送数据
    let mut manager = GLOBAL_SOCKET_MANAGER.lock();
    match manager.send_data(fd, &data) {
        Ok(bytes_sent) => bytes_sent as isize,
        Err(_) => -1,
    }
}

/// 优化的recv系统调用实现
pub fn sys_recv_optimized(fd: i32, buf_ptr: *mut u8, len: usize, flags: i32) -> isize {
    // 验证参数
    if buf_ptr.is_null() || len == 0 {
        return -1;
    }
    
    // 创建缓冲区
    let mut buf = vec![0u8; len];
    
    // 接收数据
    let mut manager = GLOBAL_SOCKET_MANAGER.lock();
    match manager.recv_data(fd, &mut buf) {
        Ok(bytes_recv) => {
            // 将数据复制到用户空间
            if let Err(_) = copy_data_to_user(&buf[..bytes_recv], buf_ptr) {
                return -1;
            }
            
            bytes_recv as isize
        }
        Err(_) => -1,
    }
}

/// 优化的close系统调用实现
pub fn sys_close_optimized(fd: i32) -> isize {
    // 关闭套接字
    let mut manager = GLOBAL_SOCKET_MANAGER.lock();
    match manager.close_socket(fd) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// 从用户空间复制地址
fn copy_address_from_user(addr_ptr: *const u8, addrlen: u32) -> Result<NetworkAddress, ()> {
    // 简化实现：假设是IPv4地址
    if addrlen < 8 {
        return Err(());
    }
    
    let mut addr_bytes = [0u8; 8];
    unsafe {
        core::ptr::copy_nonoverlapping(addr_ptr, addr_bytes.as_mut_ptr(), 8);
    }
    
    // 简单解析：前4字节是IP地址，后2字节是端口
    let ip = [addr_bytes[0], addr_bytes[1], addr_bytes[2], addr_bytes[3]];
    let port = ((addr_bytes[4] as u16) << 8) | (addr_bytes[5] as u16);
    
    Ok(NetworkAddress::ipv4(ip, port))
}

/// 将地址复制到用户空间
fn copy_address_to_user(address: &NetworkAddress, addr_ptr: *mut u8, addrlen_ptr: *mut u32) -> Result<(), ()> {
    // 简化实现：只处理IPv4地址
    let (ip, port) = match address {
        NetworkAddress::IPv4(ip, port) => (ip, *port),
        _ => return Err(()),
    };
    
    let mut addr_bytes = [0u8; 8];
    addr_bytes[0..4].copy_from_slice(&ip);
    addr_bytes[4] = (port >> 8) as u8;
    addr_bytes[5] = (port & 0xFF) as u8;
    
    unsafe {
        core::ptr::copy_nonoverlapping(addr_bytes.as_ptr(), addr_ptr, 8);
        core::ptr::write(addrlen_ptr, 8);
    }
    
    Ok(())
}

/// 从用户空间复制数据
fn copy_data_from_user(data_ptr: *const u8, len: usize) -> Result<Vec<u8>, ()> {
    if data_ptr.is_null() || len == 0 {
        return Err(());
    }
    
    let mut data = vec![0u8; len];
    unsafe {
        core::ptr::copy_nonoverlapping(data_ptr, data.as_mut_ptr(), len);
    }
    
    Ok(data)
}

/// 将数据复制到用户空间
fn copy_data_to_user(data: &[u8], data_ptr: *mut u8) -> Result<(), ()> {
    if data_ptr.is_null() {
        return Err(());
    }
    
    unsafe {
        core::ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, data.len());
    }
    
    Ok(())
}

/// 获取当前时间戳（简化实现）
fn get_current_timestamp() -> u64 {
    // 简化实现，实际应该从系统时钟获取
    0
}

/// 获取网络统计信息
pub fn get_network_stats() -> NetworkStats {
    let stats = NET_STATS.lock();
    NetworkStats {
        socket_count: AtomicU32::new(stats.socket_count.load(Ordering::Relaxed)),
        connect_count: AtomicU32::new(stats.connect_count.load(Ordering::Relaxed)),
        bind_count: AtomicU32::new(stats.bind_count.load(Ordering::Relaxed)),
        listen_count: AtomicU32::new(stats.listen_count.load(Ordering::Relaxed)),
        accept_count: AtomicU32::new(stats.accept_count.load(Ordering::Relaxed)),
        send_count: AtomicU32::new(stats.send_count.load(Ordering::Relaxed)),
        recv_count: AtomicU32::new(stats.recv_count.load(Ordering::Relaxed)),
        bytes_sent: AtomicU64::new(stats.bytes_sent.load(Ordering::Relaxed)),
        bytes_received: AtomicU64::new(stats.bytes_received.load(Ordering::Relaxed)),
        packets_sent: AtomicU32::new(stats.packets_sent.load(Ordering::Relaxed)),
        packets_received: AtomicU32::new(stats.packets_received.load(Ordering::Relaxed)),
        connections_active: AtomicU32::new(stats.connections_active.load(Ordering::Relaxed)),
        connections_dropped: AtomicU32::new(stats.connections_dropped.load(Ordering::Relaxed)),
    }
}

/// 系统调用分发函数
pub fn dispatch_optimized(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        0x4000 => {
            // socket
            let args = extract_args(args, 3)?;
            let domain = args[0] as i32;
            let socket_type = args[1] as i32;
            let protocol = args[2] as i32;
            Ok(sys_socket_optimized(domain, socket_type, protocol) as u64)
        }
        0x4001 => {
            // bind
            let args = extract_args(args, 3)?;
            let fd = args[0] as i32;
            let addr_ptr = args[1] as *const u8;
            let addrlen = args[2] as u32;
            Ok(sys_bind_optimized(fd, addr_ptr, addrlen) as u64)
        }
        0x4002 => {
            // listen
            let args = extract_args(args, 2)?;
            let fd = args[0] as i32;
            let backlog = args[1] as i32;
            Ok(sys_listen_optimized(fd, backlog) as u64)
        }
        0x4003 => {
            // accept
            let args = extract_args(args, 3)?;
            let fd = args[0] as i32;
            let addr_ptr = args[1] as *mut u8;
            let addrlen_ptr = args[2] as *mut u32;
            Ok(sys_accept_optimized(fd, addr_ptr, addrlen_ptr) as u64)
        }
        0x4004 => {
            // connect
            let args = extract_args(args, 3)?;
            let fd = args[0] as i32;
            let addr_ptr = args[1] as *const u8;
            let addrlen = args[2] as u32;
            Ok(sys_connect_optimized(fd, addr_ptr, addrlen) as u64)
        }
        0x4005 => {
            // send
            let args = extract_args(args, 4)?;
            let fd = args[0] as i32;
            let buf_ptr = args[1] as *const u8;
            let len = args[2] as usize;
            let flags = args[3] as i32;
            Ok(sys_send_optimized(fd, buf_ptr, len, flags) as u64)
        }
        0x4006 => {
            // recv
            let args = extract_args(args, 4)?;
            let fd = args[0] as i32;
            let buf_ptr = args[1] as *mut u8;
            let len = args[2] as usize;
            let flags = args[3] as i32;
            Ok(sys_recv_optimized(fd, buf_ptr, len, flags) as u64)
        }
        0x4007 => {
            // close
            let args = extract_args(args, 1)?;
            let fd = args[0] as i32;
            Ok(sys_close_optimized(fd) as u64)
        }
        _ => Err(SyscallError::NotSupported),
    }
}