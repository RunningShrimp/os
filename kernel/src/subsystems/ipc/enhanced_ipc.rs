//! Enhanced Inter-Process Communication (IPC) Mechanism
//!
//! This module provides a comprehensive IPC system with support for:
//! - Message passing with priority and routing
//! - Shared memory with access control
//! - Synchronization primitives (semaphores, mutexes, condition variables)
//! - Event notification system
//! - Remote procedure calls (RPC)
//! - POSIX-compatible IPC primitives

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::sync::Mutex;

// ============================================================================
// Constants
// ============================================================================

pub const ENHANCED_IPC_MAX_MSG_SIZE: usize = 8192;
pub const ENHANCED_IPC_MAX_QUEUE: usize = 256;
pub const ENHANCED_IPC_MAX_SHM_SIZE: usize = 64 * 1024 * 1024; // 64MB
pub const ENHANCED_IPC_MAX_CONNECTIONS: usize = 128;
pub const ENHANCED_IPC_MAX_ENDPOINTS: usize = 256;

// Message priorities
pub const MSG_PRIORITY_LOW: u8 = 0;
pub const MSG_PRIORITY_NORMAL: u8 = 1;
pub const MSG_PRIORITY_HIGH: u8 = 2;
pub const MSG_PRIORITY_CRITICAL: u8 = 3;

// Message flags
pub const MSG_FLAG_NONBLOCK: u32 = 0x01;
pub const MSG_FLAG_BROADCAST: u32 = 0x02;
pub const MSG_FLAG_URGENT: u32 = 0x04;
pub const MSG_FLAG_REPLY: u32 = 0x08;
pub const MSG_FLAG_RPC: u32 = 0x10;

// Access permissions
pub const SHM_PERM_READ: u32 = 0x01;
pub const SHM_PERM_WRITE: u32 = 0x02;
pub const SHM_PERM_EXEC: u32 = 0x04;
pub const SHM_PERM_SHARED: u32 = 0x08;

// ============================================================================
// Enhanced Types
// ============================================================================

/// Enhanced IPC Message with priority and routing
#[derive(Debug, Clone)]
pub struct EnhancedIpcMessage {
    /// Unique message ID
    pub msg_id: u64,
    /// Message type (application-defined)
    pub msg_type: u32,
    /// Source process ID
    pub src_pid: u32,
    /// Destination process ID (0 for broadcast)
    pub dst_pid: u32,
    /// Message priority
    pub priority: u8,
    /// Message flags
    pub flags: u32,
    /// Timestamp when message was sent
    pub timestamp: u64,
    /// Message data
    pub data: Vec<u8>,
    /// Reply-to message ID (for RPC)
    pub reply_to: Option<u64>,
    /// RPC timeout (in milliseconds)
    pub timeout: Option<u32>,
}

/// Enhanced Message Queue with priority support
pub struct EnhancedMessageQueue {
    /// Queue ID
    pub queue_id: u32,
    /// Messages organized by priority
    pub messages: Mutex<BTreeMap<u8, Vec<EnhancedIpcMessage>>>,
    /// Maximum number of messages
    pub max_size: usize,
    /// Current number of messages
    pub current_size: AtomicU32,
    /// Waiting processes
    pub waiting: Mutex<Vec<u32>>,
    /// Queue flags
    pub flags: u32,
}

/// Enhanced Shared Memory with access control
pub struct EnhancedSharedMemory {
    /// Shared memory ID
    pub shm_id: u32,
    /// Base address
    pub base_addr: usize,
    /// Size in bytes
    pub size: usize,
    /// Access permissions
    pub permissions: u32,
    /// Owner process ID
    pub owner_pid: u32,
    /// Reference count
    pub ref_count: AtomicU32,
    /// Access control list
    pub acl: Mutex<Vec<ShmAclEntry>>,
    /// Creation timestamp
    pub created_at: u64,
}

/// Shared Memory ACL Entry
#[derive(Debug, Clone)]
pub struct ShmAclEntry {
    /// Process ID
    pub pid: u32,
    /// Permissions (read/write/execute)
    pub permissions: u32,
}

/// Semaphore for synchronization
pub struct IpcSemaphore {
    /// Semaphore ID
    pub sem_id: u32,
    /// Current value
    pub value: AtomicU32,
    /// Maximum value
    pub max_value: u32,
    /// Waiting processes
    pub waiting: Mutex<Vec<u32>>,
    /// Creation timestamp
    pub created_at: u64,
}

/// Mutex for mutual exclusion
pub struct IpcMutex {
    /// Mutex ID
    pub mutex_id: u32,
    /// Owner process ID (0 if unlocked)
    pub owner_pid: AtomicU32,
    /// Lock count (for recursive locks)
    pub lock_count: AtomicU32,
    /// Waiting processes
    pub waiting: Mutex<Vec<u32>>,
    /// Creation timestamp
    pub created_at: u64,
}

/// Condition variable for synchronization
pub struct IpcCondition {
    /// Condition ID
    pub cond_id: u32,
    /// Waiting processes
    pub waiting: Mutex<Vec<u32>>,
    /// Associated mutex ID
    pub mutex_id: Option<u32>,
    /// Creation timestamp
    pub created_at: u64,
}

/// Event notification
pub struct IpcEvent {
    /// Event ID
    pub event_id: u32,
    /// Event type
    pub event_type: u32,
    /// Event data
    pub data: Vec<u8>,
    /// Source process ID
    pub src_pid: u32,
    /// Timestamp
    pub timestamp: u64,
    /// Processes waiting for this event
    pub waiting: Mutex<Vec<u32>>,
}

/// RPC endpoint for remote procedure calls
pub struct RpcEndpoint {
    /// Endpoint ID
    pub endpoint_id: u32,
    /// Endpoint name
    pub name: String,
    /// Owner process ID
    pub owner_pid: u32,
    /// Supported procedures
    pub procedures: Mutex<Vec<RpcProcedure>>,
    /// Active calls
    pub active_calls: Mutex<BTreeMap<u64, RpcCall>>,
    /// Next call ID
    pub next_call_id: AtomicU64,
}

/// RPC procedure definition
#[derive(Debug, Clone)]
pub struct RpcProcedure {
    /// Procedure name
    pub name: String,
    /// Procedure ID
    pub proc_id: u32,
    /// Input parameter types
    pub input_types: Vec<u32>,
    /// Output parameter types
    pub output_types: Vec<u32>,
}

/// Active RPC call
#[derive(Debug)]
pub struct RpcCall {
    /// Call ID
    pub call_id: u64,
    /// Procedure ID
    pub proc_id: u32,
    /// Caller process ID
    pub caller_pid: u32,
    /// Call arguments
    pub args: Vec<u8>,
    /// Call timestamp
    pub timestamp: u64,
    /// Timeout (in milliseconds)
    pub timeout: Option<u32>,
    /// Response data
    pub response: Option<Vec<u8>>,
    /// Call completed flag
    pub completed: bool,
}

/// IPC connection between processes
pub struct IpcConnection {
    /// Connection ID
    pub connection_id: u32,
    /// Local endpoint ID
    pub local_endpoint: u32,
    /// Remote endpoint ID
    pub remote_endpoint: u32,
    /// Remote process ID
    pub remote_pid: u32,
    /// Connection state
    pub state: ConnectionState,
    /// Connection type
    pub conn_type: ConnectionType,
    /// Connection flags
    pub flags: u32,
    /// Statistics
    pub stats: Mutex<ConnectionStats>,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Error,
}

/// Connection type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Stream,     // Reliable, ordered delivery
    Datagram,   // Unreliable, unordered delivery
    Raw,        // Raw packet delivery
    Reliable,   // Reliable but unordered delivery
}

/// Connection statistics
#[derive(Debug, Default, Clone)]
pub struct ConnectionStats {
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Connection errors
    pub errors: u64,
    /// Last activity timestamp
    pub last_activity: u64,
}

/// IPC endpoint for receiving messages
pub struct IpcEndpoint {
    /// Endpoint ID
    pub endpoint_id: u32,
    /// Endpoint name
    pub name: String,
    /// Owner process ID
    pub owner_pid: u32,
    /// Endpoint type
    pub endpoint_type: EndpointType,
    /// Message queue for this endpoint
    pub message_queue: EnhancedMessageQueue,
    /// Connected endpoints
    pub connections: Mutex<Vec<u32>>,
    /// Endpoint flags
    pub flags: u32,
}

/// Endpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    Unicast,    // Point-to-point communication
    Multicast,  // One-to-many communication
    Broadcast,  // One-to-all communication
    Request,    // Request-response pattern
    Publish,    // Publish-subscribe pattern
}

// ============================================================================
// Global State
// ============================================================================

static NEXT_MSG_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_QUEUE_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_SHM_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_SEM_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_MUTEX_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_COND_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_EVENT_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_ENDPOINT_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_CONNECTION_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_RPC_ENDPOINT_ID: AtomicU32 = AtomicU32::new(1);

static MESSAGE_QUEUES: Mutex<BTreeMap<u32, EnhancedMessageQueue>> = Mutex::new(BTreeMap::new());
static SHARED_MEMORIES: Mutex<BTreeMap<u32, EnhancedSharedMemory>> = Mutex::new(BTreeMap::new());
static SEMAPHORES: Mutex<BTreeMap<u32, IpcSemaphore>> = Mutex::new(BTreeMap::new());
static MUTEXES: Mutex<BTreeMap<u32, IpcMutex>> = Mutex::new(BTreeMap::new());
static CONDITIONS: Mutex<BTreeMap<u32, IpcCondition>> = Mutex::new(BTreeMap::new());
static EVENTS: Mutex<BTreeMap<u32, IpcEvent>> = Mutex::new(BTreeMap::new());
static ENDPOINTS: Mutex<BTreeMap<u32, IpcEndpoint>> = Mutex::new(BTreeMap::new());
static CONNECTIONS: Mutex<BTreeMap<u32, IpcConnection>> = Mutex::new(BTreeMap::new());
static RPC_ENDPOINTS: Mutex<BTreeMap<u32, RpcEndpoint>> = Mutex::new(BTreeMap::new());

// ============================================================================
// IPC Errors
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    /// No message available
    NoMessage,
    /// Queue is full
    QueueFull,
    /// Invalid message size
    InvalidMessageSize,
    /// Invalid queue ID
    InvalidQueueId,
    /// Invalid shared memory ID
    InvalidShmId,
    /// Invalid semaphore ID
    InvalidSemId,
    /// Invalid mutex ID
    InvalidMutexId,
    /// Invalid condition ID
    InvalidCondId,
    /// Invalid event ID
    InvalidEventId,
    /// Invalid endpoint ID
    InvalidEndpointId,
    /// Invalid connection ID
    InvalidConnectionId,
    /// Not owner of resource
    NotOwner,
    /// Resource not found
    ResourceNotFound,
    /// Connection not found
    ConnectionNotFound,
    /// Not connected
    NotConnected,
    /// Invalid state
    InvalidState,
    /// Semaphore overflow
    SemaphoreOverflow,
    /// No waiting process
    NoWaitingProcess,
    /// Procedure already exists
    ProcedureExists,
    /// Procedure not found
    ProcedureNotFound,
    /// RPC call not found
    CallNotFound,
    /// RPC call not completed
    CallNotCompleted,
    /// Timeout
    Timeout,
    /// Permission denied
    PermissionDenied,
    /// System error
    SystemError,
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Initialize enhanced IPC subsystem
pub fn init() {
    crate::println!("enhanced_ipc: initialized");
}

/// Create enhanced message queue
pub fn create_message_queue(max_size: usize) -> Result<u32, IpcError> {
    if max_size == 0 || max_size > ENHANCED_IPC_MAX_QUEUE {
        return Err(IpcError::InvalidMessageSize);
    }
    
    let queue_id = NEXT_QUEUE_ID.fetch_add(1, Ordering::SeqCst);
    let queue = EnhancedMessageQueue::new(queue_id, max_size);
    
    let mut queues = MESSAGE_QUEUES.lock();
    queues.insert(queue_id, queue);
    
    Ok(queue_id)
}

/// Send message to queue
pub fn send_message(queue_id: u32, msg: EnhancedIpcMessage) -> Result<(), IpcError> {
    let queues = MESSAGE_QUEUES.lock();
    let queue = queues.get(&queue_id).ok_or(IpcError::InvalidQueueId)?;
    queue.send(msg)
}

/// Receive message from queue
pub fn receive_message(queue_id: u32, timeout: Option<u32>) -> Result<EnhancedIpcMessage, IpcError> {
    let queues = MESSAGE_QUEUES.lock();
    let queue = queues.get(&queue_id).ok_or(IpcError::InvalidQueueId)?;
    queue.recv(timeout)
}

/// Create enhanced shared memory
pub fn create_shared_memory(size: usize, owner_pid: u32, permissions: u32) -> Result<u32, IpcError> {
    if size == 0 || size > ENHANCED_IPC_MAX_SHM_SIZE {
        return Err(IpcError::InvalidMessageSize);
    }
    
    // Allocate memory for shared region
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
            return Err(IpcError::SystemError);
        }
        
        // Zero page
        unsafe { core::ptr::write_bytes(page, 0, page_size); }
        
        if i == 0 {
            base_addr = page as usize;
        }
    }
    
    let shm_id = NEXT_SHM_ID.fetch_add(1, Ordering::SeqCst);
    let shm = EnhancedSharedMemory::new(shm_id, base_addr, aligned_size, owner_pid, permissions);
    
    let mut shms = SHARED_MEMORIES.lock();
    shms.insert(shm_id, shm);
    
    Ok(shm_id)
}

/// Attach to shared memory
pub fn attach_shared_memory(shm_id: u32, pid: u32) -> Result<usize, IpcError> {
    let mut shms = SHARED_MEMORIES.lock();
    let shm = shms.get_mut(&shm_id).ok_or(IpcError::InvalidShmId)?;
    
    // Check permissions
    if !shm.has_access(pid, SHM_PERM_READ) {
        return Err(IpcError::PermissionDenied);
    }
    
    shm.inc_ref();
    Ok(shm.base_addr)
}

/// Detach from shared memory
pub fn detach_shared_memory(shm_id: u32, pid: u32) -> Result<(), IpcError> {
    let mut shms = SHARED_MEMORIES.lock();
    let shm = shms.get_mut(&shm_id).ok_or(IpcError::InvalidShmId)?;
    
    shm.dec_ref();
    Ok(())
}

/// Delete shared memory
pub fn delete_shared_memory(shm_id: u32, pid: u32) -> Result<(), IpcError> {
    let mut shms = SHARED_MEMORIES.lock();
    let shm = shms.get(&shm_id).ok_or(IpcError::InvalidShmId)?;
    
    // Check if owner
    if shm.owner_pid != pid {
        return Err(IpcError::PermissionDenied);
    }
    
    // Check if no processes are attached
    if shm.ref_count() > 1 {
        return Err(IpcError::SystemError);
    }
    
    // Free memory
    let page_size = crate::mm::PAGE_SIZE;
    let pages = shm.size / page_size;
    for i in 0..pages {
        let addr = shm.base_addr + i * page_size;
        unsafe {
            crate::mm::kfree(addr as *mut u8);
        }
    }
    
    // Remove from list
    shms.remove(&shm_id);
    
    Ok(())
}

/// Create semaphore
pub fn create_semaphore(initial_value: u32, max_value: u32) -> Result<u32, IpcError> {
    let sem_id = NEXT_SEM_ID.fetch_add(1, Ordering::SeqCst);
    let semaphore = IpcSemaphore::new(sem_id, initial_value, max_value);
    
    let mut semaphores = SEMAPHORES.lock();
    semaphores.insert(sem_id, semaphore);
    
    Ok(sem_id)
}

/// Wait on semaphore
pub fn semaphore_wait(sem_id: u32, timeout: Option<u32>) -> Result<(), IpcError> {
    let semaphores = SEMAPHORES.lock();
    let semaphore = semaphores.get(&sem_id).ok_or(IpcError::InvalidSemId)?;
    semaphore.wait(timeout)
}

/// Signal semaphore
pub fn semaphore_signal(sem_id: u32) -> Result<(), IpcError> {
    let semaphores = SEMAPHORES.lock();
    let semaphore = semaphores.get(&sem_id).ok_or(IpcError::InvalidSemId)?;
    semaphore.signal()
}

/// Create mutex
pub fn create_mutex() -> Result<u32, IpcError> {
    let mutex_id = NEXT_MUTEX_ID.fetch_add(1, Ordering::SeqCst);
    let mutex = IpcMutex::new(mutex_id);
    
    let mut mutexes = MUTEXES.lock();
    mutexes.insert(mutex_id, mutex);
    
    Ok(mutex_id)
}

/// Lock mutex
pub fn mutex_lock(mutex_id: u32, timeout: Option<u32>) -> Result<(), IpcError> {
    let mutexes = MUTEXES.lock();
    let mutex = mutexes.get(&mutex_id).ok_or(IpcError::InvalidMutexId)?;
    mutex.lock(timeout)
}

/// Unlock mutex
pub fn mutex_unlock(mutex_id: u32) -> Result<(), IpcError> {
    let mutexes = MUTEXES.lock();
    let mutex = mutexes.get(&mutex_id).ok_or(IpcError::InvalidMutexId)?;
    mutex.unlock()
}

/// Create condition variable
pub fn create_condition(mutex_id: Option<u32>) -> Result<u32, IpcError> {
    let cond_id = NEXT_COND_ID.fetch_add(1, Ordering::SeqCst);
    let condition = IpcCondition::new(cond_id, mutex_id);
    
    let mut conditions = CONDITIONS.lock();
    conditions.insert(cond_id, condition);
    
    Ok(cond_id)
}

/// Wait on condition variable
pub fn condition_wait(cond_id: u32, timeout: Option<u32>) -> Result<(), IpcError> {
    let conditions = CONDITIONS.lock();
    let condition = conditions.get(&cond_id).ok_or(IpcError::InvalidCondId)?;
    condition.wait(timeout)
}

/// Signal condition variable
pub fn condition_signal(cond_id: u32) -> Result<(), IpcError> {
    let conditions = CONDITIONS.lock();
    let condition = conditions.get(&cond_id).ok_or(IpcError::InvalidCondId)?;
    condition.signal()
}

/// Broadcast condition variable
pub fn condition_broadcast(cond_id: u32) -> Result<(), IpcError> {
    let conditions = CONDITIONS.lock();
    let condition = conditions.get(&cond_id).ok_or(IpcError::InvalidCondId)?;
    condition.broadcast()
}

/// Create event
pub fn create_event(event_type: u32, src_pid: u32, data: &[u8]) -> Result<u32, IpcError> {
    let event_id = NEXT_EVENT_ID.fetch_add(1, Ordering::SeqCst);
    let event = IpcEvent::new(event_id, event_type, src_pid, data);
    
    let mut events = EVENTS.lock();
    events.insert(event_id, event);
    
    Ok(event_id)
}

/// Wait for event
pub fn event_wait(event_id: u32, timeout: Option<u32>) -> Result<(), IpcError> {
    let events = EVENTS.lock();
    let event = events.get(&event_id).ok_or(IpcError::InvalidEventId)?;
    event.wait(timeout)
}

/// Trigger event
pub fn event_trigger(event_id: u32) -> Result<(), IpcError> {
    let events = EVENTS.lock();
    let event = events.get(&event_id).ok_or(IpcError::InvalidEventId)?;
    event.trigger()
}

/// Create RPC endpoint
pub fn create_rpc_endpoint(name: String, owner_pid: u32) -> Result<u32, IpcError> {
    let endpoint_id = NEXT_RPC_ENDPOINT_ID.fetch_add(1, Ordering::SeqCst);
    let endpoint = RpcEndpoint::new(endpoint_id, name, owner_pid);
    
    let mut endpoints = RPC_ENDPOINTS.lock();
    endpoints.insert(endpoint_id, endpoint);
    
    Ok(endpoint_id)
}

/// Register RPC procedure
pub fn register_rpc_procedure(endpoint_id: u32, proc: RpcProcedure) -> Result<(), IpcError> {
    let endpoints = RPC_ENDPOINTS.lock();
    let endpoint = endpoints.get(&endpoint_id).ok_or(IpcError::InvalidEndpointId)?;
    endpoint.register_procedure(proc)
}

/// Make RPC call
pub fn make_rpc_call(endpoint_id: u32, proc_name: &str, args: &[u8], timeout: Option<u32>) -> Result<u64, IpcError> {
    let endpoints = RPC_ENDPOINTS.lock();
    let endpoint = endpoints.get(&endpoint_id).ok_or(IpcError::InvalidEndpointId)?;
    endpoint.call(proc_name, args, timeout)
}

/// Complete RPC call
pub fn complete_rpc_call(endpoint_id: u32, call_id: u64, response: &[u8]) -> Result<(), IpcError> {
    let endpoints = RPC_ENDPOINTS.lock();
    let endpoint = endpoints.get(&endpoint_id).ok_or(IpcError::InvalidEndpointId)?;
    endpoint.complete_call(call_id, response)
}

/// Get RPC call result
pub fn get_rpc_result(endpoint_id: u32, call_id: u64) -> Result<Vec<u8>, IpcError> {
    let endpoints = RPC_ENDPOINTS.lock();
    let endpoint = endpoints.get(&endpoint_id).ok_or(IpcError::InvalidEndpointId)?;
    endpoint.get_result(call_id)
}