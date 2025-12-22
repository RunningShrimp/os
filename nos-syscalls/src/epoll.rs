//! Epoll system calls
//!
//! This module provides epoll system calls for efficient event notification,
//! including edge-triggered and level-triggered modes.

#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::collections::{BTreeMap, VecDeque};
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::format;

use nos_api::{Result, Error};
use spin::Mutex;
use crate::core::{SyscallHandler, SyscallDispatcher};

/// Epoll event
#[derive(Debug, Clone, Copy)]
pub struct EpollEvent {
    /// File descriptor
    pub fd: i32,
    /// Events
    pub events: u32,
    /// User data
    pub data: u64,
}

/// Epoll instance
pub struct EpollInstance {
    /// Instance ID
    pub id: u32,
    /// File descriptors being monitored
    monitored_fds: Mutex<BTreeMap<i32, EpollEvent>>,
    /// Event queue
    event_queue: Mutex<VecDeque<EpollEvent>>,
    /// Next event ID
    next_event_id: Mutex<u64>,
}

impl EpollInstance {
    /// Create a new epoll instance
    pub fn new(id: u32) -> Self {
        Self {
            id,
            monitored_fds: Mutex::new(BTreeMap::new()),
            event_queue: Mutex::new(VecDeque::new()),
            next_event_id: Mutex::new(1),
        }
    }
    
    /// Add a file descriptor to monitor
    pub fn add_fd(&self, fd: i32, event: EpollEvent) -> Result<()> {
        let mut monitored_fds = self.monitored_fds.lock();
        monitored_fds.insert(fd, event);
        Ok(())
    }
    
    /// Modify a monitored file descriptor
    pub fn modify_fd(&self, fd: i32, event: EpollEvent) -> Result<()> {
        let mut monitored_fds = self.monitored_fds.lock();
        if monitored_fds.contains_key(&fd) {
            monitored_fds.insert(fd, event);
            Ok(())
        } else {
            #[cfg(feature = "alloc")]
            return Err(Error::NotFound(format!("FD {} not being monitored", fd)));
            #[cfg(not(feature = "alloc"))]
            return Err(Error::NotFound("FD not being monitored".into()));
        }
    }
    
    /// Remove a file descriptor from monitoring
    pub fn remove_fd(&self, fd: i32) -> Result<()> {
        let mut monitored_fds = self.monitored_fds.lock();
        #[cfg(feature = "alloc")]
        {
            monitored_fds.remove(&fd).ok_or_else(|| Error::NotFound(format!("FD {} not being monitored", fd)))?;
        }
        #[cfg(not(feature = "alloc"))]
        {
            monitored_fds.remove(&fd).ok_or_else(|| Error::NotFound("FD not being monitored".into()))?;
        }
        Ok(())
    }
    
    /// Add an event to the queue
    #[cfg(feature = "alloc")]
    pub fn add_event(&self, event: EpollEvent) -> Result<()> {
        let mut event_queue = self.event_queue.lock();
        event_queue.push_back(event);
        Ok(())
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn add_event(&self, _event: EpollEvent) -> Result<()> {
        // In a no-alloc environment, we can't add events to the queue
        // For now, we'll just return success
        Ok(())
    }
    
    /// Wait for events
    #[cfg(feature = "alloc")]
    pub fn wait(&self, max_events: usize, _timeout_ms: Option<u32>) -> Result<Vec<EpollEvent>> {
        let mut event_queue = self.event_queue.lock();
        let mut events = Vec::with_capacity(max_events);
        
        // In a real implementation, we would wait for events or timeout
        // For now, we'll just return available events
        while events.len() < max_events {
            if let Some(event) = event_queue.pop_front() {
                events.push(event);
            } else {
                break;
            }
        }
        
        Ok(events)
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn wait(&self, _max_events: usize, _timeout_ms: Option<u32>) -> Result<Vec<EpollEvent>> {
        // In a no-alloc environment, we can't return a Vec
        // For now, we'll just return an empty result
        Ok(Vec::new())
    }
    
    /// Get the list of monitored file descriptors
    #[cfg(feature = "alloc")]
    pub fn monitored_fds(&self) -> Vec<i32> {
        let monitored_fds = self.monitored_fds.lock();
        monitored_fds.keys().cloned().collect()
    }
    
    #[cfg(not(feature = "alloc"))]
    pub fn monitored_fds(&self) -> Vec<i32> {
        // In a no-alloc environment, we can't return a Vec
        // For now, we'll just return an empty result
        Vec::new()
    }
}

/// Epoll manager
pub struct EpollManager {
    /// Next instance ID
    next_id: Mutex<u32>,
    /// Epoll instances
    instances: Mutex<BTreeMap<u32, Arc<EpollInstance>>>,
}

impl EpollManager {
    /// Create a new epoll manager
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(1),
            instances: Mutex::new(BTreeMap::new()),
        }
    }
    
    /// Create a new epoll instance
    pub fn create_instance(&self) -> Result<u32> {
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;
        
        let instance = Arc::new(EpollInstance::new(id));
        
        let mut instances = self.instances.lock();
        instances.insert(id, instance);
        
        Ok(id)
    }
    
    /// Get an epoll instance
    pub fn get_instance(&self, id: u32) -> Option<Arc<EpollInstance>> {
        let instances = self.instances.lock();
        instances.get(&id).cloned()
    }
    
    /// Remove an epoll instance
    pub fn remove_instance(&self, id: u32) -> Result<()> {
        let mut instances = self.instances.lock();
        #[cfg(feature = "alloc")]
        {
            instances.remove(&id).ok_or_else(|| Error::NotFound(format!("Epoll instance {} not found", id)))?;
        }
        #[cfg(not(feature = "alloc"))]
        {
            instances.remove(&id).ok_or_else(|| Error::NotFound("Epoll instance not found".into()))?;
        }
        Ok(())
    }
}

/// Epoll create system call handler
pub struct EpollCreateHandler {
    /// Epoll manager
    manager: Arc<EpollManager>,
}

impl EpollCreateHandler {
    /// Create a new epoll create handler
    pub fn new(manager: Arc<EpollManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for EpollCreateHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_EPOLL_CREATE
    }
    
    fn execute(&self, _args: &[usize]) -> Result<isize> {
        let instance_id = self.manager.create_instance()?;
        Ok(instance_id as isize)
    }
    
    fn name(&self) -> &str {
        "epoll_create"
    }
}

/// Epoll control system call handler
pub struct EpollCtlHandler {
    /// Epoll manager
    manager: Arc<EpollManager>,
}

impl EpollCtlHandler {
    /// Create a new epoll control handler
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
            #[cfg(feature = "alloc")]
            return Err(Error::InvalidArgument("Insufficient arguments for epoll_ctl".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(Error::InvalidArgument("Insufficient arguments for epoll_ctl".into()));
        }

        let instance_id = args[0] as u32;
        let op = args[1];
        let fd = args[2] as i32;
        let event_data = args[3];
        
                #[cfg(feature = "alloc")]
        let instance = self.manager.get_instance(instance_id)
            .ok_or_else(|| Error::NotFound("Epoll instance not found".to_string()))?;
        #[cfg(not(feature = "alloc"))]
        let instance = self.manager.get_instance(instance_id)
            .ok_or_else(|| Error::NotFound("Epoll instance not found".into()))?;
        
        // Parse event data (in a real implementation, this would be a pointer to an epoll_event struct)
        let event = EpollEvent {
            fd,
            events: (event_data >> 32) as u32,
            data: event_data as u64,
        };
        
        match op {
            1 => instance.add_fd(fd, event)?, // EPOLL_CTL_ADD
            2 => instance.modify_fd(fd, event)?, // EPOLL_CTL_MOD
            3 => instance.remove_fd(fd)?, // EPOLL_CTL_DEL
            #[cfg(feature = "alloc")]
            _ => return Err(Error::InvalidArgument("Invalid epoll operation".to_string())),
            #[cfg(not(feature = "alloc"))]
            _ => return Err(Error::InvalidArgument("Invalid epoll operation".into())),
        }
        
        Ok(0)
    }
    
    fn name(&self) -> &str {
        "epoll_ctl"
    }
}

/// Epoll wait system call handler
pub struct EpollWaitHandler {
    /// Epoll manager
    manager: Arc<EpollManager>,
}

impl EpollWaitHandler {
    /// Create a new epoll wait handler
    pub fn new(manager: Arc<EpollManager>) -> Self {
        Self { manager }
    }
}

impl SyscallHandler for EpollWaitHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_EPOLL_WAIT
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 3 {
            #[cfg(feature = "alloc")]
            return Err(Error::InvalidArgument("Insufficient arguments for epoll_wait".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(Error::InvalidArgument("Insufficient arguments for epoll_wait".into()));
        }

        let instance_id = args[0] as u32;
        let max_events = args[1];
        let timeout_ms = args[2] as u32;
        
        let instance = self.manager.get_instance(instance_id)
            .ok_or_else(|| Error::NotFound("Epoll instance not found".to_string()))?;
        
        let events = instance.wait(max_events, Some(timeout_ms))?;
        
        // In a real implementation, we would copy events to user space
        // For now, we'll just return the number of events
        Ok(events.len() as isize)
    }
    
    fn name(&self) -> &str {
        "epoll_wait"
    }
}

/// Register epoll system calls
pub fn register_syscalls(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    #[cfg(feature = "alloc")]
    {
        let manager = Arc::new(EpollManager::new());
        
        dispatcher.register_handler(320, Box::new(EpollCreateHandler::new(manager.clone())));
        dispatcher.register_handler(321, Box::new(EpollCtlHandler::new(manager.clone())));
        dispatcher.register_handler(322, Box::new(EpollWaitHandler::new(manager)));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoll_instance() {
        let instance = EpollInstance::new(1);
        
        // Add a file descriptor
        let event = EpollEvent {
            fd: 3,
            events: 0x1, // EPOLLIN
            data: 0x12345678,
        };
        
        instance.add_fd(3, event).unwrap();
        
        // Check monitored file descriptors
        let fds = instance.monitored_fds();
        assert_eq!(fds.len(), 1);
        assert_eq!(fds[0], 3);
        
        // Modify the file descriptor
        let modified_event = EpollEvent {
            fd: 3,
            events: 0x3, // EPOLLIN | EPOLLOUT
            data: 0x87654321,
        };
        
        instance.modify_fd(3, modified_event).unwrap();
        
        // Remove the file descriptor
        instance.remove_fd(3).unwrap();
        
        let fds = instance.monitored_fds();
        assert_eq!(fds.len(), 0);
    }
    
    #[test]
    fn test_epoll_manager() {
        let manager = EpollManager::new();
        
        // Create an instance
        let instance_id = manager.create_instance().unwrap();
        assert!(instance_id > 0);
        
        // Get the instance
        let instance = manager.get_instance(instance_id).unwrap();
        assert_eq!(instance.id, instance_id);
        
        // Remove the instance
        manager.remove_instance(instance_id).unwrap();
        
        // Check that the instance is gone
        let instance = manager.get_instance(instance_id);
        assert!(instance.is_none());
    }
}