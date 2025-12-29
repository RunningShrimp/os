//! 网络系统调用模块
//!
//! 本模块提供网络相关的系统调用处理。

pub mod interface;
pub mod socket;

use alloc::sync::Arc;

use crate::subsystems::syscalls::interface::{SyscallHandler};
use crate::subsystems::syscalls::interface::{SyscallNumber};
use crate::subsystems::syscalls::common::SyscallArgs;
use crate::error::Result;

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
    fn handle(&self, args: &[u64]) -> SyscallResult<i64> {
        // For network syscalls, we need to dispatch based on syscall number
        // But the trait interface doesn't provide the syscall number
        // This suggests we need a different approach - possibly multiple handlers
        // For now, return invalid syscall since we can't determine which one was called
        Err(SyscallError::InvalidSyscall(self.get_syscall_number()))
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

        match self.enhanced_manager.socket(domain, socket_type, protocol) {
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
        match self.enhanced_manager.bind(sockfd, addr_ptr, addrlen) {
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
        match self.enhanced_manager.connect(sockfd, addr_ptr, addrlen) {
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
        let backlog = args[2];

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
        match self.enhanced_manager.accept(sockfd, addr_ptr, addrlen_ptr) {
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
        let flags = args[4];

        // In a real implementation, we would need to read the buffer from user space
        // For now, we'll use a placeholder implementation
        match self.enhanced_manager.send(sockfd, buf_ptr, len, flags) {
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
        let flags = args[4];

        // In a real implementation, we would need to write the buffer to user space
        // For now, we'll use a placeholder implementation
        match self.enhanced_manager.recv(sockfd, buf_ptr, len, flags) {
            Ok(bytes_received) => bytes_received as isize,
            Err(_) => -1,
        }
    }
}

/// 创建网络系统调用处理器
pub fn create_network_handler() -> Arc<dyn SyscallHandler> {
    Arc::new(NetworkSyscallHandler::new())
}
