// Process management syscalls

use super::common::{SyscallError, SyscallResult, extract_args};
use crate::mm::vm::copyinstr;
use crate::process::{myproc, PROC_TABLE};
use alloc::string::String;
use alloc::vec::Vec;

/// Dispatch process management syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        // Process creation and management
        0x1000 => sys_fork(args),           // fork
        0x1001 => sys_execve(args),         // execve
        0x1002 => sys_waitpid(args),        // waitpid
        0x1003 => sys_exit(args),           // exit
        0x1004 => sys_getpid(args),         // getpid
        0x1005 => sys_getppid(args),        // getppid
        0x1006 => sys_setuid(args),         // setuid
        0x1007 => sys_getuid(args),         // getuid
        0x1008 => sys_setgid(args),         // setgid
        0x1009 => sys_getgid(args),         // getgid
        0x100A => sys_setsid(args),         // setsid
        0x100B => sys_getsid(args),         // getsid
        0x100C => sys_nice(args),           // nice
        0x100D => sys_sched_yield(args),    // sched_yield
        0x100E => sys_sched_get_priority_max(args), // sched_get_priority_max
        0x100F => sys_sched_get_priority_min(args), // sched_get_priority_min
        0x1010 => sys_sched_setscheduler(args),     // sched_setscheduler
        0x1011 => sys_sched_getscheduler(args),     // sched_getscheduler
        0x1012 => sys_sched_setparam(args),         // sched_setparam
        0x1013 => sys_sched_getparam(args),         // sched_getparam
        0x1014 => sys_sched_setaffinity(args),      // sched_setaffinity
        0x1015 => sys_sched_getaffinity(args),      // sched_getaffinity
        0x1016 => sys_prctl(args),          // prctl
        0x1017 => sys_capget(args),         // capget
        0x1018 => sys_capset(args),         // capset
        0x1019 => sys_sbrk(args),           // sbrk
        0x101A => sys_sleep(args),          // sleep
        0x101B => sys_uptime(args),         // uptime
        0x101C => sys_setpgid(args),        // setpgid
        0x101D => sys_getpgid(args),        // getpgid
        0x101E => sys_getrlimit(args),      // getrlimit
        0x101F => sys_setrlimit(args),      // setrlimit
        0x1020 => sys_wait4(args),          // wait4
        0x1021 => sys_raise(args),          // raise
        _ => Err(SyscallError::InvalidSyscall),
    }
}

// ============================================================================
// Core Process Management System Calls Implementation
// ============================================================================

/// Fork the current process
/// Arguments: []
/// Returns: 0 in child process, child PID in parent process, error on failure
fn sys_fork(_args: &[u64]) -> SyscallResult {
    // Call the fork implementation
    match crate::process::manager::fork() {
        Some(child_pid) => {
            // Check if current process is the child
            if let Some(current_pid) = crate::process::myproc() {
                if current_pid == child_pid {
                    // In child process: return 0
                    Ok(0)
                } else {
                    // In parent process: return child PID
                    Ok(child_pid as u64)
                }
            } else {
                Err(SyscallError::NotFound)
            }
        }
        None => Err(SyscallError::OutOfMemory),
    }
}

/// Execute a program
/// Arguments: [pathname_ptr, argv_ptr, envp_ptr]
/// Returns: does not return on success, error on failure
fn sys_execve(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let pathname_ptr = args[0] as usize;
    let argv_ptr = args[1] as usize;
    let envp_ptr = args[2] as usize;

    // Check if root file system is mounted
    if !crate::vfs::is_root_mounted() {
        return Err(SyscallError::IoError);
    }

    // Get current process for pagetable
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(proc_table);

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read pathname from user space
    const MAX_PATH_LEN: usize = 4096;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };
    let path_slice = &path_buf[..path_len];

    // Read argv from user space
    let args_vec = read_user_argv_array(pagetable, argv_ptr)
        .ok_or(SyscallError::BadAddress)?;

    // Read envp from user space
    let envs_vec = read_user_argv_array(pagetable, envp_ptr)
        .ok_or(SyscallError::BadAddress)?;

    // Convert to slices
    let arg_slices: Vec<&[u8]> = args_vec.iter().map(|a| a.as_slice()).collect();
    let env_slices: Vec<&[u8]> = envs_vec.iter().map(|a| a.as_slice()).collect();

    // Convert path to string
    let path_str = String::from_utf8(path_slice.to_vec())
        .map_err(|_| SyscallError::InvalidArgument)?;

    // Resolve path
    let abs_path = {
        let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
        let cwd = proc.cwd_path.as_ref().map(|s| s.as_str()).unwrap_or("/");
        
        if path_str.starts_with('/') {
            path_str
        } else if cwd == "/" {
            format!("/{}", path_str)
        } else {
            format!("{}/{}", cwd, path_str)
        }
    };

    // Open file via VFS
    let vfs = crate::vfs::vfs();
    let mut file = vfs.open(&abs_path, crate::posix::O_RDONLY as u32)
        .map_err(|_| SyscallError::NotFound)?;

    // Read file contents
    let mut buf = alloc::vec::Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = file.read(tmp.as_mut_ptr() as usize, tmp.len())
            .unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }

    // Execute the program
    match crate::process::exec::exec(&buf, &arg_slices, &env_slices, Some(abs_path.as_bytes())) {
        Ok(_) => {
            // exec does not return on success
            // This should never be reached, but we include it for completeness
            Ok(0)
        }
        Err(e) => Err(e.into()), // Convert ExecError to SyscallError
    }
}

/// Wait for a child process to exit
/// Arguments: [pid, status_ptr, options]
/// Returns: child PID on success, error on failure
/// Note: wait4 is similar to waitpid, we implement waitpid here
fn sys_waitpid(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let pid = args[0] as i32;
    let status_ptr = args[1] as usize;
    let options = args[2] as i32; // WNOHANG, WUNTRACED, etc.

    // Get current process
    let current_pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(current_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(proc_table);

    // Convert status pointer
    let status_mut_ptr = if status_ptr != 0 {
        Some(status_ptr as *mut i32)
    } else {
        None
    };

    // Call wait implementation with options
    match crate::process::manager::waitpid(pid, status_mut_ptr.unwrap_or(core::ptr::null_mut()), options) {
        Some(child_pid) => {
            // Write status to user space if pointer provided
            if let Some(_ptr) = status_mut_ptr {
                if !pagetable.is_null() {
                    // Get exit status from process table
                    let proc_table = crate::process::manager::PROC_TABLE.lock();
                    if let Some(child_proc) = proc_table.find_ref(child_pid) {
                        let exit_status = child_proc.xstate;
                        drop(proc_table);

                        // Copy status to user space
                        unsafe {
                            let status_slice = core::slice::from_raw_parts_mut(
                                &exit_status as *const i32 as *mut u8,
                                core::mem::size_of::<i32>()
                            );
                            crate::mm::vm::copyout(pagetable, status_ptr, status_slice.as_ptr(), status_slice.len())
                                .map_err(|_| SyscallError::BadAddress)?;
                        }
                    }
                }
            }
            Ok(child_pid as u64)
        }
        None => Err(SyscallError::NotFound),
    }
}

/// Exit the current process
/// Arguments: [status]
/// Returns: does not return
fn sys_exit(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let status = args[0] as i32;

    // Call exit implementation
    crate::process::manager::exit(status);

    // Exit should not return, but if it does, return success
    Ok(0)
}

/// Get current process ID
/// Arguments: []
/// Returns: current process PID
fn sys_getpid(_args: &[u64]) -> SyscallResult {
    match crate::process::myproc() {
        Some(pid) => Ok(pid as u64),
        None => Err(SyscallError::NotFound),
    }
}

/// Get parent process ID
/// Arguments: []
/// Returns: parent process PID, or 0 if no parent
fn sys_getppid(_args: &[u64]) -> SyscallResult {
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let ppid = proc.parent.unwrap_or(0);
    Ok(ppid as u64)
}

/// Helper function to read argv array from user space
fn read_user_argv_array(pagetable: *mut crate::mm::vm::PageTable, addr: usize) -> Option<Vec<Vec<u8>>> {
    if addr == 0 {
        return None;
    }

    let mut args = Vec::new();
    let mut ptr = addr;
    const MAX_ARGS: usize = 256;
    const MAX_ARG_LEN: usize = 4096;

    unsafe {
        loop {
            if args.len() > MAX_ARGS {
                return None;
            }

            // Read pointer to argument string
            let mut arg_ptr_bytes = [0u8; core::mem::size_of::<usize>()];
            crate::mm::vm::copyin(pagetable, arg_ptr_bytes.as_mut_ptr(), ptr, core::mem::size_of::<usize>())
                .ok()?;

            let arg_ptr = usize::from_le_bytes(arg_ptr_bytes);
            if arg_ptr == 0 {
                break;
            }

            // Read argument string
            let mut arg_buf = [0u8; MAX_ARG_LEN];
            let arg_len = copyinstr(pagetable, arg_ptr, arg_buf.as_mut_ptr(), MAX_ARG_LEN)
                .ok()?;
            args.push(arg_buf[..arg_len].to_vec());

            ptr += core::mem::size_of::<usize>();
        }
    }

    Some(args)
}

fn sys_setuid(_args: &[u64]) -> SyscallResult {
    // TODO: Implement setuid syscall
    Err(SyscallError::NotSupported)
}

fn sys_getuid(_args: &[u64]) -> SyscallResult {
    // TODO: Implement getuid syscall
    Err(SyscallError::NotSupported)
}

fn sys_setgid(_args: &[u64]) -> SyscallResult {
    // TODO: Implement setgid syscall
    Err(SyscallError::NotSupported)
}

fn sys_getgid(_args: &[u64]) -> SyscallResult {
    // TODO: Implement getgid syscall
    Err(SyscallError::NotSupported)
}

/// Wait for child process to exit with extended options
/// Arguments: [pid, status_ptr, options, rusage_ptr]
/// Returns: child PID on success, error on failure
pub fn sys_wait4(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;

    let args = extract_args(args, 4)?;
    let pid = args[0] as i32;
    let status_ptr = args[1] as *mut i32;
    let options = args[2] as i32;
    let _rusage_ptr = args[3] as *mut crate::posix::Rusage; // TODO: Implement rusage support

    // Call waitpid implementation
    match crate::process::manager::waitpid(pid, status_ptr, options) {
        Some(child_pid) => Ok(child_pid as u64),
        None => {
            // No child found, check if WNOHANG was set
            if (options & crate::posix::WNOHANG) != 0 {
                Ok(0) // No child available, but don't block
            } else {
                Err(SyscallError::NotFound)
            }
        }
    }
}

/// Raise a signal for the current process
/// Arguments: [sig]
/// Returns: 0 on success, error on failure
pub fn sys_raise(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    
    let args = extract_args(args, 1)?;
    let sig = args[0] as u32;
    
    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    
    // Send signal to current process
    crate::ipc::signal::kill(pid as usize, sig)
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    Ok(0)
}

/// Kill a process
pub fn kill_process(pid: u64, signal: i32) -> Result<(), i32> {
    // Validate signal number
    if signal < 0 || signal >= crate::ipc::signal::NSIG as i32 {
        return Err(crate::reliability::errno::EINVAL);
    }
    
    // Find target process
    let mut proc_table = crate::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find(pid as crate::process::Pid) {
        if let Some(ref signals) = proc.signals {
            // Send signal to process
            signals.send_signal(signal as u32)
                .map_err(|_| crate::reliability::errno::EINVAL)?;
            
            // Wake up process if it's sleeping
            if proc.state == crate::process::ProcState::Sleeping {
                // TODO: Wake up sleeping process
                // This would involve signaling the scheduler
            }
            
            Ok(())
        } else {
            Err(crate::reliability::errno::ESRCH)
        }
    } else {
        Err(crate::reliability::errno::ESRCH)
    }
}

/// Set hostname for a process
pub fn set_hostname_for_process(_pid: u64, _hostname: &str) -> Result<(), i32> {
    // TODO: Implement set_hostname_for_process
    Err(crate::reliability::errno::ENOSYS)
}

/// Set hostname
pub fn set_hostname(_hostname: &str) -> Result<(), i32> {
    // TODO: Implement set_hostname
    Err(crate::reliability::errno::ENOSYS)
}

fn sys_setsid(_args: &[u64]) -> SyscallResult {
    // TODO: Implement setsid syscall
    Err(SyscallError::NotSupported)
}

fn sys_getsid(_args: &[u64]) -> SyscallResult {
    // TODO: Implement getsid syscall
    Err(SyscallError::NotSupported)
}

fn sys_nice(_args: &[u64]) -> SyscallResult {
    // TODO: Implement nice syscall
    Err(SyscallError::NotSupported)
}

fn sys_sched_yield(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sched_yield syscall
    Err(SyscallError::NotSupported)
}

fn sys_sched_get_priority_max(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sched_get_priority_max syscall
    Err(SyscallError::NotSupported)
}

fn sys_sched_get_priority_min(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sched_get_priority_min syscall
    Err(SyscallError::NotSupported)
}

fn sys_sched_setscheduler(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sched_setscheduler syscall
    Err(SyscallError::NotSupported)
}

fn sys_sched_getscheduler(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sched_getscheduler syscall
    Err(SyscallError::NotSupported)
}

fn sys_sched_setparam(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sched_setparam syscall
    Err(SyscallError::NotSupported)
}

fn sys_sched_getparam(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sched_getparam syscall
    Err(SyscallError::NotSupported)
}

fn sys_sched_setaffinity(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sched_setaffinity syscall
    Err(SyscallError::NotSupported)
}

fn sys_sched_getaffinity(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sched_getaffinity syscall
    Err(SyscallError::NotSupported)
}

fn sys_prctl(_args: &[u64]) -> SyscallResult {
    // TODO: Implement prctl syscall
    Err(SyscallError::NotSupported)
}

fn sys_capget(_args: &[u64]) -> SyscallResult {
    // TODO: Implement capget syscall
    Err(SyscallError::NotSupported)
}

fn sys_capset(_args: &[u64]) -> SyscallResult {
    // TODO: Implement capset syscall
    Err(SyscallError::NotSupported)
}

fn sys_sbrk(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let increment = args[0] as isize; // Can be negative for shrinking

    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Get current break (program break)
    let old_sz = proc.sz;
    let new_sz = if increment >= 0 {
        old_sz.saturating_add(increment as usize)
    } else {
        old_sz.saturating_sub((-increment) as usize)
    };

    // Validate new break address
    if new_sz >= crate::mm::vm::KERNEL_BASE {
        return Err(SyscallError::InvalidArgument);
    }

    // Check if we need to extend or shrink the heap
    if new_sz > old_sz {
        // Extend heap: allocate and map new pages
        let pages_needed = ((new_sz - old_sz + crate::mm::vm::PAGE_SIZE - 1) / crate::mm::vm::PAGE_SIZE).max(1);

        for i in 0..pages_needed {
            let va = old_sz + i * crate::mm::vm::PAGE_SIZE;

            // Allocate physical page
            let page = crate::mm::kalloc();
            if page.is_null() {
                // Clean up already allocated pages on failure
                for j in 0..i {
                    let cleanup_va = old_sz + j * crate::mm::vm::PAGE_SIZE;
                    #[cfg(target_arch = "riscv64")]
                    unsafe {
                        if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                            crate::mm::kfree(pa as *mut u8);
                        }
                    }
                    #[cfg(target_arch = "aarch64")]
                    unsafe {
                        if crate::mm::vm::unmap_page(pagetable, cleanup_va).is_ok() {
                            // Note: AArch64 unmap_page doesn't return PA, so we can't free here
                            // TODO: Implement proper physical page tracking for aarch64
                        }
                    }
                    #[cfg(target_arch = "x86_64")]
                    {
                        // x86_64 unmap implementation needed
                        // TODO: Implement proper unmapping for x86_64
                    }
                }
                return Err(SyscallError::OutOfMemory);
            }

            // Zero the page
            unsafe { core::ptr::write_bytes(page, 0, crate::mm::vm::PAGE_SIZE); }

            // Map page with read/write permissions
            let perm = crate::mm::vm::flags::PTE_R | crate::mm::vm::flags::PTE_W | crate::mm::vm::flags::PTE_U;
            unsafe {
                if crate::mm::vm::map_page(pagetable, va, page as usize, perm).is_err() {
                    crate::mm::kfree(page);
                    // Clean up already allocated pages
                    for j in 0..i {
                        let cleanup_va = old_sz + j * crate::mm::vm::PAGE_SIZE;
                        #[cfg(target_arch = "riscv64")]
                        if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                            crate::mm::kfree(pa as *mut u8);
                        }
                    }
                    return Err(SyscallError::OutOfMemory);
                }
            }
        }
    } else if new_sz < old_sz {
        // Shrink heap: unmap and free pages
        let pages_to_free = (old_sz - new_sz + crate::mm::vm::PAGE_SIZE - 1) / crate::mm::vm::PAGE_SIZE;

        for i in 0..pages_to_free {
            let va = new_sz + i * crate::mm::vm::PAGE_SIZE;

            // Unmap page and get physical address
            #[cfg(target_arch = "riscv64")]
            unsafe {
                if let Some(pa) = crate::mm::vm::riscv64::unmap_page(pagetable, va) {
                    crate::mm::kfree(pa as *mut u8);
                }
            }

            #[cfg(target_arch = "aarch64")]
            unsafe {
                if crate::mm::vm::unmap_page(pagetable, va).is_ok() {
                    // Note: AArch64 unmap_page doesn't return PA, so we can't free here
                    // TODO: Implement proper physical page tracking for aarch64
                }
            }

            #[cfg(target_arch = "x86_64")]
            {
                // x86_64 unmap implementation needed
                // TODO: Implement proper unmapping for x86_64
            }
        }

        // Flush TLB for the unmapped region
        for i in 0..pages_to_free {
            let va = new_sz + i * crate::mm::vm::PAGE_SIZE;
            crate::mm::vm::flush_tlb_page(va);
        }
    }

    // Update process size
    proc.sz = new_sz;

    // Return old break address
    Ok(old_sz as u64)
}

fn sys_sleep(_args: &[u64]) -> SyscallResult {
    // TODO: Implement sleep syscall - sleep for specified seconds
    Err(SyscallError::NotSupported)
}

fn sys_uptime(_args: &[u64]) -> SyscallResult {
    // TODO: Implement uptime syscall - get system uptime
    Err(SyscallError::NotSupported)
}

fn sys_setpgid(_args: &[u64]) -> SyscallResult {
    // TODO: Implement setpgid syscall - set process group ID
    Err(SyscallError::NotSupported)
}

fn sys_getpgid(_args: &[u64]) -> SyscallResult {
    // TODO: Implement getpgid syscall - get process group ID
    Err(SyscallError::NotSupported)
}

fn sys_getrlimit(_args: &[u64]) -> SyscallResult {
    // TODO: Implement getrlimit syscall - get resource limits
    Err(SyscallError::NotSupported)
}

fn sys_setrlimit(_args: &[u64]) -> SyscallResult {
    // TODO: Implement setrlimit syscall - set resource limits
    Err(SyscallError::NotSupported)
}

/// Set domain name
pub fn set_domainname(_domainname: &str) -> Result<(), i32> {
    // TODO: Implement set_domainname
    Err(crate::reliability::errno::ENOSYS)
}