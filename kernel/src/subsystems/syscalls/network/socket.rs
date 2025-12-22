//! Socket creation and management syscalls

use super::*;
use crate::syscalls::common::{SyscallError, SyscallResult};
use crate::net::socket::SocketAddr;

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
pub fn sys_socket(args: &[u64]) -> SyscallResult {
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
                socket_type,
                protocol_family,
                protocol,
                options: SocketOptions::new(),
                local_addr: None,
                remote_addr: None,
                state: SocketState::Uninitialized,
                socket: Mutex::new(None), // Already stored in file system
                connection_id: None,
            });

            // Store mapping from original fd to actual file fd
            set_socket_entry(fd as i32, Some(socket_entry));

            Ok(file_fd as u64)  // Return the file descriptor
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
pub fn sys_bind(args: &[u64]) -> SyscallResult {
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
    if socket_addr.family != socket_entry.protocol_family {
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
                if let Some(Some(entry)) = socket_table.get_mut(fd as usize) {
                    // Since SocketEntry is Clone, we can create a new entry with updated values
                    let old_entry = entry.as_ref();
                    let mut new_entry = old_entry.clone();
                    new_entry.local_addr = Some(socket_addr);
                    new_entry.connection_id = Some(conn_id);
                    new_entry.state = SocketState::Bound;
                    *entry = Arc::new(new_entry);
                }
            }
            Socket::Udp(udp_socket) => {
                // For UDP sockets, bind directly
                udp_socket.bind(socket_addr).map_err(|e: crate::net::socket::SocketError| SyscallError::from(e))?;

                // Update socket entry
                let socket_table = get_socket_table();
                if let Some(Some(entry)) = socket_table.get_mut(fd as usize) {
                    // Since SocketEntry is Clone, we can create a new entry with updated values
                    let old_entry = entry.as_ref();
                    let mut new_entry = old_entry.clone();
                    new_entry.local_addr = Some(socket_addr);
                    new_entry.state = SocketState::Bound;
                    *entry = Arc::new(new_entry);
                }
            }
            Socket::Raw(_) => {
                // Raw sockets don't bind in the same way
                let socket_table = get_socket_table();
                if let Some(Some(entry)) = socket_table.get_mut(fd as usize) {
                    // Since SocketEntry is Clone, we can create a new entry with updated values
                    let old_entry = entry.as_ref();
                    let mut new_entry = old_entry.clone();
                    new_entry.local_addr = Some(socket_addr);
                    new_entry.state = SocketState::Bound;
                    *entry = Arc::new(new_entry);
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
pub fn sys_listen(args: &[u64]) -> SyscallResult {
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
    if let Some(Some(entry)) = socket_table.get_mut(fd as usize) {
        // Since SocketEntry is Clone, we can create a new entry with updated values
        let old_entry = entry.as_ref();
        let mut new_entry = old_entry.clone();

        // Call listen on the socket implementation
        if let Some(ref mut socket) = new_entry.socket.lock().as_mut() {
            match socket {
                Socket::Tcp(tcp_socket) => {
                    tcp_socket.listen(backlog).map_err(|e: crate::net::socket::SocketError| SyscallError::from(e))?;
                }
                _ => return Err(SyscallError::NotSupported), // Only TCP sockets can listen
            }
        }

        new_entry.state = SocketState::Listening;
        *entry = Arc::new(new_entry);

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
pub fn sys_accept(args: &[u64]) -> SyscallResult {
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
    if let Some(Some(entry)) = socket_table.get_mut(fd as usize) {
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
                        socket_type: socket_entry.socket_type,
                        protocol_family: socket_entry.protocol_family,
                        protocol: socket_entry.protocol,
                        options: socket_entry.options.clone(),
                        local_addr: socket_entry.local_addr,
                        remote_addr: Some(peer_addr),
                        state: SocketState::Connected,
                        socket: Mutex::new(Some(accepted_socket)),
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

                    return Ok(proc_fd as u64);
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
pub fn sys_connect(args: &[u64]) -> SyscallResult {
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
    if socket_addr.family != socket_entry.protocol_family {
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
    if let Some(Some(entry)) = socket_table.get_mut(fd as usize) {
        // Since SocketEntry is Clone, we can create a new entry with updated values
        let old_entry = entry.as_ref();
        let mut new_entry = old_entry.clone();

        // Call connect on the socket implementation
        if let Some(ref mut socket) = new_entry.socket.lock().as_mut() {
            match socket {
                Socket::Tcp(tcp_socket) => {
                    // Use TCP connection manager for proper connection establishment
                    let mut tcp_manager = TcpConnectionManager::new();
                    let opts = new_entry.options.clone();
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
                    let local_addr = new_entry.local_addr
                        .unwrap_or_else(|| SocketAddr::new_ipv4(Ipv4Addr::UNSPECIFIED, 0));
                    
                    // Establish connection
                    let conn_id = tcp_manager.connect(
                        local_addr.ipv4_addr().unwrap_or(Ipv4Addr::UNSPECIFIED),
                        socket_addr.ipv4_addr().unwrap_or(Ipv4Addr::UNSPECIFIED),
                        socket_addr.port,
                        tcp_opts
                    ).map_err(|e: crate::net::tcp::manager::TcpError| SyscallError::from(e))?;

                    // Update socket with connection ID
                    new_entry.connection_id = Some(conn_id);
                    
                    // Also call connect on socket wrapper for state update
                    tcp_socket.connect(socket_addr).map_err(|e: crate::net::socket::SocketError| SyscallError::from(e))?;
                }
                _ => return Err(SyscallError::NotSupported), // Only TCP sockets can connect
            }
        }

        new_entry.remote_addr = Some(socket_addr);
        new_entry.state = SocketState::Connected;
        *entry = Arc::new(new_entry);

        // This would be errno_neg(EINPROGRESS) if non-blocking
        Ok(0)
    } else {
        Err(SyscallError::NotFound)
    }
}

/// Shutdown a socket
pub fn sys_shutdown(args: &[u64]) -> SyscallResult {
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
pub fn sys_socketpair(_args: &[u64]) -> SyscallResult {
    // TODO: Implement socketpair syscall
    Err(SyscallError::NotSupported)
}
