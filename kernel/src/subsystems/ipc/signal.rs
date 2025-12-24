// Signal handling for processes
//
// Implements POSIX-like signals:
// - Signal delivery and handling
// - Signal masks and pending signals
// - Signal actions (default, ignore, custom handler)
// - Real-time signals

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::subsystems::sync::Mutex;

// ============================================================================
// Signal Numbers (POSIX compatible)
// ============================================================================

/// Signal number type
pub type Signal = u32;

// Standard signals (1-31)
pub const SIGHUP: Signal = 1;      // Hangup
pub const SIGINT: Signal = 2;      // Interrupt (Ctrl+C)
pub const SIGQUIT: Signal = 3;     // Quit (Ctrl+\)
pub const SIGILL: Signal = 4;      // Illegal instruction
pub const SIGTRAP: Signal = 5;     // Trace/breakpoint trap
pub const SIGABRT: Signal = 6;     // Abort
pub const SIGBUS: Signal = 7;      // Bus error
pub const SIGFPE: Signal = 8;      // Floating-point exception
pub const SIGKILL: Signal = 9;     // Kill (cannot be caught)
pub const SIGUSR1: Signal = 10;    // User-defined signal 1
pub const SIGSEGV: Signal = 11;    // Segmentation violation
pub const SIGUSR2: Signal = 12;    // User-defined signal 2
pub const SIGPIPE: Signal = 13;    // Broken pipe
pub const SIGALRM: Signal = 14;    // Alarm clock
pub const SIGTERM: Signal = 15;    // Termination
pub const SIGSTKFLT: Signal = 16;  // Stack fault
pub const SIGCHLD: Signal = 17;    // Child status changed
pub const SIGCONT: Signal = 18;    // Continue
pub const SIGSTOP: Signal = 19;    // Stop (cannot be caught)
pub const SIGTSTP: Signal = 20;    // Terminal stop (Ctrl+Z)
pub const SIGTTIN: Signal = 21;    // Background read from tty
pub const SIGTTOU: Signal = 22;    // Background write to tty
pub const SIGURG: Signal = 23;     // Urgent data on socket
pub const SIGXCPU: Signal = 24;    // CPU time limit exceeded
pub const SIGXFSZ: Signal = 25;    // File size limit exceeded
pub const SIGVTALRM: Signal = 26;  // Virtual timer expired
pub const SIGPROF: Signal = 27;    // Profiling timer expired
pub const SIGWINCH: Signal = 28;   // Window size changed
pub const SIGIO: Signal = 29;      // I/O possible
pub const SIGPWR: Signal = 30;     // Power failure
pub const SIGSYS: Signal = 31;     // Bad system call

// Real-time signals (32-64)
pub const SIGRTMIN: Signal = 32;
pub const SIGRTMAX: Signal = 64;

/// Maximum signal number
pub const NSIG: usize = 65;

// Signal mask operations
pub const SIG_BLOCK: i32 = 0;
pub const SIG_UNBLOCK: i32 = 1;
pub const SIG_SETMASK: i32 = 2;

// ============================================================================
// Signal Set (Bitmask)
// ============================================================================

/// Signal set for masking/pending signals
#[derive(Debug, Clone, Copy, Default)]
pub struct SigSet {
    bits: u64,
}

impl SigSet {
    /// Create an empty signal set
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }
    
    /// Create a full signal set (all signals)
    pub const fn full() -> Self {
        Self { bits: !0 }
    }
    
    /// Add a signal to the set
    pub fn add(&mut self, sig: Signal) {
        if sig > 0 && sig < NSIG as u32 {
            self.bits |= 1 << (sig - 1);
        }
    }
    
    /// Remove a signal from the set
    pub fn remove(&mut self, sig: Signal) {
        if sig > 0 && sig < NSIG as u32 {
            self.bits &= !(1 << (sig - 1));
        }
    }
    
    /// Check if a signal is in the set
    pub fn contains(&self, sig: Signal) -> bool {
        if sig > 0 && sig < NSIG as u32 {
            (self.bits & (1 << (sig - 1))) != 0
        } else {
            false
        }
    }
    
    /// Check if set is empty
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }
    
    /// Union of two sets
    pub fn union(&self, other: &SigSet) -> SigSet {
        SigSet { bits: self.bits | other.bits }
    }
    
    /// Intersection of two sets
    pub fn intersect(&self, other: &SigSet) -> SigSet {
        SigSet { bits: self.bits & other.bits }
    }
    
    /// Complement of set
    pub fn complement(&self) -> SigSet {
        SigSet { bits: !self.bits }
    }
    
    /// Get the lowest pending signal
    pub fn first_signal(&self) -> Option<Signal> {
        if self.bits == 0 {
            return None;
        }
        Some(self.bits.trailing_zeros() as Signal + 1)
    }
    
    /// Get raw bits
    pub fn bits(&self) -> u64 {
        self.bits
    }
    
    /// Set from raw bits
    pub fn from_bits(bits: u64) -> Self {
        Self { bits }
    }
}

// ============================================================================
// Signal Action
// ============================================================================

/// Handler function type
pub type SignalHandler = extern "C" fn(Signal);

/// Special handler values
pub const SIG_DFL: usize = 0;  // Default action
pub const SIG_IGN: usize = 1;  // Ignore signal

/// Signal action flags
#[derive(Debug, Clone, Copy, Default)]
pub struct SigActionFlags(pub u32);

impl SigActionFlags {
    pub const SA_NOCLDSTOP: u32 = 1 << 0;   // Don't send SIGCHLD when children stop
    pub const SA_NOCLDWAIT: u32 = 1 << 1;   // Don't create zombie on child death
    pub const SA_SIGINFO: u32   = 1 << 2;   // Use sa_sigaction instead of sa_handler
    pub const SA_ONSTACK: u32   = 1 << 3;   // Use alternate signal stack
    pub const SA_RESTART: u32   = 1 << 4;   // Restart syscall on signal return
    pub const SA_NODEFER: u32   = 1 << 5;   // Don't block signal during handler
    pub const SA_RESETHAND: u32 = 1 << 6;   // Reset handler to SIG_DFL after delivery
}

/// Signal action structure
#[derive(Clone, Copy)]
pub struct SigAction {
    /// Handler address (SIG_DFL, SIG_IGN, or function pointer)
    pub handler: usize,
    /// Action flags
    pub flags: SigActionFlags,
    /// Signals to block during handler execution
    pub mask: SigSet,
    /// Restorer function (for returning from signal handler)
    pub restorer: usize,
}

impl Default for SigAction {
    fn default() -> Self {
        Self {
            handler: SIG_DFL,
            flags: SigActionFlags::default(),
            mask: SigSet::empty(),
            restorer: 0,
        }
    }
}

// ============================================================================
// Signal Info (for SA_SIGINFO handlers)
// ============================================================================

/// Signal information structure
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct SigInfo {
    pub signo: i32,      // Signal number
    pub errno: i32,      // Error number
    pub code: i32,       // Signal code
    pub pid: i32,        // Sending process ID
    pub uid: u32,        // Sending user ID
    pub status: i32,     // Exit value or signal
    pub addr: usize,     // Faulting address (for SIGSEGV, etc.)
    pub value: usize,    // Signal value (union with addr)
}

/// Signal codes
pub mod si_code {
    // General codes
    pub const SI_USER: i32 = 0;      // Sent by kill()
    pub const SI_KERNEL: i32 = 128;  // Sent by kernel
    pub const SI_QUEUE: i32 = -1;    // Sent by sigqueue()
    pub const SI_TIMER: i32 = -2;    // POSIX timer
    pub const SI_MESGQ: i32 = -3;    // POSIX message queue
    pub const SI_ASYNCIO: i32 = -4;  // AIO completion
    pub const SI_SIGIO: i32 = -5;    // Queued SIGIO
    
    // SIGILL codes
    pub const ILL_ILLOPC: i32 = 1;   // Illegal opcode
    pub const ILL_ILLOPN: i32 = 2;   // Illegal operand
    pub const ILL_ILLADR: i32 = 3;   // Illegal addressing mode
    pub const ILL_PRVOPC: i32 = 6;   // Privileged opcode
    
    // SIGFPE codes
    pub const FPE_INTDIV: i32 = 1;   // Integer divide by zero
    pub const FPE_INTOVF: i32 = 2;   // Integer overflow
    pub const FPE_FLTDIV: i32 = 3;   // Float divide by zero
    pub const FPE_FLTOVF: i32 = 4;   // Float overflow
    
    // SIGSEGV codes
    pub const SEGV_MAPERR: i32 = 1;  // Address not mapped
    pub const SEGV_ACCERR: i32 = 2;  // Invalid permissions
    
    // SIGBUS codes
    pub const BUS_ADRALN: i32 = 1;   // Invalid address alignment
    pub const BUS_ADRERR: i32 = 2;   // Non-existent physical address
    
    // SIGCHLD codes
    pub const CLD_EXITED: i32 = 1;   // Child has exited
    pub const CLD_KILLED: i32 = 2;   // Child was killed
    pub const CLD_DUMPED: i32 = 3;   // Child terminated abnormally
    pub const CLD_TRAPPED: i32 = 4;  // Traced child has trapped
    pub const CLD_STOPPED: i32 = 5;  // Child has stopped
    pub const CLD_CONTINUED: i32 = 6; // Child has continued
}

// ============================================================================
// Signal Delivery Flags (Enhanced)
// ============================================================================

pub mod delivery_flags {
    pub const IN_CRITICAL_SECTION: u64 = 1 << 0;
    pub const SIGNAL_UNSAFE: u64 = 1 << 1;
    pub const DEFER_REALTIME: u64 = 1 << 2;
    pub const PRESERVE_ORDER: u64 = 1 << 3;
    pub const TRACK_LATENCY: u64 = 1 << 4;
}

/// Maximum signal queue size
pub const MAX_SIGNAL_QUEUE_SIZE: usize = 64;

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

// ============================================================================
// Default Signal Actions
// ============================================================================

/// Default action for a signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultAction {
    Term,    // Terminate process
    Core,    // Terminate and dump core
    Ign,     // Ignore
    Stop,    // Stop process
    Cont,    // Continue process
}

/// Get the default action for a signal
pub fn default_action(sig: Signal) -> DefaultAction {
    match sig {
        SIGHUP | SIGINT | SIGPIPE | SIGALRM | SIGTERM | 
        SIGUSR1 | SIGUSR2 | SIGPROF | SIGVTALRM | SIGSTKFLT |
        SIGIO | SIGPWR | SIGSYS => DefaultAction::Term,
        
        SIGQUIT | SIGILL | SIGABRT | SIGFPE | SIGSEGV | 
        SIGBUS | SIGTRAP | SIGXCPU | SIGXFSZ => DefaultAction::Core,
        
        SIGCHLD | SIGURG | SIGWINCH => DefaultAction::Ign,
        
        SIGSTOP | SIGTSTP | SIGTTIN | SIGTTOU => DefaultAction::Stop,
        
        SIGCONT => DefaultAction::Cont,
        
        SIGKILL => DefaultAction::Term,  // Cannot be caught
        
        _ if sig >= SIGRTMIN && sig <= SIGRTMAX => DefaultAction::Term,
        
        _ => DefaultAction::Ign,
    }
}

/// Check if a signal can be caught or blocked
pub fn is_catchable(sig: Signal) -> bool {
    sig != SIGKILL && sig != SIGSTOP
}

// ============================================================================
// Per-Process Signal State
// ============================================================================

/// Queued signal (for real-time signals)
#[derive(Clone)]
pub struct QueuedSignal {
    pub info: SigInfo,
}

/// Signal state for a process
pub struct SignalState {
    /// Pending signals (standard)
    pending: AtomicU64,
    /// Blocked signals
    blocked: AtomicU64,
    /// Signal handlers
    handlers: Mutex<[SigAction; NSIG]>,
    /// Queued signals (for real-time signals with sigqueue)
    queued: Mutex<VecDeque<QueuedSignal>>,
    /// Saved signal mask (for sigsuspend)
    saved_mask: AtomicU64,
    /// In signal handler
    in_handler: AtomicU64,

    /// Enhanced: Signal delivery queue with ordering guarantees
    delivery_queue: Mutex<VecDeque<SignalDelivery>>,
    /// Enhanced: Signal safety state
    signal_safety: AtomicUsize,
    /// Enhanced: Signal delivery statistics
    delivery_stats: Mutex<SignalDeliveryStats>,
    /// Enhanced: Signal handler execution context
    handler_context: Mutex<Option<SignalHandlerContext>>,
    /// Enhanced: Signal mask stack for nested signal handling
    mask_stack: Mutex<Vec<SigSet>>,
    /// Enhanced: Pending signals that couldn't be delivered immediately
    pending_deferred: Mutex<VecDeque<DeferredSignal>>,
    /// Enhanced: Signal delivery flags
    delivery_flags: AtomicU64,
}

impl SignalState {
    /// Create a new signal state
    pub fn new() -> Self {
        Self {
            pending: AtomicU64::new(0),
            blocked: AtomicU64::new(0),
            handlers: Mutex::new([SigAction::default(); NSIG]),
            queued: Mutex::new(VecDeque::new()),
            saved_mask: AtomicU64::new(0),
            in_handler: AtomicU64::new(0),

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
            pending: AtomicU64::new(0),  // Pending signals not inherited
            blocked: AtomicU64::new(self.blocked.load(Ordering::Relaxed)),
            handlers: Mutex::new(*self.handlers.lock()),
            queued: Mutex::new(VecDeque::new()),
            saved_mask: AtomicU64::new(0),
            in_handler: AtomicU64::new(0),

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
        // Reset all handlers to default (except ignored ones)
        let mut handlers = self.handlers.lock();
        for (_i, action) in handlers.iter_mut().enumerate() {
            if action.handler != SIG_IGN {
                *action = SigAction::default();
            }
        }

        // Clear pending signals (except blocked ones? unclear)
        self.pending.store(0, Ordering::Release);
        self.queued.lock().clear();

        // Enhanced: Clear delivery queue
        self.delivery_queue.lock().clear();

        // Enhanced: Clear pending deferred signals
        self.pending_deferred.lock().clear();

        // Enhanced: Reset signal safety
        self.signal_safety.store(SignalSafetyLevel::Safe as usize, Ordering::Relaxed);

        // Enhanced: Clear handler context
        *self.handler_context.lock() = None;

        // Enhanced: Clear mask stack
        self.mask_stack.lock().clear();
    }
    
    /// Send a signal to the process
    pub fn send_signal(&self, sig: Signal) -> Result<(), ()> {
        if sig == 0 || sig >= NSIG as u32 {
            return Err(());
        }

        // Check if signal should be delivered to signalfd first
        let pid = crate::process::myproc().unwrap_or(0);
        if crate::syscalls::glib::deliver_signal_to_signalfd(pid as usize, sig, SigInfo {
            signo: sig as i32,
            code: si_code::SI_KERNEL,
            ..Default::default()
        }) {
            // Signal was delivered to signalfd, don't set pending bit
            return Ok(());
        }

        // Standard signals: just set bit
        let mask = 1u64 << (sig - 1);
        self.pending.fetch_or(mask, Ordering::Release);

        Ok(())
    }
    
    /// Send a signal with info (for sigqueue)
    pub fn send_signal_info(&self, sig: Signal, info: SigInfo) -> Result<(), ()> {
        if sig == 0 || sig >= NSIG as u32 {
            return Err(());
        }

        // Check if signal should be delivered to signalfd first
        let pid = crate::process::myproc().unwrap_or(0);
        if crate::syscalls::glib::deliver_signal_to_signalfd(pid as usize, sig, info) {
            // Signal was delivered to signalfd, don't set pending bit
            return Ok(());
        }

        // Set pending bit
        let mask = 1u64 << (sig - 1);
        self.pending.fetch_or(mask, Ordering::Release);

        // Queue for real-time signals
        if sig >= SIGRTMIN {
            self.queued.lock().push_back(QueuedSignal { info });
        }

        Ok(())
    }
    
    /// Get pending signals (not blocked)
    pub fn pending_signals(&self) -> SigSet {
        let pending = self.pending.load(Ordering::Acquire);
        let blocked = self.blocked.load(Ordering::Acquire);
        SigSet::from_bits(pending & !blocked)
    }
    
    /// Check if there are deliverable signals
    pub fn has_pending(&self) -> bool {
        !self.pending_signals().is_empty()
    }
    
    /// Dequeue a signal for delivery
    pub fn dequeue_signal(&self) -> Option<(Signal, SigInfo)> {
        let deliverable = self.pending_signals();
        
        // Get first deliverable signal
        let sig = deliverable.first_signal()?;
        
        // Clear pending bit
        let mask = 1u64 << (sig - 1);
        self.pending.fetch_and(!mask, Ordering::Release);
        
        // Get signal info
        let info = if sig >= SIGRTMIN {
            // Check queued signals
            let mut queued = self.queued.lock();
            queued.iter()
                .position(|q| q.info.signo == sig as i32)
                .map(|i| queued.remove(i).unwrap().info)
                .unwrap_or_else(|| SigInfo {
                    signo: sig as i32,
                    code: si_code::SI_KERNEL,
                    ..Default::default()
                })
        } else {
            SigInfo {
                signo: sig as i32,
                code: si_code::SI_KERNEL,
                ..Default::default()
            }
        };
        
        Some((sig, info))
    }
    
    /// Get signal action
    pub fn get_action(&self, sig: Signal) -> SigAction {
        if sig == 0 || sig >= NSIG as u32 {
            return SigAction::default();
        }
        self.handlers.lock()[sig as usize]
    }
    
    /// Set signal action
    pub fn set_action(&self, sig: Signal, action: SigAction) -> Result<SigAction, ()> {
        if sig == 0 || sig >= NSIG as u32 || !is_catchable(sig) {
            return Err(());
        }
        
        let mut handlers = self.handlers.lock();
        let old = handlers[sig as usize];
        handlers[sig as usize] = action;
        
        Ok(old)
    }
    
    /// Get signal mask
    pub fn get_mask(&self) -> SigSet {
        SigSet::from_bits(self.blocked.load(Ordering::Acquire))
    }
    
    /// Set signal mask
    pub fn set_mask(&self, mask: SigSet) -> SigSet {
        let old = self.blocked.swap(mask.bits(), Ordering::AcqRel);
        SigSet::from_bits(old)
    }
    
    /// Block additional signals
    pub fn block(&self, mask: SigSet) -> SigSet {
        let old = self.blocked.fetch_or(mask.bits(), Ordering::AcqRel);
        SigSet::from_bits(old)
    }
    
    /// Unblock signals
    pub fn unblock(&self, mask: SigSet) -> SigSet {
        let old = self.blocked.fetch_and(!mask.bits(), Ordering::AcqRel);
        SigSet::from_bits(old)
    }
    
    /// Save current mask and set new one (for sigsuspend)
    pub fn suspend(&self, temp_mask: SigSet) -> SigSet {
        let current = self.blocked.load(Ordering::Acquire);
        self.saved_mask.store(current, Ordering::Release);
        self.blocked.store(temp_mask.bits(), Ordering::Release);
        SigSet::from_bits(current)
    }
    
    /// Restore saved mask
    pub fn restore_mask(&self) {
        let saved = self.saved_mask.load(Ordering::Acquire);
        self.blocked.store(saved, Ordering::Release);
    }

    /// Enter signal handler (save context)
    pub fn enter_handler(&self, sig: Signal, action: &SigAction) -> (SigSet, SigSet) {
        // Save current mask
        let old_mask = self.get_mask();

        // Block additional signals during handler if SA_NODEFER not set
        let mut new_mask = action.mask;
        if (action.flags.0 & SigActionFlags::SA_NODEFER) == 0 {
            // Block the signal being handled
            new_mask.add(sig);
        }

        // Apply the new mask (union with current blocked signals)
        let handler_mask = old_mask.union(&new_mask);
        self.set_mask(handler_mask);

        // Mark that we're in a handler
        self.in_handler.fetch_add(1, Ordering::Release);

        (old_mask, handler_mask)
    }

    /// Exit signal handler (restore context)
    pub fn exit_handler(&self, old_mask: SigSet) {
        // Restore the saved mask
        self.set_mask(old_mask);

        // Mark that we're no longer in a handler
        self.in_handler.fetch_sub(1, Ordering::Release);
    }

    /// Check if currently in signal handler
    pub fn in_signal_handler(&self) -> bool {
        self.in_handler.load(Ordering::Acquire) > 0
    }
}

impl Default for SignalState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Signal Delivery
// ============================================================================

/// Signal delivery context
#[repr(C)]
pub struct SignalContext {
    /// Saved user context (registers)
    pub saved_regs: SavedRegs,
    /// Signal info
    pub info: SigInfo,
    /// Saved signal mask
    pub saved_mask: u64,
}

/// Saved registers for signal handler return
#[cfg(target_arch = "riscv64")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SavedRegs {
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
    pub sepc: usize,
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SavedRegs {
    pub regs: [usize; 31],
    pub sp: usize,
    pub elr: usize,
    pub spsr: usize,
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SavedRegs {
    pub rax: usize,
    pub rbx: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rbp: usize,
    pub rsp: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
    pub rip: usize,
    pub rflags: usize,
}

/// Result of signal delivery check
pub enum SignalDeliveryResult {
    /// No signal to deliver
    None,
    /// Signal should terminate process
    Terminate(Signal),
    /// Signal should stop process
    Stop(Signal),
    /// Signal should continue process
    Continue,
    /// Signal should be handled by user handler
    Handle {
        signal: Signal,
        info: SigInfo,
        action: SigAction,
        use_siginfo: bool,  // Whether to use sa_sigaction (SA_SIGINFO)
        restart_syscall: bool,  // Whether to restart interrupted syscall (SA_RESTART)
    },
}

/// Check and prepare signal delivery
/// Called before returning to userspace
pub fn check_signals(state: &SignalState) -> SignalDeliveryResult {
    // Get next deliverable signal
    let (sig, info) = match state.dequeue_signal() {
        Some(s) => s,
        None => return SignalDeliveryResult::None,
    };

    // Get action
    let action = state.get_action(sig);

    // Handle based on action
    if action.handler == SIG_IGN {
        // Ignored
        return SignalDeliveryResult::None;
    }

    if action.handler == SIG_DFL {
        // Default action
        match default_action(sig) {
            DefaultAction::Term | DefaultAction::Core => {
                return SignalDeliveryResult::Terminate(sig);
            }
            DefaultAction::Stop => {
                return SignalDeliveryResult::Stop(sig);
            }
            DefaultAction::Cont => {
                return SignalDeliveryResult::Continue;
            }
            DefaultAction::Ign => {
                return SignalDeliveryResult::None;
            }
        }
    }

    // User handler - check flags
    let use_siginfo = (action.flags.0 & SigActionFlags::SA_SIGINFO) != 0;
    let restart_syscall = (action.flags.0 & SigActionFlags::SA_RESTART) != 0;

    SignalDeliveryResult::Handle {
        signal: sig,
        info,
        action,
        use_siginfo,
        restart_syscall,
    }
}

/// Send signal to a process
pub fn kill(pid: usize, sig: Signal) -> Result<(), ()> {
    // This needs to interact with the process table
    // For now, just a stub that would be called from syscall handler
    let _ = (pid, sig);
    
    // TODO: Find process by PID and call state.send_signal(sig)
    
    Ok(())
}

// ============================================================================
// Signal-related System Calls Support
// ============================================================================

/// sigaction syscall support
pub fn sys_sigaction(
    state: &SignalState,
    sig: Signal,
    new_action: Option<&SigAction>,
    old_action: Option<&mut SigAction>,
) -> Result<(), ()> {
    // Get old action
    if let Some(old) = old_action {
        *old = state.get_action(sig);
    }
    
    // Set new action
    if let Some(new) = new_action {
        state.set_action(sig, *new)?;
    }
    
    Ok(())
}

/// sigprocmask syscall support
pub fn sys_sigprocmask(
    state: &SignalState,
    how: i32,
    set: Option<&SigSet>,
    oldset: Option<&mut SigSet>,
) -> Result<(), ()> {
    // Get old mask
    if let Some(old) = oldset {
        *old = state.get_mask();
    }
    
    // Set new mask
    if let Some(new) = set {
        match how {
            SIG_BLOCK => { state.block(*new); }
            SIG_UNBLOCK => { state.unblock(*new); }
            SIG_SETMASK => { state.set_mask(*new); }
            _ => return Err(()),
        }
    }
    
    Ok(())
}

/// sigsuspend syscall support
pub fn sys_sigsuspend(state: &SignalState, mask: &SigSet) -> Result<(), ()> {
    state.suspend(*mask);
    
    // Would need to sleep until signal arrives
    // This is where we'd call scheduler
    
    state.restore_mask();
    
    // sigsuspend always returns -1 with EINTR
    Err(())
}

/// sigpending syscall support
pub fn sys_sigpending(state: &SignalState) -> SigSet {
    SigSet::from_bits(state.pending.load(Ordering::Acquire))
}

// ============================================================================
// Enhanced Signal Functions
// ============================================================================

impl SignalState {
    /// Send a signal with enhanced semantics
    pub fn send_signal_enhanced(&self, sig: Signal, info: SigInfo, source_pid: i32) -> Result<(), SignalError> {
        if sig == 0 || sig >= NSIG as u32 {
            return Err(SignalError::InvalidSignal);
        }

        {
            let mut stats = self.delivery_stats.lock();
            stats.total_sent += 1;
        }

        let pid = crate::process::myproc().unwrap_or(0);
        if crate::syscalls::glib::deliver_signal_to_signalfd(pid, sig, info) {
            return Ok(());
        }

        let safety_level = self.get_signal_safety_level();
        let in_critical_section = self.is_in_critical_section();
        let priority = self.calculate_signal_priority(sig, &info);
        let is_realtime = sig >= SIGRTMIN && sig <= SIGRTMAX;

        let delivery = SignalDelivery {
            signal: sig,
            info,
            priority,
            timestamp: crate::subsystems::time::timestamp_nanos(),
            source_pid,
            is_realtime,
        };

        let should_defer = self.should_defer_signal(&delivery, safety_level, in_critical_section);

        if should_defer {
            self.defer_signal(delivery, self.get_deferral_reason(safety_level, in_critical_section));
            return Ok(());
        }

        self.queue_signal_for_delivery(delivery);
        self.wakeup_process_if_needed();

        Ok(())
    }

    fn calculate_signal_priority(&self, sig: Signal, _info: &SigInfo) -> u8 {
        if sig >= SIGRTMIN && sig <= SIGRTMAX {
            return (sig - SIGRTMIN) as u8;
        }

        match sig {
            SIGKILL | SIGSTOP => 255,
            SIGSEGV | SIGBUS | SIGFPE | SIGILL => 200,
            SIGINT | SIGQUIT => 150,
            SIGTERM | SIGHUP => 100,
            SIGCHLD => 50,
            SIGALRM | SIGVTALRM | SIGPROF => 25,
            _ => 0,
        }
    }

    fn should_defer_signal(&self, delivery: &SignalDelivery, safety_level: SignalSafetyLevel, in_critical_section: bool) -> bool {
        if safety_level == SignalSafetyLevel::Unsafe {
            return true;
        }

        if in_critical_section && !self.is_critical_section_interruptible(delivery.signal) {
            return true;
        }

        if let Some(ref ctx) = *self.handler_context.lock() {
            if ctx.signal == delivery.signal {
                return true;
            }
        }

        let current_mask = self.get_mask();
        if current_mask.contains(delivery.signal) {
            return true;
        }

        {
            let queue = self.delivery_queue.lock();
            if queue.len() >= MAX_SIGNAL_QUEUE_SIZE {
                return true;
            }
        }

        false
    }

    fn is_critical_section_interruptible(&self, sig: Signal) -> bool {
        match sig {
            SIGKILL | SIGSTOP => true,
            SIGSEGV | SIGBUS | SIGFPE | SIGILL => true,
            _ => false,
        }
    }

    fn get_deferral_reason(&self, safety_level: SignalSafetyLevel, in_critical_section: bool) -> DeferralReason {
        if safety_level == SignalSafetyLevel::Unsafe {
            return DeferralReason::SignalUnsafe;
        }

        if in_critical_section {
            return DeferralReason::CriticalSection;
        }

        let current_mask = self.get_mask();
        if !current_mask.is_empty() {
            return DeferralReason::Blocked;
        }

        {
            let queue = self.delivery_queue.lock();
            if queue.len() >= MAX_SIGNAL_QUEUE_SIZE {
                return DeferralReason::QueueFull;
            }
        }

        if let Some(_) = *self.handler_context.lock() {
            return DeferralReason::HandlerActive;
        }

        DeferralReason::SignalUnsafe
    }

    fn defer_signal(&self, delivery: SignalDelivery, reason: DeferralReason) {
        let deferred = DeferredSignal {
            signal: delivery.signal,
            info: delivery.info,
            reason,
            timestamp: delivery.timestamp,
        };

        self.pending_deferred.lock().push_back(deferred);

        {
            let mut stats = self.delivery_stats.lock();
            stats.total_blocked += 1;
        }
    }

    fn queue_signal_for_delivery(&self, delivery: SignalDelivery) {
        let mut queue = self.delivery_queue.lock();

        let mut insert_pos = queue.len();
        for (i, existing) in queue.iter().enumerate() {
            if delivery.priority > existing.priority {
                insert_pos = i;
                break;
            }
        }

        queue.insert(insert_pos, delivery);
    }

    fn wakeup_process_if_needed(&self) {
        let pid = crate::process::myproc().unwrap_or(0);

        let mut proc_table = crate::process::manager::PROC_TABLE.lock();
        if let Some(proc) = proc_table.find_mut(pid) {
            if proc.state == crate::process::ProcState::Sleeping {
                proc.state = crate::process::ProcState::Runnable;
            }
        }
    }

    pub fn process_pending_signals(&self) -> Vec<SignalDelivery> {
        let mut ready_signals = Vec::new();

        {
            let mut deferred = self.pending_deferred.lock();
            let mut i = 0;
            while i < deferred.len() {
                let def_signal = &deferred[i];

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

        {
            let mut queue = self.delivery_queue.lock();
            while let Some(delivery) = queue.pop_front() {
                if self.can_deliver_signal(&delivery) {
                    ready_signals.push(delivery);
                } else {
                    queue.push_front(delivery);
                    break;
                }
            }
        }

        ready_signals
    }

    fn can_deliver_deferred_signal(&self, def_signal: &DeferredSignal) -> bool {
        match def_signal.reason {
            DeferralReason::Blocked => {
                let current_mask = self.get_mask();
                !current_mask.contains(def_signal.signal)
            }
            DeferralReason::CriticalSection => {
                !self.is_in_critical_section()
            }
            DeferralReason::QueueFull => {
                let queue = self.delivery_queue.lock();
                queue.len() < MAX_SIGNAL_QUEUE_SIZE
            }
            DeferralReason::HandlerActive => {
                self.handler_context.lock().is_none()
            }
            DeferralReason::SignalUnsafe => {
                self.get_signal_safety_level() != SignalSafetyLevel::Unsafe
            }
        }
    }

    fn can_deliver_signal(&self, delivery: &SignalDelivery) -> bool {
        let safety_level = self.get_signal_safety_level();
        if safety_level == SignalSafetyLevel::Unsafe {
            return false;
        }

        if self.is_in_critical_section() && !self.is_critical_section_interruptible(delivery.signal) {
            return false;
        }

        let current_mask = self.get_mask();
        if current_mask.contains(delivery.signal) {
            return false;
        }

        if let Some(ref ctx) = *self.handler_context.lock() {
            if ctx.signal == delivery.signal {
                return false;
            }
        }

        true
    }

    pub fn deliver_signal(&self, delivery: &SignalDelivery) -> Result<(), SignalError> {
        let action = self.get_action(delivery.signal);

        if action.handler == SIG_IGN {
            {
                let mut stats = self.delivery_stats.lock();
                stats.total_ignored += 1;
            }
            return Ok(());
        }

        if action.handler == SIG_DFL {
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
                    {
                        let mut stats = self.delivery_stats.lock();
                        stats.total_ignored += 1;
                    }
                    return Ok(());
                }
            }
        }

        self.setup_signal_handler(delivery, &action)?;

        {
            let mut stats = self.delivery_stats.lock();
            stats.total_delivered += 1;

            let now = crate::subsystems::time::timestamp_nanos();
            let latency_us = (now - delivery.timestamp) / 1000;
            stats.avg_delivery_latency_us =
                (stats.avg_delivery_latency_us * (stats.total_delivered - 1) as f64 + latency_us as f64)
                / stats.total_delivered as f64;
            stats.max_delivery_latency_us = stats.max_delivery_latency_us.max(latency_us);
        }

        Ok(())
    }

    fn setup_signal_handler(&self, delivery: &SignalDelivery, action: &SigAction) -> Result<(), SignalError> {
        let current_mask = self.get_mask();

        {
            let mut mask_stack = self.mask_stack.lock();
            mask_stack.push(current_mask);
        }

        let mut handler_mask = action.mask;
        if (action.flags.0 & SigActionFlags::SA_NODEFER) == 0 {
            handler_mask.add(delivery.signal);
        }

        self.set_mask(handler_mask);

        let handler_context = SignalHandlerContext {
            signal: delivery.signal,
            handler_address: action.handler,
            handler_mask,
            use_siginfo: (action.flags.0 & SigActionFlags::SA_SIGINFO) != 0,
            restart_syscalls: (action.flags.0 & SigActionFlags::SA_RESTART) != 0,
            entry_timestamp: crate::subsystems::time::timestamp_nanos(),
        };

        *self.handler_context.lock() = Some(handler_context);

        self.setup_signal_frame(delivery, action)?;

        Ok(())
    }

    fn setup_signal_frame(&self, delivery: &SignalDelivery, action: &SigAction) -> Result<(), SignalError> {
        let pid = crate::process::myproc().ok_or(SignalError::ProcessNotFound)?;
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = proc_table.find_ref(pid).ok_or(SignalError::ProcessNotFound)?;
        let pagetable = proc.pagetable;
        drop(proc_table);

        if pagetable.is_null() {
            return Err(SignalError::InvalidAddress);
        }

        let use_alt_stack = (action.flags.0 & SigActionFlags::SA_ONSTACK) != 0;

        let _sp = if use_alt_stack {
            self.get_alternate_stack_address()
        } else {
            self.get_current_stack_pointer()
        };

        let _frame = SignalFrame {
            info: delivery.info,
            context: self.get_current_context(),
            handler: action.handler,
            use_siginfo: (action.flags.0 & SigActionFlags::SA_SIGINFO) != 0,
            return_address: self.get_signal_return_address(),
        };

        crate::println!("Setting up signal frame for signal {}", delivery.signal);

        Ok(())
    }

    fn get_alternate_stack_address(&self) -> usize {
        0x7ffff0000000usize
    }

    fn get_current_stack_pointer(&self) -> usize {
        0x7fffffffe000usize
    }

    fn get_current_context(&self) -> SignalContext {
        SignalContext {
            saved_regs: SavedRegs::default(),
            info: SigInfo::default(),
            saved_mask: 0,
        }
    }

    fn get_signal_return_address(&self) -> usize {
        0x7fffff000000usize
    }

    fn terminate_process(&self, sig: Signal, info: SigInfo) {
        let pid = crate::process::myproc().unwrap_or(0);

        if default_action(sig) == DefaultAction::Core {
            self.create_core_dump(sig, info);
        }

        crate::process::manager::exit_process(pid, sig as i32);
    }

    fn stop_process(&self, sig: Signal) {
        let pid = crate::process::myproc().unwrap_or(0);
        crate::process::manager::stop_process(pid, sig);
    }

    fn continue_process(&self) {
        let pid = crate::process::myproc().unwrap_or(0);
        crate::process::manager::continue_process(pid);
    }

    fn create_core_dump(&self, sig: Signal, info: SigInfo) {
        crate::println!("Creating core dump for signal {} from pid {}", sig, info.pid);
    }

    pub fn return_from_handler(&self) -> Result<(), SignalError> {
        {
            let mut mask_stack = self.mask_stack.lock();
            if let Some(old_mask) = mask_stack.pop() {
                self.set_mask(old_mask);
            }
        }

        *self.handler_context.lock() = None;

        Ok(())
    }

    pub fn get_signal_safety_level(&self) -> SignalSafetyLevel {
        match self.signal_safety.load(Ordering::Relaxed) {
            0 => SignalSafetyLevel::Unsafe,
            1 => SignalSafetyLevel::Safe,
            2 => SignalSafetyLevel::Critical,
            _ => SignalSafetyLevel::Safe,
        }
    }

    pub fn set_signal_safety_level(&self, level: SignalSafetyLevel) {
        self.signal_safety.store(level as usize, Ordering::Relaxed);
    }

    pub fn is_in_critical_section(&self) -> bool {
        (self.delivery_flags.load(Ordering::Relaxed) & delivery_flags::IN_CRITICAL_SECTION) != 0
    }

    pub fn enter_critical_section(&self) {
        self.delivery_flags.fetch_or(delivery_flags::IN_CRITICAL_SECTION, Ordering::Relaxed);
    }

    pub fn exit_critical_section(&self) {
        self.delivery_flags.fetch_and(!delivery_flags::IN_CRITICAL_SECTION, Ordering::Relaxed);
    }

    pub fn get_delivery_stats(&self) -> SignalDeliveryStats {
        self.delivery_stats.lock().clone()
    }

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
