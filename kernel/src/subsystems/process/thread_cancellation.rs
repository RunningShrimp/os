//! Enhanced Thread Cancellation Mechanism
//! 
//! This module provides POSIX-compliant thread cancellation with proper
//! cancellation points, cleanup handlers, and deferred/asynchronous cancellation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

use super::thread::{Tid, ThreadError, CancelState, CancelType};

/// Cancellation point types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationPointType {
    /// System call entry
    SyscallEntry,
    /// System call exit
    SyscallExit,
    /// Blocking operation (sleep, wait, etc.)
    BlockingOperation,
    /// Thread join
    ThreadJoin,
    /// Condition variable wait
    CondvarWait,
    /// Mutex lock
    MutexLock,
    /// File I/O operation
    FileIO,
    /// Memory allocation
    MemoryAllocation,
    /// User-defined cancellation point
    UserDefined,
}

/// Cancellation cleanup handler
#[derive(Debug)]
pub struct CancellationCleanupHandler {
    /// Handler function
    pub handler: fn(*mut u8),
    /// Argument to pass to handler
    pub arg: *mut u8,
    /// Handler ID for removal
    pub handler_id: u64,
    /// Thread ID this handler belongs to
    pub thread_id: Tid,
}

/// Cancellation request information
#[derive(Debug, Clone)]
pub struct CancellationRequest {
    /// Target thread ID
    pub target_tid: Tid,
    /// Requesting thread ID
    pub requester_tid: Tid,
    /// Cancellation type
    pub cancel_type: CancelType,
    /// Request timestamp
    pub timestamp: u64,
    /// Whether request has been delivered
    pub delivered: bool,
}

/// Cancellation statistics
#[derive(Debug, Default, Clone)]
pub struct CancellationStats {
    /// Total cancellation requests
    pub total_requests: u64,
    /// Total successful cancellations
    pub total_successful: u64,
    /// Total deferred cancellations
    pub total_deferred: u64,
    /// Total asynchronous cancellations
    pub total_asynchronous: u64,
    /// Total cleanup handlers executed
    pub total_cleanup_handlers: u64,
    /// Average cancellation latency in microseconds
    pub avg_cancellation_latency_us: f64,
    /// Maximum cancellation latency in microseconds
    pub max_cancellation_latency_us: u64,
}

/// Thread cancellation state
#[derive(Debug)]
pub struct ThreadCancellationState {
    /// Thread ID
    pub thread_id: Tid,
    /// Current cancellation state
    pub cancel_state: CancelState,
    /// Current cancellation type
    pub cancel_type: CancelType,
    /// Whether cancellation is pending
    pub pending: AtomicBool,
    /// Cancellation request information
    pub cancellation_request: Option<CancellationRequest>,
    /// Cleanup handlers stack
    pub cleanup_handlers: Mutex<Vec<CancellationCleanupHandler>>,
    /// Cancellation points stack
    pub cancellation_points: Mutex<Vec<CancellationPointType>>,
    /// Whether thread is in a cancellation-unsafe region
    pub in_unsafe_region: AtomicBool,
    /// Last cleanup handler ID
    pub next_handler_id: AtomicU64,
    /// Cancellation statistics
    pub stats: Mutex<CancellationStats>,
}

impl ThreadCancellationState {
    /// Create a new thread cancellation state
    pub fn new(thread_id: Tid) -> Self {
        Self {
            thread_id,
            cancel_state: CancelState::Enabled,
            cancel_type: CancelType::Deferred,
            pending: AtomicBool::new(false),
            cancellation_request: None,
            cleanup_handlers: Mutex::new(Vec::new()),
            cancellation_points: Mutex::new(Vec::new()),
            in_unsafe_region: AtomicBool::new(false),
            next_handler_id: AtomicU64::new(1),
            stats: Mutex::new(CancellationStats::default()),
        }
    }

    /// Set cancellation state
    pub fn set_cancel_state(&mut self, state: CancelState, old_state: &mut CancelState) -> Result<(), ThreadError> {
        *old_state = self.cancel_state;
        self.cancel_state = state;
        Ok(())
    }

    /// Set cancellation type
    pub fn set_cancel_type(&mut self, cancel_type: CancelType) -> Result<(), ThreadError> {
        self.cancel_type = cancel_type;
        Ok(())
    }

    /// Test for cancellation without blocking
    pub fn test_cancel(&self) -> bool {
        if self.cancel_state == CancelState::Disabled {
            return false;
        }

        if self.in_unsafe_region.load(Ordering::Relaxed) {
            return false;
        }

        self.pending.load(Ordering::Relaxed)
    }

    /// Test for cancellation and exit if pending
    pub fn test_cancel_exit(&self) -> ! {
        if self.test_cancel() {
            self.handle_cancellation();
        }
    }

    /// Enter a cancellation point
    pub fn enter_cancellation_point(&self, point_type: CancellationPointType) {
        self.cancellation_points.lock().push(point_type);

        // Check for cancellation
        if self.test_cancel() {
            self.handle_cancellation();
        }
    }

    /// Exit a cancellation point
    pub fn exit_cancellation_point(&self, point_type: CancellationPointType) {
        let mut points = self.cancellation_points.lock();
        if let Some(pos) = points.iter().rposition(|&p| p == point_type) {
            points.remove(pos);
        }
    }

    /// Enter cancellation-unsafe region
    pub fn enter_unsafe_region(&self) {
        self.in_unsafe_region.store(true, Ordering::Relaxed);
    }

    /// Exit cancellation-unsafe region
    pub fn exit_unsafe_region(&self) {
        self.in_unsafe_region.store(false, Ordering::Relaxed);

        // Check for cancellation after exiting unsafe region
        if self.test_cancel() {
            self.handle_cancellation();
        }
    }

    /// Push a cleanup handler
    pub fn push_cleanup_handler(&self, handler: fn(*mut u8), arg: *mut u8) -> u64 {
        let handler_id = self.next_handler_id.fetch_add(1, Ordering::SeqCst);
        let cleanup_handler = CancellationCleanupHandler {
            handler,
            arg,
            handler_id,
            thread_id: self.thread_id,
        };

        self.cleanup_handlers.lock().push(cleanup_handler);
        handler_id
    }

    /// Pop a cleanup handler
    pub fn pop_cleanup_handler(&self, handler_id: u64) -> Option<CancellationCleanupHandler> {
        let mut handlers = self.cleanup_handlers.lock();
        if let Some(pos) = handlers.iter().position(|h| h.handler_id == handler_id) {
            let handler = handlers.remove(pos);
            
            // Update statistics
            let mut stats = self.stats.lock();
            stats.total_cleanup_handlers += 1;
            
            Some(handler)
        } else {
            None
        }
    }

    /// Request cancellation of this thread
    pub fn request_cancellation(&mut self, request: CancellationRequest) -> Result<(), ThreadError> {
        if self.cancel_state == CancelState::Disabled {
            return Err(ThreadError::OperationNotPermitted);
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_requests += 1;
        }

        self.cancellation_request = Some(request.clone());
        self.pending.store(true, Ordering::Relaxed);

        // Handle asynchronous cancellation immediately
        if self.cancel_type == CancelType::Asynchronous && !self.in_unsafe_region.load(Ordering::Relaxed) {
            self.handle_cancellation();
        } else {
            // Update statistics for deferred cancellation
            let mut stats = self.stats.lock();
            stats.total_deferred += 1;
        }

        Ok(())
    }

    /// Handle cancellation by executing cleanup handlers and exiting
    fn handle_cancellation(&self) -> ! {
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_successful += 1;
            
            if self.cancel_type == CancelType::Asynchronous {
                stats.total_asynchronous += 1;
            }
            
            // Update latency statistics
            if let Some(ref request) = self.cancellation_request {
                let now = crate::subsystems::time::timestamp_nanos();
                let latency_us = (now - request.timestamp) / 1000;
                stats.avg_cancellation_latency_us = 
                    (stats.avg_cancellation_latency_us * (stats.total_successful - 1) as f64 + latency_us as f64) 
                    / stats.total_successful as f64;
                stats.max_cancellation_latency_us = stats.max_cancellation_latency_us.max(latency_us);
            }
        }

        // Execute cleanup handlers in LIFO order
        let handlers = {
            let mut handler_list = self.cleanup_handlers.lock();
            core::mem::take(&mut *handler_list)
        };

        for handler in handlers.iter().rev() {
            // Execute cleanup handler
            (handler.handler)(handler.arg);
        }

        // Exit thread with cancellation status
        super::thread_exit(crate::libc::PTHREAD_CANCELED as *mut u8);
    }

    /// Get cancellation statistics
    pub fn get_stats(&self) -> CancellationStats {
        self.stats.lock().clone()
    }

    /// Reset cancellation statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = CancellationStats::default();
    }
}

/// Global thread cancellation state manager
pub struct ThreadCancellationManager {
    /// Thread cancellation states by TID
    states: Mutex<BTreeMap<Tid, ThreadCancellationState>>,
    /// Global cancellation statistics
    global_stats: Mutex<CancellationStats>,
    /// Next cancellation request ID
    next_request_id: AtomicU64,
}

impl ThreadCancellationManager {
    /// Create a new thread cancellation manager
    pub fn new() -> Self {
        Self {
            states: Mutex::new(BTreeMap::new()),
            global_stats: Mutex::new(CancellationStats::default()),
            next_request_id: AtomicU64::new(1),
        }
    }

    /// Register a thread for cancellation management
    pub fn register_thread(&self, thread_id: Tid) -> Result<(), ThreadError> {
        let mut states = self.states.lock();
        if states.contains_key(&thread_id) {
            return Err(ThreadError::InvalidThreadId);
        }

        let state = ThreadCancellationState::new(thread_id);
        states.insert(thread_id, state);
        Ok(())
    }

    /// Unregister a thread from cancellation management
    pub fn unregister_thread(&self, thread_id: Tid) -> Result<(), ThreadError> {
        let mut states = self.states.lock();
        if !states.contains_key(&thread_id) {
            return Err(ThreadError::InvalidThreadId);
        }

        states.remove(&thread_id);
        Ok(())
    }

    /// Get cancellation state for a thread
    pub fn get_state(&self, thread_id: Tid) -> Option<&ThreadCancellationState> {
        let states = self.states.lock();
        states.get(&thread_id)
    }

    /// Get mutable cancellation state for a thread
    pub fn get_state_mut(&self, thread_id: Tid) -> Option<&mut ThreadCancellationState> {
        let mut states = self.states.lock();
        states.get_mut(&thread_id)
    }

    /// Cancel a thread
    pub fn cancel_thread(&self, target_tid: Tid, requester_tid: Tid) -> Result<(), ThreadError> {
        let request = CancellationRequest {
            target_tid,
            requester_tid,
            cancel_type: CancelType::Deferred, // Default to deferred
            timestamp: crate::subsystems::time::timestamp_nanos(),
            delivered: false,
        };

        let mut states = self.states.lock();
        if let Some(state) = states.get_mut(&target_tid) {
            state.request_cancellation(request)
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Cancel a thread with specific cancellation type
    pub fn cancel_thread_with_type(
        &self,
        target_tid: Tid,
        requester_tid: Tid,
        cancel_type: CancelType,
    ) -> Result<(), ThreadError> {
        let request = CancellationRequest {
            target_tid,
            requester_tid,
            cancel_type,
            timestamp: crate::subsystems::time::timestamp_nanos(),
            delivered: false,
        };

        let mut states = self.states.lock();
        if let Some(state) = states.get_mut(&target_tid) {
            state.request_cancellation(request)
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Set cancellation state for a thread
    pub fn set_cancel_state(
        &self,
        thread_id: Tid,
        state: CancelState,
        old_state: &mut CancelState,
    ) -> Result<(), ThreadError> {
        let mut states = self.states.lock();
        if let Some(cancel_state) = states.get_mut(&thread_id) {
            cancel_state.set_cancel_state(state, old_state)
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Set cancellation type for a thread
    pub fn set_cancel_type(
        &self,
        thread_id: Tid,
        cancel_type: CancelType,
    ) -> Result<(), ThreadError> {
        let mut states = self.states.lock();
        if let Some(state) = states.get_mut(&thread_id) {
            state.set_cancel_type(cancel_type)
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Test for cancellation on a thread
    pub fn test_cancel(&self, thread_id: Tid) -> Result<bool, ThreadError> {
        let states = self.states.lock();
        if let Some(state) = states.get(&thread_id) {
            Ok(state.test_cancel())
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Enter cancellation point for a thread
    pub fn enter_cancellation_point(
        &self,
        thread_id: Tid,
        point_type: CancellationPointType,
    ) -> Result<(), ThreadError> {
        let states = self.states.lock();
        if let Some(state) = states.get(&thread_id) {
            state.enter_cancellation_point(point_type);
            Ok(())
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Exit cancellation point for a thread
    pub fn exit_cancellation_point(
        &self,
        thread_id: Tid,
        point_type: CancellationPointType,
    ) -> Result<(), ThreadError> {
        let states = self.states.lock();
        if let Some(state) = states.get(&thread_id) {
            state.exit_cancellation_point(point_type);
            Ok(())
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Enter cancellation-unsafe region for a thread
    pub fn enter_unsafe_region(&self, thread_id: Tid) -> Result<(), ThreadError> {
        let states = self.states.lock();
        if let Some(state) = states.get(&thread_id) {
            state.enter_unsafe_region();
            Ok(())
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Exit cancellation-unsafe region for a thread
    pub fn exit_unsafe_region(&self, thread_id: Tid) -> Result<(), ThreadError> {
        let states = self.states.lock();
        if let Some(state) = states.get(&thread_id) {
            state.exit_unsafe_region();
            Ok(())
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Push cleanup handler for a thread
    pub fn push_cleanup_handler(
        &self,
        thread_id: Tid,
        handler: fn(*mut u8),
        arg: *mut u8,
    ) -> Result<u64, ThreadError> {
        let states = self.states.lock();
        if let Some(state) = states.get(&thread_id) {
            Ok(state.push_cleanup_handler(handler, arg))
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Pop cleanup handler for a thread
    pub fn pop_cleanup_handler(
        &self,
        thread_id: Tid,
        handler_id: u64,
    ) -> Result<Option<CancellationCleanupHandler>, ThreadError> {
        let states = self.states.lock();
        if let Some(state) = states.get(&thread_id) {
            Ok(state.pop_cleanup_handler(handler_id))
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Get cancellation statistics for a thread
    pub fn get_thread_stats(&self, thread_id: Tid) -> Result<CancellationStats, ThreadError> {
        let states = self.states.lock();
        if let Some(state) = states.get(&thread_id) {
            Ok(state.get_stats())
        } else {
            Err(ThreadError::InvalidThreadId)
        }
    }

    /// Get global cancellation statistics
    pub fn get_global_stats(&self) -> CancellationStats {
        self.global_stats.lock().clone()
    }

    /// Reset global cancellation statistics
    pub fn reset_global_stats(&self) {
        *self.global_stats.lock() = CancellationStats::default();
    }
}

/// Global thread cancellation manager instance
static mut CANCELLATION_MANAGER: Option<ThreadCancellationManager> = None;
static CANCELLATION_MANAGER_INIT: spin::Once = spin::Once::new();

/// Get the global thread cancellation manager
pub fn get_cancellation_manager() -> &'static ThreadCancellationManager {
    unsafe {
        CANCELLATION_MANAGER_INIT.call_once(|| {
            CANCELLATION_MANAGER = Some(ThreadCancellationManager::new());
        });
        CANCELLATION_MANAGER.as_ref().unwrap()
    }
}

/// Initialize thread cancellation for a thread
pub fn init_thread_cancellation(thread_id: Tid) -> Result<(), ThreadError> {
    let manager = get_cancellation_manager();
    manager.register_thread(thread_id)
}

/// Cleanup thread cancellation for a thread
pub fn cleanup_thread_cancellation(thread_id: Tid) -> Result<(), ThreadError> {
    let manager = get_cancellation_manager();
    manager.unregister_thread(thread_id)
}

/// Cancel a thread
pub fn cancel_thread(target_tid: Tid) -> Result<(), ThreadError> {
    let requester_tid = super::thread::current_thread().unwrap_or(0);
    let manager = get_cancellation_manager();
    manager.cancel_thread(target_tid, requester_tid)
}

/// Cancel a thread with specific cancellation type
pub fn cancel_thread_with_type(
    target_tid: Tid,
    cancel_type: CancelType,
) -> Result<(), ThreadError> {
    let requester_tid = super::thread::current_thread().unwrap_or(0);
    let manager = get_cancellation_manager();
    manager.cancel_thread_with_type(target_tid, requester_tid, cancel_type)
}

/// Set cancellation state for current thread
pub fn set_cancel_state(state: CancelState, old_state: &mut CancelState) -> Result<(), ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.set_cancel_state(thread_id, state, old_state)
}

/// Set cancellation type for current thread
pub fn set_cancel_type(cancel_type: CancelType) -> Result<(), ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.set_cancel_type(thread_id, cancel_type)
}

/// Test for cancellation on current thread
pub fn test_cancel() -> Result<bool, ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.test_cancel(thread_id)
}

/// Test for cancellation and exit if pending
pub fn test_cancel_exit() -> Result<(), ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    
    if manager.test_cancel(thread_id)? {
        // This will not return
        if let Some(state) = manager.get_state(thread_id) {
            state.handle_cancellation();
        }
    }
    
    Ok(())
}

/// Enter cancellation point for current thread
pub fn enter_cancellation_point(point_type: CancellationPointType) -> Result<(), ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.enter_cancellation_point(thread_id, point_type)
}

/// Exit cancellation point for current thread
pub fn exit_cancellation_point(point_type: CancellationPointType) -> Result<(), ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.exit_cancellation_point(thread_id, point_type)
}

/// Enter cancellation-unsafe region for current thread
pub fn enter_unsafe_region() -> Result<(), ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.enter_unsafe_region(thread_id)
}

/// Exit cancellation-unsafe region for current thread
pub fn exit_unsafe_region() -> Result<(), ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.exit_unsafe_region(thread_id)
}

/// Push cleanup handler for current thread
pub fn push_cleanup_handler(handler: fn(*mut u8), arg: *mut u8) -> Result<u64, ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.push_cleanup_handler(thread_id, handler, arg)
}

/// Pop cleanup handler for current thread
pub fn pop_cleanup_handler(handler_id: u64) -> Result<Option<CancellationCleanupHandler>, ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.pop_cleanup_handler(thread_id, handler_id)
}

/// Get cancellation statistics for current thread
pub fn get_thread_stats() -> Result<CancellationStats, ThreadError> {
    let thread_id = super::thread::current_thread().ok_or(ThreadError::InvalidThreadId)?;
    let manager = get_cancellation_manager();
    manager.get_thread_stats(thread_id)
}

/// Get global cancellation statistics
pub fn get_global_stats() -> CancellationStats {
    let manager = get_cancellation_manager();
    manager.get_global_stats()
}

/// Reset global cancellation statistics
pub fn reset_global_stats() {
    let manager = get_cancellation_manager();
    manager.reset_global_stats();
}

/// Macro for creating cancellation points
#[macro_export]
macro_rules! cancellation_point {
    () => {
        if let Err(_) = $crate::subsystems::process::thread_cancellation::enter_cancellation_point(
            $crate::subsystems::process::thread_cancellation::CancellationPointType::UserDefined
        ) {
            // Handle error
        }
    };
    ($point_type:expr) => {
        if let Err(_) = $crate::subsystems::process::thread_cancellation::enter_cancellation_point($point_type) {
            // Handle error
        }
    };
}

/// Macro for creating cancellation-unsafe regions
#[macro_export]
macro_rules! cancellation_unsafe_region {
    ($body:block) => {
        if let Err(_) = $crate::subsystems::process::thread_cancellation::enter_unsafe_region() {
            // Handle error
        } else {
            $body
        }
        if let Err(_) = $crate::subsystems::process::thread_cancellation::exit_unsafe_region() {
            // Handle error
        }
    };
}

/// Macro for creating cleanup handlers
#[macro_export]
macro_rules! cleanup_handler {
    ($handler:expr, $arg:expr) => {
        if let Ok(handler_id) = $crate::subsystems::process::thread_cancellation::push_cleanup_handler($handler, $arg) {
            // Use handler_id for later removal
            handler_id
        } else {
            // Handle error
            0
        }
    };
}