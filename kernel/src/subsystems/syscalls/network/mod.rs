//! 网络系统调用模块
//!
//! 本模块提供网络相关的系统调用处理。

pub mod interface;
pub mod socket;

use alloc::sync::Arc;
use crate::error::Result;
use crate::subsystems::syscalls::interface::{SyscallHandler, SyscallNumber, SyscallError};
use crate::subsystems::net::enhanced_network_manager;

/// 网络系统调用处理器
pub struct NetworkSyscallHandler {
    // Enhanced network manager for POSIX compatibility
    enhanced_manager: &'static crate::subsystems::net::enhanced_network::EnhancedNetworkManager,
}

impl NetworkSyscallHandler {
    /// 创建新的网络系统调用处理器
    pub fn new() -> Self {
        Self { enhanced_manager: enhanced_network_manager() }
    }
}

impl SyscallHandler for NetworkSyscallHandler {
    fn handle(&self, _args: &[u64]) -> crate::subsystems::syscalls::interface::SyscallResult<()> {
        // For network syscalls, we need to dispatch based on syscall number
        // But the trait interface doesn't provide the syscall number
        // This suggests we need a different approach - possibly multiple handlers
        // For now, return not supported error
        Err(SyscallError::NotSupported)
    }

    fn get_syscall_number(&self) -> SyscallNumber {
        // This handler shouldn't be called directly for specific syscalls
        // Each network syscall should have its own handler
        0x4000 // Default network syscall number
    }

    fn get_name(&self) -> &'static str {
        "network_syscall_handler"
    }
}

impl NetworkSyscallHandler {
    /// 创建套接字
    fn sys_socket(&self, args: &[usize]) -> isize {
        if args.len() < 3 {
            return -1; // EINVAL: Invalid argument
        }

        let domain = args[1] as i32;
        let socket_type = args[2] as i32;
        let protocol = args[3] as i32;

        match self.enhanced_manager.socket(domain, socket_type, protocol, crate::subsystems::net::enhanced_network::SocketFlags::NONE) {
            Ok(fd) => fd as isize,
            Err(_) => -1, // Error code would be set in errno
        }
    }

    /// 绑定套接字
    fn sys_bind(&self, args: &[usize]) -> isize {
        if args.len() < 3 {
            return -1; // EINVAL: Invalid argument
        }

        let sockfd = args[1];
        let addr_ptr = args[2] as *const u8;
        let addrlen = args[3];

        // In a real implementation, we would need to read the address from user space
        // For now, we'll use a placeholder implementation
        match self.enhanced_manager.bind_syscall(sockfd, addr_ptr, addrlen) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }

    /// 连接套接字
    fn sys_connect(&self, args: &[usize]) -> isize {
        if args.len() < 3 {
            return -1; // EINVAL: Invalid argument
        }

        let sockfd = args[1];
        let addr_ptr = args[2] as *const u8;
        let addrlen = args[3];

        // In a real implementation, we would need to read the address from user space
        // For now, we'll use a placeholder implementation
        match self.enhanced_manager.connect_syscall(sockfd, addr_ptr, addrlen) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }

    /// 监听套接字
    fn sys_listen(&self, args: &[usize]) -> isize {
        if args.len() < 2 {
            return -1; // EINVAL: Invalid argument
        }

        let sockfd = args[1];
        let backlog = args[2] as i32;

        match self.enhanced_manager.listen(sockfd, backlog) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }

    /// 接受连接
    fn sys_accept(&self, args: &[usize]) -> isize {
        if args.len() < 3 {
            return -1; // EINVAL: Invalid argument
        }

        let sockfd = args[1];
        let addr_ptr = args[2] as *mut u8;
        let addrlen_ptr = args[3] as *mut u32;

        // In a real implementation, we would need to write the address to user space
        // For now, we'll use a placeholder implementation
        match self.enhanced_manager.accept_syscall(sockfd, addr_ptr, addrlen_ptr) {
            Ok(new_fd) => new_fd as isize,
            Err(_) => -1,
        }
    }

    /// 发送数据
    fn sys_send(&self, args: &[usize]) -> isize {
        if args.len() < 4 {
            return -1; // EINVAL: Invalid argument
        }

        let sockfd = args[1];
        let buf_ptr = args[2] as *const u8;
        let len = args[3];
        let flags = args[4] as i32;

        // In a real implementation, we would need to read the buffer from user space
        // For now, we'll use a placeholder implementation
        match self.enhanced_manager.send_syscall(sockfd, buf_ptr, len, flags) {
            Ok(bytes_sent) => bytes_sent as isize,
            Err(_) => -1,
        }
    }

    /// 接收数据
    fn sys_recv(&self, args: &[usize]) -> isize {
        if args.len() < 4 {
            return -1; // EINVAL: Invalid argument
        }

        let sockfd = args[1];
        let buf_ptr = args[2] as *mut u8;
        let len = args[3];
        let flags = args[4] as i32;

        // In a real implementation, we would need to write the buffer to user space
        // For now, we'll use a placeholder implementation
        match self.enhanced_manager.recv_syscall(sockfd, buf_ptr, len, flags) {
            Ok(bytes_received) => bytes_received as isize,
            Err(_) => -1,
        }
    }
}

/// 创建网络系统调用处理器
pub fn create_network_handler() -> Arc<dyn SyscallHandler> {
    Arc::new(NetworkSyscallHandler::new())
}

/// Add an IP address to a network interface
///
/// 为网络接口添加IP地址
pub fn add_interface_address(
    _interface: &str,
    _ip_address: &str,
    _peer: &str,
) -> Result<()> {
    crate::println!("[syscalls::network] add_interface_address");
    // Stub implementation - always returns success
    Ok(())
}

/// Create a veth (virtual ethernet) pair
///
/// 创建veth（虚拟以太网）对
pub fn create_veth_pair(_name1: &str, _name2: &str) -> Result<()> {
    crate::println!("[syscalls::network] create_veth_pair");
    // Stub implementation - always returns success
    Ok(())
}

/// Add a network route
///
/// 添加网络路由
pub fn add_route(_destination: &str, _gateway: &str, _interface: &str) -> Result<()> {
    crate::println!("[syscalls::network] add_route");
    // Stub implementation - always returns success
    Ok(())
}

/// Bring up a network interface
///
/// 启用网络接口
pub fn interface_up(_interface: &str) -> Result<()> {
    crate::println!("[syscalls::network] interface_up");
    // Stub implementation - always returns success
    Ok(())
}

/// Create a network bridge
///
/// 创建网桥
pub fn create_bridge(_name: &str) -> Result<()> {
    crate::println!("[syscalls::network] create_bridge");
    // Stub implementation - always returns success
    Ok(())
}

/// Set interface MTU
///
/// 设置接口MTU
pub fn set_interface_mtu(_interface: &str, _mtu: u32) -> Result<()> {
    crate::println!("[syscalls::network] set_interface_mtu");
    // Stub implementation - always returns success
    Ok(())
}
