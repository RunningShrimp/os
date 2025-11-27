//! Process management for xv6-rust
//! Provides process control block, context switching, and scheduler

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr;
use core::sync::atomic::AtomicUsize;

use crate::mm::{kalloc, kfree, PAGE_SIZE};
use crate::sync::Mutex;
use crate::vm::arch::PageTable;
use crate::signal::{SignalState, SignalDeliveryResult, check_signals};

/// Maximum number of processes
pub const NPROC: usize = 64;
/// Maximum number of open files per process
pub const NOFILE: usize = 16;

/// Process ID type
pub type Pid = usize;

/// Process states (xv6 compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProcState {
    Unused = 0,
    Used = 1,
    Sleeping = 2,
    Runnable = 3,
    Running = 4,
    Zombie = 5,
}

impl Default for ProcState {
    fn default() -> Self {
        Self::Unused
    }
}

// ============================================================================
// CPU Context for context switching
// ============================================================================

/// Saved registers for context switch (architecture-specific)
#[cfg(target_arch = "riscv64")]
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Context {
    pub ra: usize,    // Return address
    pub sp: usize,    // Stack pointer
    pub s0: usize,    // Saved registers
    pub s1: usize,
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
}

#[cfg(target_arch = "riscv64")]
impl Context {
    pub const fn new() -> Self {
        Self {
            ra: 0, sp: 0, s0: 0, s1: 0, s2: 0, s3: 0, s4: 0,
            s5: 0, s6: 0, s7: 0, s8: 0, s9: 0, s10: 0, s11: 0,
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Context {
    pub x19: usize,
    pub x20: usize,
    pub x21: usize,
    pub x22: usize,
    pub x23: usize,
    pub x24: usize,
    pub x25: usize,
    pub x26: usize,
    pub x27: usize,
    pub x28: usize,
    pub x29: usize,   // Frame pointer
    pub x30: usize,   // Link register (return address)
    pub sp: usize,    // Stack pointer
}

#[cfg(target_arch = "aarch64")]
impl Context {
    pub const fn new() -> Self {
        Self {
            x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0,
            x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, sp: 0,
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Context {
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub rbx: usize,
    pub rbp: usize,
    pub rip: usize,   // Return address
}

#[cfg(target_arch = "x86_64")]
impl Context {
    pub const fn new() -> Self {
        Self {
            r15: 0, r14: 0, r13: 0, r12: 0, rbx: 0, rbp: 0, rip: 0,
        }
    }
}

// ============================================================================
// Trap Frame - saved user registers on trap/syscall entry
// ============================================================================

#[cfg(target_arch = "riscv64")]
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TrapFrame {
    pub kernel_satp: usize,    // 0: kernel page table
    pub kernel_sp: usize,      // 8: kernel stack pointer
    pub kernel_trap: usize,    // 16: usertrap()
    pub epc: usize,            // 24: saved user program counter
    pub kernel_hartid: usize,  // 32: saved kernel tp
    pub ra: usize,             // 40
    pub sp: usize,             // 48
    pub gp: usize,             // 56
    pub tp: usize,             // 64
    pub t0: usize,             // 72
    pub t1: usize,             // 80
    pub t2: usize,             // 88
    pub s0: usize,             // 96
    pub s1: usize,             // 104
    pub a0: usize,             // 112
    pub a1: usize,             // 120
    pub a2: usize,             // 128
    pub a3: usize,             // 136
    pub a4: usize,             // 144
    pub a5: usize,             // 152
    pub a6: usize,             // 160
    pub a7: usize,             // 168
    pub s2: usize,             // 176
    pub s3: usize,             // 184
    pub s4: usize,             // 192
    pub s5: usize,             // 200
    pub s6: usize,             // 208
    pub s7: usize,             // 216
    pub s8: usize,             // 224
    pub s9: usize,             // 232
    pub s10: usize,            // 240
    pub s11: usize,            // 248
    pub t3: usize,             // 256
    pub t4: usize,             // 264
    pub t5: usize,             // 272
    pub t6: usize,             // 280
}

#[cfg(target_arch = "aarch64")]
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TrapFrame {
    pub regs: [usize; 31],     // x0-x30
    pub sp: usize,             // User stack pointer
    pub elr: usize,            // Exception link register (return address)
    pub spsr: usize,           // Saved program status register
}

#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TrapFrame {
    // Pushed by software
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rbp: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rbx: usize,
    pub rax: usize,
    // Pushed by hardware
    pub error_code: usize,
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
    pub rsp: usize,
    pub ss: usize,
}

// ============================================================================
// Process Control Block (PCB)
// ============================================================================

/// Process control block
/// SAFETY: Proc contains raw pointers that are only accessed from kernel context
pub struct Proc {
    // Process state
    pub state: ProcState,
    pub pid: Pid,
    pub parent: Option<Pid>,
    
    // Memory
    pub pagetable: *mut PageTable,
    pub kstack: usize,           // Kernel stack
    pub sz: usize,               // Size of process memory
    
    // Scheduling
    pub context: Context,
    pub trapframe: *mut TrapFrame,
    
    // Wait channel (for sleeping)
    pub chan: usize,
    pub killed: bool,
    pub xstate: i32,             // Exit status
    
    // File descriptors
    pub ofile: [Option<usize>; NOFILE],  // Open files (indices into global file table)
    pub cwd: Option<usize>,              // Current directory inode
    
    // Process name for debugging
    pub name: [u8; 16],
    
    // Signal handling (initialized lazily)
    pub signals: Option<SignalState>,
}

impl Default for Proc {
    fn default() -> Self {
        Self {
            state: ProcState::Unused,
            pid: 0,
            parent: None,
            pagetable: ptr::null_mut(),
            kstack: 0,
            sz: 0,
            context: Context::default(),
            trapframe: ptr::null_mut(),
            chan: 0,
            killed: false,
            xstate: 0,
            ofile: [None; NOFILE],
            cwd: None,
            name: [0; 16],
            signals: None,
        }
    }
}

// SAFETY: Proc is only accessed while holding the process table lock,
// and raw pointers are only dereferenced in controlled kernel context
unsafe impl Send for Proc {}
unsafe impl Sync for Proc {}

impl Proc {
    pub const fn new() -> Self {
        Self {
            state: ProcState::Unused,
            pid: 0,
            parent: None,
            pagetable: ptr::null_mut(),
            kstack: 0,
            sz: 0,
            context: Context::new(),
            trapframe: ptr::null_mut(),
            chan: 0,
            killed: false,
            xstate: 0,
            ofile: [None; NOFILE],
            cwd: None,
            name: [0; 16],
            signals: None,
        }
    }

    /// Set process name
    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(15);
        self.name[..len].copy_from_slice(&bytes[..len]);
        self.name[len] = 0;
    }

    /// Get process name as str
    pub fn name_str(&self) -> &str {
        let len = self.name.iter().position(|&c| c == 0).unwrap_or(16);
        core::str::from_utf8(&self.name[..len]).unwrap_or("")
    }
}

// File descriptor management functions

/// Allocate a file descriptor for the current process
pub fn fdalloc(file_idx: usize) -> Option<i32> {
    let mut table = PROC_TABLE.lock();
    if let Some(pid) = myproc() {
        if let Some(proc) = table.find(pid) {
            for (i, fd) in proc.ofile.iter_mut().enumerate() {
                if fd.is_none() {
                    *fd = Some(file_idx);
                    return Some(i as i32);
                }
            }
        }
    }
    None
}

/// Look up a file descriptor for the current process
pub fn fdlookup(fd: i32) -> Option<usize> {
    if fd < 0 || fd >= NOFILE as i32 {
        return None;
    }
    
    let mut table = PROC_TABLE.lock();
    if let Some(pid) = myproc() {
        if let Some(proc) = table.find(pid) {
            return proc.ofile[fd as usize];
        }
    }
    None
}

/// Close a file descriptor for the current process
pub fn fdclose(fd: i32) -> bool {
    if fd < 0 || fd >= NOFILE as i32 {
        return false;
    }
    
    let mut table = PROC_TABLE.lock();
    if let Some(pid) = myproc() {
        if let Some(proc) = table.find(pid) {
            if proc.ofile[fd as usize].is_some() {
                proc.ofile[fd as usize] = None;
                return true;
            }
        }
    }
    false
}

/// Install a file descriptor at a specific index
pub fn fdinstall(fd: i32, file_idx: usize) -> Result<(), ()> {
    if fd < 0 || fd >= NOFILE as i32 {
        return Err(());
    }
    
    let mut table = PROC_TABLE.lock();
    if let Some(pid) = myproc() {
        if let Some(proc) = table.find(pid) {
            proc.ofile[fd as usize] = Some(file_idx);
            return Ok(());
        }
    }
    Err(())
}

// ============================================================================
// Process Table
// ============================================================================

/// Global process table
pub struct ProcTable {
    procs: [Proc; NPROC],
    next_pid: Pid,
}

impl ProcTable {
    pub const fn new() -> Self {
        const EMPTY_PROC: Proc = Proc::new();
        Self {
            procs: [EMPTY_PROC; NPROC],
            next_pid: 1,
        }
    }

    /// Allocate a new process
    pub fn alloc(&mut self) -> Option<&mut Proc> {
        for proc in self.procs.iter_mut() {
            if proc.state == ProcState::Unused {
                proc.pid = self.next_pid;
                self.next_pid += 1;
                proc.state = ProcState::Used;
                
                // Initialize signal state
                proc.signals = Some(SignalState::new());
                
                // Allocate kernel stack
                let kstack = kalloc();
                if kstack.is_null() {
                    proc.state = ProcState::Unused;
                    proc.signals = None;
                    return None;
                }
                proc.kstack = kstack as usize + PAGE_SIZE;  // Stack grows down
                
                // Allocate trapframe page
                let tf = kalloc();
                if tf.is_null() {
                    unsafe { kfree(kstack); }
                    proc.state = ProcState::Unused;
                    proc.signals = None;
                    return None;
                }
                proc.trapframe = tf as *mut TrapFrame;
                
                return Some(proc);
            }
        }
        None
    }

    /// Free a process's resources
    pub fn free(&mut self, pid: Pid) {
        for proc in self.procs.iter_mut() {
            if proc.pid == pid {
                // Free kernel stack
                if proc.kstack != 0 {
                    unsafe { kfree((proc.kstack - PAGE_SIZE) as *mut u8); }
                    proc.kstack = 0;
                }
                
                // Free trapframe
                if !proc.trapframe.is_null() {
                    unsafe { kfree(proc.trapframe as *mut u8); }
                    proc.trapframe = ptr::null_mut();
                }
                
                // Free page table
                if !proc.pagetable.is_null() {
                    unsafe { crate::vm::free_pagetable(proc.pagetable); }
                    proc.pagetable = ptr::null_mut();
                }
                
                proc.state = ProcState::Unused;
                proc.pid = 0;
                proc.parent = None;
                proc.sz = 0;
                proc.chan = 0;
                proc.killed = false;
                proc.xstate = 0;
                proc.name = [0; 16];
                break;
            }
        }
    }

    /// Find process by PID
    pub fn find(&mut self, pid: Pid) -> Option<&mut Proc> {
        self.procs.iter_mut().find(|p| p.pid == pid && p.state != ProcState::Unused)
    }

    /// Get current process count
    pub fn count(&self) -> usize {
        self.procs.iter().filter(|p| p.state != ProcState::Unused).count()
    }
}

pub static PROC_TABLE: Mutex<ProcTable> = Mutex::new(ProcTable::new());

// ============================================================================
// Current CPU state
// ============================================================================

/// Per-CPU state
pub struct Cpu {
    pub proc: Option<Pid>,       // Current running process
    pub context: Context,        // Scheduler context
    pub noff: i32,               // Depth of push_off nesting
    pub intena: bool,            // Were interrupts enabled before push_off?
}

impl Cpu {
    pub const fn new() -> Self {
        Self {
            proc: None,
            context: Context::new(),
            noff: 0,
            intena: false,
        }
    }
}

// For single-CPU system (simplification)
static mut CPU: Cpu = Cpu::new();

/// Get current CPU
pub fn mycpu() -> &'static mut Cpu {
    unsafe { &mut *core::ptr::addr_of_mut!(CPU) }
}

/// Get current process PID
pub fn myproc() -> Option<Pid> {
    mycpu().proc
}

// ============================================================================
// Context Switch
// ============================================================================

/// Switch from old context to new context
/// This is implemented in assembly for each architecture
#[cfg(all(feature = "baremetal", target_arch = "riscv64"))]
core::arch::global_asm!(r#"
.globl swtch
swtch:
    # Save old context
    sd ra, 0(a0)
    sd sp, 8(a0)
    sd s0, 16(a0)
    sd s1, 24(a0)
    sd s2, 32(a0)
    sd s3, 40(a0)
    sd s4, 48(a0)
    sd s5, 56(a0)
    sd s6, 64(a0)
    sd s7, 72(a0)
    sd s8, 80(a0)
    sd s9, 88(a0)
    sd s10, 96(a0)
    sd s11, 104(a0)

    # Load new context
    ld ra, 0(a1)
    ld sp, 8(a1)
    ld s0, 16(a1)
    ld s1, 24(a1)
    ld s2, 32(a1)
    ld s3, 40(a1)
    ld s4, 48(a1)
    ld s5, 56(a1)
    ld s6, 64(a1)
    ld s7, 72(a1)
    ld s8, 80(a1)
    ld s9, 88(a1)
    ld s10, 96(a1)
    ld s11, 104(a1)
    
    ret
"#);

#[cfg(all(feature = "baremetal", target_arch = "aarch64"))]
core::arch::global_asm!(r#"
.globl swtch
swtch:
    # Save old context
    stp x19, x20, [x0, #0]
    stp x21, x22, [x0, #16]
    stp x23, x24, [x0, #32]
    stp x25, x26, [x0, #48]
    stp x27, x28, [x0, #64]
    stp x29, x30, [x0, #80]
    mov x9, sp
    str x9, [x0, #96]

    # Load new context  
    ldp x19, x20, [x1, #0]
    ldp x21, x22, [x1, #16]
    ldp x23, x24, [x1, #32]
    ldp x25, x26, [x1, #48]
    ldp x27, x28, [x1, #64]
    ldp x29, x30, [x1, #80]
    ldr x9, [x1, #96]
    mov sp, x9
    
    ret
"#);

#[cfg(all(feature = "baremetal", target_arch = "x86_64"))]
core::arch::global_asm!(r#"
.intel_syntax noprefix
.globl swtch
.section .text
swtch:
    # Save old context
    mov [rdi + 0], r15
    mov [rdi + 8], r14
    mov [rdi + 16], r13
    mov [rdi + 24], r12
    mov [rdi + 32], rbx
    mov [rdi + 40], rbp
    lea rax, [rip + 1f]
    mov [rdi + 48], rax

    # Load new context
    mov r15, [rsi + 0]
    mov r14, [rsi + 8]
    mov r13, [rsi + 16]
    mov r12, [rsi + 24]
    mov rbx, [rsi + 32]
    mov rbp, [rsi + 40]
    mov rax, [rsi + 48]
    jmp rax
1:
    ret
"#);

unsafe extern "C" {
    fn swtch(old: *mut Context, new: *const Context);
}

/// Perform context switch
pub fn switch_context(old: &mut Context, new: &Context) {
    unsafe {
        swtch(old as *mut Context, new as *const Context);
    }
}

// ============================================================================
// SMP Scheduler
// ============================================================================

/// Per-CPU run queue for SMP scheduling
pub struct RunQueue {
    /// Processes ready to run on this CPU
    head: Option<Pid>,
    tail: Option<Pid>,
    count: usize,
}

impl RunQueue {
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
            count: 0,
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    pub fn len(&self) -> usize {
        self.count
    }
}

/// Global scheduler state
pub struct Scheduler {
    /// Per-CPU run queues
    run_queues: [RunQueue; crate::cpu::NCPU],
    /// Global load counter for balancing
    total_runnable: AtomicUsize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            run_queues: [const { RunQueue::new() }; crate::cpu::NCPU],
            total_runnable: AtomicUsize::new(0),
        }
    }
}

static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

/// Yield current process to scheduler
pub fn yield_proc() {
    let cpu = mycpu();
    let current_pid = match cpu.proc {
        Some(pid) => pid,
        None => return,
    };
    
    let mut table = PROC_TABLE.lock();
    if let Some(proc) = table.find(current_pid) {
        proc.state = ProcState::Runnable;
        let proc_ctx = &mut proc.context as *mut Context;
        let cpu_ctx = &cpu.context as *const Context;
        drop(table);
        
        // Switch back to scheduler
        unsafe {
            swtch(proc_ctx, cpu_ctx);
        }
    }
}

/// Select next process to run (simple round-robin for now)
fn pick_next_process(table: &mut ProcTable, cpu_id: usize) -> Option<Pid> {
    // First try processes with affinity to this CPU
    for proc in table.procs.iter_mut() {
        if proc.state == ProcState::Runnable {
            return Some(proc.pid);
        }
    }
    
    let _ = cpu_id; // Will be used for CPU affinity in future
    None
}

/// Main scheduler loop - runs on each CPU
pub fn scheduler() -> ! {
    let cpu = mycpu();
    let cpu_id = crate::cpu::cpuid();
    
    crate::println!("CPU {} entering scheduler", cpu_id);
    
    loop {
        // Enable interrupts to avoid deadlock
        unsafe {
            #[cfg(target_arch = "riscv64")]
            core::arch::asm!("csrs sstatus, {}", in(reg) 0x2usize);
            #[cfg(target_arch = "aarch64")]
            core::arch::asm!("msr daifclr, #2");
            #[cfg(target_arch = "x86_64")]
            core::arch::asm!("sti");
        }
        
        let mut table = PROC_TABLE.lock();
        
        // Find a runnable process
        if let Some(pid) = pick_next_process(&mut table, cpu_id) {
            if let Some(proc) = table.find(pid) {
                proc.state = ProcState::Running;
                cpu.proc = Some(proc.pid);
                
                let proc_ctx = &proc.context as *const Context;
                let cpu_ctx = &mut cpu.context as *mut Context;
                
                drop(table);
                
                // Switch to process
                unsafe {
                    swtch(cpu_ctx, proc_ctx);
                }
                
                // Process yielded back
                cpu.proc = None;
            }
        } else {
            drop(table);
            // No runnable process, wait for interrupt
            core::hint::spin_loop();
        }
    }
}

/// Try to balance load across CPUs
pub fn load_balance() {
    let _sched = SCHEDULER.lock();
    // TODO: Implement work stealing between CPUs
    // For now, simple round-robin scheduling works
}

// ============================================================================
// Sleep and Wakeup (xv6 style)
// ============================================================================

/// Sleep on a channel (wait for wakeup)
pub fn sleep(chan: usize) {
    let cpu = mycpu();
    let current_pid = match cpu.proc {
        Some(pid) => pid,
        None => return,
    };
    
    let mut table = PROC_TABLE.lock();
    if let Some(proc) = table.find(current_pid) {
        proc.chan = chan;
        proc.state = ProcState::Sleeping;
        
        let proc_ctx = &mut proc.context as *mut Context;
        let cpu_ctx = &cpu.context as *const Context;
        drop(table);
        
        // Switch to scheduler
        unsafe {
            swtch(proc_ctx, cpu_ctx);
        }
    }
}

/// Wake up all processes sleeping on channel
pub fn wakeup(chan: usize) {
    let mut table = PROC_TABLE.lock();
    for proc in table.procs.iter_mut() {
        if proc.state == ProcState::Sleeping && proc.chan == chan {
            proc.state = ProcState::Runnable;
        }
    }
}

// ============================================================================
// Process creation
// ============================================================================

/// Allocate a new process (like xv6 allocproc)
pub fn allocproc() -> Option<Pid> {
    let mut table = PROC_TABLE.lock();
    let proc = table.alloc()?;
    let pid = proc.pid;
    
    // Set up new context to start at forkret
    proc.context = Context::default();
    // Context will be set up to return to forkret
    
    Some(pid)
}

/// Fork: create a copy of the current process
pub fn fork() -> Option<Pid> {
    let current_pid = myproc()?;
    
    // Allocate new process
    let child_pid = allocproc()?;
    
    let mut table = PROC_TABLE.lock();
    
    // First get parent data we need (avoid double borrow)
    let parent_data = {
        let parent = table.find(current_pid)?;
        (parent.sz, parent.trapframe, parent.ofile, parent.cwd, parent.name)
    };
    
    // Now set up the child
    let child = table.find(child_pid)?;
    child.parent = Some(current_pid);
    child.sz = parent_data.0;
    
    // Copy trapframe
    if !parent_data.1.is_null() && !child.trapframe.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(parent_data.1, child.trapframe, 1);
            // Set child's return value to 0
            #[cfg(target_arch = "riscv64")]
            { (*child.trapframe).a0 = 0; }
            #[cfg(target_arch = "aarch64")]
            { (*child.trapframe).regs[0] = 0; }
            #[cfg(target_arch = "x86_64")]
            { (*child.trapframe).rax = 0; }
        }
    }
    
    // Copy file descriptors
    child.ofile = parent_data.2;
    child.cwd = parent_data.3;
    
    // Copy name
    child.name = parent_data.4;
    
    child.state = ProcState::Runnable;
    
    Some(child_pid)
}

/// Exit current process
pub fn exit(status: i32) {
    let current_pid = match myproc() {
        Some(pid) => pid,
        None => return,
    };
    
    let mut table = PROC_TABLE.lock();
    
    // Find current process index and get its parent
    let (proc_idx, parent_pid) = {
        let mut found = None;
        for (idx, proc) in table.procs.iter().enumerate() {
            if proc.pid == current_pid && proc.state != ProcState::Unused {
                found = Some((idx, proc.parent));
                break;
            }
        }
        match found {
            Some(f) => f,
            None => return,
        }
    };
    
    // Close files and update current process
    {
        let proc = &mut table.procs[proc_idx];
        for file in proc.ofile.iter_mut() {
            if file.is_some() {
                *file = None;
            }
        }
        proc.cwd = None;
        proc.xstate = status;
        proc.state = ProcState::Zombie;
    }
    
    // Reparent children to init
    for child in table.procs.iter_mut() {
        if child.parent == Some(current_pid) {
            child.parent = Some(1); // init process
        }
    }
    
    // Wake up parent
    if let Some(parent_pid) = parent_pid {
        for p in table.procs.iter_mut() {
            if p.pid == parent_pid && p.state == ProcState::Sleeping {
                p.state = ProcState::Runnable;
            }
        }
    }
    
    drop(table);
    
    // Jump to scheduler, never returns
    yield_proc();
}

/// Wait for a child to exit
pub fn wait(status: *mut i32) -> Option<Pid> {
    let current_pid = myproc()?;
    
    loop {
        let mut have_kids = false;
        let mut table = PROC_TABLE.lock();
        
        for proc in table.procs.iter_mut() {
            if proc.parent != Some(current_pid) {
                continue;
            }
            have_kids = true;
            
            if proc.state == ProcState::Zombie {
                let pid = proc.pid;
                if !status.is_null() {
                    unsafe { *status = proc.xstate; }
                }
                table.free(pid);
                return Some(pid);
            }
        }
        
        if !have_kids {
            return None;
        }
        
        drop(table);
        sleep(current_pid);
    }
}

/// Kill a process
pub fn kill(pid: Pid) -> bool {
    let mut table = PROC_TABLE.lock();
    if let Some(proc) = table.find(pid) {
        proc.killed = true;
        if proc.state == ProcState::Sleeping {
            proc.state = ProcState::Runnable;
        }
        true
    } else {
        false
    }
}

// ============================================================================
// Demo function (for testing)
// ============================================================================

/// Initialize the process subsystem
/// Creates the first process (init)
pub fn init() {
    // Create the init process
    if let Some(pid) = allocproc() {
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            proc.set_name("init");
            proc.state = ProcState::Runnable;
            crate::println!("process: created init process (pid {})", pid);
        }
    } else {
        panic!("process: failed to create init process");
    }
}

/// Create some demo processes
pub fn scheduler_demo() {
    let count = PROC_TABLE.lock().count();
    crate::println!("process: {} processes active", count);
    
    // Allocate a test process
    if let Some(pid) = allocproc() {
        crate::println!("process: allocated pid {}", pid);
        
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            proc.set_name("init");
            crate::println!("process: {} state={:?}", proc.name_str(), proc.state);
        }
    }
    
    let count = PROC_TABLE.lock().count();
    crate::println!("process: {} processes after alloc", count);
}

// Helper functions for syscalls
pub fn getpid() -> Pid {
    myproc().unwrap_or(0)
}

// ============================================================================
// Signal Integration
// ============================================================================

/// Check and handle signals before returning to userspace
/// Returns true if process should continue, false if it should terminate
pub fn handle_signals() -> bool {
    let pid = match myproc() {
        Some(p) => p,
        None => return true,
    };
    
    let mut table = PROC_TABLE.lock();
    let proc = match table.find(pid) {
        Some(p) => p,
        None => return true,
    };
    
    // Get signal state if initialized
    let signals = match &proc.signals {
        Some(s) => s,
        None => return true,
    };
    
    // Check for pending signals
    match check_signals(signals) {
        SignalDeliveryResult::None => {
            // No signals to handle
            true
        }
        SignalDeliveryResult::Terminate(sig) => {
            // Process should be terminated
            crate::println!("Process {} terminated by signal {}", pid, sig);
            proc.killed = true;
            proc.xstate = 128 + sig as i32;  // Exit code for signal
            false
        }
        SignalDeliveryResult::Stop(_sig) => {
            // Process should be stopped (job control)
            proc.state = ProcState::Sleeping;
            // Would need to handle SIGCONT to wake up
            true
        }
        SignalDeliveryResult::Continue => {
            // SIGCONT - continue if stopped
            if proc.state == ProcState::Sleeping {
                proc.state = ProcState::Runnable;
            }
            true
        }
        SignalDeliveryResult::Handle { signal, info, action } => {
            // User signal handler
            // This is complex - need to set up signal frame on user stack
            // and redirect execution to handler
            setup_signal_handler(proc, signal, &info, &action);
            true
        }
    }
}

/// Set up signal handler on user stack
/// This involves:
/// 1. Saving current register state to user stack
/// 2. Setting up signal frame with return trampoline
/// 3. Modifying trapframe to jump to signal handler
fn setup_signal_handler(
    proc: &mut Proc,
    signal: crate::signal::Signal,
    _info: &crate::signal::SigInfo,
    action: &crate::signal::SigAction,
) {
    use crate::signal::SigActionFlags;
    
    if proc.trapframe.is_null() {
        return;
    }
    
    unsafe {
        let tf = &mut *proc.trapframe;
        
        // Save current state (simplified - full implementation would save to user stack)
        #[cfg(target_arch = "riscv64")]
        {
            // Push signal context onto user stack
            let user_sp = tf.sp;
            
            // Allocate space for signal frame (simplified)
            let frame_size = 256;  // Space for saved regs + signal info
            let new_sp = user_sp - frame_size;
            
            // Would copy trapframe to user stack here...
            
            // Set up for signal handler call
            // a0 = signal number
            tf.a0 = signal as usize;
            // sp = new stack pointer
            tf.sp = new_sp;
            // ra = signal return trampoline (needs to call rt_sigreturn)
            tf.ra = action.restorer;
            // pc = signal handler
            tf.epc = action.handler;
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            let user_sp = tf.sp;
            let frame_size = 256;
            let new_sp = user_sp - frame_size;
            
            // x0 = signal number
            tf.regs[0] = signal as usize;
            // sp = new stack
            tf.sp = new_sp;
            // x30 (lr) = restorer
            tf.regs[30] = action.restorer;
            // pc = handler
            tf.elr = action.handler;
        }
        
        #[cfg(target_arch = "x86_64")]
        {
            let user_sp = tf.rsp;
            let frame_size = 256;
            let new_sp = user_sp - frame_size;
            
            // rdi = signal number (first argument)
            tf.rdi = signal as usize;
            // rsp = new stack
            tf.rsp = new_sp;
            // Push return address (restorer)
            // Note: In real implementation, would push to user stack
            tf.rip = action.handler;
        }
        
        // If SA_NODEFER not set, block signal during handler
        if (action.flags.0 & SigActionFlags::SA_NODEFER) == 0 {
            if let Some(ref signals) = proc.signals {
                signals.block(crate::signal::SigSet::from_bits(1 << (signal - 1)));
            }
        }
    }
}

/// Send a signal to a process by PID
pub fn kill_proc(pid: Pid, signal: crate::signal::Signal) -> Result<(), ()> {
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(())?;
    
    let signals = proc.signals.as_ref().ok_or(())?;
    signals.send_signal(signal)?;
    
    // Wake up process if sleeping and signal is not blocked
    if proc.state == ProcState::Sleeping {
        if let Some(ref sig_state) = proc.signals {
            if !sig_state.get_mask().contains(signal) {
                proc.state = ProcState::Runnable;
            }
        }
    }
    
    Ok(())
}

/// Get signal state for current process
pub fn current_signal_state() -> Option<&'static SignalState> {
    let pid = myproc()?;
    let mut table = PROC_TABLE.lock();
    let _proc = table.find(pid)?;
    // Note: This is unsafe - we're returning a reference that outlives the lock
    // In a real implementation, we'd need RefCell or similar
    // For now, just indicate it exists
    None  // Placeholder - proper implementation needs lifetime management
}
