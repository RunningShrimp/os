//! Socket abstraction layer
//!
//! This module provides socket types and interfaces that bridge between the
//! POSIX socket API and the underlying TCP/IP implementation.

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use super::ipv4::Ipv4Addr;
use super::tcp::manager::{TcpConnection, TcpConnectionManager};
use super::udp::UdpSocket;
use super::udp::UdpSocketState;
use super::tcp::TcpState;

/// Socket types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// Stream socket (TCP)
    Stream,
    /// Datagram socket (UDP)
    Datagram,
    /// Raw socket
    Raw,
    /// Sequential packet socket
    SeqPacket,
}

impl SocketType {
    /// Get default protocol for this socket type
    pub fn default_protocol(self) -> i32 {
        match self {
            SocketType::Stream => 6,    // TCP
            SocketType::Datagram => 17,  // UDP
            SocketType::Raw => 0,        // IP protocol
            SocketType::SeqPacket => 0,  // Implementation specific
        }
    }

    /// Check if this is a connection-oriented socket
    pub fn is_connection_oriented(self) -> bool {
        matches!(self, SocketType::Stream | SocketType::SeqPacket)
    }

    /// Check if this is a reliable socket
    pub fn is_reliable(self) -> bool {
        matches!(self, SocketType::Stream | SocketType::SeqPacket)
    }
}

/// Protocol families
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolFamily {
    /// Unspecified
    Unspecified,
    /// IPv4
    IPv4,
    /// IPv6
    IPv6,
    /// Unix domain sockets
    Unix,
    /// Netlink
    Netlink,
}

/// Socket address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddr {
    /// Address family
    pub family: ProtocolFamily,
    /// Port number
    pub port: u16,
    /// IP address (for IPv4/IPv6)
    pub ip: Ipv4Addr,
}

impl SocketAddr {
    /// Create a new IPv4 socket address
    pub fn new_ipv4(ip: Ipv4Addr, port: u16) -> Self {
        Self {
            family: ProtocolFamily::IPv4,
            port,
            ip,
        }
    }

    /// Create a new IPv4 address from octets
    pub fn new_ipv4_from_octets(a: u8, b: u8, c: u8, d: u8, port: u16) -> Self {
        Self::new_ipv4(Ipv4Addr::new(a, b, c, d), port)
    }

    /// Get the IPv4 address
    pub fn ipv4_addr(&self) -> Option<Ipv4Addr> {
        match self.family {
            ProtocolFamily::IPv4 => Some(self.ip),
            _ => None,
        }
    }

    /// Check if this is the wildcard address
    pub fn is_wildcard(&self) -> bool {
        match self.family {
            ProtocolFamily::IPv4 => self.ip == Ipv4Addr::UNSPECIFIED,
            _ => false,
        }
    }

    /// Convert to POSIX sockaddr format
    pub fn to_posix_sockaddr(&self) -> crate::posix::Sockaddr {
        let mut addr = crate::posix::Sockaddr {
            sa_family: match self.family {
                ProtocolFamily::IPv4 => crate::posix::AF_INET as u16,
                ProtocolFamily::IPv6 => crate::posix::AF_INET6 as u16,
                _ => crate::posix::AF_UNSPEC as u16,
            },
            sa_data: [0; 14],
        };

        // Copy IPv4 address and port
        if self.family == ProtocolFamily::IPv4 {
            let ip_bytes = self.ip.to_be_bytes();
            addr.sa_data[0] = (self.port >> 8) as u8;
            addr.sa_data[1] = (self.port & 0xFF) as u8;
            addr.sa_data[2] = ip_bytes[0];
            addr.sa_data[3] = ip_bytes[1];
            addr.sa_data[4] = ip_bytes[2];
            addr.sa_data[5] = ip_bytes[3];
        }

        addr
    }

    /// Convert from POSIX sockaddr format
    pub fn from_posix_sockaddr(posix_addr: &crate::posix::Sockaddr) -> Option<Self> {
        const AF_INET_U16: u16 = crate::posix::AF_INET as u16;
        match posix_addr.sa_family {
            AF_INET_U16 => {
                let port = ((posix_addr.sa_data[0] as u16) << 8) | (posix_addr.sa_data[1] as u16);
                let ip = Ipv4Addr::new(
                    posix_addr.sa_data[2],
                    posix_addr.sa_data[3],
                    posix_addr.sa_data[4],
                    posix_addr.sa_data[5],
                );
                Some(Self::new_ipv4(ip, port))
            }
            _ => None,
        }
    }
}

/// Socket options
#[derive(Debug, Clone, Copy)]
pub struct SocketOptions {
    /// SO_REUSEADDR
    pub reuse_addr: bool,
    /// SO_REUSEPORT
    pub reuse_port: bool,
    /// SO_KEEPALIVE
    pub keep_alive: bool,
    /// SO_BROADCAST
    pub broadcast: bool,
    /// SO_LINGER
    pub linger: Option<LingerOption>,
    /// SO_SNDBUF - Send buffer size
    pub sndbuf: u32,
    /// SO_RCVBUF - Receive buffer size
    pub rcvbuf: u32,
    /// TCP_NODELAY (for TCP sockets)
    pub nodelay: bool,
}

/// Linger option
#[derive(Debug, Clone, Copy)]
pub struct LingerOption {
    /// Linger enabled
    pub on: bool,
    /// Linger time in seconds
    pub time: i32,
}

impl SocketOptions {
    /// Create new socket options with defaults
    pub fn new() -> Self {
        Self {
            reuse_addr: false,
            reuse_port: false,
            keep_alive: false,
            broadcast: false,
            linger: None,
            sndbuf: 65536,
            rcvbuf: 65536,
            nodelay: false,
        }
    }

    /// Set socket option
    pub fn set_option(&mut self, option: SocketOption) -> Result<(), SocketError> {
        match option {
            SocketOption::ReuseAddr(value) => self.reuse_addr = value,
            SocketOption::ReusePort(value) => self.reuse_port = value,
            SocketOption::KeepAlive(value) => self.keep_alive = value,
            SocketOption::Broadcast(value) => self.broadcast = value,
            SocketOption::Linger(linger) => self.linger = Some(linger),
            SocketOption::SndBuf(size) => {
                if size == 0 || size > (1 << 30) { // Max 1GB
                    return Err(SocketError::InvalidValue);
                }
                self.sndbuf = size;
            }
            SocketOption::RcvBuf(size) => {
                if size == 0 || size > (1 << 30) { // Max 1GB
                    return Err(SocketError::InvalidValue);
                }
                self.rcvbuf = size;
            }
            SocketOption::NoDelay(value) => self.nodelay = value,
        }
        Ok(())
    }

    /// Get socket option
    pub fn get_option(&self, option: SocketOption) -> Result<SocketOptionValue, SocketError> {
        Ok(match option {
            SocketOption::ReuseAddr(_) => SocketOptionValue::Bool(self.reuse_addr),
            SocketOption::ReusePort(_) => SocketOptionValue::Bool(self.reuse_port),
            SocketOption::KeepAlive(_) => SocketOptionValue::Bool(self.keep_alive),
            SocketOption::Broadcast(_) => SocketOptionValue::Bool(self.broadcast),
            SocketOption::Linger(_) => {
                if let Some(linger) = self.linger {
                    SocketOptionValue::Linger(linger)
                } else {
                    SocketOptionValue::Linger(LingerOption { on: false, time: 0 })
                }
            }
            SocketOption::SndBuf(_) => SocketOptionValue::U32(self.sndbuf),
            SocketOption::RcvBuf(_) => SocketOptionValue::U32(self.rcvbuf),
            SocketOption::NoDelay(_) => SocketOptionValue::Bool(self.nodelay),
        })
    }
}

/// Socket option types
#[derive(Debug, Clone, Copy)]
pub enum SocketOption {
    ReuseAddr(bool),
    ReusePort(bool),
    KeepAlive(bool),
    Broadcast(bool),
    Linger(LingerOption),
    SndBuf(u32),
    RcvBuf(u32),
    NoDelay(bool),
}

/// Socket option values
#[derive(Debug, Clone)]
pub enum SocketOptionValue {
    Bool(bool),
    Linger(LingerOption),
    U32(u32),
}

/// High-level socket abstraction
#[derive(Debug, Clone)]
pub enum Socket {
    /// TCP socket
    Tcp(TcpSocketWrapper),
    /// UDP socket
    Udp(UdpSocketWrapper),
    /// Raw socket
    Raw(RawSocketWrapper),
}

/// TCP socket wrapper
#[derive(Debug)]
pub struct TcpSocketWrapper {
    /// TCP connection
    connection: Option<Arc<TcpConnection>>,
    /// Connection ID
    connection_id: Option<super::tcp::manager::ConnectionId>,
    /// Socket state
    state: TcpState,
    /// Socket options
    options: SocketOptions,
    /// Non-blocking mode
    nonblocking: AtomicBool,
    /// Pending connections (for listening sockets)
    pending_connections: Vec<Arc<TcpConnection>>,
}

impl Clone for TcpSocketWrapper {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            connection_id: self.connection_id,
            state: self.state,
            options: self.options,
            nonblocking: AtomicBool::new(self.nonblocking.load(Ordering::Relaxed)),
            pending_connections: self.pending_connections.clone(),
        }
    }
}

impl TcpSocketWrapper {
    /// Create a new TCP socket
    pub fn new(options: SocketOptions) -> Self {
        Self {
            connection: None,
            connection_id: None,
            state: TcpState::Closed,
            options,
            nonblocking: AtomicBool::new(false),
            pending_connections: Vec::new(),
        }
    }

    /// Bind to local address
    pub fn bind(&mut self, _addr: SocketAddr) -> Result<(), SocketError> {
        // Implementation would use TCP connection manager
        // For now, just update state
        Ok(())
    }

    /// Start listening
    pub fn listen(&mut self, backlog: i32) -> Result<(), SocketError> {
        // Implementation would create listening socket
        self.state = TcpState::Listen;
        Ok(())
    }

    /// Accept a connection
    pub fn accept(&mut self) -> Result<(Socket, SocketAddr), SocketError> {
        if self.pending_connections.is_empty() {
            return Err(SocketError::WouldBlock);
        }

        let connection = self.pending_connections.remove(0);
        let socket = Socket::Tcp(TcpSocketWrapper {
            connection: Some(connection.clone()),
            connection_id: None,
            state: TcpState::Established,
            options: self.options.clone(),
            nonblocking: AtomicBool::new(self.nonblocking.load(Ordering::Relaxed)),
            pending_connections: Vec::new(),
        });

        // Get peer address (simplified)
        let peer_addr = SocketAddr::new_ipv4_from_octets(127, 0, 0, 1, 8080);

        Ok((socket, peer_addr))
    }

    /// Connect to remote address
    pub fn connect(&mut self, _addr: SocketAddr) -> Result<(), SocketError> {
        // Implementation would use TCP connection manager
        self.state = TcpState::SynSent;
        Ok(())
    }

    /// Send data
    pub fn send(&mut self, data: &[u8]) -> Result<usize, SocketError> {
        if self.state != TcpState::Established {
            return Err(SocketError::NotConnected);
        }

        // Implementation would use TCP connection
        // For now, just return length
        Ok(data.len())
    }

    /// Send data with zero-copy optimization
    /// This method attempts to send data without copying to kernel buffers
    pub fn send_zero_copy(&mut self, data: &[u8]) -> Result<usize, SocketError> {
        if self.state != TcpState::Established {
            return Err(SocketError::NotConnected);
        }

        // Zero-copy send: Use PacketBuffer for DMA-optimized transfer
        // In a full implementation, this would:
        // 1. Pin user buffer pages to prevent swapping
        // 2. Map pages directly to network device DMA
        // 3. Send without copying to kernel space
        
        // For now, use PacketBuffer to minimize copies
        use crate::net::packet::PacketBuffer;
        
        // Create a packet buffer that can be used for DMA
        let mut packet_buf = match PacketBuffer::new(data.len()) {
            Ok(buf) => buf,
            Err(_) => return Err(SocketError::NoBufferSpace),
        };
        
        // Write data to packet buffer (single copy instead of multiple)
        if let Err(_) = packet_buf.write_bytes(data) {
            return Err(SocketError::NotConnected);
        }
        
        // Get DMA buffer for network device
        let (dma_ptr, dma_len) = packet_buf.prepare_for_dma_read();
        
        // Send via TCP connection if available
        if let Some(ref connection) = self.connection {
            use crate::sync::Mutex;
            // Get mutable access to connection
            // Note: This requires proper synchronization in real implementation
            // For now, we'll use the regular send path but with optimized buffer
            
            // In a real zero-copy implementation, we would:
            // 1. Pass the DMA buffer directly to the network device driver
            // 2. The driver would send without copying
            // 3. Release the buffer after transmission completes
            
            // For now, fall back to regular send but with optimized buffer
            return self.send(data);
        }
        
        // Fall back to regular send if no connection
        self.send(data)
    }

    /// Receive data
    pub fn recv(&mut self, buf: &mut [u8]) -> Result<usize, SocketError> {
        if self.state != TcpState::Established {
            return Err(SocketError::NotConnected);
        }

        // Implementation would use TCP connection
        // For now, just return 0
        Ok(0)
    }

    /// Receive data with zero-copy optimization
    /// This method attempts to receive data directly into user buffer
    pub fn recv_zero_copy(&mut self, buf: &mut [u8]) -> Result<usize, SocketError> {
        if self.state != TcpState::Established {
            return Err(SocketError::NotConnected);
        }

        // Zero-copy receive: Use PacketBuffer for DMA-optimized transfer
        // In a full implementation, this would:
        // 1. Pin user buffer pages to prevent swapping
        // 2. Map network device DMA directly to user pages
        // 3. Receive without copying through kernel buffers
        
        // For now, use PacketBuffer to minimize copies
        use crate::net::packet::PacketBuffer;
        
        // Receive from TCP connection if available
        if let Some(ref connection) = self.connection {
            use crate::sync::Mutex;
            
            // In a real zero-copy implementation:
            // 1. Prepare user buffer for DMA
            // 2. Register buffer with network device
            // 3. Device writes directly to user buffer
            // 4. Unregister buffer after reception
            
            // For now, use optimized receive path
            // Create a packet buffer for receiving
            let mut packet_buf = match PacketBuffer::new(buf.len()) {
                Ok(buf) => buf,
                Err(_) => return Err(SocketError::NoBufferSpace),
            };
            
            // Get DMA buffer for network device to write to
            let (dma_ptr, dma_len) = packet_buf.as_dma_buffer();
            
            // In real implementation, network device would write directly here
            // For now, receive into packet buffer first
            let mut temp_buf = alloc::vec![0u8; buf.len().min(8192)];
            let received = self.recv(&mut temp_buf)?;
            
            if received > 0 {
                // Copy from packet buffer to user buffer (minimized copy)
                let copy_len = received.min(buf.len());
                buf[..copy_len].copy_from_slice(&temp_buf[..copy_len]);
                return Ok(copy_len);
            }
            
            return Ok(0);
        }
        
        // Fall back to regular recv if no connection
        self.recv(buf)
    }

    /// Close socket
    pub fn close(&mut self) -> Result<(), SocketError> {
        self.state = TcpState::Closed;
        self.connection = None;
        self.connection_id = None;
        Ok(())
    }

    /// Get socket state
    pub fn state(&self) -> TcpState {
        self.state
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) {
        self.nonblocking.store(nonblocking, Ordering::Relaxed);
    }

    /// Check if non-blocking
    pub fn is_nonblocking(&self) -> bool {
        self.nonblocking.load(Ordering::Relaxed)
    }
}

/// UDP socket wrapper
#[derive(Debug)]
pub struct UdpSocketWrapper {
    /// UDP socket
    socket: UdpSocket,
    /// Socket state
    state: UdpSocketState,
    /// Socket options
    options: SocketOptions,
    /// Non-blocking mode
    nonblocking: AtomicBool,
}

impl Clone for UdpSocketWrapper {
    fn clone(&self) -> Self {
        Self {
            socket: self.socket.clone(),
            state: self.state,
            options: self.options,
            nonblocking: AtomicBool::new(self.nonblocking.load(Ordering::Relaxed)),
        }
    }
}

impl UdpSocketWrapper {
    /// Create a new UDP socket
    pub fn new(options: SocketOptions) -> Self {
        Self {
            socket: UdpSocket::new(),
            state: UdpSocketState::Unbound,
            options,
            nonblocking: AtomicBool::new(false),
        }
    }

    /// Bind to local address
    pub fn bind(&mut self, addr: SocketAddr) -> Result<(), SocketError> {
        if let Some(ip) = addr.ipv4_addr() {
            self.socket.bind(ip, addr.port);
            self.state = UdpSocketState::Bound;
            Ok(())
        } else {
            Err(SocketError::InvalidAddress)
        }
    }

    /// Send data to destination
    pub fn send_to(&mut self, data: &[u8], dest: SocketAddr) -> Result<usize, SocketError> {
        if self.state != UdpSocketState::Bound {
            return Err(SocketError::NotBound);
        }

        if let Some(dest_ip) = dest.ipv4_addr() {
            // Implementation would actually send the data
            Ok(data.len())
        } else {
            Err(SocketError::InvalidAddress)
        }
    }

    /// Send data with zero-copy optimization (UDP)
    pub fn send_to_zero_copy(&mut self, data: &[u8], dest: SocketAddr) -> Result<usize, SocketError> {
        if self.state != UdpSocketState::Bound {
            return Err(SocketError::NotBound);
        }

        // Zero-copy UDP send: Use PacketBuffer for DMA-optimized transfer
        use crate::net::packet::PacketBuffer;
        
        // Create packet buffer for DMA transfer
        let mut packet_buf = match PacketBuffer::new(data.len()) {
            Ok(buf) => buf,
            Err(_) => return Err(SocketError::NoBufferSpace),
        };
        
        // Write data to packet buffer
        if let Err(_) = packet_buf.write_bytes(data) {
            return Err(SocketError::NotBound);
        }
        
        // Get DMA buffer for network device
        let (_dma_ptr, _dma_len) = packet_buf.prepare_for_dma_read();
        
        // In real implementation, pass DMA buffer directly to UDP layer
        // For now, fall back to regular send
        self.send_to(data, dest)
    }

    /// Receive data
    pub fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr), SocketError> {
        if self.state != UdpSocketState::Bound {
            return Err(SocketError::NotBound);
        }

        // Implementation would actually receive data
        // For now, return 0 bytes and a dummy address
        let addr = SocketAddr::new_ipv4_from_octets(127, 0, 0, 1, 8080);
        Ok((0, addr))
    }

    /// Receive data with zero-copy optimization (UDP)
    pub fn recv_from_zero_copy(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr), SocketError> {
        if self.state != UdpSocketState::Bound {
            return Err(SocketError::NotBound);
        }

        // Zero-copy UDP receive: Use PacketBuffer for DMA-optimized transfer
        use crate::net::packet::PacketBuffer;
        
        // Create packet buffer for receiving
        let mut packet_buf = match PacketBuffer::new(buf.len()) {
            Ok(buf) => buf,
            Err(_) => return Err(SocketError::NoBufferSpace),
        };
        
        // Get DMA buffer for network device to write to
        let (_dma_ptr, _dma_len) = packet_buf.as_dma_buffer();
        
        // In real implementation, network device writes directly to DMA buffer
        // For now, receive into temporary buffer
        let mut temp_buf = alloc::vec![0u8; buf.len().min(8192)];
        let (received, addr) = self.recv_from(&mut temp_buf)?;
        
        if received > 0 {
            // Copy to user buffer (minimized copy)
            let copy_len = received.min(buf.len());
            buf[..copy_len].copy_from_slice(&temp_buf[..copy_len]);
            return Ok((copy_len, addr));
        }
        
        Ok((0, SocketAddr::new_ipv4_from_octets(127, 0, 0, 1, 8080)))
    }

    /// Close socket
    pub fn close(&mut self) -> Result<(), SocketError> {
        self.state = UdpSocketState::Closed;
        Ok(())
    }

    /// Get socket state
    pub fn state(&self) -> UdpSocketState {
        self.state
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) {
        self.nonblocking.store(nonblocking, Ordering::Relaxed);
    }

    /// Check if non-blocking
    pub fn is_nonblocking(&self) -> bool {
        self.nonblocking.load(Ordering::Relaxed)
    }
}

/// Raw socket wrapper
#[derive(Debug)]
pub struct RawSocketWrapper {
    /// Raw socket state
    state: bool, // false = closed, true = open
    /// Socket options
    options: SocketOptions,
    /// Non-blocking mode
    nonblocking: AtomicBool,
}

impl Clone for RawSocketWrapper {
    fn clone(&self) -> Self {
        Self {
            state: self.state,
            options: self.options,
            nonblocking: AtomicBool::new(self.nonblocking.load(Ordering::Relaxed)),
        }
    }
}

impl RawSocketWrapper {
    /// Create a new raw socket
    pub fn new(options: SocketOptions) -> Self {
        Self {
            state: false,
            options,
            nonblocking: AtomicBool::new(false),
        }
    }

    /// Close socket
    pub fn close(&mut self) -> Result<(), SocketError> {
        self.state = false;
        Ok(())
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) {
        self.nonblocking.store(nonblocking, Ordering::Relaxed);
    }

    /// Check if non-blocking
    pub fn is_nonblocking(&self) -> bool {
        self.nonblocking.load(Ordering::Relaxed)
    }
}

/// Socket errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketError {
    /// Invalid file descriptor
    InvalidFd,
    /// Invalid address
    InvalidAddress,
    /// Address already in use
    AddressInUse,
    /// Not connected
    NotConnected,
    /// Not bound
    NotBound,
    /// Connection refused
    ConnectionRefused,
    /// Connection timeout
    ConnectionTimeout,
    /// Connection reset
    ConnectionReset,
    /// Would block (for non-blocking sockets)
    WouldBlock,
    /// Invalid value
    InvalidValue,
    /// No buffer space available
    NoBufferSpace,
    /// Operation not supported
    NotSupported,
    /// Permission denied
    PermissionDenied,
}

/// Socket entry for socket management
#[derive(Debug, Clone)]
pub struct SocketEntry {
    /// Socket ID
    pub id: u32,
    /// Socket type
    pub socket_type: SocketType,
    /// Protocol family
    pub family: ProtocolFamily,
    /// Socket state
    pub state: SocketState,
    /// Socket address
    pub local_addr: Option<SocketAddr>,
    /// Remote address (for connected sockets)
    pub remote_addr: Option<SocketAddr>,
    /// Socket options
    pub options: SocketOptions,
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    /// Socket is unbound
    Unbound,
    /// Socket is bound to local address
    Bound,
    /// Socket is connecting
    Connecting,
    /// Socket is connected
    Connected,
    /// Socket is listening
    Listening,
    /// Socket is closing
    Closing,
    /// Socket is closed
    Closed,
}

impl SocketEntry {
    /// Create a new socket entry
    pub fn new(id: u32, socket_type: SocketType, family: ProtocolFamily) -> Self {
        Self {
            id,
            socket_type,
            family,
            state: SocketState::Unbound,
            local_addr: None,
            remote_addr: None,
            options: SocketOptions::new(),
        }
    }

    /// Check if socket is active
    pub fn is_active(&self) -> bool {
        !matches!(self.state, SocketState::Closed | SocketState::Unbound)
    }

    /// Check if socket can receive data
    pub fn can_receive(&self) -> bool {
        matches!(self.state, SocketState::Connected | SocketState::Bound | SocketState::Listening)
    }

    /// Check if socket can send data
    pub fn can_send(&self) -> bool {
        matches!(self.state, SocketState::Connected)
    }
}

/// Get global socket manager
pub fn socket_manager() -> &'static mut TcpConnectionManager {
    static mut MANAGER: Option<TcpConnectionManager> = None;
    static INIT: crate::sync::Once = crate::sync::Once::new();

    unsafe {
        INIT.call_once(|| {
            MANAGER = Some(TcpConnectionManager::new());
        });
        MANAGER.as_mut().unwrap()
    }
}