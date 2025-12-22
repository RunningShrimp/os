//! Network system calls
//!
//! This module provides network related system calls.

#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use nos_api::Result;
use crate::core::SyscallHandler;
#[cfg(feature = "log")]
use log;
#[cfg(feature = "alloc")]
use crate::core::SyscallDispatcher;

/// Register network system call handlers
#[cfg(feature = "alloc")]
pub fn register_handlers(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // Register socket system call
    dispatcher.register_handler(
        crate::types::SYS_SOCKET,
        Box::new(SocketHandler)
    );
    
    // Register connect system call
    dispatcher.register_handler(
        crate::types::SYS_CONNECT,
        Box::new(ConnectHandler)
    );
    
    // Register accept system call
    dispatcher.register_handler(
        crate::types::SYS_ACCEPT,
        Box::new(AcceptHandler)
    );
    
    Ok(())
}

/// Socket system call handler
struct SocketHandler;

impl SyscallHandler for SocketHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_SOCKET
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            #[cfg(feature = "alloc")]
        return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
        #[cfg(not(feature = "alloc"))]
        return Err(nos_api::Error::InvalidArgument("Insufficient arguments".into()));
        }
        
        let domain = args[0] as i32;
        let type_ = args[1] as i32;
        let protocol = args[2] as i32;
        
        // TODO: Implement actual socket logic using parameters:
        // domain: Address family (AF_INET, AF_INET6, etc.)
        // type_: Socket type (SOCK_STREAM, SOCK_DGRAM, etc.)
        // protocol: Protocol type (0 for default protocol)
        #[cfg(feature = "log")]
        log::trace!("socket called with: domain={}, type={}, protocol={}", domain, type_, protocol);
        
        // Ensure parameters are used even when logging is disabled
        let _ = (domain, type_, protocol);
        
        Ok(3) // Return a dummy socket descriptor
    }
    
    fn name(&self) -> &str {
        "socket"
    }
}

/// Connect system call handler
struct ConnectHandler;

impl SyscallHandler for ConnectHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_CONNECT
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".into()));
        }
        
        let sockfd = args[0] as i32;
        let addr = args[1] as *const u8;
        let addrlen = args[2] as u32;
        
        // TODO: Implement actual connect logic using parameters:
        // sockfd: Socket file descriptor
        // addr: Pointer to socket address structure
        // addrlen: Size of socket address structure
        #[cfg(feature = "log")]
        log::trace!("connect called with: sockfd={}, addr={:?}, addrlen={}", sockfd, addr, addrlen);
        
        // Ensure parameters are used even when logging is disabled
        let _ = (sockfd, addr, addrlen);
        
        Ok(0)
    }
    
    fn name(&self) -> &str {
        "connect"
    }
}

/// Accept system call handler
struct AcceptHandler;

impl SyscallHandler for AcceptHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_ACCEPT
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".into()));
        }
        
        let sockfd = args[0] as i32;
        let addr = args[1] as *mut u8;
        let addrlen = args[2] as *mut u32;
        
        // TODO: Implement actual accept logic using parameters:
        // sockfd: Socket file descriptor for listening
        // addr: Pointer to store client address
        // addrlen: Pointer to store client address length
        #[cfg(feature = "log")]
        log::trace!("accept called with: sockfd={}, addr={:?}, addrlen={:?}", sockfd, addr, addrlen);
        
        // Ensure parameters are used even when logging is disabled
        let _ = (sockfd, addr, addrlen);
        
        Ok(4) // Return a dummy socket descriptor
    }
    
    fn name(&self) -> &str {
        "accept"
    }
}