//! Microkernel Inter-Process Communication
//!
//! Provides IPC primitives for communication between microkernel services.
//! This module is specifically designed for service-to-service communication
//! in the hybrid architecture and includes:
//! - Message queues with zero-copy support
//! - Service-to-service messaging
//!
//! **Note**: For POSIX-compliant IPC (shm, msg, sem), use `crate::ipc`.
//! For high-performance IPC services, use `crate::services::ipc`.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool, Ordering};
use crate::sync::Mutex;
use crate::reliability::errno::{EINVAL, ENOMEM, EAGAIN, EFAULT, EMSGSIZE, ENOENT};

/// IPC message structure
#[derive(Debug, Clone)]
pub struct IpcMessage {
    pub id: u64,
    pub sender_id: u64,
    pub receiver_id: u64,
    pub message_type: u32,
    pub priority: u8,
    pub data: Vec<u8>,
    pub timestamp: u64,
    /// Zero-copy: Reference to shared memory region instead of copying data
    pub zero_copy_ref: Option<ZeroCopyRef>,
}

/// Zero-copy reference to shared memory
#[derive(Debug, Clone)]
pub struct ZeroCopyRef {
    pub shm_id: u64,
    pub offset: usize,
    pub length: usize,
}

impl ZeroCopyRef {
    /// Get the virtual address of the shared memory region for a process
    /// Returns the base virtual address if the shared memory is attached to the process
    pub fn get_virtual_address(&self, process_id: u64) -> Result<usize, i32> {
        let ipc_manager = get_ipc_manager().ok_or(EFAULT)?;
        
        // Check if shared memory exists
        let _shm = ipc_manager.get_shared_memory(self.shm_id)
            .ok_or(ENOENT)?;

        // Get process pagetable
        let pagetable = {
            let mut proc_table = crate::process::PROC_TABLE.lock();
            let proc = proc_table.find(process_id as crate::process::Pid)
                .ok_or(EFAULT)?;
            proc.pagetable
        };
        if pagetable.is_null() {
            return Err(EFAULT);
        }
        
        // Try to find the virtual address by translating the physical address
        // This is a simplified implementation - in a real system, we would
        // track virtual address mappings for shared memory
        let shm = ipc_manager.get_shared_memory(self.shm_id)
            .ok_or(ENOENT)?;
        
        // For now, return the physical address as virtual address
        // In a real implementation, we would maintain a mapping table
        Ok(shm.paddr.0 + self.offset)
    }
    
    /// Get a pointer to the data in shared memory (unsafe)
    /// The caller must ensure:
    /// 1. The shared memory is attached to the current process
    /// 2. The memory is not accessed after it's detached
    /// 3. The memory is not accessed concurrently without synchronization
    pub unsafe fn get_data_ptr(&self, process_id: u64) -> Result<*const u8, i32> {
        let va = self.get_virtual_address(process_id)?;
        Ok(va as *const u8)
    }
    
    /// Get a mutable pointer to the data in shared memory (unsafe)
    /// Same safety requirements as get_data_ptr
    pub unsafe fn get_data_ptr_mut(&self, process_id: u64) -> Result<*mut u8, i32> {
        let va = self.get_virtual_address(process_id)?;
        Ok(va as *mut u8)
    }
    
    /// Copy data from shared memory to a buffer
    /// This is a safe wrapper that copies the data
    pub fn copy_to_buffer(&self, process_id: u64, buffer: &mut [u8]) -> Result<usize, i32> {
        if buffer.len() < self.length {
            return Err(EMSGSIZE);
        }
        
        unsafe {
            let src_ptr = self.get_data_ptr(process_id)?;
            let src_slice = core::slice::from_raw_parts(src_ptr, self.length);
            buffer[..self.length].copy_from_slice(src_slice);
        }
        
        Ok(self.length)
    }
    
    /// Copy data from a buffer to shared memory
    /// This is a safe wrapper that copies the data
    pub fn copy_from_buffer(&self, process_id: u64, buffer: &[u8]) -> Result<usize, i32> {
        if buffer.len() > self.length {
            return Err(EMSGSIZE);
        }
        
        unsafe {
            let dst_ptr = self.get_data_ptr_mut(process_id)?;
            let dst_slice = core::slice::from_raw_parts_mut(dst_ptr, self.length);
            dst_slice[..buffer.len()].copy_from_slice(buffer);
        }
        
        Ok(buffer.len())
    }
}

impl IpcMessage {
    pub fn new(sender_id: u64, receiver_id: u64, message_type: u32, data: Vec<u8>) -> Self {
        static NEXT_MESSAGE_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: NEXT_MESSAGE_ID.fetch_add(1, Ordering::SeqCst),
            sender_id,
            receiver_id,
            message_type,
            priority: 0,
            data,
            timestamp: get_current_time(),
            zero_copy_ref: None,
        }
    }

    /// Create a zero-copy message that references shared memory
    pub fn new_zero_copy(
        sender_id: u64,
        receiver_id: u64,
        message_type: u32,
        shm_id: u64,
        offset: usize,
        length: usize,
    ) -> Self {
        static NEXT_MESSAGE_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: NEXT_MESSAGE_ID.fetch_add(1, Ordering::SeqCst),
            sender_id,
            receiver_id,
            message_type,
            priority: 0,
            data: Vec::new(), // Empty for zero-copy messages
            timestamp: get_current_time(),
            zero_copy_ref: Some(ZeroCopyRef {
                shm_id,
                offset,
                length,
            }),
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn size(&self) -> usize {
        if let Some(ref zc_ref) = self.zero_copy_ref {
            core::mem::size_of::<Self>() + zc_ref.length
        } else {
            core::mem::size_of::<Self>() + self.data.len()
        }
    }

    /// Check if this is a zero-copy message
    pub fn is_zero_copy(&self) -> bool {
        self.zero_copy_ref.is_some()
    }
}

/// Message queue
pub struct MessageQueue {
    pub id: u64,
    pub owner_id: u64,
    pub messages: Mutex<Vec<IpcMessage>>,
    pub max_messages: usize,
    pub max_message_size: usize,
    pub current_count: AtomicUsize,
    pub flags: u32, // Queue flags (e.g., blocking, non-blocking)
    /// Zero-copy: Shared memory region for message data
    pub zero_copy_shm: Option<u64>, // Shared memory ID for zero-copy messages
}

impl MessageQueue {
    pub fn new(id: u64, owner_id: u64, max_messages: usize, max_message_size: usize) -> Self {
        Self {
            id,
            owner_id,
            messages: Mutex::new(Vec::new()),
            max_messages,
            max_message_size,
            current_count: AtomicUsize::new(0),
            flags: 0,
            zero_copy_shm: None,
        }
    }

    /// Create a message queue with zero-copy support
    pub fn new_with_zero_copy(
        id: u64,
        owner_id: u64,
        max_messages: usize,
        max_message_size: usize,
        shm_id: u64,
    ) -> Self {
        Self {
            id,
            owner_id,
            messages: Mutex::new(Vec::new()),
            max_messages,
            max_message_size,
            current_count: AtomicUsize::new(0),
            flags: 0,
            zero_copy_shm: Some(shm_id),
        }
    }

    pub fn send(&self, message: IpcMessage) -> Result<(), i32> {
        // Check queue capacity
        if self.current_count.load(Ordering::SeqCst) >= self.max_messages {
            return Err(EAGAIN);
        }

        // Check message size
        if message.size() > self.max_message_size {
            return Err(EMSGSIZE);
        }

        // For zero-copy messages, verify shared memory exists and is valid
        if let Some(ref zc_ref) = message.zero_copy_ref {
            // Verify shared memory region exists
            let ipc_manager = get_ipc_manager().ok_or(EFAULT)?;
            let shm = ipc_manager.get_shared_memory(zc_ref.shm_id)
                .ok_or(ENOENT)?;
            
            // Verify offset and length are within shared memory bounds
            if zc_ref.offset + zc_ref.length > shm.size {
                return Err(EMSGSIZE);
            }
            
            // Verify message size constraint
            if zc_ref.length > self.max_message_size {
                return Err(EMSGSIZE);
            }
        }

        // Insert message in priority order (higher priority first)
        let mut messages = self.messages.lock();

        // Find insertion point based on priority
        let insert_pos = messages.iter().position(|m| m.priority < message.priority)
            .unwrap_or(messages.len());

        messages.insert(insert_pos, message);
        self.current_count.fetch_add(1, Ordering::SeqCst);

        // Update statistics
        super::MICROKERNEL_STATS.ipc_messages.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// Send a zero-copy message (data stays in shared memory)
    pub fn send_zero_copy(
        &self,
        sender_id: u64,
        receiver_id: u64,
        message_type: u32,
        shm_id: u64,
        offset: usize,
        length: usize,
    ) -> Result<(), i32> {
        if self.zero_copy_shm.is_none() {
            return Err(EINVAL); // Zero-copy not enabled for this queue
        }

        let message = IpcMessage::new_zero_copy(sender_id, receiver_id, message_type, shm_id, offset, length);
        self.send(message)
    }

    pub fn receive(&self, receiver_id: u64) -> Result<IpcMessage, i32> {
        let mut messages = self.messages.lock();

        // Find first message for this receiver
        let pos = messages.iter().position(|m| m.receiver_id == receiver_id || m.receiver_id == 0);

        if let Some(index) = pos {
            let message = messages.remove(index);
            self.current_count.fetch_sub(1, Ordering::SeqCst);
            Ok(message)
        } else {
            Err(EAGAIN)
        }
    }

    /// Receive a zero-copy message and get reference to shared memory
    pub fn receive_zero_copy(&self, receiver_id: u64) -> Result<ZeroCopyRef, i32> {
        let message = self.receive(receiver_id)?;
        
        if let Some(zc_ref) = message.zero_copy_ref {
            Ok(zc_ref)
        } else {
            Err(EINVAL) // Not a zero-copy message
        }
    }

    pub fn peek(&self, receiver_id: u64) -> Option<IpcMessage> {
        let messages = self.messages.lock();

        messages.iter()
            .find(|m| m.receiver_id == receiver_id || m.receiver_id == 0)
            .cloned()
    }

    pub fn count(&self) -> usize {
        self.current_count.load(Ordering::SeqCst)
    }

    pub fn is_full(&self) -> bool {
        self.current_count.load(Ordering::SeqCst) >= self.max_messages
    }

    pub fn is_empty(&self) -> bool {
        self.current_count.load(Ordering::SeqCst) == 0
    }
}

/// Shared memory region
#[derive(Debug)]
pub struct SharedMemoryRegion {
    pub id: u64,
    pub owner_id: u64,
    pub size: usize,
    pub paddr: crate::mm::phys::PhysAddr,
    pub ref_count: AtomicUsize,
    pub permissions: u32, // Read/Write/Execute permissions
}

impl Clone for SharedMemoryRegion {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            owner_id: self.owner_id,
            size: self.size,
            paddr: self.paddr,
            ref_count: AtomicUsize::new(self.ref_count.load(Ordering::SeqCst)),
            permissions: self.permissions,
        }
    }
}

impl SharedMemoryRegion {
    pub fn new(id: u64, owner_id: u64, size: usize, paddr: crate::mm::phys::PhysAddr) -> Self {
        Self {
            id,
            owner_id,
            size,
            paddr,
            ref_count: AtomicUsize::new(1),
            permissions: 0o666, // Default read/write for owner and group
        }
    }

    pub fn inc_ref(&self) -> usize {
        self.ref_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn dec_ref(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::SeqCst) - 1
    }

    pub fn get_ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
    }
}

/// Semaphore for IPC synchronization
pub struct IpcSemaphore {
    pub id: u64,
    pub owner_id: u64,
    pub value: AtomicUsize,
    pub max_value: usize,
    pub waiting_queue: Mutex<Vec<u64>>, // Queue of waiting process IDs
}

impl IpcSemaphore {
    pub fn new(id: u64, owner_id: u64, initial_value: usize, max_value: usize) -> Self {
        Self {
            id,
            owner_id,
            value: AtomicUsize::new(initial_value),
            max_value,
            waiting_queue: Mutex::new(Vec::new()),
        }
    }

    pub fn wait(&self, process_id: u64) -> Result<(), i32> {
        loop {
            let current_value = self.value.load(Ordering::SeqCst);

            if current_value > 0 {
                // Try to decrement
                match self.value.compare_exchange_weak(
                    current_value,
                    current_value - 1,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return Ok(()), // Successfully acquired
                    Err(_) => continue, // Value changed, retry
                }
            } else {
                // Add to waiting queue
                let mut queue = self.waiting_queue.lock();
                if !queue.contains(&process_id) {
                    queue.push(process_id);
                }
                return Err(EAGAIN); // Would block
            }
        }
    }

    pub fn signal(&self) -> Result<(), i32> {
        // Wake up one waiting process
        let mut queue = self.waiting_queue.lock();

        if let Some(process_id) = queue.pop() {
            // Wake up the waiting process (implementation-specific)
            self.wake_process(process_id)?;
        } else {
            // No waiting processes, increment semaphore value
            let current_value = self.value.load(Ordering::SeqCst);
            if current_value < self.max_value {
                self.value.store(current_value + 1, Ordering::SeqCst);
            } else {
                return Err(EAGAIN); // Semaphore would overflow
            }
        }

        Ok(())
    }

    fn wake_process(&self, process_id: u64) -> Result<(), i32> {
        // In a real implementation, this would wake up the specified process
        // For now, just return success
        Ok(())
    }

    pub fn get_value(&self) -> usize {
        self.value.load(Ordering::SeqCst)
    }
}

/// Event channel for async notifications
pub struct EventChannel {
    pub id: u64,
    pub owner_id: u64,
    pub subscribers: Mutex<BTreeMap<u64, u32>>, // subscriber_id -> event_mask
    pub pending_events: Mutex<Vec<IpcEvent>>,
}

#[derive(Debug, Clone)]
pub struct IpcEvent {
    pub event_type: u32,
    pub source_id: u64,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl EventChannel {
    pub fn new(id: u64, owner_id: u64) -> Self {
        Self {
            id,
            owner_id,
            subscribers: Mutex::new(BTreeMap::new()),
            pending_events: Mutex::new(Vec::new()),
        }
    }

    pub fn subscribe(&self, subscriber_id: u64, event_mask: u32) -> Result<(), i32> {
        let mut subscribers = self.subscribers.lock();
        subscribers.insert(subscriber_id, event_mask);
        Ok(())
    }

    pub fn unsubscribe(&self, subscriber_id: u64) -> Result<(), i32> {
        let mut subscribers = self.subscribers.lock();
        subscribers.remove(&subscriber_id);
        Ok(())
    }

    pub fn publish(&self, event_type: u32, source_id: u64, data: Vec<u8>) -> Result<(), i32> {
        let event = IpcEvent {
            event_type,
            source_id,
            data,
            timestamp: get_current_time(),
        };

        let mut pending = self.pending_events.lock();
        pending.push(event);

        Ok(())
    }

    pub fn get_events(&self, subscriber_id: u64) -> Vec<IpcEvent> {
        let subscribers = self.subscribers.lock();
        let event_mask = subscribers.get(&subscriber_id).copied().unwrap_or(u32::MAX);

        let mut pending = self.pending_events.lock();
        let mut result = Vec::new();

        pending.retain(|event| {
            if event.event_type & event_mask != 0 {
                result.push(event.clone());
                false // Remove from pending
            } else {
                true // Keep in pending
            }
        });

        result
    }
}

/// IPC manager
pub struct IpcManager {
    pub message_queues: Mutex<BTreeMap<u64, MessageQueue>>,
    pub shared_memory: Mutex<BTreeMap<u64, SharedMemoryRegion>>,
    pub semaphores: Mutex<BTreeMap<u64, IpcSemaphore>>,
    pub event_channels: Mutex<BTreeMap<u64, EventChannel>>,
    pub next_queue_id: AtomicU64,
    pub next_shm_id: AtomicU64,
    pub next_sem_id: AtomicU64,
    pub next_channel_id: AtomicU64,
}

impl IpcManager {
    pub fn new() -> Self {
        Self {
            message_queues: Mutex::new(BTreeMap::new()),
            shared_memory: Mutex::new(BTreeMap::new()),
            semaphores: Mutex::new(BTreeMap::new()),
            event_channels: Mutex::new(BTreeMap::new()),
            next_queue_id: AtomicU64::new(1),
            next_shm_id: AtomicU64::new(1),
            next_sem_id: AtomicU64::new(1),
            next_channel_id: AtomicU64::new(1),
        }
    }

    pub fn create_message_queue(&self, owner_id: u64, max_messages: usize, max_message_size: usize) -> Result<u64, i32> {
        let id = self.next_queue_id.fetch_add(1, Ordering::SeqCst);
        let queue = MessageQueue::new(id, owner_id, max_messages, max_message_size);

        let mut queues = self.message_queues.lock();
        queues.insert(id, queue);

        Ok(id)
    }

    /// Create a message queue with zero-copy support
    pub fn create_zero_copy_message_queue(
        &self,
        owner_id: u64,
        max_messages: usize,
        max_message_size: usize,
    ) -> Result<u64, i32> {
        // Create shared memory region for zero-copy messages
        let shm_size = max_messages * max_message_size;
        let shm_id = self.create_shared_memory(owner_id, shm_size)?;

        let id = self.next_queue_id.fetch_add(1, Ordering::SeqCst);
        let queue = MessageQueue::new_with_zero_copy(id, owner_id, max_messages, max_message_size, shm_id);

        let mut queues = self.message_queues.lock();
        queues.insert(id, queue);

        Ok(id)
    }

    pub fn destroy_message_queue(&self, queue_id: u64) -> Result<(), i32> {
        let mut queues = self.message_queues.lock();
        queues.remove(&queue_id).ok_or(ENOENT)?;
        Ok(())
    }

    pub fn get_message_queue(&self, _queue_id: u64) -> Option<MessageQueue> {
        None
    }

    pub fn send_message(&self, queue_id: u64, message: IpcMessage) -> Result<(), i32> {
        let queues = self.message_queues.lock();
        let queue = queues.get(&queue_id).ok_or(ENOENT)?;
        queue.send(message)
    }

    pub fn receive_message(&self, queue_id: u64, receiver_id: u64) -> Result<IpcMessage, i32> {
        let queues = self.message_queues.lock();
        let queue = queues.get(&queue_id).ok_or(ENOENT)?;
        queue.receive(receiver_id)
    }

    /// Send a zero-copy message
    pub fn send_zero_copy_message(
        &self,
        queue_id: u64,
        sender_id: u64,
        receiver_id: u64,
        message_type: u32,
        shm_id: u64,
        offset: usize,
        length: usize,
    ) -> Result<(), i32> {
        let queues = self.message_queues.lock();
        let queue = queues.get(&queue_id).ok_or(ENOENT)?;
        queue.send_zero_copy(sender_id, receiver_id, message_type, shm_id, offset, length)
    }

    /// Receive a zero-copy message
    pub fn receive_zero_copy_message(&self, queue_id: u64, receiver_id: u64) -> Result<ZeroCopyRef, i32> {
        let queues = self.message_queues.lock();
        let queue = queues.get(&queue_id).ok_or(ENOENT)?;
        queue.receive_zero_copy(receiver_id)
    }

    /// Get virtual address for zero-copy shared memory access
    /// This attaches the shared memory to the process if not already attached
    pub fn get_zero_copy_address(
        &self,
        zc_ref: &ZeroCopyRef,
        process_id: u64,
    ) -> Result<usize, i32> {
        // Try to attach shared memory if not already attached
        // In a real implementation, we would check if already attached
        let va = self.attach_shared_memory(zc_ref.shm_id, process_id, None)?;
        Ok(va + zc_ref.offset)
    }

    /// Release zero-copy shared memory reference
    /// This detaches the shared memory from the process when no longer needed
    pub fn release_zero_copy(
        &self,
        zc_ref: &ZeroCopyRef,
        process_id: u64,
        va: usize,
    ) -> Result<(), i32> {
        // Detach shared memory
        self.detach_shared_memory(zc_ref.shm_id, process_id, va)
    }

    pub fn create_shared_memory(&self, owner_id: u64, size: usize) -> Result<u64, i32> {
        // Allocate physical memory
        let memory_manager = super::memory::get_memory_manager()
            .ok_or(EFAULT)?;

        let paddr_usize = memory_manager.allocate_physical_page()?; // For simplicity, allocate one page
        let paddr = crate::mm::phys::PhysAddr::new(paddr_usize);

        let id = self.next_shm_id.fetch_add(1, Ordering::SeqCst);
        let shm = SharedMemoryRegion::new(id, owner_id, size, paddr);

        let mut shared_mem = self.shared_memory.lock();
        shared_mem.insert(id, shm);

        Ok(id)
    }

    pub fn destroy_shared_memory(&self, shm_id: u64) -> Result<(), i32> {
        let mut shared_mem = self.shared_memory.lock();

        if let Some(shm) = shared_mem.remove(&shm_id) {
            // Free physical memory
            if let Some(memory_manager) = super::memory::get_memory_manager() {
                let _ = memory_manager.free_physical_page(shm.paddr.0);
            }
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    pub fn get_shared_memory(&self, shm_id: u64) -> Option<SharedMemoryRegion> {
        let shared_mem = self.shared_memory.lock();
        shared_mem.get(&shm_id).cloned()
    }

    /// Attach shared memory to a process's address space
    pub fn attach_shared_memory(
        &self,
        shm_id: u64,
        process_id: u64,
        addr: Option<usize>,
    ) -> Result<usize, i32> {
        let shared_mem = self.shared_memory.lock();
        let shm = shared_mem.get(&shm_id).ok_or(ENOENT)?;
        
        // Increment reference count
        shm.inc_ref();

        // Get process pagetable
        let pagetable = {
            let mut proc_table = crate::process::PROC_TABLE.lock();
            let proc = proc_table.find(process_id as crate::process::Pid)
                .ok_or(EFAULT)?;
            proc.pagetable
        };
        if pagetable.is_null() {
            return Err(EFAULT);
        }
        
        // Find virtual address (use provided or find free range)
        let va = match addr {
            Some(a) => a,
            None => {
                // Find free virtual address range
                crate::mm::vm::find_free_range(shm.size)
                    .ok_or(ENOMEM)?
            }
        };
        
        // Map physical pages to virtual address space
        let paddr = shm.paddr.0;
        let perm = crate::mm::vm::flags::PTE_U | crate::mm::vm::flags::PTE_R | crate::mm::vm::flags::PTE_W;
        
        unsafe {
            crate::mm::vm::map_pages(pagetable, va, paddr, shm.size, perm)
                .map_err(|_| EFAULT)?;
        }
        
        Ok(va)
    }

    /// Detach shared memory from a process's address space
    pub fn detach_shared_memory(
        &self,
        shm_id: u64,
        process_id: u64,
        addr: usize,
    ) -> Result<(), i32> {
        let shared_mem = self.shared_memory.lock();
        let shm = shared_mem.get(&shm_id).ok_or(ENOENT)?;

        // Get process pagetable
        let pagetable = {
            let mut proc_table = crate::process::PROC_TABLE.lock();
            let proc = proc_table.find(process_id as crate::process::Pid)
                .ok_or(EFAULT)?;
            proc.pagetable
        };
        if pagetable.is_null() {
            return Err(EFAULT);
        }
        
        // Unmap pages
        unsafe {
            let mut current = addr;
            let end = addr + shm.size;
            while current < end {
                crate::mm::vm::unmap_page(pagetable, current)
                    .map_err(|_| EFAULT)?;
                current += crate::mm::PAGE_SIZE;
            }
        }
        
        // Decrement reference count
        let ref_count = shm.dec_ref();
        
        // If no more references and marked for removal, destroy it
        if ref_count == 0 {
            drop(shared_mem);
            let _ = self.destroy_shared_memory(shm_id);
        }
        
        Ok(())
    }

    pub fn create_semaphore(&self, owner_id: u64, initial_value: usize, max_value: usize) -> Result<u64, i32> {
        let id = self.next_sem_id.fetch_add(1, Ordering::SeqCst);
        let semaphore = IpcSemaphore::new(id, owner_id, initial_value, max_value);

        let mut semaphores = self.semaphores.lock();
        semaphores.insert(id, semaphore);

        Ok(id)
    }

    pub fn destroy_semaphore(&self, sem_id: u64) -> Result<(), i32> {
        let mut semaphores = self.semaphores.lock();
        semaphores.remove(&sem_id).ok_or(ENOENT)?;
        Ok(())
    }

    pub fn get_semaphore(&self, _sem_id: u64) -> Option<IpcSemaphore> {
        None
    }

    pub fn create_event_channel(&self, owner_id: u64) -> Result<u64, i32> {
        let id = self.next_channel_id.fetch_add(1, Ordering::SeqCst);
        let channel = EventChannel::new(id, owner_id);

        let mut channels = self.event_channels.lock();
        channels.insert(id, channel);

        Ok(id)
    }

    pub fn get_event_channel(&self, _channel_id: u64) -> Option<EventChannel> {
        None
    }
}

/// Global IPC manager
static mut GLOBAL_IPC_MANAGER: Option<IpcManager> = None;
static IPC_INIT: AtomicBool = AtomicBool::new(false);

/// Initialize IPC subsystem
pub fn init() -> Result<(), i32> {
    if IPC_INIT.load(Ordering::SeqCst) {
        return Ok(());
    }

    let manager = IpcManager::new();

    unsafe {
        GLOBAL_IPC_MANAGER = Some(manager);
    }

    IPC_INIT.store(true, Ordering::SeqCst);
    Ok(())
}

/// Get global IPC manager
pub fn get_ipc_manager() -> Option<&'static IpcManager> {
    unsafe {
        GLOBAL_IPC_MANAGER.as_ref()
    }
}

/// Get mutable global IPC manager
pub fn get_ipc_manager_mut() -> Option<&'static mut IpcManager> {
    unsafe {
        GLOBAL_IPC_MANAGER.as_mut()
    }
}

/// Get current time in nanoseconds
fn get_current_time() -> u64 {
    crate::time::get_time_ns()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message() {
        let data = vec![1, 2, 3, 4];
        let message = IpcMessage::new(1, 2, 100, data.clone())
            .with_priority(5);

        assert_eq!(message.sender_id, 1);
        assert_eq!(message.receiver_id, 2);
        assert_eq!(message.message_type, 100);
        assert_eq!(message.priority, 5);
        assert_eq!(message.data, data);
    }

    #[test]
    fn test_message_queue() {
        let queue = MessageQueue::new(1, 1, 10, 1024);

        let message1 = IpcMessage::new(1, 2, 100, vec![1, 2, 3]);
        let message2 = IpcMessage::new(1, 3, 101, vec![4, 5, 6]).with_priority(5);

        assert_eq!(queue.count(), 0);
        assert!(queue.is_empty());

        assert_eq!(queue.send(message1.clone()), Ok(()));
        assert_eq!(queue.send(message2), Ok(()));

        assert_eq!(queue.count(), 2);
        assert!(!queue.is_empty());

        // Higher priority message should be received first
        let received = queue.receive(3).unwrap();
        assert_eq!(received.message_type, 101);
        assert_eq!(received.priority, 5);

        let received = queue.receive(2).unwrap();
        assert_eq!(received.message_type, 100);
        assert_eq!(received.priority, 0);
    }

    #[test]
    fn test_ipc_semaphore() {
        let semaphore = IpcSemaphore::new(1, 1, 2, 5);

        assert_eq!(semaphore.get_value(), 2);

        // Wait should succeed twice
        assert_eq!(semaphore.wait(100), Ok(()));
        assert_eq!(semaphore.get_value(), 1);

        assert_eq!(semaphore.wait(101), Ok(()));
        assert_eq!(semaphore.get_value(), 0);

        // Third wait should fail (would block)
        assert_eq!(semaphore.wait(102), Err(EAGAIN));
        assert_eq!(semaphore.get_value(), 0);

        // Signal should wake up one waiter
        assert_eq!(semaphore.signal(), Ok(()));
        assert_eq!(semaphore.get_value(), 0); // Still 0 because waiter was woken up
    }

    #[test]
    fn test_shared_memory_region() {
        let paddr = crate::mm::vm::PhysAddr::new(0x1000);
        let shm = SharedMemoryRegion::new(1, 1, 4096, paddr);

        assert_eq!(shm.id, 1);
        assert_eq!(shm.owner_id, 1);
        assert_eq!(shm.size, 4096);
        assert_eq!(shm.paddr, paddr);
        assert_eq!(shm.get_ref_count(), 1);

        assert_eq!(shm.inc_ref(), 2);
        assert_eq!(shm.get_ref_count(), 2);

        assert_eq!(shm.dec_ref(), 1);
        assert_eq!(shm.get_ref_count(), 1);
    }

    #[test]
    fn test_event_channel() {
        let channel = EventChannel::new(1, 1);

        assert_eq!(channel.subscribe(100, 0x1), Ok(()));
        assert_eq!(channel.subscribe(101, 0x2), Ok(()));

        assert_eq!(channel.publish(0x1, 1, vec![1, 2, 3]), Ok(()));
        assert_eq!(channel.publish(0x2, 2, vec![4, 5, 6]), Ok(()));

        // Subscriber 100 should only get event type 0x1
        let events = channel.get_events(100);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, 0x1);

        // Subscriber 101 should only get event type 0x2
        let events = channel.get_events(101);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, 0x2);
    }
}
