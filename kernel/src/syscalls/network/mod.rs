//! Network/socket syscalls

pub mod socket;
pub mod data;
pub mod options;
pub mod interface;

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::sync::Mutex;
use crate::net::NetworkError;
use crate::net::socket::{Socket, SocketOptions, SocketType, ProtocolFamily, TcpSocketWrapper, UdpSocketWrapper};
use crate::net::tcp::manager::{ConnectionId, TcpConnectionManager};
use crate::net::ipv4::Ipv4Addr;

// Re-export interface functions
pub use interface::{add_route, interface_up, add_interface_address, set_interface_mtu, create_veth_pair, create_bridge};

// NOTE: explicit conversion from SocketOptions -> TcpOptions should be done where needed
// rather than providing a blanket `From` impl here to avoid conflicting implementations.

/// Socket descriptor table entry
pub struct SocketEntry {
    /// Socket type (TCP, UDP, RAW)
    pub socket_type: SocketType,
    /// Protocol family (AF_INET, etc.)
    pub protocol_family: ProtocolFamily,
    /// Protocol number
    pub protocol: i32,
    /// Socket options
    pub options: SocketOptions,
    /// Local address (bound address)
    pub local_addr: Option<crate::net::socket::SocketAddr>,
    /// Remote address (connected address)
    pub remote_addr: Option<crate::net::socket::SocketAddr>,
    /// Socket state
    pub state: SocketState,
    /// Actual socket implementation
    pub socket: Mutex<Option<Socket>>,
    /// TCP connection ID (for TCP sockets)
    pub connection_id: Option<ConnectionId>,
}

impl core::fmt::Debug for SocketEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SocketEntry")
            .field("socket_type", &self.socket_type)
            .field("protocol_family", &self.protocol_family)
            .field("protocol", &self.protocol)
            .field("options", &self.options)
            .field("local_addr", &self.local_addr)
            .field("remote_addr", &self.remote_addr)
            .field("state", &self.state)
            .field("socket", &"<Mutex<Option<Socket>>>")
            .field("connection_id", &self.connection_id)
            .finish()
    }
}

impl Clone for SocketEntry {
    fn clone(&self) -> Self {
        Self {
            socket_type: self.socket_type,
            protocol_family: self.protocol_family,
            protocol: self.protocol,
            options: self.options,
            local_addr: self.local_addr,
            remote_addr: self.remote_addr,
            state: self.state,
            socket: crate::sync::Mutex::new(self.socket.lock().clone()),
            connection_id: self.connection_id,
        }
    }
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    /// Uninitialized
    Uninitialized,
    /// Bound to local address
    Bound,
    /// Listening (for TCP)
    Listening,
    /// Connected (for TCP/UDP)
    Connected,
    /// Closing
    Closing,
    /// Closed
    Closed,
}

/// Global socket table
static mut SOCKET_TABLE: Option<Vec<Option<Arc<SocketEntry>>>> = None;
static SOCKET_TABLE_INIT: AtomicBool = AtomicBool::new(false);

/// Get the global socket table
fn get_socket_table() -> &'static mut Vec<Option<Arc<SocketEntry>>> {
    unsafe {
        if !SOCKET_TABLE_INIT.load(Ordering::SeqCst) {
            SOCKET_TABLE = Some(Vec::with_capacity(1024));
            if let Some(ref mut table) = SOCKET_TABLE {
                // Initialize with None entries
                table.resize(1024, None);
            }
            SOCKET_TABLE_INIT.store(true, Ordering::SeqCst);
        }
        SOCKET_TABLE.as_mut().expect("SOCKET_TABLE should be initialized")
    }
}

/// Allocate a socket descriptor - now uses unified file descriptor system
fn alloc_socket_fd() -> isize {
    // Use the unified file descriptor system instead of separate socket table
    match crate::fs::file_alloc() {
        Some(fd) => fd as isize,
        None => -1,
    }
}

/// Get socket entry by file descriptor - unified with file system
fn get_socket_entry(fd: i32) -> Option<Arc<SocketEntry>> {
    // First try to get from legacy socket table for compatibility
    if fd >= 0 && (fd as usize) < get_socket_table().len() {
        let table = get_socket_table();
        if let Some(entry) = table[fd as usize].clone() {
            return Some(entry);
        }
    }

    // If not in socket table, try to get from file system
    let file_idx = crate::process::fdlookup(fd)?;
    let ft = crate::fs::FILE_TABLE.lock();
    let file = ft.get(file_idx)?;

        if file.ftype == crate::fs::FileType::Socket {
        if let Some(ref socket) = file.socket {
            // Create a SocketEntry from the file socket for compatibility
            let socket_entry = SocketEntry {
                socket_type: match socket {
                    Socket::Tcp(_) => SocketType::Stream,
                    Socket::Udp(_) => SocketType::Datagram,
                    Socket::Raw(_) => SocketType::Raw,
                },
                protocol_family: ProtocolFamily::IPv4, // Default
                protocol: 0,
                options: SocketOptions::new(),
                local_addr: None,
                remote_addr: None,
                state: SocketState::Uninitialized,
                socket: Mutex::new(Some(socket.clone())),
                connection_id: None,
            };
            Some(Arc::new(socket_entry))
        } else {
            None
        }
    } else {
        None
    }
}

/// Set socket entry
fn set_socket_entry(fd: i32, entry: Option<Arc<SocketEntry>>) {
    if fd >= 0 && (fd as usize) < get_socket_table().len() {
        let table = get_socket_table();
        table[fd as usize] = entry;
    }
}

/// Free socket entry
fn free_socket_entry(fd: i32) {
    set_socket_entry(fd, None);

    // Also close the file descriptor if it exists
    if let Some(file_idx) = crate::process::fdlookup(fd) {
        crate::fs::file_close(file_idx);
    }
}

/// Update socket in file system
fn update_socket_in_file(fd: i32, socket: Option<Socket>) -> Result<(), NetworkError> {
    if let Some(file_idx) = crate::process::fdlookup(fd) {
        let mut ft = crate::fs::file::FILE_TABLE.lock();
        if let Some(file) = ft.get_mut(file_idx) {
            file.socket = socket;
            Ok(())
        } else {
            Err(NetworkError::DeviceError)
        }
    } else {
        Err(NetworkError::DeviceError)
    }
}

/// Convert POSIX socket type to internal type
fn posix_to_socket_type(type_: i32) -> Option<SocketType> {
    use crate::posix::{SOCK_STREAM, SOCK_DGRAM, SOCK_RAW, SOCK_SEQPACKET};
    match type_ {
        s if s == SOCK_STREAM => Some(SocketType::Stream),
        s if s == SOCK_DGRAM => Some(SocketType::Datagram),
        s if s == SOCK_RAW => Some(SocketType::Raw),
        s if s == SOCK_SEQPACKET => Some(SocketType::SeqPacket),
        _ => None,
    }
}

/// Convert POSIX domain to protocol family
fn posix_to_protocol_family(domain: i32) -> Option<ProtocolFamily> {
    use crate::posix::{AF_INET, AF_INET6, AF_UNSPEC};
    match domain {
        d if d == AF_INET => Some(ProtocolFamily::IPv4),
        d if d == AF_INET6 => Some(ProtocolFamily::IPv6),
        d if d == AF_UNSPEC => Some(ProtocolFamily::Unspecified),
        _ => None,
    }
}

/// Dispatch network/socket syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> super::SyscallResult {
    match syscall_id {
        // Socket operations
        0x4000 => socket::sys_socket(args),         // socket
        0x4001 => socket::sys_bind(args),           // bind
        0x4002 => socket::sys_listen(args),         // listen
        0x4003 => socket::sys_accept(args),         // accept
        0x4004 => socket::sys_connect(args),        // connect
        0x4009 => socket::sys_shutdown(args),       // shutdown
        0x400A => options::sys_getsockname(args),   // getsockname
        0x400B => options::sys_getpeername(args),   // getpeername
        0x400C => options::sys_setsockopt(args),    // setsockopt
        0x400D => options::sys_getsockopt(args),    // getsockopt
        0x400E => socket::sys_socketpair(args),     // socketpair
        // Data operations
        0x4005 => data::sys_send(args),             // send
        0x4006 => data::sys_recv(args),             // recv
        0x4007 => data::sys_sendto(args),           // sendto
        0x4008 => data::sys_recvfrom(args),         // recvfrom
        0x400F => data::sys_sendmsg(args),          // sendmsg
        0x4010 => data::sys_recvmsg(args),          // recvmsg
        // Network interface operations
        0x4011 => interface::sys_ifconfig(args),    // ifconfig
        0x4012 => interface::sys_ifinfo(args),      // ifinfo
        0x4013 => interface::sys_iflist(args),      // iflist
        _ => Err(super::common::SyscallError::InvalidSyscall),
    }
}

/// Convert NetworkError to SyscallError for syscall layer
impl From<NetworkError> for super::common::SyscallError {
   fn from(error: NetworkError) -> Self {
       match error {
           NetworkError::NoRouteToHost => super::common::SyscallError::NotFound,
           NetworkError::InterfaceNotFound => super::common::SyscallError::NotFound,
           NetworkError::InvalidPacket => super::common::SyscallError::InvalidArgument,
           NetworkError::BufferExhausted => super::common::SyscallError::OutOfMemory,
           NetworkError::DeviceError => super::common::SyscallError::IoError,
       }
   }
}

/// Convert TcpError to SyscallError for syscall layer
impl From<crate::net::tcp::manager::TcpError> for super::common::SyscallError {
    fn from(error: crate::net::tcp::manager::TcpError) -> Self {
        use crate::net::tcp::manager::TcpError;
        match error {
            TcpError::InvalidPacket => super::common::SyscallError::InvalidArgument,
            TcpError::ConnectionNotFound => super::common::SyscallError::NotFound,
            TcpError::InvalidConnection => super::common::SyscallError::InvalidArgument,
            TcpError::NotConnected => super::common::SyscallError::NotSupported,
            TcpError::BufferFull => super::common::SyscallError::NoBufferSpace,
            TcpError::PortInUse => super::common::SyscallError::InvalidArgument,
            TcpError::NoPortsAvailable => super::common::SyscallError::NoBufferSpace,
            TcpError::ConnectionReset => super::common::SyscallError::ConnectionReset,
            TcpError::ConnectionTimeout => super::common::SyscallError::TimedOut,
        }
    }
}

/// Convert SocketError to SyscallError for syscall layer
impl From<crate::net::socket::SocketError> for super::common::SyscallError {
    fn from(error: crate::net::socket::SocketError) -> Self {
        use crate::net::socket::SocketError;
        match error {
            SocketError::InvalidFd => super::common::SyscallError::BadFileDescriptor,
            SocketError::InvalidAddress => super::common::SyscallError::InvalidArgument,
            SocketError::AddressInUse => super::common::SyscallError::InvalidArgument,
            SocketError::NotBound => super::common::SyscallError::InvalidArgument,
            SocketError::NotConnected => super::common::SyscallError::NotSupported,
            SocketError::ConnectionRefused => super::common::SyscallError::ConnectionRefused,
            SocketError::ConnectionTimeout => super::common::SyscallError::TimedOut,
            SocketError::ConnectionReset => super::common::SyscallError::ConnectionReset,
            SocketError::WouldBlock => super::common::SyscallError::WouldBlock,
            SocketError::InvalidValue => super::common::SyscallError::InvalidArgument,
            SocketError::NoBufferSpace => super::common::SyscallError::NoBufferSpace,
            SocketError::NotSupported => super::common::SyscallError::NotSupported,
            SocketError::PermissionDenied => super::common::SyscallError::PermissionDenied,
        }
    }
}
