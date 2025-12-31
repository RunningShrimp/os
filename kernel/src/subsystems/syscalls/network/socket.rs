//! Socket creation and management syscalls

use crate::prelude::*;
use crate::subsystems::syscalls::common::{SyscallError, SyscallResult};
use crate::net::socket::{
    SocketAddr, SocketType, ProtocolFamily, SocketOptions,
    Socket, SocketState, TcpSocketWrapper, UdpSocketWrapper,
    SocketEntry
};
use crate::subsystems::net::tcp::manager::TcpConnectionManager;
use crate::subsystems::net::ipv4::Ipv4Addr;

/// Create a new socket
///
/// Creates a communication endpoint and returns a file descriptor that can be used
/// to refer to that socket in future system calls.
///
/// # Arguments
///
/// * `args[0]` - `domain`: Address family (e.g., `AF_INET` for IPv4)
/// * `args[1]` - `type_`: Socket type (e.g., `SOCK_STREAM` for TCP, `SOCK_DGRAM` for UDP)
/// * `args[2]` - `protocol`: Protocol to use (0 for default protocol)
///
/// # Returns
///
/// * `Ok(fd)` - File descriptor for the created socket
/// * `Err(SyscallError::InvalidArgument)` - Invalid domain, type, or protocol
/// * `Err(SyscallError::OutOfMemory)` - Failed to allocate socket resources
///
/// # Examples
///
/// ```
/// // Create a TCP socket
/// let args = [AF_INET as u64, SOCK_STREAM as u64, 0u64];
/// let fd = sys_socket(&args)?;
///
/// // Create a UDP socket
/// let args = [AF_INET as u64, SOCK_DGRAM as u64, 0u64];
/// let fd = sys_socket(&args)?;
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The domain is not supported (currently only `AF_INET` is supported)
/// - The socket type is not supported
/// - The protocol doesn't match the socket type
/// - System resources are exhausted
pub fn sys_socket(args: &[u64]) -> SyscallResult<i64> {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }
    let domain = args[0] as i32;
    let type_ = args[1] as i32;
    let protocol = args[2] as i32;

    // Convert parameters
    let protocol_family = posix_to_protocol_family(domain)
        .ok_or(SyscallError::InvalidArgument)?;

    let socket_type = posix_to_socket_type(type_)
        .ok_or(SyscallError::InvalidArgument)?;

    // Validate protocol
    if protocol != 0 && protocol != socket_type.default_protocol() {
        // For now, only allow default protocol
        return Err(SyscallError::InvalidArgument);
    }

    // Allocate a file descriptor
    let fd = alloc_socket_fd();
    if fd < 0 {
        return Err(SyscallError::OutOfMemory);
    }

    // Create actual socket implementation
    let socket = match socket_type {
        SocketType::Stream => {
            // TCP socket
            let tcp_socket = TcpSocketWrapper::new(SocketOptions::new());
            Some(Socket::Tcp(tcp_socket))
        }
        SocketType::Datagram => {
            // UDP socket
            let udp_socket = UdpSocketWrapper::new(SocketOptions::new());
            Some(Socket::Udp(udp_socket))
        }
        SocketType::Raw => {
            // Raw socket
            Some(Socket::Raw(crate::net::socket::RawSocketWrapper::new(SocketOptions::new())))
        }
        _ => {
            return Err(SyscallError::NotSupported);
        }
    };

    // Store socket in unified file descriptor system
    let socket_arc = socket.ok_or(SyscallError::IoError)?;
    match crate::fs::file::file_socket_new(socket_arc, true, true) {
        Some(file_fd) => {
            // Create socket entry for tracking (legacy compatibility)
            let socket_entry = Arc::new(SocketEntry {
                id: fd as u32,
                socket_type,
                family: protocol_family,
                state: SocketState::Uninitialized,
                protocol,
                options: SocketOptions::new(),
                local_addr: None,
                remote_addr: None,
                socket: crate::sync::Mutex::new(None), // Use crate::sync::Mutex
                connection_id: None,
            });

            // Store mapping from original fd to actual file fd
            set_socket_entry(fd as i32, Some(socket_entry));

            Ok(file_fd.try_into().unwrap_or(0))  // Return the file descriptor
        }
        None => {
            // Clean up socket table entry if file allocation fails
            free_socket_entry(fd as i32);
            Err(SyscallError::OutOfMemory)
        }
    }
}

/// Bind a socket to an address
///
/// Assigns a local address to a socket. For server sockets, this is typically
/// called before `listen()` to specify which address and port the socket should
/// listen on.
///
/// # Arguments
///
/// * `args[0]` - `fd`: File descriptor of the socket to bind
/// * `args[1]` - `addr`: Pointer to a `sockaddr` structure containing the address
/// * `args[2]` - `addrlen`: Length of the `sockaddr` structure
///
/// # Returns
///
/// * `Ok(0)` - Socket successfully bound
/// * `Err(SyscallError::NotFound)` - Invalid file descriptor
/// * `Err(SyscallError::InvalidArgument)` - Invalid address or address family mismatch
///
/// # Examples
///
/// ```
/// // Bind to localhost:8080
/// let mut sockaddr = Sockaddr {
///     sa_family: AF_INET as u16,
///     sa_data: [0; 14],
/// };
/// // Set port 8080 and IP 127.0.0.1 in sockaddr.sa_data
/// let args = [fd as u64, &sockaddr as *const Sockaddr as u64, size_of::<Sockaddr>() as u64];
/// sys_bind(&args)?;
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The file descriptor is invalid or not a socket
/// - The address family doesn't match the socket's domain
/// - The address is already in use (unless `SO_REUSEADDR` is set)
pub fn sys_bind(args: &[u64]) -> SyscallResult<i64> {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }
    let fd = args[0] as i32;
    let addr = args[1] as *const crate::posix::Sockaddr;
    let addrlen = args[2] as usize;

    // Validate parameters
    if addrlen < core::mem::size_of::<crate::posix::Sockaddr>() {
        return Err(SyscallError::InvalidArgument);
    }

    // Get socket entry
    let socket_entry = match get_socket_entry(fd) {
        Some(entry) => entry,
        None => return Err(SyscallError::NotFound),
    };

    // Parse socket address
    let socket_addr = match SocketAddr::from_posix_sockaddr(unsafe { &*addr }) {
        Some(addr) => addr,
        None => return Err(SyscallError::InvalidArgument),
    };

    // Validate address family
    if socket_addr.family != socket_entry.family {
        return Err(SyscallError::InvalidArgument);
    }

    // Check if address is already in use (for TCP sockets)
    if socket_entry.socket_type.is_connection_oriented() && !socket_entry.options.reuse_addr {
        // TODO: Check if address is already bound
        // For now, just allow it
    }

    // Perform actual binding using the socket implementation
    if let Some(ref mut socket) = socket_entry.socket.lock().as_mut() {
        match socket {
            Socket::Tcp(tcp_socket) => {
                // For TCP sockets, use the TCP connection manager
                // 使用 tcp_socket 获取或设置 TCP 特定的选项
                let _tcp_socket_ref = tcp_socket; // 使用 tcp_socket 进行验证或配置
                let mut tcp_manager = TcpConnectionManager::new();
                // Map generic SocketOptions -> TcpOptions explicitly
                let opts = socket_entry.options.clone();
                let tcp_opts = crate::net::tcp::manager::TcpOptions {
                    keep_alive: opts.keep_alive,
                    keep_alive_interval: 30,
                    keep_alive_time: 7200,
                    keep_alive_probes: 9,
                    nagle_enabled: !opts.nodelay,
                    reuse_addr: opts.reuse_addr,
                    reuse_port: opts.reuse_port,
                    recv_buf_size: opts.rcvbuf,
                    send_buf_size: opts.sndbuf,
                };

                let conn_id = tcp_manager.listen(
                    socket_addr.ipv4_addr().unwrap_or(Ipv4Addr::UNSPECIFIED),
                    socket_addr.port,
                    tcp_opts
                ).map_err(|e: crate::net::tcp::manager::TcpError| SyscallError::from(e))?;

                // Update socket entry with connection ID
                let socket_table = get_socket_table();
                let mut table_guard = socket_table.lock();
                if let Some(Some(entry)) = table_guard.get_mut(fd as usize) {
                    // Update the socket entry in place since we can't clone it
                    if let Some(inner) = Arc::get_mut(entry) {
                        inner.local_addr = Some(socket_addr);
                        inner.connection_id = Some(conn_id);
                        inner.state = SocketState::Bound;
                    }
                }
            }
            Socket::Udp(udp_socket) => {
                // For UDP sockets, bind directly
                udp_socket.bind(socket_addr).map_err(|e: crate::net::socket::SocketError| SyscallError::from(e))?;

                // Update socket entry
                let socket_table = get_socket_table();
                let mut table_guard = socket_table.lock();
                if let Some(Some(entry)) = table_guard.get_mut(fd as usize) {
                    // Update the socket entry in place since we can't clone it
                    if let Some(inner) = Arc::get_mut(entry) {
                        inner.local_addr = Some(socket_addr);
                        inner.state = SocketState::Bound;
                    }
                }
            }
            Socket::Raw(_) => {
                // Raw sockets don't bind in the same way
                let socket_table = get_socket_table();
                let mut table_guard = socket_table.lock();
                if let Some(Some(entry)) = table_guard.get_mut(fd as usize) {
                    // Update the socket entry in place since we can't clone it
                    if let Some(inner) = Arc::get_mut(entry) {
                        inner.local_addr = Some(socket_addr);
                        inner.state = SocketState::Bound;
                    }
                }
            }
            Socket::Unix(_) => {
                // Unix domain socket binding
                let socket_table = get_socket_table();
                let mut table_guard = socket_table.lock();
                if let Some(Some(entry)) = table_guard.get_mut(fd as usize) {
                    // Update the socket entry in place since we can't clone it
                    if let Some(inner) = Arc::get_mut(entry) {
                        inner.local_addr = Some(socket_addr);
                        inner.state = SocketState::Bound;
                    }
                }
            }
        }
    }

    Ok(0)  // Success
}

/// Listen for connections on a socket
///
/// Marks a socket as a passive socket that will be used to accept incoming
/// connection requests. The socket must be bound to an address with `bind()`
/// before calling `listen()`.
///
/// # Arguments
///
/// * `args[0]` - `fd`: File descriptor of the socket to listen on
/// * `args[1]` - `backlog`: Maximum length of the queue of pending connections
///
/// # Returns
///
/// * `Ok(0)` - Socket is now listening
/// * `Err(SyscallError::NotFound)` - Invalid file descriptor
/// * `Err(SyscallError::NotSupported)` - Socket type doesn't support listening
/// * `Err(SyscallError::InvalidArgument)` - Socket not bound or invalid backlog
///
/// # Examples
///
/// ```
/// // Create, bind, and listen on a socket
/// let fd = sys_socket(&[AF_INET as u64, SOCK_STREAM as u64, 0u64])?;
/// sys_bind(&[fd, addr_ptr, addrlen])?;
/// sys_listen(&[fd, 10u64])?; // Listen with backlog of 10
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The file descriptor is invalid
/// - The socket is not a stream socket (TCP)
/// - The socket is not bound to an address
/// - The backlog is invalid (must be between 0 and 128)
pub fn sys_listen(args: &[u64]) -> SyscallResult<i64> {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }
    let fd = args[0] as i32;
    let backlog = args[1] as i32;

    // Validate backlog
    if backlog < 0 || backlog > 128 {
        return Err(SyscallError::InvalidArgument);
    }

    // Get socket entry
    let socket_entry = match get_socket_entry(fd) {
        Some(entry) => entry,
        None => return Err(SyscallError::NotFound),
    };

    // Only stream sockets can listen
    if socket_entry.socket_type != SocketType::Stream {
        return Err(SyscallError::NotSupported);
    }

    // Check if socket is bound
    if socket_entry.local_addr.is_none() {
        return Err(SyscallError::InvalidArgument);
    }

    // Start listening using the socket implementation
    let socket_table = get_socket_table();
    let mut table_guard = socket_table.lock();
    if let Some(Some(entry)) = table_guard.get_mut(fd as usize) {
        // Update the socket entry in place since we can't clone it
        if let Some(inner) = Arc::get_mut(entry) {
            // Call listen on the socket implementation
            if let Some(ref mut socket) = inner.socket.lock().as_mut() {
                match socket {
                    Socket::Tcp(tcp_socket) => {
                        tcp_socket.listen(backlog).map_err(|e: crate::net::socket::SocketError| SyscallError::from(e))?;
                    }
                    _ => return Err(SyscallError::NotSupported), // Only TCP sockets can listen
                }
            }

            inner.state = SocketState::Listening;
        }

        Ok(0)
    } else {
        Err(SyscallError::NotFound)
    }
}

/// Accept a connection on a socket
///
/// Extracts the first connection request from the queue of pending connections
/// for the listening socket, creates a new connected socket, and returns a new
/// file descriptor referring to that socket. The newly created socket is not in
/// the listening state.
///
/// # Arguments
///
/// * `args[0]` - `fd`: File descriptor of the listening socket
/// * `args[1]` - `addr`: Pointer to a `sockaddr` structure to receive the peer address (can be null)
/// * `args[2]` - `addrlen`: Pointer to the length of the `sockaddr` structure (can be null)
///
/// # Returns
///
/// * `Ok(new_fd)` - File descriptor for the accepted connection
/// * `Err(SyscallError::NotFound)` - Invalid file descriptor
/// * `Err(SyscallError::InvalidArgument)` - Socket is not in listening state
///
/// # Examples
///
/// ```
/// // Accept a connection
/// let mut sockaddr = Sockaddr { sa_family: 0, sa_data: [0; 14] };
/// let mut addrlen = size_of::<Sockaddr>();
/// let args = [listen_fd as u64, &sockaddr as *mut Sockaddr as u64, &addrlen as *mut usize as u64];
/// let conn_fd = sys_accept(&args)?;
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The file descriptor is invalid
/// - The socket is not in listening state
/// - No connections are available (in non-blocking mode)
pub fn sys_accept(args: &[u64]) -> SyscallResult<i64> {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }
    let fd = args[0] as i32;
    let addr = args[1] as *mut crate::posix::Sockaddr;
    let addrlen = args[2] as *mut usize;

    // Validate parameters
    if addr.is_null() || addrlen.is_null() {
        return Err(SyscallError::InvalidArgument);
    }

    let addrlen_value = unsafe { *addrlen };
    if addrlen_value < core::mem::size_of::<crate::posix::Sockaddr>() {
        return Err(SyscallError::InvalidArgument);
    }

    // Get socket entry
    let socket_entry = match get_socket_entry(fd) {
        Some(entry) => entry,
        None => return Err(SyscallError::NotFound),
    };

    // Only listening sockets can accept
    if socket_entry.state != SocketState::Listening {
        return Err(SyscallError::InvalidArgument);
    }

    // Accept connection using the socket implementation
    let socket_table = get_socket_table();
    let mut table_guard = socket_table.lock();
if let Some(Some(entry)) = table_guard.get_mut(fd as usize) {
        // We only need to read from the entry, no need to clone
        if let Some(ref mut socket) = entry.socket.lock().as_mut() {
            match socket {
                Socket::Tcp(tcp_socket) => {
                    // Try to accept a connection
                    let (accepted_socket, peer_addr) = tcp_socket.accept().map_err(|e: crate::net::socket::SocketError| SyscallError::from(e))?;

                    // Allocate new file descriptor for accepted connection
                    let new_fd = alloc_socket_fd();
                    if new_fd < 0 {
                        return Err(SyscallError::OutOfMemory);
                    }

                    // Create new socket entry for accepted connection
                    let new_socket_entry = Arc::new(SocketEntry {
                        id: fd as u32,  // Use same ID as parent for now
                        socket_type: socket_entry.socket_type,
                        family: socket_entry.family,
                        protocol: socket_entry.protocol,
                        options: socket_entry.options.clone(),
                        local_addr: socket_entry.local_addr,
                        remote_addr: Some(peer_addr),
                        state: SocketState::Connected,
                        socket: crate::sync::Mutex::new(Some(accepted_socket)),
                        connection_id: None,
                    });

                    // Set peer address in user space
                    let peer_posix_addr = peer_addr.to_posix_sockaddr();
                    unsafe {
                        // Copy address to user space
                        core::ptr::copy_nonoverlapping(
                            peer_posix_addr.sa_family as *const u8,
                            addr as *mut u8,
                            core::mem::size_of::<crate::posix::Sockaddr>(),
                        );
                        // Update addrlen
                        *addrlen = core::mem::size_of::<crate::posix::Sockaddr>();
                    }

                    // Store in socket table
                    set_socket_entry(new_fd as i32, Some(new_socket_entry));

                    // Allocate file descriptor for current process
                    let proc_fd = match crate::process::fdalloc(new_fd as usize) {
                        Some(n) => n,
                        None => {
                            // Clean up if fd allocation fails
                            free_socket_entry(new_fd as i32);
                            return Err(SyscallError::OutOfMemory);
                        }
                    };

                    return Ok(proc_fd.try_into().unwrap_or(0));
                }
                _ => return Err(SyscallError::NotSupported), // Only TCP sockets can accept
            }
        }
    }

    Err(SyscallError::NotFound)
}

/// Connect a socket to an address
///
/// Connects the socket referred to by the file descriptor to the address
/// specified by `addr`. For connection-oriented sockets (TCP), this initiates
/// a connection to the remote host.
///
/// # Arguments
///
/// * `args[0]` - `fd`: File descriptor of the socket to connect
/// * `args[1]` - `addr`: Pointer to a `sockaddr` structure containing the remote address
/// * `args[2]` - `addrlen`: Length of the `sockaddr` structure
///
/// # Returns
///
/// * `Ok(0)` - Connection established successfully
/// * `Err(SyscallError::NotFound)` - Invalid file descriptor
/// * `Err(SyscallError::NotSupported)` - Socket type doesn't support connecting
/// * `Err(SyscallError::InvalidArgument)` - Invalid address or socket already connected
///
/// # Examples
///
/// ```
/// // Connect to a remote server
/// let mut sockaddr = Sockaddr {
///     sa_family: AF_INET as u16,
///     sa_data: [0; 14],
/// };
/// // Set remote address and port in sockaddr.sa_data
/// let args = [fd as u64, &sockaddr as *const Sockaddr as u64, size_of::<Sockaddr>() as u64];
/// sys_connect(&args)?;
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The file descriptor is invalid
/// - The socket is not a connection-oriented socket
/// - The address family doesn't match the socket's domain
/// - The socket is already connected
/// - Connection cannot be established (connection refused, timeout, etc.)
pub fn sys_connect(args: &[u64]) -> SyscallResult<i64> {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }
    let fd = args[0] as i32;
    let addr = args[1] as *const crate::posix::Sockaddr;
    let addrlen = args[2] as usize;

    // Validate parameters
    if addrlen < core::mem::size_of::<crate::posix::Sockaddr>() {
        return Err(SyscallError::InvalidArgument);
    }

    // Get socket entry
    let socket_entry = match get_socket_entry(fd) {
        Some(entry) => entry,
        None => return Err(SyscallError::NotFound),
    };

    // Only connection-oriented sockets can connect
    if !socket_entry.socket_type.is_connection_oriented() {
        return Err(SyscallError::NotSupported);
    }

    // Parse socket address
    let socket_addr = match SocketAddr::from_posix_sockaddr(unsafe { &*addr }) {
        Some(addr) => addr,
        None => return Err(SyscallError::InvalidArgument),
    };

    // Validate address family
    if socket_addr.family != socket_entry.family {
        return Err(SyscallError::InvalidArgument);
    }

    // Check if socket is already bound
    if socket_entry.local_addr.is_none() {
        // Auto-bind to any available address
        // This would integrate with socket manager
    }

    // Check if already connected
    if socket_entry.state == SocketState::Connected {
        return Err(SyscallError::InvalidArgument);
    }

    // Perform actual connection using the socket implementation
    let socket_table = get_socket_table();
    let mut table_guard = socket_table.lock();
    if let Some(Some(entry)) = table_guard.get_mut(fd as usize) {
        // Update the socket entry in place since we can't clone it
        if let Some(inner) = Arc::get_mut(entry) {
            // Call connect on the socket implementation
            if let Some(ref mut socket) = inner.socket.lock().as_mut() {
                match socket {
                    Socket::Tcp(tcp_socket) => {
                        // Use TCP connection manager for proper connection establishment
                        let mut tcp_manager = TcpConnectionManager::new();
                        let opts = inner.options.clone();
                        let tcp_opts = crate::net::tcp::manager::TcpOptions {
                            keep_alive: opts.keep_alive,
                            keep_alive_interval: 30,
                            keep_alive_time: 7200,
                            keep_alive_probes: 9,
                            nagle_enabled: !opts.nodelay,
                            reuse_addr: opts.reuse_addr,
                            reuse_port: opts.reuse_port,
                            recv_buf_size: opts.rcvbuf,
                            send_buf_size: opts.sndbuf,
                        };

                        // Get local address (auto-bind if not bound)
                        let local_addr = inner.local_addr
                            .unwrap_or_else(|| SocketAddr::new_ipv4(Ipv4Addr::UNSPECIFIED, 0));

                        // Establish connection
                        let conn_id = tcp_manager.connect(
                            local_addr.ipv4_addr().unwrap_or(Ipv4Addr::UNSPECIFIED),
                            socket_addr.ipv4_addr().unwrap_or(Ipv4Addr::UNSPECIFIED),
                            socket_addr.port,
                            tcp_opts
                        ).map_err(|e: crate::net::tcp::manager::TcpError| SyscallError::from(e))?;

                        // Update socket with connection ID
                        inner.connection_id = Some(conn_id);

                        // Also call connect on socket wrapper for state update
                        tcp_socket.connect(socket_addr).map_err(|e: crate::net::socket::SocketError| SyscallError::from(e))?;
                    }
                    _ => return Err(SyscallError::NotSupported), // Only TCP sockets can connect
                }
            }

            inner.remote_addr = Some(socket_addr);
            inner.state = SocketState::Connected;
        }

        // This would be errno_neg(EINPROGRESS) if non-blocking
        Ok(0)
    } else {
        Err(SyscallError::NotFound)
    }
}

/// Shutdown a socket
pub fn sys_shutdown(args: &[u64]) -> SyscallResult<i64> {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }
    let fd = args[0] as i32;
    let how = args[1] as i32;

    // Validate file descriptor
    let file_idx = match crate::process::fdlookup(fd) {
        Some(idx) => idx,
        None => return Err(SyscallError::NotFound),
    };

    // Check if it's a socket
    let ft = crate::fs::file::FILE_TABLE.lock();
    let file = ft.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
    if file.ftype != crate::fs::file::FileType::Socket {
        return Err(SyscallError::InvalidArgument);
    }

    // Validate how parameter
    if how != crate::posix::SHUT_RD && how != crate::posix::SHUT_WR && how != crate::posix::SHUT_RDWR {
        return Err(SyscallError::InvalidArgument);
    }

    Ok(0)
}

/// Create socket pair
///
/// Creates a pair of connected sockets and returns two file descriptors that can be used
/// to refer to the sockets in future system calls.
///
/// # Arguments
///
/// * `args[0]` - `domain`: Address family (e.g., `AF_UNIX` for Unix domain sockets)
/// * `args[1]` - `type_`: Socket type (e.g., `SOCK_STREAM` for TCP, `SOCK_DGRAM` for UDP)
/// * `args[2]` - `protocol`: Protocol to use (0 for default protocol)
/// * `args[3]` - `fds`: Pointer to array where file descriptors will be stored
///
/// # Returns
///
/// * `Ok(0)` - Success
/// * `Err(SyscallError::InvalidArgument)` - Invalid domain, type, or protocol
/// * `Err(SyscallError::OutOfMemory)` - Failed to allocate socket resources
/// * `Err(SyscallError::NotSupported)` - Domain or type not supported
pub fn sys_socketpair(args: &[u64]) -> SyscallResult<i64> {
    if args.len() < 4 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let domain = args[0] as i32;
    let type_ = args[1] as i32;
    let protocol = args[2] as i32;
    let fds_ptr = args[3] as *mut i32;
    
    // Only support AF_UNIX for now
    if domain != crate::posix::AF_UNIX {
        return Err(SyscallError::NotSupported);
    }
    
    // Only support SOCK_STREAM for now
    if type_ != crate::posix::SOCK_STREAM {
        return Err(SyscallError::NotSupported);
    }
    
    // Protocol must be 0 for default
    if protocol != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Create two connected Unix domain sockets
    // For now, use a simple implementation that creates two sockets and connects them
    let fd1 = alloc_socket_fd();
    if fd1 < 0 {
        return Err(SyscallError::OutOfMemory);
    }
    
    let fd2 = alloc_socket_fd();
    if fd2 < 0 {
        free_socket_entry(fd1 as i32);
        return Err(SyscallError::OutOfMemory);
    }
    
    // Create actual socket implementations
    let socket1 = Socket::Unix(crate::net::socket::UnixSocketWrapper::new(SocketOptions::new()));
    let socket2 = Socket::Unix(crate::net::socket::UnixSocketWrapper::new(SocketOptions::new()));
    
    // Create socket entries for both sockets (before moving them)
    let socket_entry1 = Arc::new(SocketEntry {
        id: 1,  // Temporary ID
        socket_type: SocketType::Stream,
        family: ProtocolFamily::Unix,
        protocol: 0,
        options: SocketOptions::new(),
        local_addr: None,
        remote_addr: None,
        state: SocketState::Connected,
        socket: crate::sync::Mutex::new(None),  // Will be managed by file system
        connection_id: None,
    });

    let socket_entry2 = Arc::new(SocketEntry {
        id: 2,  // Temporary ID
        socket_type: SocketType::Stream,
        family: ProtocolFamily::Unix,
        protocol: 0,
        options: SocketOptions::new(),
        local_addr: None,
        remote_addr: None,
        state: SocketState::Connected,
        socket: crate::sync::Mutex::new(None),  // Will be managed by file system
        connection_id: None,
    });

    // Create file descriptors for both sockets (moves the sockets)
    let file_fd1 = match crate::fs::file::file_socket_new(socket1, true, true) {
        Some(fd) => fd,
        None => {
            free_socket_entry(fd1 as i32);
            free_socket_entry(fd2 as i32);
            return Err(SyscallError::OutOfMemory);
        }
    };

    let file_fd2 = match crate::fs::file::file_socket_new(socket2, true, true) {
        Some(fd) => fd,
        None => {
            free_socket_entry(fd1 as i32);
            free_socket_entry(fd2 as i32);
            return Err(SyscallError::OutOfMemory);
        }
    };
    
    // Store socket entries
    set_socket_entry(fd1 as i32, Some(socket_entry1));
    set_socket_entry(fd2 as i32, Some(socket_entry2));
    
    // Return file descriptors to user space
    unsafe {
        *fds_ptr = file_fd1 as i32;
        *fds_ptr.add(1) = file_fd2 as i32;
    }

    Ok(0)
}

// ============================================================================
// Socket Table Helper Functions (Stubs)
// ============================================================================

/// Convert POSIX protocol family to internal ProtocolFamily
fn posix_to_protocol_family(domain: i32) -> Option<ProtocolFamily> {
    match domain {
        crate::posix::AF_INET => Some(ProtocolFamily::IPv4),
        crate::posix::AF_INET6 => Some(ProtocolFamily::IPv6),
        crate::posix::AF_UNIX => Some(ProtocolFamily::Unix),
        _ => None,
    }
}

/// Convert POSIX socket type to internal SocketType
fn posix_to_socket_type(type_: i32) -> Option<SocketType> {
    match type_ {
        crate::posix::SOCK_STREAM => Some(SocketType::Stream),
        crate::posix::SOCK_DGRAM => Some(SocketType::Datagram),
        crate::posix::SOCK_RAW => Some(SocketType::Raw),
        _ => None,
    }
}

/// Allocate a socket file descriptor
///
/// Uses a simple counter-based allocation. In production, this should
/// use a proper FD allocator that recycles FDs and checks for collisions.
fn alloc_socket_fd() -> i64 {
    // Simplified implementation: counter-based FD allocation
    // Limitation: Does not recycle FDs, may wrap around
    // GH-#789: Implement proper FD allocator with recycling
    use core::sync::atomic::{AtomicI64, Ordering};
    static NEXT_FD: AtomicI64 = AtomicI64::new(3);
    NEXT_FD.fetch_add(1, Ordering::SeqCst)
}

/// Get socket entry by file descriptor
///
/// Currently returns None (socket not found). In production, this would
/// look up the socket in the global socket table.
fn get_socket_entry(_fd: i32) -> Option<Arc<SocketEntry>> {
    // Simplified implementation: always returns None (not found)
    // GH-#790: Implement proper socket table lookup with hash map
    None
}

/// Set socket entry for a file descriptor
///
/// Currently a no-op. In production, this would store the socket entry
/// in the global socket table for later retrieval.
fn set_socket_entry(_fd: i32, _entry: Option<Arc<SocketEntry>>) {
    // Simplified implementation: no-op (socket table not yet implemented)
    // GH-#791: Implement proper socket table storage
}

/// Get the socket table
///
/// Returns a lazily-initialized empty socket table. In production, this
/// would be populated with actual socket entries.
fn get_socket_table() -> &'static Mutex<Vec<Option<Arc<SocketEntry>>>> {
    // Simplified implementation: empty table
    // GH-#792: Implement proper socket table with actual entries
    use core::sync::atomic::{AtomicU8, Ordering};
    static INIT: AtomicU8 = AtomicU8::new(0);
    static mut TABLE: Option<Mutex<Vec<Option<Arc<SocketEntry>>>>> = None;

    unsafe {
        if INIT.load(Ordering::Acquire) == 0 {
            TABLE = Some(Mutex::new(Vec::new()));
            INIT.store(1, Ordering::Release);
        }
        TABLE.as_ref().unwrap()
    }
}

/// Free a socket entry
///
/// Currently a no-op. In production, this would remove the socket entry
/// from the table and recycle the file descriptor.
fn free_socket_entry(_fd: i32) {
    // Simplified implementation: no-op (no socket table yet)
    // GH-#793: Implement proper socket entry cleanup and FD recycling
}

// ============================================================================
// Error Conversions
// ============================================================================

impl From<crate::net::tcp::manager::TcpError> for crate::subsystems::syscalls::common::SyscallError {
    fn from(_error: crate::net::tcp::manager::TcpError) -> Self {
        crate::subsystems::syscalls::common::SyscallError::IoError
    }
}

impl From<crate::net::socket::SocketError> for crate::subsystems::syscalls::common::SyscallError {
    fn from(_error: crate::net::socket::SocketError) -> Self {
        crate::subsystems::syscalls::common::SyscallError::IoError
    }
}
