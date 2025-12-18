//! Context Switch Implementation
//!
//! This module provides efficient context switching between threads/processes.
//! Implements architecture-specific optimizations for fast context switches.

use core::arch::asm;
use crate::subsystems::process::{Context, Thread};

/// Context switch error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextSwitchError {
    /// Invalid context pointer
    InvalidContext,
    /// Stack corruption detected
    StackCorruption,
    /// Invalid thread state
    InvalidThreadState,
}

/// Context switch statistics
#[derive(Debug, Default)]
pub struct ContextSwitchStats {
    /// Total number of context switches
    pub total_switches: u64,
    /// Total time spent in context switches (nanoseconds)
    pub total_switch_time_ns: u64,
    /// Average context switch time (nanoseconds)
    pub avg_switch_time_ns: u64,
    /// Fast path switches (optimized)
    pub fast_path_switches: u64,
    /// Slow path switches (full save/restore)
    pub slow_path_switches: u64,
}

impl ContextSwitchStats {
    /// Record a context switch
    pub fn record_switch(&mut self, elapsed_ns: u64, is_fast_path: bool) {
        self.total_switches += 1;
        self.total_switch_time_ns += elapsed_ns;
        self.avg_switch_time_ns = self.total_switch_time_ns / self.total_switches;
        
        if is_fast_path {
            self.fast_path_switches += 1;
        } else {
            self.slow_path_switches += 1;
        }
    }
    
    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Global context switch statistics
static mut CONTEXT_SWITCH_STATS: Option<ContextSwitchStats> = None;
static CONTEXT_SWITCH_STATS_INIT: crate::sync::Once = crate::sync::Once::new();

/// Get context switch statistics
pub fn get_context_switch_stats() -> &'static mut ContextSwitchStats {
    unsafe {
        CONTEXT_SWITCH_STATS_INIT.call_once(|| {
            CONTEXT_SWITCH_STATS = Some(ContextSwitchStats::default());
        });
        CONTEXT_SWITCH_STATS.as_mut().unwrap()
    }
}

/// Perform context switch from current thread to next thread
/// 
/// # Arguments
/// * `current` - Mutable reference to current thread's context
/// * `next` - Immutable reference to next thread's context
/// 
/// # Returns
/// * `Ok(())` on successful context switch
/// * `Err(ContextSwitchError)` on failure
/// 
/// # Safety
/// This function is unsafe as it manipulates CPU registers directly.
/// Callers must ensure:
/// - Both contexts are valid and properly initialized
/// - The current context points to the currently executing thread
/// - The next context points to a thread ready to run
/// - Stack pointers are valid and within allocated stack memory
pub unsafe fn context_switch(current: &mut Context, next: &Context) -> Result<(), ContextSwitchError> {
    // Validate contexts
    if current as *const _ == next as *const _ {
        return Err(ContextSwitchError::InvalidContext);
    }
    
    // Record start time for statistics
    let start_time = crate::time::timestamp_nanos();
    
    // Perform architecture-specific context switch
    #[cfg(target_arch = "x86_64")]
    {
        x86_64_context_switch(current, next)?;
    }
    
    #[cfg(target_arch = "riscv64")]
    {
        riscv64_context_switch(current, next)?;
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        aarch64_context_switch(current, next)?;
    }
    
    // Record statistics
    let elapsed = crate::time::timestamp_nanos().saturating_sub(start_time);
    let stats = get_context_switch_stats();
    stats.record_switch(elapsed, false); // Full context switch is slow path
    
    Ok(())
}

/// Fast path context switch for same process threads
/// 
/// This optimized version skips certain steps when switching between
/// threads of the same process (e.g., same page table).
/// 
/// # Arguments
/// * `current` - Mutable reference to current thread's context
/// * `next` - Immutable reference to next thread's context
/// * `same_process` - Whether both threads belong to the same process
/// 
/// # Returns
/// * `Ok(())` on successful context switch
/// * `Err(ContextSwitchError)` on failure
pub unsafe fn fast_context_switch(
    current: &mut Context, 
    next: &Context, 
    same_process: bool
) -> Result<(), ContextSwitchError> {
    // Validate contexts
    if current as *const _ == next as *const _ {
        return Err(ContextSwitchError::InvalidContext);
    }
    
    // Record start time for statistics
    let start_time = crate::time::timestamp_nanos();
    
    // Perform optimized context switch
    #[cfg(target_arch = "x86_64")]
    {
        x86_64_fast_context_switch(current, next, same_process)?;
    }
    
    #[cfg(target_arch = "riscv64")]
    {
        riscv64_fast_context_switch(current, next, same_process)?;
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        aarch64_fast_context_switch(current, next, same_process)?;
    }
    
    // Record statistics
    let elapsed = crate::time::timestamp_nanos().saturating_sub(start_time);
    let stats = get_context_switch_stats();
    stats.record_switch(elapsed, true); // Fast path switch
    
    Ok(())
}

/// Initialize a new context for a thread
/// 
/// # Arguments
/// * `context` - Mutable reference to the context to initialize
/// * `stack_top` - Top of the stack for this thread
/// * `entry_point` - Entry point function address
/// * `arg` - Argument to pass to the entry point
/// * `is_user` - Whether this is a user thread (affects privilege levels)
pub fn init_context(
    context: &mut Context,
    stack_top: usize,
    entry_point: usize,
    arg: usize,
    is_user: bool,
) {
    #[cfg(target_arch = "x86_64")]
    {
        // Set up stack pointer and instruction pointer
        context.rsp = stack_top;
        context.rip = entry_point;
        
        // Pass argument in RDI (first integer argument register)
        // We'll need to modify the trap frame to pass this argument
        // when the thread first starts
    }
    
    #[cfg(target_arch = "riscv64")]
    {
        // Set up stack pointer and return address
        context.sp = stack_top;
        context.ra = entry_point;
        
        // Pass argument in a0 (first argument register)
        // We'll need to modify the trap frame to pass this argument
        // when the thread first starts
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        // Set up stack pointer and link register
        context.sp = stack_top;
        context.lr = entry_point;
        
        // Pass argument in x0 (first argument register)
        // We'll need to modify the trap frame to pass this argument
        // when the thread first starts
    }
}

/// Architecture-specific implementations

#[cfg(target_arch = "x86_64")]
unsafe fn x86_64_context_switch(current: &mut Context, next: &Context) -> Result<(), ContextSwitchError> {
    // Validate stack pointers
    if current.rsp == 0 || next.rsp == 0 {
        return Err(ContextSwitchError::StackCorruption);
    }
    
    // Save current context and restore next context
    // Using inline assembly for direct register manipulation
    asm!(
        // Save current registers to current context
        "mov [rdi + 0x00], rbx",    // Save RBX
        "mov [rdi + 0x08], rbp",    // Save RBP
        "mov [rdi + 0x10], r12",    // Save R12
        "mov [rdi + 0x18], r13",    // Save R13
        "mov [rdi + 0x20], r14",    // Save R14
        "mov [rdi + 0x28], r15",    // Save R15
        "mov [rdi + 0x30], rsp",    // Save RSP
        "mov [rdi + 0x38], rip",    // Save RIP
        
        // Restore registers from next context
        "mov rbx, [rsi + 0x00]",    // Restore RBX
        "mov rbp, [rsi + 0x08]",    // Restore RBP
        "mov r12, [rsi + 0x10]",    // Restore R12
        "mov r13, [rsi + 0x18]",    // Restore R13
        "mov r14, [rsi + 0x20]",    // Restore R14
        "mov r15, [rsi + 0x28]",    // Restore R15
        "mov rsp, [rsi + 0x30]",    // Restore RSP
        "mov rip, [rsi + 0x38]",    // Restore RIP
        
        // Input/output operands
        in("rdi") current as *mut Context,
        in("rsi") next as *const Context,
        
        // Clobbers
        options(nostack, preserves_flags)
    );
    
    Ok(())
}

#[cfg(target_arch = "x86_64")]
unsafe fn x86_64_fast_context_switch(
    current: &mut Context, 
    next: &Context, 
    same_process: bool
) -> Result<(), ContextSwitchError> {
    // Validate stack pointers
    if current.rsp == 0 || next.rsp == 0 {
        return Err(ContextSwitchError::StackCorruption);
    }
    
    // For same process threads, we can skip page table switches
    // and potentially other process-specific state
    
    // Optimized context switch with fewer instructions
    asm!(
        // Save only essential registers
        "mov [rdi + 0x00], rbx",    // Save RBX
        "mov [rdi + 0x08], rbp",    // Save RBP
        "mov [rdi + 0x30], rsp",    // Save RSP
        
        // Restore essential registers
        "mov rbx, [rsi + 0x00]",    // Restore RBX
        "mov rbp, [rsi + 0x08]",    // Restore RBP
        "mov rsp, [rsi + 0x30]",    // Restore RSP
        
        // Jump to next context (more efficient than call/ret)
        "jmp [rsi + 0x38]",         // Jump to RIP
        
        // Input/output operands
        in("rdi") current as *mut Context,
        in("rsi") next as *const Context,
        
        // Clobbers
        options(nostack, preserves_flags, noreturn)
    );
}

#[cfg(target_arch = "riscv64")]
unsafe fn riscv64_context_switch(current: &mut Context, next: &Context) -> Result<(), ContextSwitchError> {
    // Validate stack pointers
    if current.sp == 0 || next.sp == 0 {
        return Err(ContextSwitchError::StackCorruption);
    }
    
    // Save current context and restore next context
    asm!(
        // Save current registers to current context
        "sd ra, 0(a0)",     // Save RA
        "sd sp, 8(a0)",     // Save SP
        "sd s0, 16(a0)",    // Save S0
        "sd s1, 24(a0)",    // Save S1
        "sd s2, 32(a0)",    // Save S2
        "sd s3, 40(a0)",    // Save S3
        "sd s4, 48(a0)",    // Save S4
        "sd s5, 56(a0)",    // Save S5
        "sd s6, 64(a0)",    // Save S6
        "sd s7, 72(a0)",    // Save S7
        "sd s8, 80(a0)",    // Save S8
        "sd s9, 88(a0)",    // Save S9
        "sd s10, 96(a0)",   // Save S10
        "sd s11, 104(a0)",  // Save S11
        
        // Restore registers from next context
        "ld ra, 0(a1)",     // Restore RA
        "ld sp, 8(a1)",     // Restore SP
        "ld s0, 16(a1)",    // Restore S0
        "ld s1, 24(a1)",    // Restore S1
        "ld s2, 32(a1)",    // Restore S2
        "ld s3, 40(a1)",    // Restore S3
        "ld s4, 48(a1)",    // Restore S4
        "ld s5, 56(a1)",    // Restore S5
        "ld s6, 64(a1)",    // Restore S6
        "ld s7, 72(a1)",    // Restore S7
        "ld s8, 80(a1)",    // Restore S8
        "ld s9, 88(a1)",    // Restore S9
        "ld s10, 96(a1)",   // Restore S10
        "ld s11, 104(a1)",  // Restore S11
        
        // Input/output operands
        in("a0") current as *mut Context,
        in("a1") next as *const Context,
        
        // Clobbers
        options(nostack, preserves_flags)
    );
    
    Ok(())
}

#[cfg(target_arch = "riscv64")]
unsafe fn riscv64_fast_context_switch(
    current: &mut Context, 
    next: &Context, 
    same_process: bool
) -> Result<(), ContextSwitchError> {
    // Validate stack pointers
    if current.sp == 0 || next.sp == 0 {
        return Err(ContextSwitchError::StackCorruption);
    }
    
    // Optimized context switch with fewer instructions
    asm!(
        // Save only essential registers
        "sd ra, 0(a0)",     // Save RA
        "sd sp, 8(a0)",     // Save SP
        "sd s0, 16(a0)",    // Save S0
        "sd s1, 24(a0)",    // Save S1
        
        // Restore essential registers
        "ld ra, 0(a1)",     // Restore RA
        "ld sp, 8(a1)",     // Restore SP
        "ld s0, 16(a1)",    // Restore S0
        "ld s1, 24(a1)",    // Restore S1
        
        // Jump to next context
        "jr ra",            // Jump to RA
        
        // Input/output operands
        in("a0") current as *mut Context,
        in("a1") next as *const Context,
        
        // Clobbers
        options(nostack, preserves_flags, noreturn)
    );
}

#[cfg(target_arch = "aarch64")]
unsafe fn aarch64_context_switch(current: &mut Context, next: &Context) -> Result<(), ContextSwitchError> {
    // Validate stack pointers
    if current.sp == 0 || next.sp == 0 {
        return Err(ContextSwitchError::StackCorruption);
    }
    
    // Save current context and restore next context
    asm!(
        // Save current registers to current context
        "stp x19, x20, [x0, #0]",   // Save X19, X20
        "stp x21, x22, [x0, #16]",  // Save X21, X22
        "stp x23, x24, [x0, #32]",  // Save X23, X24
        "stp x25, x26, [x0, #48]",  // Save X25, X26
        "stp x27, x28, [x0, #64]",  // Save X27, X28
        "stp x29, x30, [x0, #80]",  // Save FP (X29), LR (X30)
        "str xzr, [x0, #96]",       // Clear padding
        "str sp, [x0, #104]",       // Save SP
        
        // Restore registers from next context
        "ldp x19, x20, [x1, #0]",   // Restore X19, X20
        "ldp x21, x22, [x1, #16]",  // Restore X21, X22
        "ldp x23, x24, [x1, #32]",  // Restore X23, X24
        "ldp x25, x26, [x1, #48]",  // Restore X25, X26
        "ldp x27, x28, [x1, #64]",  // Restore X27, X28
        "ldp x29, x30, [x1, #80]",  // Restore FP (X29), LR (X30)
        "ldr sp, [x1, #104]",       // Restore SP
        
        // Input/output operands
        in("x0") current as *mut Context,
        in("x1") next as *const Context,
        
        // Clobbers
        options(nostack, preserves_flags)
    );
    
    Ok(())
}

#[cfg(target_arch = "aarch64")]
unsafe fn aarch64_fast_context_switch(
    current: &mut Context, 
    next: &Context, 
    same_process: bool
) -> Result<(), ContextSwitchError> {
    // Validate stack pointers
    if current.sp == 0 || next.sp == 0 {
        return Err(ContextSwitchError::StackCorruption);
    }
    
    // Optimized context switch with fewer instructions
    asm!(
        // Save only essential registers
        "stp x29, x30, [x0, #80]",  // Save FP (X29), LR (X30)
        "str sp, [x0, #104]",       // Save SP
        
        // Restore essential registers
        "ldp x29, x30, [x1, #80]",  // Restore FP (X29), LR (X30)
        "ldr sp, [x1, #104]",       // Restore SP
        
        // Jump to next context
        "ret x30",                  // Return to LR
        
        // Input/output operands
        in("x0") current as *mut Context,
        in("x1") next as *const Context,
        
        // Clobbers
        options(nostack, preserves_flags, noreturn)
    );
}

/// Get the current context pointer
/// 
/// This function returns a pointer to the current thread's context.
/// It's typically used in interrupt handlers and system calls.
/// 
/// # Returns
/// * `Some(&mut Context)` if there's a current thread
/// * `None` if no current thread (e.g., early boot)
pub fn get_current_context() -> Option<&'static mut Context> {
    crate::subsystems::process::thread::get_current_thread_mut()
        .map(|thread| &mut thread.context)
}

/// Save current context to a buffer
/// 
/// This function saves the current CPU context to a buffer.
/// It's typically used in interrupt handlers.
/// 
/// # Arguments
/// * `context` - Mutable reference to the context to save to
/// 
/// # Returns
/// * `Ok(())` on success
/// * `Err(ContextSwitchError)` on failure
pub unsafe fn save_current_context(context: &mut Context) -> Result<(), ContextSwitchError> {
    #[cfg(target_arch = "x86_64")]
    {
        asm!(
            // Save current registers to context
            "mov [rdi + 0x00], rbx",    // Save RBX
            "mov [rdi + 0x08], rbp",    // Save RBP
            "mov [rdi + 0x10], r12",    // Save R12
            "mov [rdi + 0x18], r13",    // Save R13
            "mov [rdi + 0x20], r14",    // Save R14
            "mov [rdi + 0x28], r15",    // Save R15
            "mov [rdi + 0x30], rsp",    // Save RSP
            "lea rax, [rip + 1]",       // Get RIP
            "mov [rdi + 0x38], rax",    // Save RIP
            
            // Input/output operands
            in("rdi") context as *mut Context,
            
            // Clobbers
            options(nostack, preserves_flags)
        );
    }
    
    #[cfg(target_arch = "riscv64")]
    {
        asm!(
            // Save current registers to context
            "sd ra, 0(a0)",     // Save RA
            "sd sp, 8(a0)",     // Save SP
            "sd s0, 16(a0)",    // Save S0
            "sd s1, 24(a0)",    // Save S1
            "sd s2, 32(a0)",    // Save S2
            "sd s3, 40(a0)",    // Save S3
            "sd s4, 48(a0)",    // Save S4
            "sd s5, 56(a0)",    // Save S5
            "sd s6, 64(a0)",    // Save S6
            "sd s7, 72(a0)",    // Save S7
            "sd s8, 80(a0)",    // Save S8
            "sd s9, 88(a0)",    // Save S9
            "sd s10, 96(a0)",   // Save S10
            "sd s11, 104(a0)",  // Save S11
            
            // Input/output operands
            in("a0") context as *mut Context,
            
            // Clobbers
            options(nostack, preserves_flags)
        );
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        asm!(
            // Save current registers to context
            "stp x19, x20, [x0, #0]",   // Save X19, X20
            "stp x21, x22, [x0, #16]",  // Save X21, X22
            "stp x23, x24, [x0, #32]",  // Save X23, X24
            "stp x25, x26, [x0, #48]",  // Save X25, X26
            "stp x27, x28, [x0, #64]",  // Save X27, X28
            "stp x29, x30, [x0, #80]",  // Save FP (X29), LR (X30)
            "str xzr, [x0, #96]",       // Clear padding
            "str sp, [x0, #104]",       // Save SP
            
            // Input/output operands
            in("x0") context as *mut Context,
            
            // Clobbers
            options(nostack, preserves_flags)
        );
    }
    
    Ok(())
}

/// Restore context from a buffer
/// 
/// This function restores the CPU context from a buffer.
/// It's typically used to return from interrupt handlers.
/// 
/// # Arguments
/// * `context` - Immutable reference to the context to restore from
/// 
/// # Returns
/// This function does not return if successful
pub unsafe fn restore_context(context: &Context) -> ! {
    #[cfg(target_arch = "x86_64")]
    {
        asm!(
            // Restore registers from context
            "mov rbx, [rdi + 0x00]",    // Restore RBX
            "mov rbp, [rdi + 0x08]",    // Restore RBP
            "mov r12, [rdi + 0x10]",    // Restore R12
            "mov r13, [rdi + 0x18]",    // Restore R13
            "mov r14, [rdi + 0x20]",    // Restore R14
            "mov r15, [rdi + 0x28]",    // Restore R15
            "mov rsp, [rdi + 0x30]",    // Restore RSP
            "mov rax, [rdi + 0x38]",    // Get RIP
            "jmp rax",                  // Jump to RIP
            
            // Input/output operands
            in("rdi") context as *const Context,
            
            // Clobbers
            options(nostack, preserves_flags, noreturn)
        );
    }
    
    #[cfg(target_arch = "riscv64")]
    {
        asm!(
            // Restore registers from context
            "ld ra, 0(a0)",     // Restore RA
            "ld sp, 8(a0)",     // Restore SP
            "ld s0, 16(a0)",    // Restore S0
            "ld s1, 24(a0)",    // Restore S1
            "ld s2, 32(a0)",    // Restore S2
            "ld s3, 40(a0)",    // Restore S3
            "ld s4, 48(a0)",    // Restore S4
            "ld s5, 56(a0)",    // Restore S5
            "ld s6, 64(a0)",    // Restore S6
            "ld s7, 72(a0)",    // Restore S7
            "ld s8, 80(a0)",    // Restore S8
            "ld s9, 88(a0)",    // Restore S9
            "ld s10, 96(a0)",   // Restore S10
            "ld s11, 104(a0)",  // Restore S11
            
            // Jump to restored context
            "ret",              // Return to RA
            
            // Input/output operands
            in("a0") context as *const Context,
            
            // Clobbers
            options(nostack, preserves_flags, noreturn)
        );
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        asm!(
            // Restore registers from context
            "ldp x19, x20, [x0, #0]",   // Restore X19, X20
            "ldp x21, x22, [x0, #16]",  // Restore X21, X22
            "ldp x23, x24, [x0, #32]",  // Restore X23, X24
            "ldp x25, x26, [x0, #48]",  // Restore X25, X26
            "ldp x27, x28, [x0, #64]",  // Restore X27, X28
            "ldp x29, x30, [x0, #80]",  // Restore FP (X29), LR (X30)
            "ldr sp, [x0, #104]",       // Restore SP
            
            // Jump to restored context
            "ret x30",                  // Return to LR
            
            // Input/output operands
            in("x0") context as *const Context,
            
            // Clobbers
            options(nostack, preserves_flags, noreturn)
        );
    }
}

/// Initialize the context switch subsystem
pub fn init() {
    // Initialize statistics
    get_context_switch_stats();
    
    crate::println!("context_switch: Context switch subsystem initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_context_init() {
        let mut context = Context::new();
        let stack_top = 0x80000000;
        let entry_point = 0x100000;
        let arg = 0x42;
        
        init_context(&mut context, stack_top, entry_point, arg, false);
        
        #[cfg(target_arch = "x86_64")]
        {
            assert_eq!(context.rsp, stack_top);
            assert_eq!(context.rip, entry_point);
        }
        
        #[cfg(target_arch = "riscv64")]
        {
            assert_eq!(context.sp, stack_top);
            assert_eq!(context.ra, entry_point);
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            assert_eq!(context.sp, stack_top);
            assert_eq!(context.lr, entry_point);
        }
    }
    
    #[test]
    fn test_context_switch_stats() {
        let mut stats = ContextSwitchStats::default();
        
        stats.record_switch(1000, true);
        assert_eq!(stats.total_switches, 1);
        assert_eq!(stats.total_switch_time_ns, 1000);
        assert_eq!(stats.avg_switch_time_ns, 1000);
        assert_eq!(stats.fast_path_switches, 1);
        assert_eq!(stats.slow_path_switches, 0);
        
        stats.record_switch(2000, false);
        assert_eq!(stats.total_switches, 2);
        assert_eq!(stats.total_switch_time_ns, 3000);
        assert_eq!(stats.avg_switch_time_ns, 1500);
        assert_eq!(stats.fast_path_switches, 1);
        assert_eq!(stats.slow_path_switches, 1);
        
        stats.reset();
        assert_eq!(stats.total_switches, 0);
        assert_eq!(stats.total_switch_time_ns, 0);
        assert_eq!(stats.avg_switch_time_ns, 0);
        assert_eq!(stats.fast_path_switches, 0);
        assert_eq!(stats.slow_path_switches, 0);
    }
}