//! Advanced POSIX Signal Handling
//!
//! This module implements advanced POSIX signal handling features including:
//! - Queued signal delivery (sigqueue)
//! - Synchronous signal waiting (sigtimedwait, sigwaitinfo)
//! - Alternate signal stack management (sigaltstack)
//! - Thread signal mask management (pthread_sigmask)
//! - Real-time signal support (SIGRTMIN-SIGRTMAX)

use crate::posix::{SigSet, SigInfoT, SigVal, Pid, Uid, SIGRTMIN, SIGRTMAX};
use crate::process::{Pid as ProcessId};
use crate::sync::Mutex;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::microkernel::scheduler;

/// Maximum number of pending signals per process
pub const MAX_PENDING_SIGNALS: usize = 64;

/// Signal queue entry for queued signals
#[derive(Clone)]
pub struct QueuedSignal {
    /// Signal information
    pub info: SigInfoT,
    /// Queue timestamp (nanoseconds since boot)
    pub timestamp: u64,
    /// Whether this signal has been delivered
    pub delivered: bool,
}

impl core::fmt::Debug for QueuedSignal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("QueuedSignal")
            .field("timestamp", &self.timestamp)
            .field("delivered", &self.delivered)
            .finish()
    }
}

impl QueuedSignal {
    /// Create a new queued signal
    pub fn new(info: SigInfoT) -> Self {
        Self {
            info,
            timestamp: crate::time::get_timestamp(),
            delivered: false,
        }
    }

    /// Create a queued signal from sigqueue parameters
    pub fn from_sigqueue(sig: i32, pid: Pid, uid: Uid, value: SigVal) -> Self {
        let info = SigInfoT {
            si_signo: sig,
            si_code: crate::posix::SI_QUEUE,
            si_pid: pid,
            si_uid: uid,
            si_status: 0,
            si_utime: 0,
            si_stime: 0,
            si_value: value,
            si_timerid: 0,
            si_overrun: 0,
            si_addr: 0,
            si_band: 0,
            si_fd: -1,
        };
        Self::new(info)
    }
}

/// Per-process signal queue
pub struct SignalQueue {
    /// Queue of pending signals
    pending: Mutex<VecDeque<QueuedSignal>>,
    /// Signal mask for blocked signals
    sigmask: Mutex<SigSet>,
    /// Number of signals in queue
    count: AtomicUsize,
}

impl core::fmt::Debug for SignalQueue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SignalQueue")
            .field("count", &self.count)
            .finish()
    }
}

impl SignalQueue {
    /// Create a new signal queue
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(VecDeque::with_capacity(MAX_PENDING_SIGNALS)),
            sigmask: Mutex::new(SigSet::empty()),
            count: AtomicUsize::new(0),
        }
    }

    /// Add a signal to the queue
    pub fn enqueue(&self, signal: QueuedSignal) -> Result<(), SignalQueueError> {
        // Check if queue is full
        if self.count.load(Ordering::Acquire) >= MAX_PENDING_SIGNALS {
            return Err(SignalQueueError::QueueFull);
        }

        // Check if signal is blocked
        {
            let sigmask = self.sigmask.lock();
            if sigmask.has(signal.info.si_signo) {
                return Err(SignalQueueError::SignalBlocked);
            }
        }

        // Add to queue
        {
            let mut pending = self.pending.lock();
            pending.push_back(signal);
        }
        
        self.count.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    /// Remove and return the next signal from the queue
    pub fn dequeue(&self, sigmask: Option<&SigSet>) -> Option<QueuedSignal> {
        let mut pending = self.pending.lock();
        
        // Find first signal not blocked by mask
        if let Some(mask) = sigmask {
            let mut index = 0;
            while index < pending.len() {
                if !mask.has(pending[index].info.si_signo) {
                    let signal = pending.remove(index).unwrap();
                    self.count.fetch_sub(1, Ordering::AcqRel);
                    return Some(signal);
                }
                index += 1;
            }
            None
        } else {
            // No mask, return first signal
            let signal = pending.pop_front()?;
            self.count.fetch_sub(1, Ordering::AcqRel);
            Some(signal)
        }
    }

    /// Check if there are pending signals matching the mask
    pub fn has_pending(&self, sigmask: Option<&SigSet>) -> bool {
        let pending = self.pending.lock();
        
        if let Some(mask) = sigmask {
            pending.iter().any(|s| !mask.has(s.info.si_signo))
        } else {
            !pending.is_empty()
        }
    }

    /// Get the number of pending signals
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Set the signal mask
    pub fn set_sigmask(&self, mask: SigSet) {
        *self.sigmask.lock() = mask;
    }

    /// Get the current signal mask
    pub fn get_sigmask(&self) -> SigSet {
        *self.sigmask.lock()
    }

    /// Clear all pending signals
    pub fn clear(&self) {
        let mut pending = self.pending.lock();
        pending.clear();
        self.count.store(0, Ordering::Release);
    }

    /// Get statistics about the signal queue
    pub fn get_stats(&self) -> SignalQueueStats {
        let pending = self.pending.lock();
        let mut real_time_count = 0;
        let mut standard_count = 0;
        
        for signal in pending.iter() {
            if signal.info.si_signo >= SIGRTMIN && signal.info.si_signo <= SIGRTMAX {
                real_time_count += 1;
            } else {
                standard_count += 1;
            }
        }

        SignalQueueStats {
            total_pending: pending.len(),
            real_time_pending: real_time_count,
            standard_pending: standard_count,
            max_capacity: MAX_PENDING_SIGNALS,
        }
    }
}

/// Signal queue statistics
#[derive(Debug, Clone, Copy)]
pub struct SignalQueueStats {
    /// Total number of pending signals
    pub total_pending: usize,
    /// Number of pending real-time signals
    pub real_time_pending: usize,
    /// Number of pending standard signals
    pub standard_pending: usize,
    /// Maximum queue capacity
    pub max_capacity: usize,
}

/// Signal queue errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalQueueError {
    /// Signal queue is full
    QueueFull,
    /// Signal is blocked by mask
    SignalBlocked,
    /// Invalid signal number
    InvalidSignal,
    /// Process not found
    ProcessNotFound,
}

/// Alternate signal stack management
#[derive(Debug)]
pub struct AlternateSignalStack {
    /// Stack base address
    pub base: *mut u8,
    /// Stack size
    pub size: usize,
    /// Stack flags
    pub flags: i32,
    /// Whether the stack is currently in use
    pub in_use: bool,
}

impl AlternateSignalStack {
    /// Create a new alternate signal stack
    pub fn new(size: usize) -> Result<Self, SignalStackError> {
        if size < crate::posix::MINSIGSTKSZ {
            return Err(SignalStackError::StackTooSmall);
        }

        // Allocate stack memory
        let base = unsafe {
            alloc::alloc::alloc(
                alloc::alloc::Layout::from_size_align(size, 16)
                    .map_err(|_| SignalStackError::InvalidSize)?
            ) as *mut u8
        };

        if base.is_null() {
            return Err(SignalStackError::AllocationFailed);
        }

        Ok(Self {
            base,
            size,
            flags: 0,
            in_use: false,
        })
    }

    /// Get the stack as a StackT structure
    pub fn as_stackt(&self) -> crate::posix::StackT {
        crate::posix::StackT {
            ss_sp: self.base,
            ss_flags: if self.in_use { crate::posix::SS_ONSTACK } else { crate::posix::SS_DISABLE },
            ss_size: self.size,
        }
    }

    /// Mark the stack as in use
    pub fn set_in_use(&mut self, in_use: bool) {
        self.in_use = in_use;
    }

    /// Check if the stack is valid for the given address
    pub fn contains_address(&self, addr: usize) -> bool {
        let base = self.base as usize;
        addr >= base && addr < base + self.size
    }
}

impl Drop for AlternateSignalStack {
    fn drop(&mut self) {
        if !self.base.is_null() {
            unsafe {
                alloc::alloc::dealloc(
                    self.base,
                    alloc::alloc::Layout::from_size_align(self.size, 16).unwrap(),
                );
            }
        }
    }
}

/// Signal stack errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalStackError {
    /// Stack size is too small
    StackTooSmall,
    /// Invalid stack size
    InvalidSize,
    /// Memory allocation failed
    AllocationFailed,
    /// Stack is already in use
    StackInUse,
    /// No alternate stack configured
    NoAlternateStack,
}

/// Thread signal mask management
pub struct ThreadSignalMask {
    /// Current signal mask
    mask: Mutex<SigSet>,
    /// Pending signals for this thread
    pending: Mutex<VecDeque<QueuedSignal>>,
    /// Thread ID
    thread_id: ProcessId,
}

impl ThreadSignalMask {
    /// Create a new thread signal mask
    pub fn new(thread_id: ProcessId) -> Self {
        Self {
            mask: Mutex::new(SigSet::empty()),
            pending: Mutex::new(VecDeque::new()),
            thread_id,
        }
    }

    /// Set the signal mask
    pub fn set_mask(&self, how: i32, new_mask: &SigSet, old_mask: Option<&mut SigSet>) -> Result<(), SignalMaskError> {
        let mut current_mask = self.mask.lock();
        
        // Save old mask if requested
        if let Some(old) = old_mask {
            *old = *current_mask;
        }

        // Apply new mask based on how
        match how {
            crate::posix::SIG_BLOCK => {
                // Add signals to current mask
                current_mask.bits |= new_mask.bits;
            }
            crate::posix::SIG_UNBLOCK => {
                // Remove signals from current mask
                current_mask.bits &= !new_mask.bits;
            }
            crate::posix::SIG_SETMASK => {
                // Set mask to new mask
                current_mask.bits = new_mask.bits;
            }
            _ => return Err(SignalMaskError::InvalidHow),
        }

        Ok(())
    }

    /// Get the current signal mask
    pub fn get_mask(&self) -> SigSet {
        *self.mask.lock()
    }

    /// Check if a signal is blocked
    pub fn is_blocked(&self, sig: i32) -> bool {
        self.mask.lock().has(sig)
    }

    /// Add a pending signal for this thread
    pub fn add_pending(&self, signal: QueuedSignal) -> Result<(), SignalMaskError> {
        let mut pending = self.pending.lock();
        
        // Check if we have too many pending signals
        if pending.len() >= MAX_PENDING_SIGNALS {
            return Err(SignalMaskError::TooManyPending);
        }

        pending.push_back(signal);
        Ok(())
    }

    /// Get next pending signal
    pub fn get_pending(&self) -> Option<QueuedSignal> {
        let mut pending = self.pending.lock();
        pending.pop_front()
    }

    /// Check if there are pending signals
    pub fn has_pending(&self) -> bool {
        !self.pending.lock().is_empty()
    }

    /// Get the thread ID
    pub fn thread_id(&self) -> ProcessId {
        self.thread_id
    }
}

/// Signal mask errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalMaskError {
    /// Invalid how parameter
    InvalidHow,
    /// Too many pending signals
    TooManyPending,
    /// Invalid signal number
    InvalidSignal,
}

/// Global signal queue registry
pub static SIGNAL_QUEUE_REGISTRY: Mutex<SignalQueueRegistry> = Mutex::new(SignalQueueRegistry::new());

/// Signal queue registry for managing per-process signal queues
#[derive(Debug)]
pub struct SignalQueueRegistry {
    /// Map from process ID to signal queue
    queues: alloc::collections::BTreeMap<ProcessId, Arc<SignalQueue>>,
}

impl SignalQueueRegistry {
    /// Create a new signal queue registry
    pub const fn new() -> Self {
        Self {
            queues: alloc::collections::BTreeMap::new(),
        }
    }

    /// Get or create a signal queue for a process
    pub fn get_or_create_queue(&mut self, pid: ProcessId) -> Arc<SignalQueue> {
        if let Some(queue) = self.queues.get(&pid) {
            queue.clone()
        } else {
            let queue = Arc::new(SignalQueue::new());
            self.queues.insert(pid, queue.clone());
            queue
        }
    }

    /// Remove a signal queue for a process
    pub fn remove_queue(&mut self, pid: ProcessId) -> Option<Arc<SignalQueue>> {
        self.queues.remove(&pid)
    }

    /// Get a signal queue for a process
    pub fn get_queue(&self, pid: ProcessId) -> Option<Arc<SignalQueue>> {
        self.queues.get(&pid).cloned()
    }

    /// Get all signal queues
    pub fn get_all_queues(&self) -> impl Iterator<Item = (&ProcessId, &Arc<SignalQueue>)> {
        self.queues.iter()
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> SignalRegistryStats {
        let mut total_pending = 0;
        let mut total_real_time = 0;
        let mut total_standard = 0;
        
        for queue in self.queues.values() {
            let stats = queue.get_stats();
            total_pending += stats.total_pending;
            total_real_time += stats.real_time_pending;
            total_standard += stats.standard_pending;
        }

        SignalRegistryStats {
            total_processes: self.queues.len(),
            total_pending_signals: total_pending,
            total_real_time_signals: total_real_time,
            total_standard_signals: total_standard,
        }
    }
}

/// Signal registry statistics
#[derive(Debug, Clone, Copy)]
pub struct SignalRegistryStats {
    /// Total number of processes with signal queues
    pub total_processes: usize,
    /// Total number of pending signals across all processes
    pub total_pending_signals: usize,
    /// Total number of pending real-time signals
    pub total_real_time_signals: usize,
    /// Total number of pending standard signals
    pub total_standard_signals: usize,
}

/// Send a queued signal to a process (sigqueue implementation)
pub fn sigqueue(pid: Pid, sig: i32, value: SigVal) -> Result<(), SignalQueueError> {
    // Validate signal number
    if sig <= 0 || sig > 64 {
        return Err(SignalQueueError::InvalidSignal);
    }

    // Get current process for UID
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SignalQueueError::ProcessNotFound),
    };

    let current_uid = {
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = match proc_table.find_ref(current_pid) {
            Some(p) => p,
            None => return Err(SignalQueueError::ProcessNotFound),
        };
        proc.uid as Uid
    };

    // Create queued signal
    let signal = QueuedSignal::from_sigqueue(sig, current_pid, current_uid, value);

    // Get or create signal queue for target process
    let queue = {
        let mut registry = SIGNAL_QUEUE_REGISTRY.lock();
        registry.get_or_create_queue(pid)
    };

    // Add signal to queue
    queue.enqueue(signal)
}

/// Wait for a signal synchronously (sigtimedwait implementation)
pub fn sigtimedwait(
    sigmask: &SigSet,
    timeout: Option<&crate::posix::Timespec>,
) -> Result<SigInfoT, SignalWaitError> {
    // Get current process
    let pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SignalWaitError::ProcessNotFound),
    };

    // Get signal queue for current process
    let queue = {
        let mut registry = SIGNAL_QUEUE_REGISTRY.lock();
        registry.get_or_create_queue(pid)
    };

    // Calculate timeout deadline
    let deadline = if let Some(to) = timeout {
        if to.tv_sec < 0 || to.tv_nsec < 0 || to.tv_nsec >= 1_000_000_000 {
            return Err(SignalWaitError::InvalidTimeout);
        }
        
        let current_time = crate::time::get_timestamp();
        let timeout_ns = to.tv_sec as u64 * 1_000_000_000 + to.tv_nsec as u64;
        Some(current_time + timeout_ns)
    } else {
        None
    };

    // Wait for signal
    loop {
        // Check if there's a pending signal
        if let Some(signal) = queue.dequeue(Some(sigmask)) {
            return Ok(signal.info);
        }

        // Check timeout
        if let Some(deadline) = deadline {
            let current_time = crate::time::get_timestamp();
            if current_time >= deadline {
                return Err(SignalWaitError::Timeout);
            }
        }

        // Yield to scheduler (in a real implementation, we'd block properly)
        scheduler::yield_cpu();
    }
}

/// Wait for a signal without timeout (sigwaitinfo implementation)
pub fn sigwaitinfo(sigmask: &SigSet) -> Result<SigInfoT, SignalWaitError> {
    sigtimedwait(sigmask, None)
}

/// Set alternate signal stack (sigaltstack implementation)
pub fn sigaltstack(new_stack: Option<&crate::posix::StackT>, old_stack: Option<&mut crate::posix::StackT>) -> Result<(), SignalStackError> {
    // Get current process
    let pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SignalStackError::NoAlternateStack),
    };

    let mut proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = match proc_table.find(pid) {
        Some(p) => p,
        None => return Err(SignalStackError::NoAlternateStack),
    };

    // Save old stack if requested
    if let Some(old) = old_stack {
        if let Some(ref alt_stack) = proc.alt_signal_stack {
            *old = *alt_stack;
        } else {
            *old = crate::posix::StackT::default();
        }
    }

    // Set new stack if provided
    if let Some(new) = new_stack {
        // Validate new stack
        if new.ss_flags & crate::posix::SS_ONSTACK != 0 {
            return Err(SignalStackError::StackInUse);
        }

        if new.ss_flags & crate::posix::SS_DISABLE != 0 {
            // Disable alternate stack
            proc.alt_signal_stack = None;
        } else {
            // Validate stack size
            if new.ss_size < crate::posix::MINSIGSTKSZ {
                return Err(SignalStackError::StackTooSmall);
            }

            // Create new alternate stack
            let alt_stack = AlternateSignalStack::new(new.ss_size)?;
            proc.alt_signal_stack = Some(alt_stack.as_stackt());
        }
    }

    Ok(())
}

/// Set thread signal mask (pthread_sigmask implementation)
pub fn pthread_sigmask(
    how: i32,
    new_mask: &SigSet,
    old_mask: Option<&mut SigSet>,
) -> Result<(), SignalMaskError> {
    // Get current thread
    let thread_id = match crate::process::thread::current_thread() {
        Some(tid) => tid,
        None => return Err(SignalMaskError::InvalidSignal),
    };

    // Create a thread signal mask instance
    let thread_mask = ThreadSignalMask::new(thread_id);

    // Set the mask
    thread_mask.set_mask(how, new_mask, old_mask)
}

/// Check if a signal is a real-time signal
pub fn is_real_time_signal(sig: i32) -> bool {
    sig >= SIGRTMIN && sig <= SIGRTMAX
}

/// Get real-time signal range
pub fn get_real_time_signal_range() -> (i32, i32) {
    (SIGRTMIN, SIGRTMAX)
}

/// Initialize advanced signal handling subsystem
pub fn init_advanced_signal() {
    crate::println!("[signal] Initializing advanced signal handling subsystem");
    
    // Initialize signal queue registry
    let mut registry = SIGNAL_QUEUE_REGISTRY.lock();
    registry.queues.clear();
    
    crate::println!("[signal] Advanced signal handling initialized");
    crate::println!("[signal] Real-time signal range: {}-{}", SIGRTMIN, SIGRTMAX);
    crate::println!("[signal] Max pending signals per process: {}", MAX_PENDING_SIGNALS);
}

/// Cleanup advanced signal handling subsystem
pub fn cleanup_advanced_signal() {
    crate::println!("[signal] Cleaning up advanced signal handling subsystem");
    
    let mut registry = SIGNAL_QUEUE_REGISTRY.lock();
    let stats = registry.get_stats();
    
    crate::println!("[signal] Cleanup stats:");
    crate::println!("[signal]   Processes with queues: {}", stats.total_processes);
    crate::println!("[signal]   Total pending signals: {}", stats.total_pending_signals);
    crate::println!("[signal]   Real-time signals: {}", stats.total_real_time_signals);
    crate::println!("[signal]   Standard signals: {}", stats.total_standard_signals);
    
    registry.queues.clear();
}

/// Signal wait errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalWaitError {
    /// Process not found
    ProcessNotFound,
    /// Timeout occurred
    Timeout,
    /// Invalid timeout value
    InvalidTimeout,
    /// Signal was interrupted
    Interrupted,
    /// Invalid signal mask
    InvalidMask,
}