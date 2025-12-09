//! Core Process Management
//!
//! This module provides the core process management functionality:
//! - Process table management
//! - Process creation and termination
//! - Process scheduling
//! - Process state management
//!
//! **Note**: For process management as a service (via IPC), see `crate::services::process`.
//! This module provides the low-level primitives that the service layer uses.

extern crate alloc;

use core::ptr::null_mut;
use alloc::string::String;
use hashbrown::HashMap;
use alloc::vec::Vec;
use crate::sync::Mutex;
use crate::compat::DefaultHasherBuilder;
use crate::mm::{kalloc, kfree, PAGE_SIZE};
use crate::ipc::signal::SignalState;
use crate::mm::vm::{PageTable, free_pagetable};

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
    Stopped, // Stopped (e.g., by SIGSTOP signal)
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
    pub rax: usize, // System call return value (a0)
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

/// Cached file descriptor information for fast access
/// This cache stores frequently accessed file descriptors (0-7) to avoid repeated lookups
#[derive(Debug, Clone, Copy)]
struct CachedFd {
    /// File index in global file table
    file_idx: Option<usize>,
    /// File type (cached to avoid file table lookup)
    file_type: crate::fs::file::FileType,
    /// Validity flag (true if cache entry is valid)
    valid: bool,
}

impl CachedFd {
    const fn new() -> Self {
        Self {
            file_idx: None,
            file_type: crate::fs::file::FileType::None,
            valid: false,
        }
    }
}

impl Default for CachedFd {
    fn default() -> Self {
        Self {
            file_idx: None,
            file_type: crate::fs::file::FileType::None,
            valid: false,
        }
    }
}

/// Process control block
use crate::posix;

pub struct Proc {
    pub pid: Pid,
    pub pgid: Pid,  // Process group ID
    pub sid: Pid,   // Session ID
    // User/Group IDs (POSIX credentials)
    pub uid: posix::Uid,   // Real user ID
    pub gid: posix::Gid,   // Real group ID
    pub euid: posix::Uid,  // Effective user ID
    pub egid: posix::Gid,  // Effective group ID
    pub suid: posix::Uid,  // Saved set-user-ID
    pub sgid: posix::Gid,  // Saved set-group-ID
    pub state: ProcState,
    pub parent: Option<Pid>,
    pub kstack: usize,
    pub trapframe: *mut TrapFrame,
    pub context: Context,
    pub ofile: [Option<usize>; NOFILE],  // Open file descriptors (index into FILE_TABLE)
    /// Cached file descriptors for fast access (FDs 0-7)
    /// This cache reduces file table lookups for commonly used file descriptors
    fd_cache: [CachedFd; 8],
    pub cwd_path: Option<String>,
    pub cwd: Option<usize>,  // Current working directory file index
    pub signals: Option<SignalState>,
    pub alt_signal_stack: Option<crate::posix::StackT>,  // Alternate signal stack
    pub rlimits: [crate::posix::Rlimit; 16],  // Resource limits
    pub chan: usize,  // Sleep channel
    pub killed: bool,
    pub xstate: i32,  // Exit status
    pub sz: usize,    // Memory size
    pub pagetable: *mut PageTable,  // Page table pointer
    pub nice: i32,    // Process nice value (-20 to 19)
    pub umask: u32,   // File creation mask
}

// Safety: Process control block is protected by PROC_TABLE mutex
unsafe impl Send for Proc {}

impl Proc {
    pub const fn new() -> Self {
        Self {
            pid: 0,
            pgid: 0,
            sid: 0,
            uid: 0,
            gid: 0,
            euid: 0,
            egid: 0,
            suid: 0,
            sgid: 0,
            rlimits: [crate::posix::Rlimit { rlim_cur: 0, rlim_max: 0 }; 16],
            state: ProcState::Unused,
            parent: None,
            kstack: 0,
            trapframe: null_mut(),
            context: Context::new(),
            ofile: [None; NOFILE],
            fd_cache: [CachedFd::new(); 8],
            cwd_path: None,
            cwd: None,
            signals: None,
            alt_signal_stack: None,
            chan: 0,
            killed: false,
            xstate: 0,
            sz: 0,
            pagetable: null_mut(),
            nice: 0,
            umask: 0o022,  // Default umask
        }
    }
    
    /// Get cached file descriptor information (O(1) lookup for FDs 0-7)
    /// 
    /// This function provides fast access to commonly used file descriptors
    /// without requiring a file table lookup. The cache is automatically
    /// updated when file descriptors are opened or closed.
    /// 
    /// # Arguments
    /// 
    /// * `fd` - File descriptor number (must be 0-7 for cached FDs)
    /// 
    /// # Returns
    /// 
    /// * `Some(file_idx)` if the file descriptor is open and cached
    /// * `None` if the file descriptor is not open or not cached
    #[inline]
    pub fn get_cached_fd(&self, fd: i32) -> Option<usize> {
        if fd >= 0 && fd < 8 {
            let cached = &self.fd_cache[fd as usize];
            if cached.valid {
                return cached.file_idx;
            }
        }
        None
    }
    
    /// Update file descriptor cache when a file descriptor is opened
    /// 
    /// This should be called whenever a file descriptor is allocated
    /// to keep the cache synchronized with the actual file descriptor table.
    /// 
    /// # Arguments
    /// 
    /// * `fd` - File descriptor number (must be 0-7 for cached FDs)
    /// * `file_idx` - File index in global file table
    fn update_fd_cache(&mut self, fd: i32, file_idx: usize) {
        if fd >= 0 && fd < 8 {
            // Get file type from file table to cache it
            let file_type = crate::fs::file::FILE_TABLE.lock()
                .get(file_idx)
                .map(|f| f.ftype)
                .unwrap_or(crate::fs::file::FileType::None);
            
            self.fd_cache[fd as usize] = CachedFd {
                file_idx: Some(file_idx),
                file_type,
                valid: true,
            };
        }
    }
    
    /// Invalidate file descriptor cache when a file descriptor is closed
    /// 
    /// This should be called whenever a file descriptor is closed
    /// to keep the cache synchronized with the actual file descriptor table.
    /// 
    /// # Arguments
    /// 
    /// * `fd` - File descriptor number (must be 0-7 for cached FDs)
    fn invalidate_fd_cache(&mut self, fd: i32) {
        if fd >= 0 && fd < 8 {
            self.fd_cache[fd as usize] = CachedFd::default();
        }
    }
    
    /// Invalidate all file descriptor caches
    /// 
    /// This should be called when all file descriptors are closed
    /// (e.g., during process exit).
    fn invalidate_all_fd_cache(&mut self) {
        for cached in &mut self.fd_cache {
            *cached = CachedFd::default();
        }
    }
}

// ============================================================================
// Process Table
// ============================================================================

/// Resource pools for efficient allocation
struct ResourcePools {
    stack_pool: Vec<usize>,      // store kernel stack addresses
    trapframe_pool: Vec<usize>,  // store trapframe addresses
}

impl ResourcePools {
    const fn new() -> Self {
        Self {
            stack_pool: Vec::new(),
            trapframe_pool: Vec::new(),
        }
    }

    /// Get a kernel stack from pool or allocate new one
    fn alloc_stack(&mut self) -> Option<usize> {
        self.stack_pool.pop().or_else(|| {
            let stack = kalloc();
            if stack.is_null() { None } else { Some(stack as usize) }
        })
    }

    /// Return kernel stack to pool
    fn free_stack(&mut self, stack_addr: usize) {
        if stack_addr != 0 && self.stack_pool.len() < NPROC {
            self.stack_pool.push(stack_addr);
        } else if stack_addr != 0 {
            unsafe { kfree(stack_addr as *mut u8); }
        }
    }

    /// Get a trapframe from pool or allocate new one
    fn alloc_trapframe(&mut self) -> Option<usize> {
        self.trapframe_pool.pop().or_else(|| {
            let tf = kalloc() as *mut TrapFrame;
            if tf.is_null() { None } else { Some(tf as usize) }
        })
    }

    /// Return trapframe to pool
    fn free_trapframe(&mut self, tf_addr: usize) {
        if tf_addr != 0 && self.trapframe_pool.len() < NPROC {
            self.trapframe_pool.push(tf_addr);
        } else if tf_addr != 0 {
            unsafe { kfree(tf_addr as *mut u8); }
        }
    }
}

/// Process table with O(1) average-case PID lookup
pub struct ProcTable {
    procs: [Proc; NPROC],
    next_pid: Pid,
    pid_to_index: HashMap<Pid, usize, DefaultHasherBuilder>,  // O(1) average-case PID lookup using HashMap (always initialized)
    parent_to_children: HashMap<Pid, Vec<Pid>, DefaultHasherBuilder>,  // O(1) child lookup by parent PID (always initialized)
    free_list: Vec<usize>,  // Free process slot indices for O(1) allocation
    resource_pools: ResourcePools,  // Resource pools for efficient allocation
    initialized: bool,  // Track initialization state
}

impl ProcTable {
    /// Create a new process table for static initialization
    pub const fn const_new() -> Self {
        const INIT_PROC: Proc = Proc::new();
        Self {
            procs: [INIT_PROC; NPROC],
            next_pid: 1,
            pid_to_index: HashMap::with_hasher(DefaultHasherBuilder),  // Will be properly initialized at runtime
            parent_to_children: HashMap::with_hasher(DefaultHasherBuilder),  // Will be properly initialized at runtime
            free_list: Vec::new(),
            resource_pools: ResourcePools::new(),
            initialized: false,
        }
    }

    /// Create a new process table
    /// HashMap is always initialized (not Option)
    pub fn new() -> Self {
        const INIT_PROC: Proc = Proc::new();
        let mut table = Self {
            procs: [INIT_PROC; NPROC],
            next_pid: 1,
            pid_to_index: HashMap::with_hasher(DefaultHasherBuilder),  // Always initialized
            parent_to_children: HashMap::with_hasher(DefaultHasherBuilder),  // Always initialized
            free_list: Vec::new(),
            resource_pools: ResourcePools::new(),
            initialized: false,
        };
        // Pre-allocate capacity for optimal performance
        table.pid_to_index.reserve(NPROC);
        table.parent_to_children.reserve(NPROC);
        table.initialized = true;
        table
    }
    
    /// Get the number of processes in the table
    pub fn len(&self) -> usize {
        self.pid_to_index.len()
    }

    /// Ensure the process table is initialized
    /// This is a no-op now since HashMap is always initialized, but kept for compatibility
    #[inline(always)]
    fn ensure_initialized(&mut self) {
        // No-op: HashMap is always initialized in new()
    }

    /// Allocate a new process - O(1) with free list
    pub fn alloc(&mut self) -> Option<&mut Proc> {
        // Ensure initialized first (before any borrows)
        self.ensure_initialized();
        
        // Initialize free list if it's empty
        if self.free_list.is_empty() {
            // Populate free list with all available indices
            for i in 0..NPROC {
                self.free_list.push(i);
            }
        }

        // Get the first available index from free list
        let idx = self.free_list.pop()?;
        let proc = &mut self.procs[idx];
        
        // Ensure the process is actually unused (sanity check)
        if proc.state != ProcState::Unused {
            // Push it back if it's not unused
            self.free_list.push(idx);
            return None;
        }

        let new_pid = self.next_pid;
        proc.pid = new_pid;
        self.next_pid += 1;
    proc.state = ProcState::Used;
    
    // Initialize process group and session ID
    proc.pgid = new_pid;
    proc.sid = new_pid;
    
    // Initialize signal state
    proc.signals = Some(SignalState::new());
        
        // Allocate kernel stack from pool
        let kstack = match self.resource_pools.alloc_stack() {
            Some(stack) => stack,
            None => {
                proc.state = ProcState::Unused;
                proc.signals = None;
                self.free_list.push(idx); // Return to free list
                return None;
            }
        };
        proc.kstack = kstack + PAGE_SIZE;  // Stack grows down

        // Allocate trapframe from pool
        let tf = match self.resource_pools.alloc_trapframe() {
            Some(trapframe) => trapframe,
            None => {
                self.resource_pools.free_stack(kstack);
                proc.state = ProcState::Unused;
                proc.signals = None;
                self.free_list.push(idx); // Return to free list
                return None;
            }
        };
        proc.trapframe = tf as *mut TrapFrame;
        
        // Store PID and index before inserting into HashMap
        let pid = new_pid;
        
        // Add to pid_to_index map for O(1) lookups
        // Note: We can safely insert here because ensure_initialized() was called earlier
        self.pid_to_index.insert(pid, idx);
        // RCU 分片注册（占位，不改变主路径）
        crate::process::rcu_table::with_sharded(|s| s.register(pid));
        
        // Return reference to the proc (idx is valid, managed internally)
        Some(proc)
    }

    /// Find process by PID - O(1) average-case with HashMap
    /// No fallback path - HashMap is always initialized
    #[inline]
    pub fn find(&mut self, pid: Pid) -> Option<&mut Proc> {
        // Bounds checking: PID must be valid (non-zero)
        if pid == 0 {
            return None;
        }

        // Ensure initialized (should already be done, but safe check)
        self.ensure_initialized();

        // Direct HashMap lookup - no Option unwrapping needed
        if let Some(&idx) = self.pid_to_index.get(&pid) {
            // Index is guaranteed valid since it's managed internally
            // Use unsafe access to skip bounds checking for maximum performance
            Some(unsafe { self.procs.get_unchecked_mut(idx) })
        } else {
            None
        }
    }

    /// Find process by PID (immutable) - O(1) average-case
    /// No fallback path - HashMap is always initialized
    #[inline]
    pub fn find_ref(&self, pid: Pid) -> Option<&Proc> {
        // Bounds checking: PID must be valid (non-zero)
        if pid == 0 {
            return None;
        }

        // Direct HashMap lookup - no Option unwrapping needed
        // Note: We can't call ensure_initialized here because &self is immutable
        // But HashMap is always initialized in new(), so this is safe
        if let Some(&idx) = self.pid_to_index.get(&pid) {
            // Index is guaranteed valid since it's managed internally
            // Use unsafe access to skip bounds checking for maximum performance
            Some(unsafe { self.procs.get_unchecked(idx) })
        } else {
            None
        }
    }

    /// Add child to parent's children list - O(1) average-case
    /// No fallback path - HashMap is always initialized
    #[inline]
    fn add_child_to_parent(&mut self, parent_pid: Pid, child_pid: Pid) {
        self.ensure_initialized();
        self.parent_to_children.entry(parent_pid).or_insert_with(Vec::new).push(child_pid);
    }

    /// Remove child from parent's children list - O(1) average-case
    /// No fallback path - HashMap is always initialized
    #[inline]
    fn remove_child_from_parent(&mut self, parent_pid: Pid, child_pid: Pid) {
        self.ensure_initialized();
        if let Some(children) = self.parent_to_children.get_mut(&parent_pid) {
            children.retain(|&pid| pid != child_pid);
            // Remove empty parent entry to save memory
            if children.is_empty() {
                self.parent_to_children.remove(&parent_pid);
            }
        }
    }

    /// Get children of a parent process - O(1) average-case
    /// No fallback path - HashMap is always initialized
    #[inline]
    pub fn get_children(&self, parent_pid: Pid) -> Option<&Vec<Pid>> {
        self.parent_to_children.get(&parent_pid)
    }

    /// Free a process
    pub fn free(&mut self, pid: Pid) {
        self.ensure_initialized();
        
        if let Some(&idx) = self.pid_to_index.get(&pid) {
            // Get parent PID before mutable borrow
            let parent_pid = self.procs[idx].parent;
            
            // Remove from parent's children list before freeing
            if let Some(parent_pid) = parent_pid {
                self.remove_child_from_parent(parent_pid, pid);
            }
            
            let proc = &mut self.procs[idx];

                // Free kernel stack using resource pool
                if proc.kstack != 0 {
                    let stack_addr = (proc.kstack - PAGE_SIZE) as usize;
                    self.resource_pools.free_stack(stack_addr);
                    proc.kstack = 0;
                }

                // Free trapframe using resource pool
                if !proc.trapframe.is_null() {
                    let tf_addr = proc.trapframe as usize;
                    self.resource_pools.free_trapframe(tf_addr);
                    proc.trapframe = null_mut();
                }

                // Free page table and all user pages
                if !proc.pagetable.is_null() {
                    unsafe { free_pagetable(proc.pagetable); }
                    proc.pagetable = null_mut();
                }

            // Reset process state
            proc.state = ProcState::Unused;
            proc.signals = None;
            proc.cwd_path = None;
            proc.cwd = None;
            proc.sz = 0;
            proc.parent = None;

            // Remove from PID map
            self.pid_to_index.remove(&pid);
            crate::process::rcu_table::with_sharded(|s| s.remove(pid));

            // Add back to free list for O(1) reuse
            self.free_list.push(idx);
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

    /// Validate process resource consistency (for debugging)
    #[cfg(debug_assertions)]
    pub fn validate_resources(&self) -> bool {
        for proc in self.iter() {
            if proc.state != ProcState::Unused {
                // Check that allocated resources are valid
                if proc.kstack != 0 && proc.trapframe.is_null() {
                    return false; // Inconsistent allocation
                }
                if proc.trapframe.is_null() && proc.kstack != 0 {
                    return false; // Inconsistent allocation
                }
            }
        }
        true
    }

    /// Get resource usage statistics
    pub fn resource_stats(&self) -> (usize, usize, usize, usize) {
        let mut used_procs = 0;
        let mut free_procs = 0;
        let stack_pool_size = self.resource_pools.stack_pool.len();
        let trapframe_pool_size = self.resource_pools.trapframe_pool.len();

        for proc in self.iter() {
            match proc.state {
                ProcState::Unused => free_procs += 1,
                _ => used_procs += 1,
            }
        }

        (used_procs, free_procs, stack_pool_size, trapframe_pool_size)
    }
}

// ============================================================================
// Global State
// ============================================================================

/// Global process table
pub static PROC_TABLE: Mutex<ProcTable> = Mutex::new(ProcTable::const_new());

/// Current process PID for each CPU (indexed by CPU ID)
static mut CURRENT_PID: [Option<Pid>; 8] = [None; 8];

// ============================================================================
// Public API
// ============================================================================

/// Initialize process subsystem
pub fn init() {
    let mut table = PROC_TABLE.lock();

    // Ensure hash maps are initialized with pre-allocated capacity for optimal performance
    table.ensure_initialized();

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

    // Extract all parent data first, then release borrow
    let (parent_pgid, parent_sid, parent_uid, parent_gid, parent_euid, parent_egid, parent_suid, parent_sgid, parent_nice, parent_umask, parent_ofile, parent_cwd_path, parent_cwd, parent_rlimits, parent_pagetable, parent_sz, parent_trapframe) = {
        let parent = table.find(parent_pid)?;
        (parent.pgid, parent.sid, parent.uid, parent.gid, parent.euid, parent.egid, parent.suid, parent.sgid, parent.nice, parent.umask, parent.ofile.clone(), parent.cwd_path.clone(), parent.cwd, parent.rlimits.clone(), parent.pagetable, parent.sz, parent.trapframe)
    };

    // Allocate child process (now we can use table mutably again)
    let child = table.alloc()?;
    let child_pid = child.pid;

    // Initialize child process state
    child.parent = Some(parent_pid);
    child.state = ProcState::Runnable;
    child.pgid = parent_pgid;
    child.sid = parent_sid;
    // Inherit credentials from parent
    child.uid = parent_uid;
    child.gid = parent_gid;
    child.euid = parent_euid;
    child.egid = parent_egid;
    child.suid = parent_suid;
    child.sgid = parent_sgid;
    child.nice = parent_nice;
    child.umask = parent_umask;
    
    // Drop mutable borrow of child before calling add_child_to_parent
    drop(child);

    // Add child to parent's children list for O(1) wait() lookup
    table.ensure_initialized();
    let child_idx = table.pid_to_index.get(&child_pid).copied();
    
    table.add_child_to_parent(parent_pid, child_pid);
    
    // Re-acquire child reference for remaining initialization
    let child = if let Some(idx) = child_idx {
        &mut table.procs[idx]
    } else {
        return None;
    };

    // Copy parent's file descriptors (shallow copy)
    child.ofile.copy_from_slice(&parent_ofile);
    // Increment file reference counts for copied file descriptors
    for fd in &child.ofile {
        if let Some(file) = fd {
            // Note: This will be properly implemented when the file system is complete
            // For now, we acknowledge that file references should be incremented
            crate::println!("[process] Copied file descriptor, should increment ref count");
        }
    }

    // Copy working directory
    child.cwd_path = parent_cwd_path;
    child.cwd = parent_cwd;

    // Copy resource limits
    child.rlimits.copy_from_slice(&parent_rlimits);

    // Copy page table with copy-on-write semantics
    if let Some(pagetable) = unsafe { crate::mm::vm::copy_pagetable(parent_pagetable) } {
        child.pagetable = pagetable;
        child.sz = parent_sz;
    } else {
        // Failed to copy pagetable, clean up and return None
        table.free(child_pid);
        return None;
    }

    // Copy trapframe from parent and set child's return value to 0
    unsafe {
        *child.trapframe = *parent_trapframe;
    }
    
    // Set return value to 0 for child process (architecture-specific register)
    unsafe {
        #[cfg(target_arch = "riscv64")]
        {
            (*child.trapframe).a0 = 0;
        }

        #[cfg(target_arch = "aarch64")]
        {
            (*child.trapframe).regs[0] = 0; // On aarch64, a0 is regs[0]
        }

        #[cfg(target_arch = "x86_64")]
        {
            (*child.trapframe).rax = 0; // On x86_64, rax is the return value register
        }
    }

    Some(child_pid)
}

/// Exit current process
pub fn exit(status: i32) {
    if let Some(pid) = myproc() {
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            // Encode exit status according to POSIX format
            // Normal exit: bits 8-15 contain exit code, bit 7 is 0
            proc.xstate = (status & 0xff) << 8;
            proc.state = ProcState::Zombie;

            // Close all open files efficiently
            for fd_slot in proc.ofile.iter_mut() {
                if let Some(fd_idx) = *fd_slot {
                    crate::fs::file_close(fd_idx);
                    *fd_slot = None;
                }
            }

            // Invalidate all file descriptor caches
            proc.invalidate_all_fd_cache();

            // Clear working directory references
            proc.cwd_path = None;
            proc.cwd = None;

            // Wake up parent
            if let Some(parent_pid) = proc.parent {
                wakeup_pid(&mut table, parent_pid);
            }

            // Reparent children to init
            reparent_children(&mut table, pid);
        }
    }

    // Yield CPU to allow scheduler to clean up
    yield_cpu();
}

/// Wait for child process to exit - O(1) child lookup using parent_to_children index
pub fn wait(status: *mut i32) -> Option<Pid> {
    let parent_pid = myproc()?;

    loop {
        let mut table = PROC_TABLE.lock();
        let mut zombie_child: Option<(Pid, i32)> = None;

        // Use O(1) lookup to get children list
        if let Some(children) = table.get_children(parent_pid) {
            // Check each child for zombie state - O(k) where k is number of children
            for &child_pid in children.iter() {
                if let Some(child_proc) = table.find_ref(child_pid) {
                    if child_proc.state == ProcState::Zombie {
                        zombie_child = Some((child_pid, child_proc.xstate));
                        break;
                    }
                }
            }
        } else {
            // No children exist
            return None;
        }

        // If found a zombie, free it and return
        if let Some((child_pid, xstate)) = zombie_child {
            if !status.is_null() {
                unsafe { *status = xstate; }
            }
            table.free(child_pid);
            return Some(child_pid);
        }

        // Sleep waiting for child
        drop(table);
        sleep(parent_pid as usize);
    }
}

/// Wait for a specific child process with options
/// Arguments: pid - child PID to wait for (-1 for any child), status - pointer to status, options - wait options
/// Returns: child PID on success, None on failure
pub fn waitpid(pid: i32, status: *mut i32, options: i32) -> Option<Pid> {
    use crate::posix;

    let parent_pid = myproc()?;
    let no_hang = (options & posix::WNOHANG) != 0;
    let untraced = (options & posix::WUNTRACED) != 0;

    loop {
        let mut table = PROC_TABLE.lock();
        let mut found_child: Option<(Pid, i32)> = None;

        // Determine which children to check
        let children_to_check: Vec<Pid> = if pid == -1 {
            // Wait for any child
            if let Some(children) = table.get_children(parent_pid) {
                children.clone()
            } else {
                // No children exist
                return None;
            }
        } else if pid > 0 {
            // Wait for specific child
            vec![pid as Pid]
        } else {
            // TODO: Support process group waiting (pid < -1) and same group (pid == 0)
            // For now, treat as any child
            if let Some(children) = table.get_children(parent_pid) {
                children.clone()
            } else {
                return None;
            }
        };

        // Check each child for matching state
        for &child_pid in &children_to_check {
            if let Some(child_proc) = table.find_ref(child_pid) {
                // Check if this is actually a child of the current process
                if child_proc.parent != Some(parent_pid) {
                    continue;
                }

                // Check for zombie state (exited)
                if child_proc.state == ProcState::Zombie {
                    found_child = Some((child_pid, child_proc.xstate));
                    break;
                }

                // Check for stopped state if WUNTRACED is set
                if untraced && child_proc.state == ProcState::Stopped {
                    // Encode stopped status: bit 7 set, bits 8-15 contain stop signal
                    let stop_status = (child_proc.xstate & 0xff) | 0x7f;
                    found_child = Some((child_pid, stop_status));
                    break;
                }

                // Check for running state if WNOHANG is set (no child available yet)
                if no_hang && (child_proc.state == ProcState::Running || child_proc.state == ProcState::Runnable) {
                    // No status available yet, but don't block
                    return None;
                }
            }
        }

        // If found a child, return status (don't clean up stopped processes)
        if let Some((child_pid, xstate)) = found_child {
            // Write status to user space if requested
            if !status.is_null() {
                unsafe { *status = xstate; }
            }

            // Only clean up zombie processes, not stopped ones
            if let Some(child_proc) = table.find_ref(child_pid) {
                if child_proc.state == ProcState::Zombie {
                    table.free(child_pid);
                }
            }

            return Some(child_pid);
        }

        // If WNOHANG is set and no child found, return immediately
        if no_hang {
            return None;
        }

        // Sleep waiting for child state change
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
        if let Some(ref mut signals) = proc.signals {
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
/// Returns 0 if no process is currently running (should not happen in normal operation)
pub fn getpid() -> Pid {
    myproc().unwrap_or(0)
}

/// Allocate file descriptor for current process
/// 
/// This function allocates a file descriptor and updates the cache
/// for commonly used file descriptors (0-7) to enable O(1) lookup.
pub fn fdalloc(file_idx: usize) -> Option<i32> {
    let pid = myproc()?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid)?;
    
    for (i, slot) in proc.ofile.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(file_idx);
            let fd = i as i32;
            // Update cache for commonly used file descriptors (0-7)
            proc.update_fd_cache(fd, file_idx);
            return Some(fd);
        }
    }
    None
}

/// Close file descriptor for current process
///
/// This function closes a file descriptor and invalidates the cache
/// for commonly used file descriptors (0-7).
pub fn fdclose(fd: i32) -> Option<usize> {
    if let Some(pid) = myproc() {
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            if fd >= 0 && (fd as usize) < NOFILE {
                if let Some(file_idx) = proc.ofile[fd as usize] {
                    proc.ofile[fd as usize] = None;
                    // Invalidate cache for commonly used file descriptors (0-7)
                    proc.invalidate_fd_cache(fd);
                    return Some(file_idx);
                }
            }
        }
    }
    None
}

/// Lookup file descriptor
/// 
/// This function provides O(1) lookup for file descriptors.
/// For commonly used file descriptors (0-7), it uses the cache
/// to avoid repeated file table lookups.
pub fn fdlookup(fd: i32) -> Option<usize> {
    let pid = myproc()?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(pid)?;
    
    // Try cache first for commonly used file descriptors (0-7)
    if fd >= 0 && fd < 8 {
        if let Some(file_idx) = proc.get_cached_fd(fd) {
            return Some(file_idx);
        }
    }
    
    // Fall back to regular lookup
    if fd >= 0 && (fd as usize) < NOFILE {
        proc.ofile[fd as usize]
    } else {
        None
    }
}

/// Install a file at a specific file descriptor
/// 
/// This function installs a file at a specific file descriptor and
/// updates the cache for commonly used file descriptors (0-7).
pub fn fdinstall(fd: i32, file_idx: usize) -> Result<(), ()> {
    let pid = myproc().ok_or(())?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(())?;
    
    if fd >= 0 && (fd as usize) < NOFILE {
        proc.ofile[fd as usize] = Some(file_idx);
        // Update cache for commonly used file descriptors (0-7)
        proc.update_fd_cache(fd, file_idx);
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
    // Collect children to reparent
    let mut children_to_reparent = Vec::new();
    
    if let Some(children) = table.get_children(parent_pid) {
        children_to_reparent.extend_from_slice(children);
    }
    
    // Reparent each child to init (PID 1)
    for &child_pid in &children_to_reparent {
        // Remove from old parent's children list first
        table.remove_child_from_parent(parent_pid, child_pid);
        
        // Then update child's parent
        if let Some(child_proc) = table.find(child_pid) {
            child_proc.parent = Some(1); // Reparent to init
            
            // Add to init's children list
            table.add_child_to_parent(1, child_pid);
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

        // Use the new thread-based scheduler
        crate::process::thread::schedule();
    }
}
