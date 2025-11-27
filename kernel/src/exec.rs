//! exec system call - load and execute ELF binaries
//!
//! This module implements the exec() system call which loads an ELF binary
//! and replaces the current process's memory image with it.

extern crate alloc;

use alloc::vec::Vec;
use core::ptr;

use crate::elf::{ElfLoader, ElfError};
use crate::mm::{kalloc, kfree, PAGE_SIZE};
use crate::process::{myproc, TrapFrame, PROC_TABLE};
use crate::vm::arch::PageTable;

/// Maximum size of executable file (16 MB)
pub const MAX_EXEC_SIZE: usize = 16 * 1024 * 1024;

/// Maximum number of arguments
pub const MAX_ARGS: usize = 32;

/// Maximum argument string length
pub const MAX_ARG_LEN: usize = 4096;

/// User stack size (2 pages)
pub const USER_STACK_SIZE: usize = PAGE_SIZE * 2;

/// User stack top address
pub const USER_STACK_TOP: usize = 0x8000_0000;

/// Errors that can occur during exec
#[derive(Debug, Clone, Copy)]
pub enum ExecError {
    /// File not found
    FileNotFound,
    /// File too large
    FileTooLarge,
    /// Invalid ELF format
    InvalidElf,
    /// Memory allocation failed
    OutOfMemory,
    /// Too many arguments
    TooManyArgs,
    /// Argument too long
    ArgTooLong,
    /// No current process
    NoProcess,
    /// Permission denied
    PermissionDenied,
}

impl From<ElfError> for ExecError {
    fn from(_: ElfError) -> Self {
        ExecError::InvalidElf
    }
}

/// Execute a program from ELF data
///
/// Loads an ELF binary from `elf_data` and replaces the current process's
/// memory image with it. The `argv` array contains command line arguments.
///
/// Returns the entry point on success, or an error.
pub fn exec(elf_data: &[u8], argv: &[&[u8]]) -> Result<usize, ExecError> {
    // Validate arguments
    if argv.len() > MAX_ARGS {
        return Err(ExecError::TooManyArgs);
    }
    
    for arg in argv {
        if arg.len() > MAX_ARG_LEN {
            return Err(ExecError::ArgTooLong);
        }
    }
    
    if elf_data.len() > MAX_EXEC_SIZE {
        return Err(ExecError::FileTooLarge);
    }
    
    // Parse and validate ELF
    let loader = ElfLoader::new(elf_data)?;
    
    // Get current process PID
    let pid = myproc().ok_or(ExecError::NoProcess)?;
    
    // Create new page table
    let new_pagetable = create_user_pagetable()?;
    
    // Load ELF using the ElfLoader API
    let elf_info = loader.load(|vaddr, _readable, _writable, _executable| {
        // Allocate a physical page
        let pa = kalloc();
        if pa.is_null() {
            return None;
        }
        
        // Zero the page
        unsafe {
            ptr::write_bytes(pa, 0, PAGE_SIZE);
        }
        
        // Map the page in the page table
        unsafe {
            if map_page(new_pagetable, vaddr, pa as usize, true, true, true).is_err() {
                kfree(pa);
                return None;
            }
        }
        
        // Return pointer to the mapped page for ELF loader to copy data
        Some(pa)
    })?;
    
    // Set up user stack
    let stack_bottom = USER_STACK_TOP - USER_STACK_SIZE;
    for offset in (0..USER_STACK_SIZE).step_by(PAGE_SIZE) {
        let pa = kalloc();
        if pa.is_null() {
            // TODO: Clean up allocated pages on failure
            return Err(ExecError::OutOfMemory);
        }
        unsafe {
            ptr::write_bytes(pa, 0, PAGE_SIZE);
            if map_page(new_pagetable, stack_bottom + offset, pa as usize, true, true, false).is_err() {
                kfree(pa);
                return Err(ExecError::OutOfMemory);
            }
        }
    }
    
    // Push arguments onto stack and get stack pointer
    let (sp, argc, argv_ptr) = push_args_to_stack(argv)?;
    
    let entry = elf_info.entry;
    
    // Update process with new address space
    {
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            // Free old page table
            let old_pagetable = proc.pagetable;
            if !old_pagetable.is_null() {
                free_user_pagetable(old_pagetable);
            }
            
            // Install new page table
            proc.pagetable = new_pagetable;
            proc.sz = USER_STACK_TOP;
            
            // Set up trapframe for return to user
            let tf = proc.trapframe;
            if !tf.is_null() {
                unsafe {
                    setup_trapframe(tf, entry, sp, argc, argv_ptr);
                }
            }
            
            // Activate new page table
            unsafe {
                activate_pagetable(new_pagetable);
            }
        } else {
            // Process not found, clean up
            free_user_pagetable(new_pagetable);
            return Err(ExecError::NoProcess);
        }
    }
    
    Ok(entry)
}

/// Set up trapframe for returning to user space
unsafe fn setup_trapframe(tf: *mut TrapFrame, entry: usize, sp: usize, argc: usize, argv: usize) {
    #[cfg(target_arch = "riscv64")]
    {
        (*tf).epc = entry;
        (*tf).sp = sp;
        (*tf).a0 = argc;
        (*tf).a1 = argv;
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        (*tf).elr = entry;
        (*tf).sp = sp;
        (*tf).regs[0] = argc;
        (*tf).regs[1] = argv;
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        (*tf).rip = entry;
        (*tf).rsp = sp;
        (*tf).rdi = argc;
        (*tf).rsi = argv;
    }
}

/// Create a new user page table with kernel mappings
fn create_user_pagetable() -> Result<*mut PageTable, ExecError> {
    let pagetable = kalloc();
    if pagetable.is_null() {
        return Err(ExecError::OutOfMemory);
    }
    let pagetable = pagetable as *mut PageTable;
    unsafe {
        ptr::write_bytes(pagetable as *mut u8, 0, PAGE_SIZE);
        // TODO: Copy kernel mappings to user page table
    }
    Ok(pagetable)
}

/// Free a user page table and all its pages
fn free_user_pagetable(pagetable: *mut PageTable) {
    if pagetable.is_null() {
        return;
    }
    // TODO: Walk page table and free all user pages
    unsafe {
        kfree(pagetable as *mut u8);
    }
}

/// Map a page in the page table
unsafe fn map_page(
    _pagetable: *mut PageTable,
    _va: usize,
    _pa: usize,
    _user: bool,
    _write: bool,
    _exec: bool,
) -> Result<(), ()> {
    // TODO: Implement proper page table mapping
    Ok(())
}

/// Activate a page table (switch address space)
unsafe fn activate_pagetable(_pagetable: *mut PageTable) {
    #[cfg(target_arch = "riscv64")]
    {
        // Write satp register with Sv39 mode
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        // Write TTBR0_EL1 register
    }
    
    #[cfg(target_arch = "x86_64")]
    {
        // Write CR3 register
    }
}

/// Push arguments onto user stack
fn push_args_to_stack(argv: &[&[u8]]) -> Result<(usize, usize, usize), ExecError> {
    let argc = argv.len();
    
    // Calculate space needed
    let mut strings_size = 0;
    for arg in argv {
        strings_size += arg.len() + 1; // +1 for null terminator
    }
    
    let pointers_size = (argc + 1) * core::mem::size_of::<usize>();
    let total_size = strings_size + pointers_size;
    
    // Align to 16 bytes
    let aligned_size = (total_size + 15) & !15;
    
    let sp = USER_STACK_TOP - aligned_size;
    let argv_ptr = sp;
    
    // TODO: Actually copy strings and set up pointers
    
    Ok((sp, argc, argv_ptr))
}

/// Execute init process (stub)
pub fn exec_init() -> Result<(), ExecError> {
    Err(ExecError::FileNotFound)
}

/// System call handler for exec
pub fn sys_exec(path: usize, argv: usize) -> isize {
    let path_slice = match read_user_string(path) {
        Some(s) => s,
        None => return -1,
    };
    
    let args = match read_user_argv(argv) {
        Some(a) => a,
        None => return -1,
    };
    
    let arg_slices: Vec<&[u8]> = args.iter().map(|a| a.as_slice()).collect();
    
    // TODO: Read ELF file from filesystem using path
    let _ = path_slice;
    let _ = arg_slices;
    
    -1 // File not found
}

/// Read a null-terminated string from user space
fn read_user_string(addr: usize) -> Option<Vec<u8>> {
    if addr == 0 {
        return None;
    }
    
    let mut result = Vec::new();
    let mut ptr = addr;
    
    unsafe {
        loop {
            let byte = *(ptr as *const u8);
            if byte == 0 {
                break;
            }
            result.push(byte);
            ptr += 1;
            
            if result.len() > MAX_ARG_LEN {
                return None;
            }
        }
    }
    
    Some(result)
}

/// Read argv array from user space
fn read_user_argv(addr: usize) -> Option<Vec<Vec<u8>>> {
    if addr == 0 {
        return None;
    }
    
    let mut args = Vec::new();
    let mut ptr = addr;
    
    unsafe {
        loop {
            let arg_ptr = *(ptr as *const usize);
            if arg_ptr == 0 {
                break;
            }
            
            match read_user_string(arg_ptr) {
                Some(s) => args.push(s),
                None => return None,
            }
            
            ptr += core::mem::size_of::<usize>();
            
            if args.len() > MAX_ARGS {
                return None;
            }
        }
    }
    
    Some(args)
}
