//! Epoll system calls
//!
//! This module provides system calls for the epoll API,
//! which is an efficient I/O event notification mechanism.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::format;
use alloc::vec::Vec;
use nos_api::{Result, Error};
use spin::Mutex;
use crate::{SyscallHandler, SyscallDispatcher};

/// Epoll event flags
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum EpollEventFlags {
    Read = 0x1,
    Write = 0x2,
    Hangup = 0x4,
    Error = 0x8,
    Priority = 0x10,
    OneShot = 0x40000000,
    EdgeTriggered = 0x80000000,
}

impl EpollEventFlags {
    #[allow(clippy::if_same_then_else)]
    pub fn from_bits(bits: u32) -> Self {
        if bits & 0x80000000 != 0 {
            Self::EdgeTriggered
        } else if bits & 0x40000000 != 0 {
            Self::OneShot
        } else if bits & 0x10 != 0 {
            Self::Priority
        } else if bits & 0x8 != 0 {
            Self::Error
        } else if bits & 0x4 != 0 {
            Self::Hangup
        } else if bits & 0x2 != 0 {
            Self::Write
        } else if bits & 0x1 != 0 {
            Self::Read
        } else {
            Self::Read
        }
    }
    
    pub fn to_bits(&self) -> u32 {
        *self as u32
    }
}

/// Epoll operation types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
pub enum EpollOp {
    Add = 1,
    Delete = 2,
    Modify = 3,
}

impl EpollOp {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => Self::Add,
            2 => Self::Delete,
            3 => Self::Modify,
            _ => Self::Add,
        }
    }
    
    pub fn to_i32(&self) -> i32 {
        *self as i32
    }
}

/// Epoll event
#[derive(Debug, Clone)]
pub struct EpollEvent {
    pub events: EpollEventFlags,
    pub data: u64,
}

/// Epoll instance
pub struct EpollInstance {
    /// Instance ID
    pub id: u64,
    /// Registered file descriptors
    pub fds: Mutex<BTreeMap<i32, EpollEvent>>,
    /// Event queue
    pub events: Mutex<Vec<EpollEvent>>,
    /// Event notification mode
    pub mode: EpollMode,
    /// Maximum number of events
    pub max_events: usize,
}

/// Epoll event notification mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EpollMode {
    LevelTriggered,
    EdgeTriggered,
}

impl EpollInstance {
    pub fn new(id: u64, mode: EpollMode, max_events: usize) -> Self {
        Self {
            id,
            fds: Mutex::new(BTreeMap::new()),
            events: Mutex::new(Vec::new()),
            mode,
            max_events,
        }
    }
    
    pub fn add_fd(&self, fd: i32, event: EpollEvent) -> Result<()> {
        let mut fds = self.fds.lock();
        if fds.contains_key(&fd) {
            return Err(Error::InvalidArgument(format!("FD {} already registered", fd)));
        }
        fds.insert(fd, event);
        sys_trace_with_args!("Added FD {} to epoll instance {}", fd, self.id);
        Ok(())
    }
    
    pub fn modify_fd(&self, fd: i32, event: EpollEvent) -> Result<()> {
        let mut fds = self.fds.lock();
        if !fds.contains_key(&fd) {
            return Err(Error::NotFound(format!("FD {} not registered", fd)));
        }
        fds.insert(fd, event);
        sys_trace_with_args!("Modified FD {} in epoll instance {}", fd, self.id);
        Ok(())
    }
    
    pub fn delete_fd(&self, fd: i32) -> Result<()> {
        let mut fds = self.fds.lock();
        fds.remove(&fd).ok_or_else(|| Error::NotFound(format!("FD {} not registered", fd)))?;
        sys_trace_with_args!("Deleted FD {} from epoll instance {}", fd, self.id);
        Ok(())
    }
    
    pub fn notify_event(&self, event: EpollEvent) {
        let mut events = self.events.lock();
        if events.len() < self.max_events {
            events.push(event);
        }
    }
    
    pub fn wait_events(&self, max_events: usize) -> Vec<EpollEvent> {
        let mut events = self.events.lock();
        let count = max_events.min(events.len());
        
        events.drain(0..count).collect()
    }
    
    pub fn get_registered_fds(&self) -> Vec<i32> {
        let fds = self.fds.lock();
        fds.keys().cloned().collect()
    }
    
    pub fn get_event_count(&self) -> usize {
        self.events.lock().len()
    }
}

/// Epoll manager
pub struct EpollManager {
    /// Next instance ID
    next_id: Mutex<u64>,
    /// Active instances
    instances: Mutex<BTreeMap<u64, Arc<EpollInstance>>>,
    /// Default mode for new instances
    default_mode: Mutex<EpollMode>,
}

impl Default for EpollManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EpollManager {
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(1),
            instances: Mutex::new(BTreeMap::new()),
            default_mode: Mutex::new(EpollMode::LevelTriggered),
        }
    }
    
    pub fn create_instance(&self, mode: EpollMode, max_events: usize) -> Result<u64> {
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;
        
        let instance = Arc::new(EpollInstance::new(id, mode, max_events));
        
        let mut instances = self.instances.lock();
        instances.insert(id, instance);
        
        sys_trace_with_args!("Created epoll instance {} with mode {:?}", id, mode);
        
        Ok(id)
    }
    
    pub fn get_instance(&self, id: u64) -> Option<Arc<EpollInstance>> {
        let instances = self.instances.lock();
        instances.get(&id).cloned()
    }
    
    pub fn close_instance(&self, id: u64) -> Result<()> {
        let mut instances = self.instances.lock();
        instances.remove(&id).ok_or_else(|| Error::NotFound(format!("Epoll instance {} not found", id)))?;
        sys_trace_with_args!("Closed epoll instance {}", id);
        Ok(())
    }
    
    pub fn set_default_mode(&self, mode: EpollMode) {
        *self.default_mode.lock() = mode;
    }
    
    pub fn get_default_mode(&self) -> EpollMode {
        *self.default_mode.lock()
    }
}

/// Epoll create system call handler
pub struct EpollCreateHandler {
    manager: Arc<EpollManager>,
}

impl EpollCreateHandler {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(EpollManager::new()),
        }
    }
    
    pub fn manager(&self) -> &Arc<EpollManager> {
        &self.manager
    }
}

impl Default for EpollCreateHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SyscallHandler for EpollCreateHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_EPOLL_CREATE
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        let mode = if !args.is_empty() && args[0] != 0 {
            EpollMode::EdgeTriggered
        } else {
            EpollMode::LevelTriggered
        };
        
        let max_events = if args.len() > 1 { args[1] } else { 1024 };
        
        let id = self.manager.create_instance(mode, max_events)?;
        
        Ok(id as isize)
    }
    
    fn name(&self) -> &str {
        "epoll_create"
    }
}

/// Epoll control system call handler
pub struct EpollCtlHandler {
    manager: Arc<EpollManager>,
}

impl EpollCtlHandler {
    pub fn new(manager: Arc<EpollManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for EpollCtlHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_EPOLL_CTL
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 4 {
            return Err(Error::InvalidArgument("Insufficient arguments for epoll_ctl".to_string()));
        }

        let epoll_id = args[0] as u64;
        let op = EpollOp::from_i32(args[1] as i32);
        let fd = args[2] as i32;
        let event_flags = EpollEventFlags::from_bits(args[3] as u32);
        let event_data = if args.len() > 4 { args[4] as u64 } else { 0 };
        
        let instance = self.manager.get_instance(epoll_id)
            .ok_or_else(|| Error::NotFound(format!("Epoll instance {} not found", epoll_id)))?;
        
        let event = EpollEvent {
            events: event_flags,
            data: event_data,
        };
        
        match op {
            EpollOp::Add => instance.add_fd(fd, event)?,
            EpollOp::Delete => instance.delete_fd(fd)?,
            EpollOp::Modify => instance.modify_fd(fd, event)?,
        }
        
        sys_trace_with_args!("epoll_ctl: instance={}, op={:?}, fd={:?}, flags={:?}",
                   epoll_id, op, fd, event_flags);
        
        Ok(0)
    }
    
    fn name(&self) -> &str {
        "epoll_ctl"
    }
}

/// Epoll wait system call handler
pub struct EpollWaitHandler {
    manager: Arc<EpollManager>,
}

impl EpollWaitHandler {
    pub fn new(manager: Arc<EpollManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for EpollWaitHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_EPOLL_WAIT
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 2 {
            return Err(Error::InvalidArgument("Insufficient arguments for epoll_wait".to_string()));
        }

        let epoll_id = args[0] as u64;
        let max_events = args[1];
        
        let instance = self.manager.get_instance(epoll_id)
            .ok_or_else(|| Error::NotFound(format!("Epoll instance {} not found", epoll_id)))?;
        
        let events = instance.wait_events(max_events);
        
        sys_trace_with_args!("epoll_wait: instance={}, returned {} events", epoll_id, events.len());
        
        Ok(events.len() as isize)
    }
    
    fn name(&self) -> &str {
        "epoll_wait"
    }
}

/// Epoll close system call handler
pub struct EpollCloseHandler {
    manager: Arc<EpollManager>,
}

impl EpollCloseHandler {
    pub fn new(manager: Arc<EpollManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for EpollCloseHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_EPOLL_CLOSE
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.is_empty() {
            return Err(Error::InvalidArgument("Insufficient arguments for epoll_close".to_string()));
        }

        let epoll_id = args[0] as u64;
        self.manager.close_instance(epoll_id)?;
        
        Ok(0)
    }
    
    fn name(&self) -> &str {
        "epoll_close"
    }
}

/// Register epoll system calls
pub fn register_syscalls(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    let handler = EpollCreateHandler::new();
    let manager = handler.manager().clone();
    
    dispatcher.register_handler(1006, Box::new(handler));
    dispatcher.register_handler(1007, Box::new(EpollCtlHandler::new(manager.clone())));
    dispatcher.register_handler(1008, Box::new(EpollWaitHandler::new(manager.clone())));
    dispatcher.register_handler(1009, Box::new(EpollCloseHandler::new(manager)));
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoll_event_flags() {
        assert_eq!(EpollEventFlags::from_bits(0x1), EpollEventFlags::Read);
        assert_eq!(EpollEventFlags::from_bits(0x2), EpollEventFlags::Write);
        assert_eq!(EpollEventFlags::from_bits(0x80000000), EpollEventFlags::EdgeTriggered);
        assert_eq!(EpollEventFlags::Read.to_bits(), 0x1);
    }

    #[test]
    fn test_epoll_op() {
        assert_eq!(EpollOp::from_i32(1), EpollOp::Add);
        assert_eq!(EpollOp::from_i32(2), EpollOp::Delete);
        assert_eq!(EpollOp::from_i32(3), EpollOp::Modify);
        assert_eq!(EpollOp::Add.to_i32(), 1);
    }

    #[test]
    fn test_epoll_instance() {
        let instance = EpollInstance::new(1, EpollMode::LevelTriggered, 10);
        
        let event = EpollEvent {
            events: EpollEventFlags::Read,
            data: 42,
        };
        
        instance.add_fd(3, event.clone()).unwrap();
        assert_eq!(instance.get_registered_fds(), vec![3]);
        
        instance.modify_fd(3, EpollEvent {
            events: EpollEventFlags::Write,
            data: 43,
        }).unwrap();
        
        instance.notify_event(event);
        assert_eq!(instance.get_event_count(), 1);
        
        let events = instance.wait_events(10);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_epoll_manager() {
        let manager = EpollManager::new();
        
        let id = manager.create_instance(EpollMode::EdgeTriggered, 100).unwrap();
        assert!(id > 0);
        
        let instance = manager.get_instance(id).unwrap();
        assert_eq!(instance.id, id);
        assert_eq!(instance.mode, EpollMode::EdgeTriggered);
        
        manager.close_instance(id).unwrap();
        assert!(manager.get_instance(id).is_none());
    }

    #[test]
    fn test_epoll_handler() {
        let handler = EpollCreateHandler::new();
        assert_eq!(handler.name(), "epoll_create");
        
        let result = handler.execute(&[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_epoll_ctl_handler() {
        let manager = EpollManager::new();
        let create_handler = EpollCreateHandler::new();
        let ctl_handler = EpollCtlHandler::new(manager.clone());
        
        let id = create_handler.execute(&[1, 100]).unwrap() as u64;
        
        let result = ctl_handler.execute(&[id as usize, 1, 3, 0x1, 42]);
        assert!(result.is_ok());
    }
}
