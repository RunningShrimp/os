//! System Call Layered Architecture Example
//!
//! This module demonstrates how to use the new layered system call architecture.
//! It shows how to create a dispatcher, register handlers, and dispatch system calls.

use alloc::sync::Arc;
use crate::syscalls::interface::{SyscallDispatcher, SyscallContext};
use crate::syscalls::dispatch::{SyscallDispatcherImpl, DispatcherConfig};
use crate::syscalls::implementation::register_all_handlers;
use crate::syscalls::context::SyscallContextImpl;
use crate::syscalls::interface::syscall_numbers::*;

/// Example of using the layered system call architecture
pub fn example_usage() -> Result<(), crate::syscalls::interface::SyscallError> {
    // Create a system call context
    let context = Arc::new(SyscallContextImpl::new());
    
    // Create a dispatcher with default configuration
    let context_clone = Arc::clone(&context);
    let mut dispatcher = SyscallDispatcherImpl::with_default_config(context_clone);
    
    // Register all system call handlers
    register_all_handlers(&mut dispatcher)?;
    
    // Dispatch a getpid system call
    let pid = dispatcher.dispatch(SYS_GETPID, &[])?;
    crate::println!("PID: {}", pid);
    
    // Dispatch a fork system call
    let child_pid = dispatcher.dispatch(SYS_FORK, &[])?;
    crate::println!("Child PID: {}", child_pid);
    
    // Dispatch a mmap system call
    let mmap_args = [0u64, 4096u64, 3u64, 0x22u64, 0u64, 0u64]; // addr, length, prot, flags, fd, offset
    let mapped_addr = dispatcher.dispatch(SYS_MMAP, &mmap_args)?;
    crate::println!("Mapped address: 0x{:x}", mapped_addr);
    
    // Dispatch an open system call
    let open_args = [0x1000u64, 0u64, 0o644u64]; // pathname_ptr, flags, mode
    let fd = dispatcher.dispatch(SYS_OPEN, &open_args)?;
    crate::println!("File descriptor: {}", fd);
    
    // Dispatch a read system call
    let read_args = [fd, 0x2000u64, 1024u64]; // fd, buf_ptr, count
    let bytes_read = dispatcher.dispatch(SYS_READ, &read_args)?;
    crate::println!("Bytes read: {}", bytes_read);
    
    // Dispatch a write system call
    let write_args = [fd, 0x2000u64, 1024u64]; // fd, buf_ptr, count
    let bytes_written = dispatcher.dispatch(SYS_WRITE, &write_args)?;
    crate::println!("Bytes written: {}", bytes_written);
    
    // Dispatch a close system call
    let close_args = [fd]; // fd
    let close_result = dispatcher.dispatch(SYS_CLOSE, &close_args)?;
    crate::println!("Close result: {}", close_result);
    
    // Dispatch a socket system call
    let socket_args = [2u64, 1u64, 0u64]; // domain, socket_type, protocol
    let sockfd = dispatcher.dispatch(SYS_SOCKET, &socket_args)?;
    crate::println!("Socket descriptor: {}", sockfd);
    
    // Dispatch a bind system call
    let bind_args = [sockfd, 0x3000u64, 16u64]; // sockfd, addr_ptr, addrlen
    let bind_result = dispatcher.dispatch(SYS_BIND, &bind_args)?;
    crate::println!("Bind result: {}", bind_result);
    
    Ok(())
}

/// Example of using custom dispatcher configuration
pub fn custom_config_example() -> Result<(), crate::syscalls::interface::SyscallError> {
    // Create a custom dispatcher configuration
    let config = DispatcherConfig {
        enable_fast_path: true,
        enable_monitoring: true,
        enable_validation: true,
        max_cache_size: 512,
    };
    
    // Create a dispatcher with custom configuration
    let context = Arc::new(SyscallContextImpl::new());
    
    // Create a dispatcher with custom configuration
    let context_clone = Arc::clone(&context);
    let mut dispatcher = SyscallDispatcherImpl::new(config, context_clone);
    
    // Register all system call handlers
    register_all_handlers(&mut dispatcher)?;
    
    // Dispatch system calls
    let pid = dispatcher.dispatch(SYS_GETPID, &[])?;
    crate::println!("PID: {}", pid);
    
    Ok(())
}

/// Example of checking system call support
pub fn check_support_example() -> Result<(), crate::syscalls::interface::SyscallError> {
    // Create a system call context
    let context = Arc::new(SyscallContextImpl::new());
    
    // Create a dispatcher with default configuration
    let context_clone = Arc::clone(&context);
    let mut dispatcher = SyscallDispatcherImpl::with_default_config(context_clone);
    
    // Register all system call handlers
    register_all_handlers(&mut dispatcher)?;
    
    // Check if system calls are supported
    let is_getpid_supported = dispatcher.is_supported(SYS_GETPID);
    crate::println!("getpid supported: {}", is_getpid_supported);
    
    let is_fork_supported = dispatcher.is_supported(SYS_FORK);
    crate::println!("fork supported: {}", is_fork_supported);
    
    let is_mmap_supported = dispatcher.is_supported(SYS_MMAP);
    crate::println!("mmap supported: {}", is_mmap_supported);
    
    let is_open_supported = dispatcher.is_supported(SYS_OPEN);
    crate::println!("open supported: {}", is_open_supported);
    
    let is_socket_supported = dispatcher.is_supported(SYS_SOCKET);
    crate::println!("socket supported: {}", is_socket_supported);
    
    // Get system call names
    let getpid_name = dispatcher.get_name(SYS_GETPID);
    crate::println!("getpid name: {:?}", getpid_name);
    
    let fork_name = dispatcher.get_name(SYS_FORK);
    crate::println!("fork name: {:?}", fork_name);
    
    let mmap_name = dispatcher.get_name(SYS_MMAP);
    crate::println!("mmap name: {:?}", mmap_name);
    
    let open_name = dispatcher.get_name(SYS_OPEN);
    crate::println!("open name: {:?}", open_name);
    
    let socket_name = dispatcher.get_name(SYS_SOCKET);
    crate::println!("socket name: {:?}", socket_name);
    
    Ok(())
}

/// Example of using system call context
pub fn context_example() -> Result<(), crate::syscalls::interface::SyscallError> {
    // Create a system call context with specific values
    let context = Arc::new(SyscallContextImpl::with_values(
        1234, // pid
        1000, // uid
        1000, // gid
        "/home/user", // cwd
    ));
    
    // Create a dispatcher with the context
    let context_clone = Arc::clone(&context);
    let mut dispatcher = SyscallDispatcherImpl::with_default_config(context_clone);
    
    // Register all system call handlers
    register_all_handlers(&mut dispatcher)?;
    
    // Dispatch a getpid system call
    let pid = dispatcher.dispatch(SYS_GETPID, &[])?;
    crate::println!("PID: {}", pid);
    
    // Check permissions
    let context = dispatcher.get_context();
    crate::println!("Has permission to read: {}", context.has_permission("read"));
    crate::println!("Has permission to write: {}", context.has_permission("write"));
    crate::println!("Has permission to execute: {}", context.has_permission("execute"));
    
    Ok(())
}

/// Example of error handling
pub fn error_handling_example() {
    // Create a system call context
    let context = Arc::new(SyscallContextImpl::new());
    
    // Create a dispatcher with default configuration
    let context_clone = Arc::clone(&context);
    let dispatcher = SyscallDispatcherImpl::with_default_config(context_clone);
    
    // Try to dispatch an unsupported system call
    let result = dispatcher.dispatch(0x9999, &[]);
    match result {
        Ok(value) => crate::println!("Unexpected success: {}", value),
        Err(error) => {
            crate::println!("Expected error: {:?}", error);
            crate::println!("Error code: {}", error.to_errno());
        }
    }
    
    // Try to dispatch a system call with invalid arguments
    let mut dispatcher = SyscallDispatcherImpl::with_default_config(context);
    register_all_handlers(&mut dispatcher).unwrap();
    
    let result = dispatcher.dispatch(SYS_MMAP, &[]); // Missing arguments
    match result {
        Ok(value) => crate::println!("Unexpected success: {}", value),
        Err(error) => {
            crate::println!("Expected error: {:?}", error);
            crate::println!("Error code: {}", error.to_errno());
        }
    }
}

/// Run all examples
pub fn run_all_examples() {
    crate::println!("=== Running layered architecture examples ===");
    
    crate::println!("\n--- Basic usage example ---");
    if let Err(error) = example_usage() {
        crate::println!("Error: {:?}", error);
    }
    
    crate::println!("\n--- Custom configuration example ---");
    if let Err(error) = custom_config_example() {
        crate::println!("Error: {:?}", error);
    }
    
    crate::println!("\n--- Check support example ---");
    if let Err(error) = check_support_example() {
        crate::println!("Error: {:?}", error);
    }
    
    crate::println!("\n--- Context example ---");
    if let Err(error) = context_example() {
        crate::println!("Error: {:?}", error);
    }
    
    crate::println!("\n--- Error handling example ---");
    error_handling_example();
    
    crate::println!("\n=== All examples completed ===");
}