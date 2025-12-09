//! POSIX-compliant Inter-Process Communication (IPC) mechanism
//!
//! This module provides basic POSIX IPC primitives:
//! - Shared memory (shm_*)
//! - Message queues (msg_*)
//! - Semaphores (sem_*)
//!
//! **Note**: For microkernel service communication, use `crate::microkernel::ipc`.
//! For high-performance IPC, use `crate::services::ipc`.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use crate::sync::Mutex;

// ============================================================================
// Constants
// ============================================================================

pub const IPC_MAX_MSG_SIZE: usize = 4096;
pub const IPC_MAX_QUEUE: usize = 128;

// ============================================================================
// Types
// ============================================================================

/// IPC Message Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcType {
    SharedMemory,
    MessageQueue,
    Semaphore,
    SystemCall,
}

/// IPC Message Header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IpcMsgHeader {
    pub msg_type: u32,       // Message type (application-defined)
    pub msg_size: u32,       // Message size in bytes (excluding header)
    pub src_pid: u32,        // Source process ID
    pub dst_pid: u32,        // Destination process ID
    pub flags: u32,          // Message flags (non-blocking, etc.)
}

/// IPC Message (fixed size for simplicity)
#[derive(Debug)]
pub struct IpcMessage {
    pub header: IpcMsgHeader,
    pub data: Vec<u8>,       // Message data
}

/// Shared Memory Region
pub struct SharedMemory {
    pub base_addr: usize,
    pub size: usize,
    pub permissions: u32,    // Read/write/execute permissions
    pub ref_count: u32,      // Number of processes using this region
}

/// Message Queue
pub struct MessageQueue {
    pub queue_id: u32,
    pub messages: Mutex<Vec<IpcMessage>>,
    pub max_size: usize,     // Maximum number of messages in queue
}

// ============================================================================
// Global State
// ============================================================================

static SHARED_MEMORIES: Mutex<Vec<SharedMemory>> = Mutex::new(Vec::new());
static MESSAGE_QUEUES: Mutex<Vec<MessageQueue>> = Mutex::new(Vec::new());
static NEXT_SHM_ID: Mutex<u32> = Mutex::new(1);
static NEXT_MSGQ_ID: Mutex<u32> = Mutex::new(1);

// ============================================================================
// Public API
// ============================================================================

impl IpcMessage {
    /// Create a new IPC message
    pub fn new(src_pid: u32, dst_pid: u32, msg_type: u32, data: &[u8]) -> Self {
        Self {
            header: IpcMsgHeader {
                msg_type,
                msg_size: data.len() as u32,
                src_pid,
                dst_pid,
                flags: 0,
            },
            data: data.to_vec(),
        }
    }
}

impl SharedMemory {
    /// Create a new shared memory region
    pub fn new(base_addr: usize, size: usize, permissions: u32) -> Self {
        Self {
            base_addr,
            size,
            permissions,
            ref_count: 1,
        }
    }
}

impl MessageQueue {
    /// Create a new message queue
    pub fn new(queue_id: u32, max_size: usize) -> Self {
        Self {
            queue_id,
            messages: Mutex::new(Vec::new()),
            max_size,
        }
    }
}

/// Initialize IPC subsystem
pub fn init() {
    // Initialize shared memory and message queue pools
    crate::println!("ipc: initialized");
}

/// Create shared memory region
/// Returns: shared memory ID on success
pub fn shm_create(size: usize, permissions: u32) -> Option<u32> {
    if size == 0 || size > 16 * 1024 * 1024 {  // Max 16MB
        return None;
    }
    
    // Allocate memory for shared region
    // Round up to page size
    let page_size = crate::mm::PAGE_SIZE;
    let aligned_size = (size + page_size - 1) & !(page_size - 1);
    
    // Allocate pages
    let pages_needed = aligned_size / page_size;
    let mut base_addr = 0usize;
    
    for i in 0..pages_needed {
        let page = crate::mm::kalloc();
        if page.is_null() {
            // Cleanup on failure
            for j in 0..i {
                let cleanup_addr = base_addr + j * page_size;
                unsafe {
                    crate::mm::kfree(cleanup_addr as *mut u8);
                }
            }
            return None;
        }
        
        // Zero the page
        unsafe { core::ptr::write_bytes(page, 0, page_size); }
        
        if i == 0 {
            base_addr = page as usize;
        }
    }
    
    // Get next SHM ID
    let mut next_id = NEXT_SHM_ID.lock();
    let shm_id = *next_id;
    *next_id += 1;
    drop(next_id);
    
    // Create SharedMemory struct
    let shm = SharedMemory::new(base_addr, aligned_size, permissions);
    
    // Add to global list
    let mut shm_list = SHARED_MEMORIES.lock();
    shm_list.push(shm);
    
    Some(shm_id)
}

/// Attach to shared memory region
/// Returns: virtual address where shared memory is mapped
pub fn shm_attach(shm_id: u32) -> Option<usize> {
    let mut shm_list = SHARED_MEMORIES.lock();
    
    // Find the shared memory by ID (ID corresponds to index + 1)
    if shm_id == 0 || shm_id as usize > shm_list.len() {
        return None;
    }
    
    let shm = &mut shm_list[shm_id as usize - 1];
    shm.ref_count += 1;
    
    // Return the base address
    // In a real implementation, we would map this into the calling process's
    // address space at an available virtual address
    Some(shm.base_addr)
}

/// Detach from shared memory region
pub fn shm_detach(addr: usize) -> bool {
    let mut shm_list = SHARED_MEMORIES.lock();
    
    // Find shared memory by address
    for shm in shm_list.iter_mut() {
        if shm.base_addr == addr {
            if shm.ref_count > 0 {
                shm.ref_count -= 1;
            }
            return true;
        }
    }
    
    false
}

/// Delete shared memory region
pub fn shm_delete(shm_id: u32) -> bool {
    let mut shm_list = SHARED_MEMORIES.lock();
    
    if shm_id == 0 || shm_id as usize > shm_list.len() {
        return false;
    }
    
    let idx = shm_id as usize - 1;
    let shm = &shm_list[idx];
    
    // Only delete if no processes are attached
    if shm.ref_count > 0 {
        return false;
    }
    
    // Free the memory
    let page_size = crate::mm::PAGE_SIZE;
    let pages = shm.size / page_size;
    for i in 0..pages {
        let addr = shm.base_addr + i * page_size;
        unsafe {
            crate::mm::kfree(addr as *mut u8);
        }
    }
    
    // Remove from list (swap with last and pop)
    shm_list.swap_remove(idx);
    
    true
}

/// Create message queue
/// Returns: true on success
pub fn msg_create(queue_id: u32) -> bool {
    let mut mq_list = MESSAGE_QUEUES.lock();
    
    // Check if queue already exists
    for mq in mq_list.iter() {
        if mq.queue_id == queue_id {
            return false;  // Queue already exists
        }
    }
    
    // Create new queue
    let mq = MessageQueue::new(queue_id, IPC_MAX_QUEUE);
    mq_list.push(mq);
    
    true
}

/// Send message to queue
/// Returns: true on success
pub fn msg_send(queue_id: u32, msg: &IpcMessage) -> bool {
    let mq_list = MESSAGE_QUEUES.lock();
    
    // Find the queue
    for mq in mq_list.iter() {
        if mq.queue_id == queue_id {
            let mut messages = mq.messages.lock();
            
            // Check if queue is full
            if messages.len() >= mq.max_size {
                return false;
            }
            
            // Clone the message and add to queue
            let new_msg = IpcMessage {
                header: msg.header,
                data: msg.data.clone(),
            };
            messages.push(new_msg);
            
            return true;
        }
    }
    
    false  // Queue not found
}

/// Receive message from queue
/// msg_type: 0 = any message, >0 = specific type
pub fn msg_recv(queue_id: u32, msg_type: u32) -> Option<IpcMessage> {
    let mq_list = MESSAGE_QUEUES.lock();
    
    // Find the queue
    for mq in mq_list.iter() {
        if mq.queue_id == queue_id {
            let mut messages = mq.messages.lock();
            
            if messages.is_empty() {
                return None;
            }
            
            // Find matching message
            if msg_type == 0 {
                // Return first message
                return Some(messages.remove(0));
            } else {
                // Find message with matching type
                for (i, m) in messages.iter().enumerate() {
                    if m.header.msg_type == msg_type {
                        return Some(messages.remove(i));
                    }
                }
            }
            
            return None;
        }
    }
    
    None  // Queue not found
}

/// Delete message queue
pub fn msg_delete(queue_id: u32) -> bool {
    let mut mq_list = MESSAGE_QUEUES.lock();
    
    // Find and remove the queue
    for (i, mq) in mq_list.iter().enumerate() {
        if mq.queue_id == queue_id {
            mq_list.swap_remove(i);
            return true;
        }
    }
    
    false  // Queue not found
}

/// Get message queue by ID
pub fn msg_get(queue_id: u32) -> bool {
    let mq_list = MESSAGE_QUEUES.lock();
    
    for mq in mq_list.iter() {
        if mq.queue_id == queue_id {
            return true;
        }
    }
    
    false
}

/// Get shared memory info
pub fn shm_info(shm_id: u32) -> Option<(usize, usize, u32)> {
    let shm_list = SHARED_MEMORIES.lock();
    
    if shm_id == 0 || shm_id as usize > shm_list.len() {
        return None;
    }
    
    let shm = &shm_list[shm_id as usize - 1];
    Some((shm.base_addr, shm.size, shm.ref_count))
}

pub mod signal;
pub mod pipe;

#[cfg(feature = "kernel_tests")]
pub mod tests;
