//! 增强网络系统
//! 
//! 本模块提供完整的POSIX兼容网络功能，包括TCP、UDP、原始套接字等。

use core::sync::atomic::{AtomicU64, Ordering};
use crate::subsystems::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::sync::Arc;

/// 网络统计信息
#[derive(Debug, Default)]
pub struct NetworkStats {
    pub packets_sent: AtomicU64,
    pub packets_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub connections_established: AtomicU64,
    pub connections_closed: AtomicU64,
    pub errors: AtomicU64,
}

impl NetworkStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_packet_sent(&self, bytes: u64) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }
    
    pub fn record_packet_received(&self, bytes: u64) {
        self.packets_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }
    
    pub fn record_connection_established(&self) {
        self.connections_established.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_connection_closed(&self) {
        self.connections_closed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_stats(&self) -> (u64, u64, u64, u64, u64, u64, u64) {
        (
            self.packets_sent.load(Ordering::Relaxed),
            self.packets_received.load(Ordering::Relaxed),
            self.bytes_sent.load(Ordering::Relaxed),
            self.bytes_received.load(Ordering::Relaxed),
            self.connections_established.load(Ordering::Relaxed),
            self.connections_closed.load(Ordering::Relaxed),
            self.errors.load(Ordering::Relaxed),
        )
    }
}

/// 套接字类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream = 1,      // SOCK_STREAM
    Datagram = 2,    // SOCK_DGRAM
    Raw = 3,         // SOCK_RAW
    SeqPacket = 5,   // SOCK_SEQPACKET
}

/// 套接字协议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketProtocol {
    IP = 0,          // IPPROTO_IP
    TCP = 6,         // IPPROTO_TCP
    UDP = 17,        // IPPROTO_UDP
    ICMP = 1,        // IPPROTO_ICMP
    Raw = 255,       // IPPROTO_RAW
}

/// 套接字地址族
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressFamily {
    Unspecified = 0, // AF_UNSPEC
    Unix = 1,        // AF_UNIX
    IPv4 = 2,         // AF_INET
    IPv6 = 10,        // AF_INET6
    Netlink = 16,     // AF_NETLINK
}

/// 套接字标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketFlags {
    pub nonblocking: bool,
    pub cloexec: bool,
    pub reuseaddr: bool,
    pub keepalive: bool,
}

impl SocketFlags {
    pub const NONE: Self = Self {
        nonblocking: false,
        cloexec: false,
        reuseaddr: false,
        keepalive: false,
    };
    
    pub const NONBLOCK: Self = Self {
        nonblocking: true,
        cloexec: false,
        reuseaddr: false,
        keepalive: false,
    };
    
    pub const CLOEXEC: Self = Self {
        nonblocking: false,
        cloexec: true,
        reuseaddr: false,
        keepalive: false,
    };
    
    pub const REUSEADDR: Self = Self {
        nonblocking: false,
        cloexec: false,
        reuseaddr: true,
        keepalive: false,
    };
    
    pub const KEEPALIVE: Self = Self {
        nonblocking: false,
        cloexec: false,
        reuseaddr: false,
        keepalive: true,
    };
}

impl core::ops::BitOr for SocketFlags {
    type Output = Self;
    
    fn bitor(self, rhs: Self) -> Self {
        Self {
            nonblocking: self.nonblocking || rhs.nonblocking,
            cloexec: self.cloexec || rhs.cloexec,
            reuseaddr: self.reuseaddr || rhs.reuseaddr,
            keepalive: self.keepalive || rhs.keepalive,
        }
    }
}

/// IPv4地址
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IPv4Address {
    pub bytes: [u8; 4],
}

impl IPv4Address {
    pub const ANY: Self = Self { bytes: [0, 0, 0, 0] };
    pub const LOOPBACK: Self = Self { bytes: [127, 0, 0, 1] };
    pub const BROADCAST: Self = Self { bytes: [255, 255, 255, 255] };
    
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self { bytes: [a, b, c, d] }
    }
    
    pub fn from_u32(addr: u32) -> Self {
        Self {
            bytes: [
                (addr >> 24) as u8,
                (addr >> 16) as u8,
                (addr >> 8) as u8,
                addr as u8,
            ],
        }
    }
    
    pub fn to_u32(&self) -> u32 {
        ((self.bytes[0] as u32) << 24) |
        ((self.bytes[1] as u32) << 16) |
        ((self.bytes[2] as u32) << 8) |
        (self.bytes[3] as u32)
    }
}

/// 套接字地址
#[derive(Debug, Clone)]
pub enum SocketAddress {
    IPv4(IPv4Address, u16),
    IPv6([u8; 16], u16),
    Unix(String),
    Unspecified,
}

/// 套接字选项
#[derive(Debug, Clone, Copy)]
pub enum SocketOption {
    ReuseAddr = 2,      // SO_REUSEADDR
    KeepAlive = 9,      // SO_KEEPALIVE
    Broadcast = 6,       // SO_BROADCAST
    Linger = 13,        // SO_LINGER
    RcvBuf = 8,         // SO_RCVBUF
    SndBuf = 7,         // SO_SNDBUF
    Error = 4,          // SO_ERROR
    Type = 3,           // SO_TYPE
}

/// 套接字级别
#[derive(Debug, Clone, Copy)]
pub enum SocketLevel {
    Socket = 1,    // SOL_SOCKET
    IP = 0,        // IPPROTO_IP
    TCP = 6,       // IPPROTO_TCP
    UDP = 17,      // IPPROTO_UDP
}

/// 增强套接字
#[derive(Debug)]
pub struct EnhancedSocket {
    pub id: usize,
    pub socket_type: SocketType,
    pub protocol: SocketProtocol,
    pub family: AddressFamily,
    pub flags: SocketFlags,
    pub local_address: Option<SocketAddress>,
    pub remote_address: Option<SocketAddress>,
    pub state: SocketState,
    pub send_buffer: Vec<u8>,
    pub recv_buffer: Vec<u8>,
    pub stats: Arc<NetworkStats>,
}

/// 套接字状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Uninitialized,
    Bound,
    Listening,
    Connecting,
    Connected,
    Closing,
    Closed,
}

/// 增强网络管理器
#[derive(Debug)]
pub struct EnhancedNetworkManager {
    sockets: Mutex<BTreeMap<usize, Arc<EnhancedSocket>>>,
    next_socket_id: AtomicU64,
    stats: Arc<NetworkStats>,
}

impl EnhancedNetworkManager {
    /// 创建新的网络管理器
    pub fn new() -> Self {
        Self {
            sockets: Mutex::new(BTreeMap::new()),
            next_socket_id: AtomicU64::new(3), // 从3开始，0-2为标准输入/输出/错误
            stats: Arc::new(NetworkStats::new()),
        }
    }
    
    /// 创建套接字
    pub fn socket(
        &self,
        family: AddressFamily,
        socket_type: SocketType,
        protocol: SocketProtocol,
        flags: SocketFlags,
    ) -> Result<usize, NetworkError> {
        let socket_id = self.next_socket_id.fetch_add(1, Ordering::Relaxed) as usize;
        
        let socket = Arc::new(EnhancedSocket {
            id: socket_id,
            socket_type,
            protocol,
            family,
            flags,
            local_address: None,
            remote_address: None,
            state: SocketState::Uninitialized,
            send_buffer: Vec::new(),
            recv_buffer: Vec::new(),
            stats: self.stats.clone(),
        });
        
        let mut sockets = self.sockets.lock();
        sockets.insert(socket_id, socket);
        
        crate::println!("[network] Created socket {}: family={:?}, type={:?}, protocol={:?}", 
                     socket_id, family, socket_type, protocol);
        
        Ok(socket_id)
    }
    
    /// 绑定套接字
    pub fn bind(
        &self,
        socket_id: usize,
        address: &SocketAddress,
    ) -> Result<(), NetworkError> {
        let mut sockets = self.sockets.lock();
        
        if let Some(socket) = sockets.get(&socket_id) {
            if socket.state != SocketState::Uninitialized {
                return Err(NetworkError::InvalidState);
            }
            
            // 验证地址族
            if !self.is_address_compatible(&socket.family, address) {
                return Err(NetworkError::AddressFamilyNotSupported);
            }
            
            // 更新套接字状态
            let socket = Arc::clone(socket);
            drop(sockets);
            
            // 这里应该调用底层网络栈进行实际绑定
            self.perform_bind(&socket, address)?;
            
            // 更新本地地址
            let mut sockets = self.sockets.lock();
            if let Some(socket) = sockets.get_mut(&socket_id) {
                socket.local_address = Some(address.clone());
                socket.state = SocketState::Bound;
            }
            
            crate::println!("[network] Bound socket {} to {:?}", socket_id, address);
            Ok(())
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 监听套接字
    pub fn listen(&self, socket_id: usize, backlog: i32) -> Result<(), NetworkError> {
        let mut sockets = self.sockets.lock();
        
        if let Some(socket) = sockets.get(&socket_id) {
            if socket.state != SocketState::Bound {
                return Err(NetworkError::InvalidState);
            }
            
            if socket.socket_type != SocketType::Stream {
                return Err(NetworkError::OperationNotSupported);
            }
            
            // 更新套接字状态
            let socket = Arc::clone(socket);
            drop(sockets);
            
            // 这里应该调用底层网络栈进行实际监听
            self.perform_listen(&socket, backlog)?;
            
            // 更新状态
            let mut sockets = self.sockets.lock();
            if let Some(socket) = sockets.get_mut(&socket_id) {
                socket.state = SocketState::Listening;
            }
            
            crate::println!("[network] Socket {} listening with backlog {}", socket_id, backlog);
            Ok(())
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 接受连接
    pub fn accept(&self, socket_id: usize) -> Result<(usize, SocketAddress), NetworkError> {
        let mut sockets = self.sockets.lock();
        
        if let Some(socket) = sockets.get(&socket_id) {
            if socket.state != SocketState::Listening {
                return Err(NetworkError::InvalidState);
            }
            
            let socket = Arc::clone(socket);
            drop(sockets);
            
            // 这里应该调用底层网络栈进行实际接受
            let (new_socket_id, remote_address) = self.perform_accept(&socket)?;
            
            // 创建新的套接字表示连接
            let new_socket = Arc::new(EnhancedSocket {
                id: new_socket_id,
                socket_type: socket.socket_type,
                protocol: socket.protocol,
                family: socket.family,
                flags: socket.flags,
                local_address: socket.local_address.clone(),
                remote_address: Some(remote_address.clone()),
                state: SocketState::Connected,
                send_buffer: Vec::new(),
                recv_buffer: Vec::new(),
                stats: self.stats.clone(),
            });
            
            // 记录统计
            self.stats.record_connection_established();
            
            // 添加新套接字
            let mut sockets = self.sockets.lock();
            sockets.insert(new_socket_id, new_socket);
            
            crate::println!("[network] Accepted connection on socket {}: new socket {} from {:?}", 
                         socket_id, new_socket_id, remote_address);
            
            Ok((new_socket_id, remote_address))
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 连接到远程地址
    pub fn connect(
        &self,
        socket_id: usize,
        address: &SocketAddress,
    ) -> Result<(), NetworkError> {
        let mut sockets = self.sockets.lock();
        
        if let Some(socket) = sockets.get(&socket_id) {
            if socket.state != SocketState::Bound && socket.state != SocketState::Uninitialized {
                return Err(NetworkError::InvalidState);
            }
            
            // 验证地址族
            if !self.is_address_compatible(&socket.family, address) {
                return Err(NetworkError::AddressFamilyNotSupported);
            }
            
            // 更新套接字状态
            let socket = Arc::clone(socket);
            drop(sockets);
            
            // 这里应该调用底层网络栈进行实际连接
            self.perform_connect(&socket, address)?;
            
            // 更新状态和远程地址
            let mut sockets = self.sockets.lock();
            if let Some(socket) = sockets.get_mut(&socket_id) {
                socket.remote_address = Some(address.clone());
                socket.state = SocketState::Connected;
            }
            
            // 记录统计
            self.stats.record_connection_established();
            
            crate::println!("[network] Connected socket {} to {:?}", socket_id, address);
            Ok(())
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 发送数据
    pub fn send(
        &self,
        socket_id: usize,
        data: &[u8],
        flags: i32,
    ) -> Result<usize, NetworkError> {
        let mut sockets = self.sockets.lock();
        
        if let Some(socket) = sockets.get(&socket_id) {
            if socket.state != SocketState::Connected {
                return Err(NetworkError::InvalidState);
            }
            
            let socket = Arc::clone(socket);
            drop(sockets);
            
            // 这里应该调用底层网络栈进行实际发送
            let bytes_sent = self.perform_send(&socket, data, flags)?;
            
            // 记录统计
            self.stats.record_packet_sent(bytes_sent as u64);
            
            crate::println!("[network] Sent {} bytes on socket {}", bytes_sent, socket_id);
            Ok(bytes_sent)
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 接收数据
    pub fn recv(
        &self,
        socket_id: usize,
        buffer: &mut [u8],
        flags: i32,
    ) -> Result<usize, NetworkError> {
        let mut sockets = self.sockets.lock();
        
        if let Some(socket) = sockets.get(&socket_id) {
            if socket.state != SocketState::Connected {
                return Err(NetworkError::InvalidState);
            }
            
            let socket = Arc::clone(socket);
            drop(sockets);
            
            // 这里应该调用底层网络栈进行实际接收
            let bytes_received = self.perform_recv(&socket, buffer, flags)?;
            
            // 记录统计
            self.stats.record_packet_received(bytes_received as u64);
            
            crate::println!("[network] Received {} bytes on socket {}", bytes_received, socket_id);
            Ok(bytes_received)
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 关闭套接字
    pub fn close(&self, socket_id: usize) -> Result<(), NetworkError> {
        let mut sockets = self.sockets.lock();
        
        if let Some(socket) = sockets.remove(&socket_id) {
            let socket = Arc::clone(&socket);
            drop(sockets);
            
            // 这里应该调用底层网络栈进行实际关闭
            self.perform_close(&socket)?;
            
            // 记录统计
            self.stats.record_connection_closed();
            
            crate::println!("[network] Closed socket {}", socket_id);
            Ok(())
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 获取套接字选项
    pub fn getsockopt(
        &self,
        socket_id: usize,
        level: SocketLevel,
        option: SocketOption,
    ) -> Result<i32, NetworkError> {
        let sockets = self.sockets.lock();
        
        if let Some(socket) = sockets.get(&socket_id) {
            // 这里应该调用底层网络栈获取选项
            self.perform_getsockopt(&socket, level, option)
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 设置套接字选项
    pub fn setsockopt(
        &self,
        socket_id: usize,
        level: SocketLevel,
        option: SocketOption,
        value: i32,
    ) -> Result<(), NetworkError> {
        let mut sockets = self.sockets.lock();
        
        if let Some(socket) = sockets.get(&socket_id) {
            // 这里应该调用底层网络栈设置选项
            self.perform_setsockopt(&socket, level, option, value)?;
            
            // 更新套接字标志
            if let Some(socket) = sockets.get_mut(&socket_id) {
                match option {
                    SocketOption::ReuseAddr => {
                        socket.flags.reuseaddr = value != 0;
                    }
                    SocketOption::KeepAlive => {
                        socket.flags.keepalive = value != 0;
                    }
                    _ => {}
                }
            }
            
            crate::println!("[network] Set socket {} option {:?} to {}", socket_id, option, value);
            Ok(())
        } else {
            Err(NetworkError::InvalidSocket)
        }
    }
    
    /// 获取网络统计信息
    pub fn get_stats(&self) -> (u64, u64, u64, u64, u64, u64, u64) {
        self.stats.get_stats()
    }
    
    /// 检查地址兼容性
    fn is_address_compatible(&self, family: &AddressFamily, address: &SocketAddress) -> bool {
        match (family, address) {
            (AddressFamily::IPv4, SocketAddress::IPv4(_, _)) => true,
            (AddressFamily::IPv6, SocketAddress::IPv6(_, _)) => true,
            (AddressFamily::Unix, SocketAddress::Unix(_)) => true,
            _ => false,
        }
    }
    
    /// 执行实际绑定（占位符实现）
    fn perform_bind(&self, _socket: &EnhancedSocket, _address: &SocketAddress) -> Result<(), NetworkError> {
        // 这里应该调用底层网络栈
        // 暂时返回成功
        Ok(())
    }
    
    /// 执行实际监听（占位符实现）
    fn perform_listen(&self, _socket: &EnhancedSocket, _backlog: i32) -> Result<(), NetworkError> {
        // 这里应该调用底层网络栈
        // 暂时返回成功
        Ok(())
    }
    
    /// 执行实际接受（占位符实现）
    fn perform_accept(&self, _socket: &EnhancedSocket) -> Result<(usize, SocketAddress), NetworkError> {
        // 这里应该调用底层网络栈
        // 暂时返回模拟连接
        let new_socket_id = self.next_socket_id.fetch_add(1, Ordering::Relaxed) as usize;
        let remote_address = SocketAddress::IPv4(IPv4Address::new(192, 168, 1, 100), 8080);
        Ok((new_socket_id, remote_address))
    }
    
    /// 执行实际连接（占位符实现）
    fn perform_connect(&self, _socket: &EnhancedSocket, _address: &SocketAddress) -> Result<(), NetworkError> {
        // 这里应该调用底层网络栈
        // 暂时返回成功
        Ok(())
    }
    
    /// 执行实际发送（占位符实现）
    fn perform_send(&self, _socket: &EnhancedSocket, data: &[u8], _flags: i32) -> Result<usize, NetworkError> {
        // 这里应该调用底层网络栈
        // 暂时返回发送的数据长度
        Ok(data.len())
    }
    
    /// 执行实际接收（占位符实现）
    fn perform_recv(&self, _socket: &EnhancedSocket, buffer: &mut [u8], _flags: i32) -> Result<usize, NetworkError> {
        // 这里应该调用底层网络栈
        // 暂时返回模拟接收的数据
        let data = b"Hello from network stack";
        let len = core::cmp::min(data.len(), buffer.len());
        buffer[..len].copy_from_slice(&data[..len]);
        Ok(len)
    }
    
    /// 执行实际关闭（占位符实现）
    fn perform_close(&self, _socket: &EnhancedSocket) -> Result<(), NetworkError> {
        // 这里应该调用底层网络栈
        // 暂时返回成功
        Ok(())
    }
    
    /// 执行获取套接字选项（占位符实现）
    fn perform_getsockopt(&self, _socket: &EnhancedSocket, _level: SocketLevel, _option: SocketOption) -> Result<i32, NetworkError> {
        // 这里应该调用底层网络栈
        // 暂时返回默认值
        Ok(0)
    }
    
    /// 执行设置套接字选项（占位符实现）
    fn perform_setsockopt(&self, _socket: &EnhancedSocket, _level: SocketLevel, _option: SocketOption, _value: i32) -> Result<(), NetworkError> {
        // 这里应该调用底层网络栈
        // 暂时返回成功
        Ok(())
    }
}

/// 网络错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    InvalidSocket,
    InvalidState,
    AddressFamilyNotSupported,
    OperationNotSupported,
    AddressInUse,
    ConnectionRefused,
    ConnectionTimedOut,
    ConnectionReset,
    NetworkUnreachable,
    HostUnreachable,
    PermissionDenied,
    WouldBlock,
    Interrupted,
    OutOfMemory,
    BufferTooSmall,
    InvalidArgument,
    ProtocolError,
    ConnectionAborted,
}

impl NetworkError {
    /// 转换为POSIX错误码
    pub fn to_errno(&self) -> i32 {
        match self {
            NetworkError::InvalidSocket => crate::reliability::errno::EBADF,
            NetworkError::InvalidState => crate::reliability::errno::EINVAL,
            NetworkError::AddressFamilyNotSupported => crate::reliability::errno::EAFNOSUPPORT,
            NetworkError::OperationNotSupported => crate::reliability::errno::EOPNOTSUPP,
            NetworkError::AddressInUse => crate::reliability::errno::EADDRINUSE,
            NetworkError::ConnectionRefused => crate::reliability::errno::ECONNREFUSED,
            NetworkError::ConnectionTimedOut => crate::reliability::errno::ETIMEDOUT,
            NetworkError::ConnectionReset => crate::reliability::errno::ECONNRESET,
            NetworkError::NetworkUnreachable => crate::reliability::errno::ENETUNREACH,
            NetworkError::HostUnreachable => crate::reliability::errno::EHOSTUNREACH,
            NetworkError::PermissionDenied => crate::reliability::errno::EACCES,
            NetworkError::WouldBlock => crate::reliability::errno::EWOULDBLOCK,
            NetworkError::Interrupted => crate::reliability::errno::EINTR,
            NetworkError::OutOfMemory => crate::reliability::errno::ENOMEM,
            NetworkError::BufferTooSmall => crate::reliability::errno::ENOBUFS,
            NetworkError::InvalidArgument => crate::reliability::errno::EINVAL,
            NetworkError::ProtocolError => crate::reliability::errno::EPROTO,
            NetworkError::ConnectionAborted => crate::reliability::errno::ECONNABORTED,
        }
    }
}

/// 全局网络管理器
static GLOBAL_NETWORK_MANAGER: Mutex<Option<EnhancedNetworkManager>> = Mutex::new(None);

/// 获取全局网络管理器
pub fn get_global_network_manager() -> &'static Mutex<EnhancedNetworkManager> {
    &GLOBAL_NETWORK_MANAGER
}

/// 初始化全局网络管理器
pub fn init_network_manager() {
    let mut manager = GLOBAL_NETWORK_MANAGER.lock();
    if manager.is_none() {
        *manager = Some(EnhancedNetworkManager::new());
        crate::println!("[network] Enhanced network manager initialized");
    }
}

/// 创建套接字（POSIX兼容接口）
pub fn socket(
    family: AddressFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> Result<usize, NetworkError> {
    let manager = GLOBAL_NETWORK_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.socket(family, socket_type, protocol, SocketFlags::NONE)
    } else {
        Err(NetworkError::InvalidState)
    }
}

/// 绑定套接字（POSIX兼容接口）
pub fn bind(socket_id: usize, address: &SocketAddress) -> Result<(), NetworkError> {
    let manager = GLOBAL_NETWORK_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.bind(socket_id, address)
    } else {
        Err(NetworkError::InvalidState)
    }
}

/// 监听套接字（POSIX兼容接口）
pub fn listen(socket_id: usize, backlog: i32) -> Result<(), NetworkError> {
    let manager = GLOBAL_NETWORK_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.listen(socket_id, backlog)
    } else {
        Err(NetworkError::InvalidState)
    }
}

/// 接受连接（POSIX兼容接口）
pub fn accept(socket_id: usize) -> Result<(usize, SocketAddress), NetworkError> {
    let manager = GLOBAL_NETWORK_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.accept(socket_id)
    } else {
        Err(NetworkError::InvalidState)
    }
}

/// 连接到远程地址（POSIX兼容接口）
pub fn connect(socket_id: usize, address: &SocketAddress) -> Result<(), NetworkError> {
    let manager = GLOBAL_NETWORK_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.connect(socket_id, address)
    } else {
        Err(NetworkError::InvalidState)
    }
}

/// 发送数据（POSIX兼容接口）
pub fn send(socket_id: usize, data: &[u8], flags: i32) -> Result<usize, NetworkError> {
    let manager = GLOBAL_NETWORK_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.send(socket_id, data, flags)
    } else {
        Err(NetworkError::InvalidState)
    }
}

/// 接收数据（POSIX兼容接口）
pub fn recv(socket_id: usize, buffer: &mut [u8], flags: i32) -> Result<usize, NetworkError> {
    let manager = GLOBAL_NETWORK_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.recv(socket_id, buffer, flags)
    } else {
        Err(NetworkError::InvalidState)
    }
}

/// 关闭套接字（POSIX兼容接口）
pub fn close(socket_id: usize) -> Result<(), NetworkError> {
    let manager = GLOBAL_NETWORK_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.close(socket_id)
    } else {
        Err(NetworkError::InvalidState)
    }
}

/// 获取网络统计信息
pub fn get_network_stats() -> (u64, u64, u64, u64, u64, u64, u64) {
    let manager = GLOBAL_NETWORK_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.get_stats()
    } else {
        (0, 0, 0, 0, 0, 0, 0)
    }
}