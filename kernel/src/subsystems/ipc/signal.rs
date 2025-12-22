// Signal handling for processes
//
// Implements POSIX-like signals:
// - Signal delivery and handling
// - Signal masks and pending signals
// - Signal actions (default, ignore, custom handler)
// - Real-time signals

extern crate alloc;

use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU64, Ordering};

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
