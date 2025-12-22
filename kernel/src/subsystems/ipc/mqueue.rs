//! POSIX Message Queue Implementation
//! 
//! This module provides POSIX-compliant message queues (mqueue) with
//! priority-based message delivery, notification mechanisms, and resource limits.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

use crate::subsystems::process::Pid;
use crate::subsystems::ipc::signal::{SIGEV_SIGNAL, SIGEV_NONE};

/// Message queue attributes
#[derive(Debug, Clone)]
pub struct MqAttr {
    /// Maximum number of messages
    pub mq_maxmsg: u64,
    /// Maximum message size
    pub mq_msgsize: u64,
    /// Current number of messages
    pub mq_curmsgs: u64,
    /// Message queue flags
    pub mq_flags: i32,
}

impl Default for MqAttr {
    fn default() -> Self {
        Self {
            mq_maxmsg: 10,      // Default: 10 messages
            mq_msgsize: 8192,   // Default: 8KB messages
            mq_curmsgs: 0,
            mq_flags: 0,
        }
    }
}

/// Message queue notification
#[derive(Debug, Clone)]
pub struct MqNotify {
    /// Notification type
    pub notify_type: i32,
    /// Signal number for signal notification
    pub signal: i32,
    /// Process ID for signal notification
    pub pid: Pid,
    /// Notification function pointer
    pub notify_function: usize,
}

impl Default for MqNotify {
    fn default() -> Self {
        Self {
            notify_type: SIGEV_NONE,
            signal: 0,
            pid: 0,
            notify_function: 0,
        }
    }
}

/// Message queue message
#[derive(Debug, Clone)]
pub struct MqMessage {
    /// Message priority (0 = lowest)
    pub priority: u32,
    /// Message data
    pub data: Vec<u8>,
    /// Message timestamp
    pub timestamp: u64,
    /// Sender process ID
    pub sender_pid: Pid,
}

/// Message queue statistics
#[derive(Debug, Default, Clone)]
pub struct MqStats {
    /// Total messages sent
    pub total_messages_sent: u64,
    /// Total messages received
    pub total_messages_received: u64,
    /// Total messages dropped (queue full)
    pub total_messages_dropped: u64,
    /// Total notifications sent
    pub total_notifications_sent: u64,
    /// Average queue depth
    pub avg_queue_depth: f64,
    /// Maximum queue depth
    pub max_queue_depth: u64,
    /// Total wait time in microseconds
    pub total_wait_time_us: u64,
    /// Maximum wait time in microseconds
    pub max_wait_time_us: u64,
}

/// Message queue
pub struct MessageQueue {
    /// Queue name
    pub name: String,
    /// Queue attributes
    pub attr: MqAttr,
    /// Message storage
    pub messages: Mutex<VecDeque<MqMessage>>,
    /// Notification settings
    pub notify: Mutex<MqNotify>,
    /// Queue is open
    pub open: AtomicBool,
    /// Queue statistics
    pub stats: Mutex<MqStats>,
    /// Waiting senders
    pub waiting_senders: Mutex<Vec<Pid>>,
    /// Waiting receivers
    pub waiting_receivers: Mutex<Vec<Pid>>,
    /// Last activity timestamp
    pub last_activity: AtomicU64,
}

impl MessageQueue {
    /// Create a new message queue
    pub fn new(name: String, attr: MqAttr) -> Self {
        Self {
            name,
            attr,
            messages: Mutex::new(VecDeque::new()),
            notify: Mutex::new(MqNotify::default()),
            open: AtomicBool::new(true),
            stats: Mutex::new(MqStats::default()),
            waiting_senders: Mutex::new(Vec::new()),
            waiting_receivers: Mutex::new(Vec::new()),
            last_activity: AtomicU64::new(crate::subsystems::time::timestamp_nanos()),
        }
    }

    /// Send a message to the queue
    pub fn send(&self, msg: MqMessage, timeout_ms: u32) -> Result<(), MqError> {
        // Update last activity
        self.last_activity.store(crate::subsystems::time::timestamp_nanos(), Ordering::Relaxed);

        // Check if queue is open
        if !self.open.load(Ordering::Relaxed) {
            return Err(MqError::QueueClosed);
        }

        // Check message size
        if msg.data.len() > self.attr.mq_msgsize as usize {
            return Err(MqError::MessageTooLarge);
        }

        // Check queue capacity
        {
            let mut messages = self.messages.lock();
            if messages.len() >= self.attr.mq_maxmsg as usize {
                // Check if we should wait
                if timeout_ms == 0 {
                    return Err(MqError::QueueFull);
                }

                // Add to waiting senders
                let current_pid = crate::process::myproc().unwrap_or(0);
                self.waiting_senders.lock().push(current_pid);

                // Release lock and wait
                drop(messages);

                // Wait for space or timeout
                let start_time = crate::subsystems::time::timestamp_nanos();
                loop {
                    // Check if space is available
                    {
                        let messages = self.messages.lock();
                        if messages.len() < self.attr.mq_maxmsg as usize {
                            // Remove from waiting senders
                            self.waiting_senders.lock().retain(|&pid| pid != current_pid);
                            break;
                        }
                    }

                    // Check timeout
                    let elapsed = (crate::subsystems::time::timestamp_nanos() - start_time) / 1000000;
                    if elapsed >= timeout_ms as u64 {
                        // Remove from waiting senders
                        self.waiting_senders.lock().retain(|&pid| pid != current_pid);
                        return Err(MqError::Timeout);
                    }

                    // Yield CPU
                    crate::subsystems::process::thread::thread_yield();
                }
            }

            // Insert message in priority order
            let mut insert_pos = messages.len();
            for (i, existing_msg) in messages.iter().enumerate() {
                if msg.priority > existing_msg.priority {
                    insert_pos = i;
                    break;
                }
            }
            messages.insert(insert_pos, msg);

            // Update current message count
            self.attr.mq_curmsgs = messages.len() as u64;

            // Update statistics
            {
                let mut stats = self.stats.lock();
                stats.total_messages_sent += 1;
                
                // Update queue depth statistics
                let current_depth = messages.len() as u64;
                stats.avg_queue_depth = 
                    (stats.avg_queue_depth * (stats.total_messages_sent - 1) as f64 + current_depth as f64) 
                    / stats.total_messages_sent as f64;
                stats.max_queue_depth = stats.max_queue_depth.max(current_depth);
            }

            // Wake up waiting receivers
            self.wakeup_receivers();
        }

        Ok(())
    }

    /// Receive a message from the queue
    pub fn receive(&self, max_size: usize, timeout_ms: u32) -> Result<MqMessage, MqError> {
        // Update last activity
        self.last_activity.store(crate::subsystems::time::timestamp_nanos(), Ordering::Relaxed);

        // Check if queue is open
        if !self.open.load(Ordering::Relaxed) {
            return Err(MqError::QueueClosed);
        }

        // Wait for message if queue is empty
        let start_time = crate::subsystems::time::timestamp_nanos();
        let msg = loop {
            {
                let mut messages = self.messages.lock();
                
                if let Some(msg) = messages.pop_front() {
                    // Update current message count
                    self.attr.mq_curmsgs = messages.len() as u64;
                    
                    // Update statistics
                    {
                        let mut stats = self.stats.lock();
                        stats.total_messages_received += 1;
                        
                        // Update wait time statistics
                        let wait_time = (crate::subsystems::time::timestamp_nanos() - start_time) / 1000;
                        stats.total_wait_time_us += wait_time;
                        stats.max_wait_time_us = stats.max_wait_time_us.max(wait_time);
                    }
                    
                    // Wake up waiting senders
                    self.wakeup_senders();
                    
                    break msg;
                } else if timeout_ms == 0 {
                    return Err(MqError::QueueEmpty);
                } else {
                    // Add to waiting receivers
                    let current_pid = crate::process::myproc().unwrap_or(0);
                    self.waiting_receivers.lock().push(current_pid);
                    
                    // Release lock and wait
                    drop(messages);
                }
            }

            // Check timeout
            let elapsed = (crate::subsystems::time::timestamp_nanos() - start_time) / 1000000;
            if elapsed >= timeout_ms as u64 {
                // Remove from waiting receivers
                let current_pid = crate::process::myproc().unwrap_or(0);
                self.waiting_receivers.lock().retain(|&pid| pid != current_pid);
                return Err(MqError::Timeout);
            }

            // Yield CPU
            crate::subsystems::process::thread::thread_yield();
        };

        // Check message size
        if msg.data.len() > max_size {
            return Err(MqError::MessageTooLarge);
        }

        Ok(msg)
    }

    /// Get queue attributes
    pub fn get_attr(&self) -> MqAttr {
        let messages = self.messages.lock();
        let mut attr = self.attr.clone();
        attr.mq_curmsgs = messages.len() as u64;
        attr
    }

    /// Set queue attributes
    pub fn set_attr(&mut self, attr: MqAttr) -> Result<(), MqError> {
        // Can't change maxmsg or msgsize while queue has messages
        {
            let messages = self.messages.lock();
            if !messages.is_empty() && 
               (attr.mq_maxmsg != self.attr.mq_maxmsg || 
                attr.mq_msgsize != self.attr.mq_msgsize) {
                return Err(MqError::InvalidOperation);
            }
        }

        self.attr = attr;
        Ok(())
    }

    /// Set notification settings
    pub fn set_notify(&self, notify: MqNotify) -> Result<(), MqError> {
        let mut current_notify = self.notify.lock();
        *current_notify = notify;
        Ok(())
    }

    /// Get notification settings
    pub fn get_notify(&self) -> MqNotify {
        self.notify.lock().clone()
    }

    /// Close the message queue
    pub fn close(&self) -> Result<(), MqError> {
        // Mark as closed
        self.open.store(false, Ordering::Relaxed);

        // Wake up all waiting processes
        self.wakeup_all_waiters();

        // Clear messages
        {
            let mut messages = self.messages.lock();
            let dropped_count = messages.len() as u64;
            messages.clear();
            
            // Update statistics
            let mut stats = self.stats.lock();
            stats.total_messages_dropped += dropped_count;
        }

        Ok(())
    }

    /// Unlink (destroy) the message queue
    pub fn unlink(&self) -> Result<(), MqError> {
        // Close first if not already closed
        if self.open.load(Ordering::Relaxed) {
            self.close()?;
        }

        // Clear all data
        {
            let mut messages = self.messages.lock();
            messages.clear();
        }

        {
            let mut notify = self.notify.lock();
            *notify = MqNotify::default();
        }

        Ok(())
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> MqStats {
        self.stats.lock().clone()
    }

    /// Reset queue statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = MqStats::default();
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.messages.lock().is_empty()
    }

    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        let messages = self.messages.lock();
        messages.len() >= self.attr.mq_maxmsg as usize
    }

    /// Get current message count
    pub fn get_message_count(&self) -> u64 {
        self.messages.lock().len() as u64
    }

    /// Wake up waiting receivers
    fn wakeup_receivers(&self) {
        let mut waiting_receivers = self.waiting_receivers.lock();
        
        for &pid in waiting_receivers.iter() {
            // Wake up process
            let mut proc_table = crate::process::manager::PROC_TABLE.lock();
            if let Some(proc) = proc_table.find_mut(pid) {
                if proc.state == crate::process::ProcState::Sleeping {
                    proc.state = crate::process::ProcState::Runnable;
                }
            }
        }
        
        waiting_receivers.clear();
    }

    /// Wake up waiting senders
    fn wakeup_senders(&self) {
        let mut waiting_senders = self.waiting_senders.lock();
        
        for &pid in waiting_senders.iter() {
            // Wake up process
            let mut proc_table = crate::process::manager::PROC_TABLE.lock();
            if let Some(proc) = proc_table.find_mut(pid) {
                if proc.state == crate::process::ProcState::Sleeping {
                    proc.state = crate::process::ProcState::Runnable;
                }
            }
        }
        
        waiting_senders.clear();
    }

    /// Wake up all waiting processes
    fn wakeup_all_waiters(&self) {
        self.wakeup_receivers();
        self.wakeup_senders();
    }

    /// Send notification if configured
    fn send_notification(&self) {
        let notify = self.notify.lock();
        
        match notify.notify_type {
            SIGEV_SIGNAL => {
                // Send signal to process
                if let Some(proc) = crate::process::manager::PROC_TABLE.lock().find_ref(notify.pid) {
                    if let Some(ref signals) = proc.signals {
                        let _ = signals.send_signal(notify.signal as u32);
                    }
                }
                
                // Update statistics
                let mut stats = self.stats.lock();
                stats.total_notifications_sent += 1;
            }
            _ => {
                // Other notification types not implemented yet
            }
        }
    }
}

/// Message queue errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MqError {
    /// Queue is full
    QueueFull,
    /// Queue is empty
    QueueEmpty,
    /// Message is too large
    MessageTooLarge,
    /// Operation timed out
    Timeout,
    /// Queue is closed
    QueueClosed,
    /// Invalid operation
    InvalidOperation,
    /// Invalid argument
    InvalidArgument,
    /// Permission denied
    PermissionDenied,
    /// Queue does not exist
    QueueNotFound,
    /// Resource limit exceeded
    ResourceLimitExceeded,
    /// System error
    SystemError,
}

/// Global message queue manager
pub struct MessageQueueManager {
    /// Message queues by name
    queues: Mutex<alloc::collections::BTreeMap<String, *mut MessageQueue>>,
    /// Next queue ID
    next_queue_id: AtomicU64,
    /// Global statistics
    global_stats: Mutex<MqStats>,
    /// Maximum queues allowed
    max_queues: AtomicUsize,
}

impl MessageQueueManager {
    /// Create a new message queue manager
    pub fn new() -> Self {
        Self {
            queues: Mutex::new(alloc::collections::BTreeMap::new()),
            next_queue_id: AtomicU64::new(1),
            global_stats: Mutex::new(MqStats::default()),
            max_queues: AtomicUsize::new(256), // Default: 256 queues
        }
    }

    /// Create a new message queue
    pub fn create_queue(&self, name: String, attr: MqAttr) -> Result<u64, MqError> {
        // Check queue limit
        let queues = self.queues.lock();
        if queues.len() >= self.max_queues.load(Ordering::Relaxed) {
            return Err(MqError::ResourceLimitExceeded);
        }

        // Check if queue already exists
        if queues.contains_key(&name) {
            return Err(MqError::QueueNotFound);
        }

        drop(queues);

        // Create new queue
        let queue = MessageQueue::new(name.clone(), attr);
        let queue_id = self.next_queue_id.fetch_add(1, Ordering::SeqCst);
        
        // Add to manager
        {
            let mut queues = self.queues.lock();
            queues.insert(name, Box::into_raw(Box::new(queue)));
        }

        Ok(queue_id)
    }

    /// Open an existing message queue
    pub fn open_queue(&self, name: &str) -> Result<u64, MqError> {
        let queues = self.queues.lock();
        
        if let Some(&queue_ptr) = queues.get(name) {
            let queue = unsafe { &mut *queue_ptr };
            
            // Check if queue is open
            if queue.open.load(Ordering::Relaxed) {
                Ok(queue_id_from_name(name))
            } else {
                Err(MqError::QueueClosed)
            }
        } else {
            Err(MqError::QueueNotFound)
        }
    }

    /// Close a message queue
    pub fn close_queue(&self, name: &str) -> Result<(), MqError> {
        let mut queues = self.queues.lock();
        
        if let Some(&queue_ptr) = queues.get(name) {
            let queue = unsafe { &mut *queue_ptr };
            queue.close()
        } else {
            Err(MqError::QueueNotFound)
        }
    }

    /// Unlink (destroy) a message queue
    pub fn unlink_queue(&self, name: &str) -> Result<(), MqError> {
        let mut queues = self.queues.lock();
        
        if let Some(queue_ptr) = queues.remove(name) {
            let queue = unsafe { Box::from_raw(queue_ptr) };
            queue.unlink()
        } else {
            Err(MqError::QueueNotFound)
        }
    }

    /// Get a message queue by name
    pub fn get_queue(&self, name: &str) -> Option<&'static mut MessageQueue> {
        let queues = self.queues.lock();
        queues.get(name).map(|&ptr| unsafe { &mut **ptr })
    }

    /// Send a message to a queue
    pub fn send_message(&self, name: &str, msg: MqMessage, timeout_ms: u32) -> Result<(), MqError> {
        let queue = self.get_queue(name).ok_or(MqError::QueueNotFound)?;
        queue.send(msg, timeout_ms)
    }

    /// Receive a message from a queue
    pub fn receive_message(&self, name: &str, max_size: usize, timeout_ms: u32) -> Result<MqMessage, MqError> {
        let queue = self.get_queue(name).ok_or(MqError::QueueNotFound)?;
        queue.receive(max_size, timeout_ms)
    }

    /// Get queue attributes
    pub fn get_queue_attr(&self, name: &str) -> Result<MqAttr, MqError> {
        let queue = self.get_queue(name).ok_or(MqError::QueueNotFound)?;
        Ok(queue.get_attr())
    }

    /// Set queue attributes
    pub fn set_queue_attr(&self, name: &str, attr: MqAttr) -> Result<(), MqError> {
        let queue = self.get_queue(name).ok_or(MqError::QueueNotFound)?;
        queue.set_attr(attr)
    }

    /// Set queue notification
    pub fn set_queue_notify(&self, name: &str, notify: MqNotify) -> Result<(), MqError> {
        let queue = self.get_queue(name).ok_or(MqError::QueueNotFound)?;
        queue.set_notify(notify)
    }

    /// Get queue statistics
    pub fn get_queue_stats(&self, name: &str) -> Result<MqStats, MqError> {
        let queue = self.get_queue(name).ok_or(MqError::QueueNotFound)?;
        Ok(queue.get_stats())
    }

    /// Reset queue statistics
    pub fn reset_queue_stats(&self, name: &str) -> Result<(), MqError> {
        let queue = self.get_queue(name).ok_or(MqError::QueueNotFound)?;
        queue.reset_stats();
        Ok(())
    }

    /// Get global statistics
    pub fn get_global_stats(&self) -> MqStats {
        self.global_stats.lock().clone()
    }

    /// Reset global statistics
    pub fn reset_global_stats(&self) {
        *self.global_stats.lock() = MqStats::default();
    }

    /// Set maximum number of queues
    pub fn set_max_queues(&self, max_queues: usize) {
        self.max_queues.store(max_queues, Ordering::Relaxed);
    }

    /// Get maximum number of queues
    pub fn get_max_queues(&self) -> usize {
        self.max_queues.load(Ordering::Relaxed)
    }

    /// Get number of active queues
    pub fn get_queue_count(&self) -> usize {
        self.queues.lock().len()
    }

    /// List all queue names
    pub fn list_queues(&self) -> Vec<String> {
        self.queues.lock().keys().cloned().collect()
    }
}

/// Generate queue ID from name
fn queue_id_from_name(name: &str) -> u64 {
    // Simple hash function for demo
    let mut hash = 0u64;
    for byte in name.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

/// Global message queue manager instance
static mut MQ_MANAGER: Option<MessageQueueManager> = None;
static MQ_MANAGER_INIT: spin::Once = spin::Once::new();

/// Initialize the global message queue manager
pub fn init_mq_manager() {
    MQ_MANAGER_INIT.call_once(|| {
        unsafe {
            MQ_MANAGER = Some(MessageQueueManager::new());
        }
    });
}

/// Get the global message queue manager
pub fn get_mq_manager() -> Option<&'static MessageQueueManager> {
    unsafe {
        MQ_MANAGER.as_ref()
    }
}

/// Create a new message queue
pub fn mq_create(name: String, attr: MqAttr) -> Result<u64, MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.create_queue(name, attr)
}

/// Open an existing message queue
pub fn mq_open(name: &str) -> Result<u64, MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.open_queue(name)
}

/// Close a message queue
pub fn mq_close(name: &str) -> Result<(), MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.close_queue(name)
}

/// Unlink (destroy) a message queue
pub fn mq_unlink(name: &str) -> Result<(), MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.unlink_queue(name)
}

/// Send a message to a queue
pub fn mq_send(name: &str, msg: MqMessage, timeout_ms: u32) -> Result<(), MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.send_message(name, msg, timeout_ms)
}

/// Receive a message from a queue
pub fn mq_receive(name: &str, max_size: usize, timeout_ms: u32) -> Result<MqMessage, MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.receive_message(name, max_size, timeout_ms)
}

/// Get queue attributes
pub fn mq_getattr(name: &str) -> Result<MqAttr, MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.get_queue_attr(name)
}

/// Set queue attributes
pub fn mq_setattr(name: &str, attr: MqAttr) -> Result<(), MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.set_queue_attr(name, attr)
}

/// Set queue notification
pub fn mq_notify(name: &str, notify: MqNotify) -> Result<(), MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.set_queue_notify(name, notify)
}

/// Get queue statistics
pub fn mq_get_stats(name: &str) -> Result<MqStats, MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.get_queue_stats(name)
}

/// Reset queue statistics
pub fn mq_reset_stats(name: &str) -> Result<(), MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.reset_queue_stats(name)
}

/// Get global statistics
pub fn mq_get_global_stats() -> Result<MqStats, MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    Ok(manager.get_global_stats())
}

/// Reset global statistics
pub fn mq_reset_global_stats() -> Result<(), MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    manager.reset_global_stats();
    Ok(())
}

/// List all queue names
pub fn mq_list_queues() -> Result<Vec<String>, MqError> {
    let manager = get_mq_manager().ok_or(MqError::SystemError)?;
    Ok(manager.list_queues())
}