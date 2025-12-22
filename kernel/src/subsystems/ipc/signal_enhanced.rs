//! Enhanced Signal Handling for POSIX Compatibility
//! 
//! This module provides comprehensive POSIX-compliant signal handling with
//! advanced features including signal safety, proper signal delivery ordering,
//! and enhanced signal semantics.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::subsystems::sync::Mutex;
use super::signal::*;

/// Enhanced signal state with POSIX-compliant semantics
pub struct EnhancedSignalState {
    /// Base signal state
    base: SignalState,
    
    /// Signal delivery queue with ordering guarantees
    delivery_queue: Mutex<VecDeque<SignalDelivery>>,
    
    /// Signal safety state
    signal_safety: AtomicUsize,
    
    /// Signal delivery statistics
    delivery_stats: Mutex<SignalDeliveryStats>,
    
    /// Signal handler execution context
    handler_context: Mutex<Option<SignalHandlerContext>>,
    
    /// Signal mask stack for nested signal handling
    mask_stack: Mutex<Vec<SigSet>>,
    
    /// Pending signals that couldn't be delivered immediately
    pending_deferred: Mutex<VecDeque<DeferredSignal>>,
    
    /// Signal delivery flags
    delivery_flags: AtomicU64,
}

/// Signal delivery entry with metadata
#[derive(Debug, Clone)]
pub struct SignalDelivery {
    /// Signal number
    pub signal: Signal,
    /// Signal information
    pub info: SigInfo,
    /// Delivery priority (higher = higher priority)
    pub priority: u8,
    /// Delivery timestamp
    pub timestamp: u64,
    /// Source process ID
    pub source_pid: i32,
    /// Whether this is a real-time signal
    pub is_realtime: bool,
}

/// Signal delivery statistics
#[derive(Debug, Default, Clone)]
pub struct SignalDeliveryStats {
    /// Total signals sent
    pub total_sent: u64,
    /// Total signals delivered
    pub total_delivered: u64,
    /// Total signals ignored
    pub total_ignored: u64,
    /// Total signals blocked
    pub total_blocked: u64,
    /// Total signals lost (queue overflow)
    pub total_lost: u64,
    /// Average delivery latency in microseconds
    pub avg_delivery_latency_us: f64,
    /// Maximum delivery latency in microseconds
    pub max_delivery_latency_us: u64,
}

/// Signal handler execution context
#[derive(Debug, Clone)]
pub struct SignalHandlerContext {
    /// Signal being handled
    pub signal: Signal,
    /// Handler function address
    pub handler_address: usize,
    /// Signal mask during handler execution
    pub handler_mask: SigSet,
    /// Whether using SA_SIGINFO
    pub use_siginfo: bool,
    /// Whether handler should restart syscalls
    pub restart_syscalls: bool,
    /// Handler entry timestamp
    pub entry_timestamp: u64,
}

/// Deferred signal for later delivery
#[derive(Debug, Clone)]
pub struct DeferredSignal {
    /// Signal number
    pub signal: Signal,
    /// Signal information
    pub info: SigInfo,
    /// Reason for deferral
    pub reason: DeferralReason,
    /// Deferral timestamp
    pub timestamp: u64,
}

/// Reason for signal deferral
#[derive(Debug, Clone, PartialEq)]
pub enum DeferralReason {
    /// Signal was blocked
    Blocked,
    /// Process was in critical section
    CriticalSection,
    /// Signal queue was full
    QueueFull,
    /// Handler was already executing for this signal
    HandlerActive,
    /// System was in signal-unsafe state
    SignalUnsafe,
}

/// Signal delivery flags
pub mod delivery_flags {
    pub const IN_CRITICAL_SECTION: u64 = 1 << 0;
    pub const SIGNAL_UNSAFE: u64 = 1 << 1;
    pub const DEFER_REALTIME: u64 = 1 << 2;
    pub const PRESERVE_ORDER: u64 = 1 << 3;
    pub const TRACK_LATENCY: u64 = 1 << 4;
}

/// Signal safety levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalSafetyLevel {
    /// Signal-unsafe - cannot handle signals
    Unsafe,
    /// Signal-safe - can handle basic signals
    Safe,
    /// Signal-critical - must handle signals immediately
    Critical,
}

impl EnhancedSignalState {
    /// Create a new enhanced signal state
    pub fn new() -> Self {
        Self {
            base: SignalState::new(),
            delivery_queue: Mutex::new(VecDeque::new()),
            signal_safety: AtomicUsize::new(SignalSafetyLevel::Safe as usize),
            delivery_stats: Mutex::new(SignalDeliveryStats::default()),
            handler_context: Mutex::new(None),
            mask_stack: Mutex::new(Vec::new()),
            pending_deferred: Mutex::new(VecDeque::new()),
            delivery_flags: AtomicU64::new(0),
        }
    }
    
    /// Fork signal state (copy to child)
    pub fn fork(&self) -> Self {
        Self {
            base: self.base.fork(),
            delivery_queue: Mutex::new(VecDeque::new()),
            signal_safety: AtomicUsize::new(SignalSafetyLevel::Safe as usize),
            delivery_stats: Mutex::new(SignalDeliveryStats::default()),
            handler_context: Mutex::new(None),
            mask_stack: Mutex::new(Vec::new()),
            pending_deferred: Mutex::new(VecDeque::new()),
            delivery_flags: AtomicU64::new(self.delivery_flags.load(Ordering::Relaxed)),
        }
    }
    
    /// Reset signal state after exec
    pub fn exec_reset(&self) {
        self.base.exec_reset();
        
        // Clear delivery queue
        self.delivery_queue.lock().clear();
        
        // Clear pending deferred signals
        self.pending_deferred.lock().clear();
        
        // Reset signal safety
        self.signal_safety.store(SignalSafetyLevel::Safe as usize, Ordering::Relaxed);
        
        // Clear handler context
        *self.handler_context.lock() = None;
        
        // Clear mask stack
        self.mask_stack.lock().clear();
    }
    
    /// Send a signal with enhanced semantics
    pub fn send_signal_enhanced(&self, sig: Signal, info: SigInfo, source_pid: i32) -> Result<(), SignalError> {
        // Validate signal
        if sig == 0 || sig >= NSIG as u32 {
            return Err(SignalError::InvalidSignal);
        }
        
        // Update statistics
        {
            let mut stats = self.delivery_stats.lock();
            stats.total_sent += 1;
        }
        
        // Check if signal should be delivered to signalfd first
        let pid = crate::process::myproc().unwrap_or(0);
        if crate::syscalls::glib::deliver_signal_to_signalfd(pid, sig, info) {
            return Ok(());
        }
        
        // Get current signal safety level
        let safety_level = self.get_signal_safety_level();
        
        // Check if we're in a critical section
        let in_critical_section = self.is_in_critical_section();
        
        // Determine signal priority
        let priority = self.calculate_signal_priority(sig, &info);
        
        // Check if signal is real-time
        let is_realtime = sig >= SIGRTMIN && sig <= SIGRTMAX;
        
        // Create delivery entry
        let delivery = SignalDelivery {
            signal: sig,
            info,
            priority,
            timestamp: crate::subsystems::time::timestamp_nanos(),
            source_pid,
            is_realtime,
        };
        
        // Determine if signal should be deferred
        let should_defer = self.should_defer_signal(&delivery, safety_level, in_critical_section);
        
        if should_defer {
            self.defer_signal(delivery, self.get_deferral_reason(safety_level, in_critical_section));
            return Ok(());
        }
        
        // Add to delivery queue
        self.queue_signal_for_delivery(delivery);
        
        // Wake up target process if sleeping
        self.wakeup_process_if_needed();
        
        Ok(())
    }
    
    /// Calculate signal priority based on signal type and information
    fn calculate_signal_priority(&self, sig: Signal, info: &SigInfo) -> u8 {
        // Real-time signals have priority based on signal number
        if sig >= SIGRTMIN && sig <= SIGRTMAX {
            return (sig - SIGRTMIN) as u8;
        }
        
        // Standard signals have fixed priorities
        match sig {
            SIGKILL | SIGSTOP => 255,  // Highest priority
            SIGSEGV | SIGBUS | SIGFPE | SIGILL => 200,  // Exception signals
            SIGINT | SIGQUIT => 150,  // Terminal signals
            SIGTERM | SIGHUP => 100,  // Termination signals
            SIGCHLD => 50,  // Child status signals
            SIGALRM | SIGVTALRM | SIGPROF => 25,  // Timer signals
            _ => 0,  // Lowest priority
        }
    }
    
    /// Check if signal should be deferred
    fn should_defer_signal(&self, delivery: &SignalDelivery, safety_level: SignalSafetyLevel, in_critical_section: bool) -> bool {
        // Check signal safety level
        if safety_level == SignalSafetyLevel::Unsafe {
            return true;
        }
        
        // Check if in critical section
        if in_critical_section && !self.is_critical_section_interruptible(delivery.signal) {
            return true;
        }
        
        // Check if handler is already active for this signal
        if let Some(ref ctx) = *self.handler_context.lock() {
            if ctx.signal == delivery.signal {
                return true;
            }
        }
        
        // Check if signal is blocked
        let current_mask = self.base.get_mask();
        if current_mask.contains(delivery.signal) {
            return true;
        }
        
        // Check delivery queue capacity
        {
            let queue = self.delivery_queue.lock();
            if queue.len() >= MAX_SIGNAL_QUEUE_SIZE {
                return true;
            }
        }
        
        false
    }
    
    /// Check if critical section can be interrupted by signal
    fn is_critical_section_interruptible(&self, sig: Signal) -> bool {
        // Some signals can interrupt critical sections
        match sig {
            SIGKILL | SIGSTOP => true,  // Always interruptible
            SIGSEGV | SIGBUS | SIGFPE | SIGILL => true,  // Exception signals
            _ => false,
        }
    }
    
    /// Get deferral reason based on current state
    fn get_deferral_reason(&self, safety_level: SignalSafetyLevel, in_critical_section: bool) -> DeferralReason {
        if safety_level == SignalSafetyLevel::Unsafe {
            return DeferralReason::SignalUnsafe;
        }
        
        if in_critical_section {
            return DeferralReason::CriticalSection;
        }
        
        // Check if signal is blocked
        let current_mask = self.base.get_mask();
        if !current_mask.is_empty() {
            return DeferralReason::Blocked;
        }
        
        // Check delivery queue capacity
        {
            let queue = self.delivery_queue.lock();
            if queue.len() >= MAX_SIGNAL_QUEUE_SIZE {
                return DeferralReason::QueueFull;
            }
        }
        
        // Check if handler is already active
        if let Some(ref ctx) = *self.handler_context.lock() {
            return DeferralReason::HandlerActive;
        }
        
        DeferralReason::SignalUnsafe  // Default
    }
    
    /// Defer a signal for later delivery
    fn defer_signal(&self, delivery: SignalDelivery, reason: DeferralReason) {
        let deferred = DeferredSignal {
            signal: delivery.signal,
            info: delivery.info,
            reason,
            timestamp: delivery.timestamp,
        };
        
        self.pending_deferred.lock().push_back(deferred);
        
        // Update statistics
        {
            let mut stats = self.delivery_stats.lock();
            stats.total_blocked += 1;
        }
    }
    
    /// Queue signal for delivery
    fn queue_signal_for_delivery(&self, delivery: SignalDelivery) {
        let mut queue = self.delivery_queue.lock();
        
        // Insert signal in priority order
        let mut insert_pos = queue.len();
        for (i, existing) in queue.iter().enumerate() {
            if delivery.priority > existing.priority {
                insert_pos = i;
                break;
            }
        }
        
        queue.insert(insert_pos, delivery);
    }
    
    /// Wake up process if needed for signal delivery
    fn wakeup_process_if_needed(&self) {
        let pid = crate::process::myproc().unwrap_or(0);
        
        // Find process and wake it up if sleeping
        let mut proc_table = crate::process::manager::PROC_TABLE.lock();
        if let Some(proc) = proc_table.find_mut(pid) {
            if proc.state == crate::process::ProcState::Sleeping {
                proc.state = crate::process::ProcState::Runnable;
            }
        }
    }
    
    /// Process pending signals for delivery
    pub fn process_pending_signals(&self) -> Vec<SignalDelivery> {
        let mut ready_signals = Vec::new();
        
        // Check deferred signals
        {
            let mut deferred = self.pending_deferred.lock();
            let mut i = 0;
            while i < deferred.len() {
                let def_signal = &deferred[i];
                
                // Check if signal can now be delivered
                if self.can_deliver_deferred_signal(def_signal) {
                    let delivery = SignalDelivery {
                        signal: def_signal.signal,
                        info: def_signal.info,
                        priority: self.calculate_signal_priority(def_signal.signal, &def_signal.info),
                        timestamp: crate::subsystems::time::timestamp_nanos(),
                        source_pid: def_signal.info.pid,
                        is_realtime: def_signal.signal >= SIGRTMIN && def_signal.signal <= SIGRTMAX,
                    };
                    
                    ready_signals.push(delivery);
                    deferred.remove(i);
                } else {
                    i += 1;
                }
            }
        }
        
        // Check delivery queue
        {
            let mut queue = self.delivery_queue.lock();
            while let Some(delivery) = queue.pop_front() {
                if self.can_deliver_signal(&delivery) {
                    ready_signals.push(delivery);
                } else {
                    // Put it back and stop processing
                    queue.push_front(delivery);
                    break;
                }
            }
        }
        
        ready_signals
    }
    
    /// Check if deferred signal can be delivered
    fn can_deliver_deferred_signal(&self, def_signal: &DeferredSignal) -> bool {
        match def_signal.reason {
            DeferralReason::Blocked => {
                // Check if signal is no longer blocked
                let current_mask = self.base.get_mask();
                !current_mask.contains(def_signal.signal)
            }
            DeferralReason::CriticalSection => {
                // Check if no longer in critical section
                !self.is_in_critical_section()
            }
            DeferralReason::QueueFull => {
                // Check if queue has space
                let queue = self.delivery_queue.lock();
                queue.len() < MAX_SIGNAL_QUEUE_SIZE
            }
            DeferralReason::HandlerActive => {
                // Check if handler is no longer active
                self.handler_context.lock().is_none()
            }
            DeferralReason::SignalUnsafe => {
                // Check if signal safety level is safe
                self.get_signal_safety_level() != SignalSafetyLevel::Unsafe
            }
        }
    }
    
    /// Check if signal can be delivered
    fn can_deliver_signal(&self, delivery: &SignalDelivery) -> bool {
        // Check signal safety level
        let safety_level = self.get_signal_safety_level();
        if safety_level == SignalSafetyLevel::Unsafe {
            return false;
        }
        
        // Check if in critical section
        if self.is_in_critical_section() && !self.is_critical_section_interruptible(delivery.signal) {
            return false;
        }
        
        // Check if signal is blocked
        let current_mask = self.base.get_mask();
        if current_mask.contains(delivery.signal) {
            return false;
        }
        
        // Check if handler is already active for this signal
        if let Some(ref ctx) = *self.handler_context.lock() {
            if ctx.signal == delivery.signal {
                return false;
            }
        }
        
        true
    }
    
    /// Deliver a signal to the process
    pub fn deliver_signal(&self, delivery: &SignalDelivery) -> Result<(), SignalError> {
        // Get signal action
        let action = self.base.get_action(delivery.signal);
        
        // Handle based on action
        if action.handler == SIG_IGN {
            // Update statistics
            {
                let mut stats = self.delivery_stats.lock();
                stats.total_ignored += 1;
            }
            return Ok(());
        }
        
        if action.handler == SIG_DFL {
            // Default action
            match default_action(delivery.signal) {
                DefaultAction::Term | DefaultAction::Core => {
                    self.terminate_process(delivery.signal, delivery.info);
                    return Ok(());
                }
                DefaultAction::Stop => {
                    self.stop_process(delivery.signal);
                    return Ok(());
                }
                DefaultAction::Cont => {
                    self.continue_process();
                    return Ok(());
                }
                DefaultAction::Ign => {
                    // Update statistics
                    {
                        let mut stats = self.delivery_stats.lock();
                        stats.total_ignored += 1;
                    }
                    return Ok(());
                }
            }
        }
        
        // User handler
        self.setup_signal_handler(delivery, &action)?;
        
        // Update statistics
        {
            let mut stats = self.delivery_stats.lock();
            stats.total_delivered += 1;
            
            // Update latency statistics
            let now = crate::subsystems::time::timestamp_nanos();
            let latency_us = (now - delivery.timestamp) / 1000;
            stats.avg_delivery_latency_us = 
                (stats.avg_delivery_latency_us * (stats.total_delivered - 1) as f64 + latency_us as f64) 
                / stats.total_delivered as f64;
            stats.max_delivery_latency_us = stats.max_delivery_latency_us.max(latency_us);
        }
        
        Ok(())
    }
    
    /// Setup signal handler execution
    fn setup_signal_handler(&self, delivery: &SignalDelivery, action: &SigAction) -> Result<(), SignalError> {
        // Save current signal mask
        let current_mask = self.base.get_mask();
        
        // Push current mask to stack
        {
            let mut mask_stack = self.mask_stack.lock();
            mask_stack.push(current_mask);
        }
        
        // Calculate handler mask
        let mut handler_mask = action.mask;
        if (action.flags.0 & SigActionFlags::SA_NODEFER) == 0 {
            // Block signal being handled
            handler_mask.add(delivery.signal);
        }
        
        // Apply handler mask
        self.base.set_mask(handler_mask);
        
        // Create handler context
        let handler_context = SignalHandlerContext {
            signal: delivery.signal,
            handler_address: action.handler,
            handler_mask,
            use_siginfo: (action.flags.0 & SigActionFlags::SA_SIGINFO) != 0,
            restart_syscalls: (action.flags.0 & SigActionFlags::SA_RESTART) != 0,
            entry_timestamp: crate::subsystems::time::timestamp_nanos(),
        };
        
        // Store handler context
        *self.handler_context.lock() = Some(handler_context);
        
        // Setup signal frame for user handler
        self.setup_signal_frame(delivery, action)?;
        
        Ok(())
    }
    
    /// Setup signal frame for user handler
    fn setup_signal_frame(&self, delivery: &SignalDelivery, action: &SigAction) -> Result<(), SignalError> {
        // Get current process
        let pid = crate::process::myproc().ok_or(SignalError::ProcessNotFound)?;
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = proc_table.find_ref(pid).ok_or(SignalError::ProcessNotFound)?;
        let pagetable = proc.pagetable;
        drop(proc_table);
        
        if pagetable.is_null() {
            return Err(SignalError::InvalidAddress);
        }
        
        // Determine if using alternate stack
        let use_alt_stack = (action.flags.0 & SigActionFlags::SA_ONSTACK) != 0;
        
        // Allocate signal frame on user stack or alternate stack
        let sp = if use_alt_stack {
            // Use alternate signal stack
            self.get_alternate_stack_address()
        } else {
            // Use current stack pointer
            self.get_current_stack_pointer()
        };
        
        // Setup signal frame structure
        let frame = SignalFrame {
            info: delivery.info,
            context: self.get_current_context(),
            handler: action.handler,
            use_siginfo: (action.flags.0 & SigActionFlags::SA_SIGINFO) != 0,
            return_address: self.get_signal_return_address(),
        };
        
        // Copy signal frame to user space
        // This would involve copying the frame to the user stack
        // and setting up registers to jump to the handler
        
        // For now, just a placeholder
        crate::println!("Setting up signal frame for signal {}", delivery.signal);
        
        Ok(())
    }
    
    /// Get alternate signal stack address
    fn get_alternate_stack_address(&self) -> usize {
        // This would get the alternate stack address from process structure
        // For now, return a placeholder
        0x7ffff0000000usize
    }
    
    /// Get current stack pointer
    fn get_current_stack_pointer(&self) -> usize {
        // This would get the current stack pointer from registers
        // For now, return a placeholder
        0x7fffffffe000usize
    }
    
    /// Get current execution context
    fn get_current_context(&self) -> SignalContext {
        // This would get the current register state
        // For now, return a placeholder
        SignalContext {
            saved_regs: SavedRegs::default(),
            info: SigInfo::default(),
            saved_mask: 0,
        }
    }
    
    /// Get signal return address
    fn get_signal_return_address(&self) -> usize {
        // This would get the address of the signal return trampoline
        // For now, return a placeholder
        0x7fffff000000usize
    }
    
    /// Terminate process due to signal
    fn terminate_process(&self, sig: Signal, info: SigInfo) {
        let pid = crate::process::myproc().unwrap_or(0);
        
        // Create core dump if needed
        if default_action(sig) == DefaultAction::Core {
            self.create_core_dump(sig, info);
        }
        
        // Terminate process
        crate::process::manager::exit_process(pid, sig as i32);
    }
    
    /// Stop process due to signal
    fn stop_process(&self, sig: Signal) {
        let pid = crate::process::myproc().unwrap_or(0);
        
        // Stop process
        crate::process::manager::stop_process(pid, sig);
    }
    
    /// Continue stopped process
    fn continue_process(&self) {
        let pid = crate::process::myproc().unwrap_or(0);
        
        // Continue process
        crate::process::manager::continue_process(pid);
    }
    
    /// Create core dump for process
    fn create_core_dump(&self, sig: Signal, info: SigInfo) {
        // This would create a core dump file
        // For now, just log
        crate::println!("Creating core dump for signal {} from pid {}", sig, info.pid);
    }
    
    /// Return from signal handler
    pub fn return_from_handler(&self) -> Result<(), SignalError> {
        // Restore signal mask
        {
            let mut mask_stack = self.mask_stack.lock();
            if let Some(old_mask) = mask_stack.pop() {
                self.base.set_mask(old_mask);
            }
        }
        
        // Clear handler context
        *self.handler_context.lock() = None;
        
        Ok(())
    }
    
    /// Get current signal safety level
    pub fn get_signal_safety_level(&self) -> SignalSafetyLevel {
        match self.signal_safety.load(Ordering::Relaxed) {
            0 => SignalSafetyLevel::Unsafe,
            1 => SignalSafetyLevel::Safe,
            2 => SignalSafetyLevel::Critical,
            _ => SignalSafetyLevel::Safe,
        }
    }
    
    /// Set signal safety level
    pub fn set_signal_safety_level(&self, level: SignalSafetyLevel) {
        self.signal_safety.store(level as usize, Ordering::Relaxed);
    }
    
    /// Check if in critical section
    pub fn is_in_critical_section(&self) -> bool {
        (self.delivery_flags.load(Ordering::Relaxed) & delivery_flags::IN_CRITICAL_SECTION) != 0
    }
    
    /// Enter critical section
    pub fn enter_critical_section(&self) {
        self.delivery_flags.fetch_or(delivery_flags::IN_CRITICAL_SECTION, Ordering::Relaxed);
    }
    
    /// Exit critical section
    pub fn exit_critical_section(&self) {
        self.delivery_flags.fetch_and(!delivery_flags::IN_CRITICAL_SECTION, Ordering::Relaxed);
    }
    
    /// Get signal delivery statistics
    pub fn get_delivery_stats(&self) -> SignalDeliveryStats {
        self.delivery_stats.lock().clone()
    }
    
    /// Reset signal delivery statistics
    pub fn reset_delivery_stats(&self) {
        *self.delivery_stats.lock() = SignalDeliveryStats::default();
    }
}

/// Signal frame for user handler
#[repr(C)]
#[derive(Debug, Clone)]
pub struct SignalFrame {
    /// Signal information
    pub info: SigInfo,
    /// Saved context
    pub context: SignalContext,
    /// Handler address
    pub handler: usize,
    /// Using SA_SIGINFO
    pub use_siginfo: bool,
    /// Return address
    pub return_address: usize,
}

/// Maximum signal queue size
const MAX_SIGNAL_QUEUE_SIZE: usize = 64;

/// Signal errors
#[derive(Debug, Clone, PartialEq)]
pub enum SignalError {
    /// Invalid signal number
    InvalidSignal,
    /// Process not found
    ProcessNotFound,
    /// Invalid address
    InvalidAddress,
    /// Signal queue full
    QueueFull,
    /// Signal blocked
    SignalBlocked,
    /// System error
    SystemError,
}

/// Global enhanced signal state instances
static mut ENHANCED_SIGNAL_STATES: alloc::collections::BTreeMap<usize, *mut EnhancedSignalState> = alloc::collections::BTreeMap::new();
static SIGNAL_STATES_INIT: spin::Once = spin::Once::new();

/// Initialize enhanced signal state for a process
pub fn init_enhanced_signal_state(pid: usize) -> Result<(), SignalError> {
    SIGNAL_STATES_INIT.call_once(|| {
        unsafe {
            ENHANCED_SIGNAL_STATES = alloc::collections::BTreeMap::new();
        }
    });
    
    let state = EnhancedSignalState::new();
    unsafe {
        ENHANCED_SIGNAL_STATES.insert(pid, Box::into_raw(Box::new(state)));
    }
    
    Ok(())
}

/// Get enhanced signal state for a process
pub fn get_enhanced_signal_state(pid: usize) -> Option<&'static mut EnhancedSignalState> {
    unsafe {
        ENHANCED_SIGNAL_STATES.get_mut(&pid).map(|ptr| &mut **ptr)
    }
}

/// Cleanup enhanced signal state for a process
pub fn cleanup_enhanced_signal_state(pid: usize) {
    unsafe {
        if let Some(ptr) = ENHANCED_SIGNAL_STATES.remove(&pid) {
            let _ = Box::from_raw(ptr);
        }
    }
}

/// Send signal with enhanced semantics
pub fn send_signal_enhanced(pid: usize, sig: Signal, info: SigInfo, source_pid: i32) -> Result<(), SignalError> {
    let state = get_enhanced_signal_state(pid).ok_or(SignalError::ProcessNotFound)?;
    state.send_signal_enhanced(sig, info, source_pid)
}

/// Process pending signals for a process
pub fn process_pending_signals(pid: usize) -> Vec<SignalDelivery> {
    if let Some(state) = get_enhanced_signal_state(pid) {
        state.process_pending_signals()
    } else {
        Vec::new()
    }
}

/// Deliver a signal to a process
pub fn deliver_signal(pid: usize, delivery: &SignalDelivery) -> Result<(), SignalError> {
    let state = get_enhanced_signal_state(pid).ok_or(SignalError::ProcessNotFound)?;
    state.deliver_signal(delivery)
}

/// Return from signal handler
pub fn return_from_handler(pid: usize) -> Result<(), SignalError> {
    let state = get_enhanced_signal_state(pid).ok_or(SignalError::ProcessNotFound)?;
    state.return_from_handler()
}

/// Set signal safety level for a process
pub fn set_signal_safety_level(pid: usize, level: SignalSafetyLevel) -> Result<(), SignalError> {
    let state = get_enhanced_signal_state(pid).ok_or(SignalError::ProcessNotFound)?;
    state.set_signal_safety_level(level);
    Ok(())
}

/// Enter critical section for a process
pub fn enter_critical_section(pid: usize) -> Result<(), SignalError> {
    let state = get_enhanced_signal_state(pid).ok_or(SignalError::ProcessNotFound)?;
    state.enter_critical_section();
    Ok(())
}

/// Exit critical section for a process
pub fn exit_critical_section(pid: usize) -> Result<(), SignalError> {
    let state = get_enhanced_signal_state(pid).ok_or(SignalError::ProcessNotFound)?;
    state.exit_critical_section();
    Ok(())
}

/// Get signal delivery statistics for a process
pub fn get_delivery_stats(pid: usize) -> Option<SignalDeliveryStats> {
    get_enhanced_signal_state(pid).map(|state| state.get_delivery_stats())
}