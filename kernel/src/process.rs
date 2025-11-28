//! Process management for xv6-rust kernel
//! Implements process creation, scheduling, and lifecycle management

extern crate alloc;

use core::ptr::null_mut;
use alloc::string::String;
use alloc::collections::BTreeMap;
use crate::sync::Mutex;
use crate::mm::{kalloc, kfree, PAGE_SIZE};
use crate::signal::SignalState;
use crate::vm::PageTable;

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of processes
pub const NPROC: usize = 64;

/// Maximum number of file descriptors per process
pub const NOFILE: usize = 16;

// ============================================================================
// Types
// ============================================================================

/// Process ID type
pub type Pid = usize;

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcState {
    Unused,
    Used,
    Sleeping,
    Runnable,
    Running,
    Zombie,
}

/// CPU context for context switching
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Context {
    // Callee-saved registers
    #[cfg(target_arch = "riscv64")]
    pub ra: usize,
    #[cfg(target_arch = "riscv64")]
    pub sp: usize,
    #[cfg(target_arch = "riscv64")]
    pub s0: usize,
    #[cfg(target_arch = "riscv64")]
    pub s1: usize,
    #[cfg(target_arch = "riscv64")]
    pub s2: usize,
    #[cfg(target_arch = "riscv64")]
    pub s3: usize,
    #[cfg(target_arch = "riscv64")]
    pub s4: usize,
    #[cfg(target_arch = "riscv64")]
    pub s5: usize,
    #[cfg(target_arch = "riscv64")]
    pub s6: usize,
    #[cfg(target_arch = "riscv64")]
    pub s7: usize,
    #[cfg(target_arch = "riscv64")]
    pub s8: usize,
    #[cfg(target_arch = "riscv64")]
    pub s9: usize,
    #[cfg(target_arch = "riscv64")]
    pub s10: usize,
    #[cfg(target_arch = "riscv64")]
    pub s11: usize,

    #[cfg(target_arch = "aarch64")]
    pub x19: usize,
    #[cfg(target_arch = "aarch64")]
    pub x20: usize,
    #[cfg(target_arch = "aarch64")]
    pub x21: usize,
    #[cfg(target_arch = "aarch64")]
    pub x22: usize,
    #[cfg(target_arch = "aarch64")]
    pub x23: usize,
    #[cfg(target_arch = "aarch64")]
    pub x24: usize,
    #[cfg(target_arch = "aarch64")]
    pub x25: usize,
    #[cfg(target_arch = "aarch64")]
    pub x26: usize,
    #[cfg(target_arch = "aarch64")]
    pub x27: usize,
    #[cfg(target_arch = "aarch64")]
    pub x28: usize,
    #[cfg(target_arch = "aarch64")]
    pub fp: usize,  // x29
    #[cfg(target_arch = "aarch64")]
    pub lr: usize,  // x30
    #[cfg(target_arch = "aarch64")]
    pub sp: usize,

    #[cfg(target_arch = "x86_64")]
    pub rbx: usize,
    #[cfg(target_arch = "x86_64")]
    pub rbp: usize,
    #[cfg(target_arch = "x86_64")]
    pub r12: usize,
    #[cfg(target_arch = "x86_64")]
    pub r13: usize,
    #[cfg(target_arch = "x86_64")]
    pub r14: usize,
    #[cfg(target_arch = "x86_64")]
    pub r15: usize,
    #[cfg(target_arch = "x86_64")]
    pub rsp: usize,
    #[cfg(target_arch = "x86_64")]
    pub rip: usize,
}

impl Context {
    pub const fn new() -> Self {
        #[cfg(target_arch = "riscv64")]
        {
            Self {
                ra: 0, sp: 0, s0: 0, s1: 0, s2: 0, s3: 0, s4: 0,
                s5: 0, s6: 0, s7: 0, s8: 0, s9: 0, s10: 0, s11: 0,
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0,
                x25: 0, x26: 0, x27: 0, x28: 0, fp: 0, lr: 0, sp: 0,
            }
        }
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                rbx: 0, rbp: 0, r12: 0, r13: 0, r14: 0, r15: 0, rsp: 0, rip: 0,
            }
        }
    }
}

/// Trap frame - saved registers on trap/interrupt
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapFrame {
    #[cfg(target_arch = "riscv64")]
    pub kernel_satp: usize,
    #[cfg(target_arch = "riscv64")]
    pub kernel_sp: usize,
    #[cfg(target_arch = "riscv64")]
    pub kernel_trap: usize,
    #[cfg(target_arch = "riscv64")]
    pub epc: usize,
    #[cfg(target_arch = "riscv64")]
    pub kernel_hartid: usize,
    #[cfg(target_arch = "riscv64")]
    pub ra: usize,
    #[cfg(target_arch = "riscv64")]
    pub sp: usize,
    #[cfg(target_arch = "riscv64")]
    pub gp: usize,
    #[cfg(target_arch = "riscv64")]
    pub tp: usize,
    #[cfg(target_arch = "riscv64")]
    pub t0: usize,
    #[cfg(target_arch = "riscv64")]
    pub t1: usize,
    #[cfg(target_arch = "riscv64")]
    pub t2: usize,
    #[cfg(target_arch = "riscv64")]
    pub s0: usize,
    #[cfg(target_arch = "riscv64")]
    pub s1: usize,
    #[cfg(target_arch = "riscv64")]
    pub a0: usize,
    #[cfg(target_arch = "riscv64")]
    pub a1: usize,
    #[cfg(target_arch = "riscv64")]
    pub a2: usize,
    #[cfg(target_arch = "riscv64")]
    pub a3: usize,
    #[cfg(target_arch = "riscv64")]
    pub a4: usize,
    #[cfg(target_arch = "riscv64")]
    pub a5: usize,
    #[cfg(target_arch = "riscv64")]
    pub a6: usize,
    #[cfg(target_arch = "riscv64")]
    pub a7: usize,
    #[cfg(target_arch = "riscv64")]
    pub s2: usize,
    #[cfg(target_arch = "riscv64")]
    pub s3: usize,
    #[cfg(target_arch = "riscv64")]
    pub s4: usize,
    #[cfg(target_arch = "riscv64")]
    pub s5: usize,
    #[cfg(target_arch = "riscv64")]
    pub s6: usize,
    #[cfg(target_arch = "riscv64")]
    pub s7: usize,
    #[cfg(target_arch = "riscv64")]
    pub s8: usize,
    #[cfg(target_arch = "riscv64")]
    pub s9: usize,
    #[cfg(target_arch = "riscv64")]
    pub s10: usize,
    #[cfg(target_arch = "riscv64")]
    pub s11: usize,
    #[cfg(target_arch = "riscv64")]
    pub t3: usize,
    #[cfg(target_arch = "riscv64")]
    pub t4: usize,
    #[cfg(target_arch = "riscv64")]
    pub t5: usize,
    #[cfg(target_arch = "riscv64")]
    pub t6: usize,

    #[cfg(target_arch = "aarch64")]
    pub regs: [usize; 31],
    #[cfg(target_arch = "aarch64")]
    pub sp: usize,
    #[cfg(target_arch = "aarch64")]
    pub elr: usize,
    #[cfg(target_arch = "aarch64")]
    pub spsr: usize,

    #[cfg(target_arch = "x86_64")]
    pub rax: usize,
    #[cfg(target_arch = "x86_64")]
    pub rbx: usize,
    #[cfg(target_arch = "x86_64")]
    pub rcx: usize,
    #[cfg(target_arch = "x86_64")]
    pub rdx: usize,
    #[cfg(target_arch = "x86_64")]
    pub rsi: usize,
    #[cfg(target_arch = "x86_64")]
    pub rdi: usize,
    #[cfg(target_arch = "x86_64")]
    pub rbp: usize,
    #[cfg(target_arch = "x86_64")]
    pub r8: usize,
    #[cfg(target_arch = "x86_64")]
    pub r9: usize,
    #[cfg(target_arch = "x86_64")]
    pub r10: usize,
    #[cfg(target_arch = "x86_64")]
    pub r11: usize,
    #[cfg(target_arch = "x86_64")]
    pub r12: usize,
    #[cfg(target_arch = "x86_64")]
    pub r13: usize,
    #[cfg(target_arch = "x86_64")]
    pub r14: usize,
    #[cfg(target_arch = "x86_64")]
    pub r15: usize,
    #[cfg(target_arch = "x86_64")]
    pub rsp: usize,
    #[cfg(target_arch = "x86_64")]
    pub rip: usize,
    #[cfg(target_arch = "x86_64")]
    pub rflags: usize,
    #[cfg(target_arch = "x86_64")]
    pub cs: usize,
    #[cfg(target_arch = "x86_64")]
    pub ss: usize,
}

impl TrapFrame {
    pub const fn new() -> Self {
        #[cfg(target_arch = "riscv64")]
        {
            Self {
                kernel_satp: 0, kernel_sp: 0, kernel_trap: 0, epc: 0, kernel_hartid: 0,
                ra: 0, sp: 0, gp: 0, tp: 0, t0: 0, t1: 0, t2: 0,
                s0: 0, s1: 0, a0: 0, a1: 0, a2: 0, a3: 0, a4: 0, a5: 0, a6: 0, a7: 0,
                s2: 0, s3: 0, s4: 0, s5: 0, s6: 0, s7: 0, s8: 0, s9: 0, s10: 0, s11: 0,
                t3: 0, t4: 0, t5: 0, t6: 0,
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                regs: [0; 31], sp: 0, elr: 0, spsr: 0,
            }
        }
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                rax: 0, rbx: 0, rcx: 0, rdx: 0, rsi: 0, rdi: 0, rbp: 0,
                r8: 0, r9: 0, r10: 0, r11: 0, r12: 0, r13: 0, r14: 0, r15: 0,
                rsp: 0, rip: 0, rflags: 0, cs: 0, ss: 0,
            }
        }
    }
}

/// Process control block
pub struct Proc {
    pub pid: Pid,
    pub state: ProcState,
    pub parent: Option<Pid>,
    pub kstack: usize,
    pub trapframe: *mut TrapFrame,
    pub context: Context,
    pub ofile: [Option<usize>; NOFILE],  // Open file descriptors (index into FILE_TABLE)
    pub cwd_path: Option<String>,
    pub cwd: Option<usize>,  // Current working directory file index
    pub signals: Option<SignalState>,
    pub chan: usize,  // Sleep channel
    pub killed: bool,
    pub xstate: i32,  // Exit status
    pub sz: usize,    // Memory size
    pub pagetable: *mut PageTable,  // Page table pointer
}

// Safety: Process control block is protected by PROC_TABLE mutex
unsafe impl Send for Proc {}

impl Proc {
    pub const fn new() -> Self {
        Self {
            pid: 0,
            state: ProcState::Unused,
            parent: None,
            kstack: 0,
            trapframe: null_mut(),
            context: Context::new(),
            ofile: [None; NOFILE],
            cwd_path: None,
            cwd: None,
            signals: None,
            chan: 0,
            killed: false,
            xstate: 0,
            sz: 0,
            pagetable: null_mut(),
        }
    }
}

// ============================================================================
// Process Table
// ============================================================================

/// Process table with O(1) PID lookup
pub struct ProcTable {
    procs: [Proc; NPROC],
    next_pid: Pid,
    pid_to_index: BTreeMap<Pid, usize>,  // For O(log n) lookup, could use HashMap for O(1)
}

impl ProcTable {
    pub const fn new() -> Self {
        const INIT_PROC: Proc = Proc::new();
        Self {
            procs: [INIT_PROC; NPROC],
            next_pid: 1,
            pid_to_index: BTreeMap::new(),
        }
    }

    /// Allocate a new process - O(n) scan for unused slot
    /// TODO: Optimize with free list for O(1)
    pub fn alloc(&mut self) -> Option<&mut Proc> {
        for (i, proc) in self.procs.iter_mut().enumerate() {
            if proc.state == ProcState::Unused {
                let new_pid = self.next_pid;
                proc.pid = new_pid;
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
                    unsafe { kfree((proc.kstack - PAGE_SIZE) as *mut u8); }
                    proc.state = ProcState::Unused;
                    proc.signals = None;
                    return None;
                }
                proc.trapframe = tf as *mut TrapFrame;
                
                // Add to pid_to_index map for O(1) lookups
                self.pid_to_index.insert(new_pid, i);
                
                return Some(proc);
            }
        }
        None
    }

    /// Find process by PID - O(log n) with BTreeMap
    pub fn find(&mut self, pid: Pid) -> Option<&mut Proc> {
        if let Some(&idx) = self.pid_to_index.get(&pid) {
            Some(&mut self.procs[idx])
        } else {
            None
        }
    }

    /// Find process by PID (immutable) - O(log n)
    pub fn find_ref(&self, pid: Pid) -> Option<&Proc> {
        if let Some(&idx) = self.pid_to_index.get(&pid) {
            Some(&self.procs[idx])
        } else {
            None
        }
    }

    /// Free a process
    pub fn free(&mut self, pid: Pid) {
        if let Some(&idx) = self.pid_to_index.get(&pid) {
            let proc = &mut self.procs[idx];
            
            // Free kernel stack
            if proc.kstack != 0 {
                unsafe { kfree((proc.kstack - PAGE_SIZE) as *mut u8); }
                proc.kstack = 0;
            }
            
            // Free trapframe
            if !proc.trapframe.is_null() {
                unsafe { kfree(proc.trapframe as *mut u8); }
                proc.trapframe = null_mut();
            }
            
            proc.state = ProcState::Unused;
            proc.signals = None;
            self.pid_to_index.remove(&pid);
        }
    }

    /// Get iterator over all processes
    pub fn iter(&self) -> impl Iterator<Item = &Proc> {
        self.procs.iter()
    }

    /// Get mutable iterator over all processes
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Proc> {
        self.procs.iter_mut()
    }
}

// ============================================================================
// Global State
// ============================================================================

/// Global process table
pub static PROC_TABLE: Mutex<ProcTable> = Mutex::new(ProcTable::new());

/// Current process PID for each CPU (indexed by CPU ID)
static mut CURRENT_PID: [Option<Pid>; 8] = [None; 8];

// ============================================================================
// Public API
// ============================================================================

/// Initialize process subsystem
pub fn init() {
    let mut table = PROC_TABLE.lock();
    
    // Create init process (PID 1)
    if let Some(proc) = table.alloc() {
        proc.state = ProcState::Runnable;
        proc.cwd_path = Some(String::from("/"));
        crate::println!("process: init process created (pid={})", proc.pid);
    }
}

/// Get current process PID
pub fn myproc() -> Option<Pid> {
    let cpu_id = crate::cpu::cpuid();
    unsafe { CURRENT_PID[cpu_id] }
}

/// Set current process PID
fn set_current(pid: Option<Pid>) {
    let cpu_id = crate::cpu::cpuid();
    unsafe { CURRENT_PID[cpu_id] = pid; }
}

/// Fork current process
pub fn fork() -> Option<Pid> {
    let parent_pid = myproc()?;
    let mut table = PROC_TABLE.lock();
    
    // First, get parent info
    let parent_ofile;
    let parent_cwd_path;
    {
        let parent = table.find(parent_pid)?;
        parent_ofile = parent.ofile;
        parent_cwd_path = parent.cwd_path.clone();
    }
    
    // Then allocate child
    let child = table.alloc()?;
    let child_pid = child.pid;
    child.parent = Some(parent_pid);
    child.state = ProcState::Runnable;
    
    // Copy parent's file descriptors
    for i in 0..NOFILE {
        child.ofile[i] = parent_ofile[i];
        // TODO: Increment file reference counts
    }
    child.cwd_path = parent_cwd_path;
    
    Some(child_pid)
}

/// Exit current process
pub fn exit(status: i32) {
    if let Some(pid) = myproc() {
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            proc.xstate = status;
            proc.state = ProcState::Zombie;
            
            // Close all open files
            for i in 0..NOFILE {
                if let Some(fd_idx) = proc.ofile[i] {
                    crate::file::file_close(fd_idx);
                    proc.ofile[i] = None;
                }
            }
            
            // Wake up parent
            if let Some(parent_pid) = proc.parent {
                wakeup_pid(&mut table, parent_pid);
            }
            
            // Reparent children to init
            reparent_children(&mut table, pid);
        }
    }
    
    // Yield CPU
    yield_cpu();
}

/// Wait for child process to exit
pub fn wait(status: *mut i32) -> Option<Pid> {
    let parent_pid = myproc()?;
    
    loop {
        let mut table = PROC_TABLE.lock();
        let mut found_child = false;
        let mut zombie_child: Option<(Pid, i32)> = None;
        
        // First pass: find a zombie child or check if any children exist
        for proc in table.iter() {
            if proc.parent == Some(parent_pid) {
                found_child = true;
                
                if proc.state == ProcState::Zombie {
                    zombie_child = Some((proc.pid, proc.xstate));
                    break;
                }
            }
        }
        
        // If found a zombie, free it and return
        if let Some((child_pid, xstate)) = zombie_child {
            if !status.is_null() {
                unsafe { *status = xstate; }
            }
            table.free(child_pid);
            return Some(child_pid);
        }
        
        if !found_child {
            return None;
        }
        
        // Sleep waiting for child
        drop(table);
        sleep(parent_pid as usize);
    }
}

/// Kill a process
pub fn kill(pid: usize) -> bool {
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

/// Kill a process with a signal
pub fn kill_proc(pid: Pid, sig: u32) -> Result<(), ()> {
    let mut table = PROC_TABLE.lock();
    if let Some(proc) = table.find(pid) {
        if let Some(ref signals) = proc.signals {
            let _ = signals.send_signal(sig);
        }
        if proc.state == ProcState::Sleeping {
            proc.state = ProcState::Runnable;
        }
        Ok(())
    } else {
        Err(())
    }
}

/// Get current process PID
pub fn getpid() -> Pid {
    myproc().unwrap_or(0)
}

/// Allocate file descriptor for current process
pub fn fdalloc(file_idx: usize) -> Option<i32> {
    let pid = myproc()?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid)?;
    
    for (i, slot) in proc.ofile.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(file_idx);
            return Some(i as i32);
        }
    }
    None
}

/// Close file descriptor for current process
pub fn fdclose(fd: i32) {
    if let Some(pid) = myproc() {
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            if fd >= 0 && (fd as usize) < NOFILE {
                if let Some(file_idx) = proc.ofile[fd as usize] {
                    crate::file::file_close(file_idx);
                    proc.ofile[fd as usize] = None;
                }
            }
        }
    }
}

/// Lookup file descriptor
pub fn fdlookup(fd: i32) -> Option<usize> {
    let pid = myproc()?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(pid)?;
    
    if fd >= 0 && (fd as usize) < NOFILE {
        proc.ofile[fd as usize]
    } else {
        None
    }
}

/// Install a file at a specific file descriptor
pub fn fdinstall(fd: i32, file_idx: usize) -> Result<(), ()> {
    let pid = myproc().ok_or(())?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(())?;
    
    if fd >= 0 && (fd as usize) < NOFILE {
        proc.ofile[fd as usize] = Some(file_idx);
        Ok(())
    } else {
        Err(())
    }
}

/// Sleep on a channel
pub fn sleep(chan: usize) {
    if let Some(pid) = myproc() {
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            proc.chan = chan;
            proc.state = ProcState::Sleeping;
        }
    }
    yield_cpu();
}

/// Wake up all processes sleeping on a channel
pub fn wakeup(chan: usize) {
    let mut table = PROC_TABLE.lock();
    for proc in table.iter_mut() {
        if proc.state == ProcState::Sleeping && proc.chan == chan {
            proc.state = ProcState::Runnable;
        }
    }
}

fn wakeup_pid(table: &mut ProcTable, pid: Pid) {
    if let Some(proc) = table.find(pid) {
        if proc.state == ProcState::Sleeping {
            proc.state = ProcState::Runnable;
        }
    }
}

fn reparent_children(table: &mut ProcTable, parent_pid: Pid) {
    for proc in table.iter_mut() {
        if proc.parent == Some(parent_pid) {
            proc.parent = Some(1); // Reparent to init
        }
    }
}

/// Yield CPU to scheduler
pub fn yield_cpu() {
    if let Some(pid) = myproc() {
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            if proc.state == ProcState::Running {
                proc.state = ProcState::Runnable;
            }
        }
    }
    // Switch to scheduler
    sched();
}

/// Switch to scheduler context
fn sched() {
    // Context switch to scheduler
    // This is simplified; real implementation needs assembly
    crate::arch::wfi();
}

/// Main scheduler loop
pub fn scheduler() -> ! {
    loop {
        // Enable interrupts to allow timer interrupts
        crate::arch::intr_on();
        
        let mut table = PROC_TABLE.lock();
        
        for proc in table.iter_mut() {
            if proc.state == ProcState::Runnable {
                proc.state = ProcState::Running;
                set_current(Some(proc.pid));
                
                // Switch to process
                // In a real implementation, this would do a context switch
                // For now, just mark it as running
                
                break;
            }
        }
        
        drop(table);
        
        // Wait for interrupt
        crate::arch::wfi();
    }
}