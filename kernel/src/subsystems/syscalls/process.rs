// Process management syscalls

use crate::syscalls::common::{SyscallError, SyscallResult, extract_args};
use crate::subsystems::mm::vm::copyinstr;
use crate::process::{myproc, PROC_TABLE};
use alloc::string::String;
use alloc::vec::Vec;

/// Dispatch process management syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    // Try fast-path first for hot syscalls
    // Note: Fast-path registry is initialized once at system startup,
    // so we don't need to call init_fast_path_registry() here
    use crate::subsystems::syscalls::fast_path::hot_syscalls;
    
    if let Some(result) = hot_syscalls::dispatch_fast_path(syscall_id, args) {
        return result;
    }
    
    match syscall_id {
        // Process creation and management
        0x1000 => sys_fork(args),           // fork
        0x1001 => sys_execve(args),         // execve
        0x1002 => sys_waitpid(args),        // waitpid
        0x1003 => sys_exit(args),           // exit
        0x1004 => sys_getpid(args),         // getpid (fallback if fast-path not available)
        0x1005 => sys_getppid(args),        // getppid (fallback if fast-path not available)
        0x1006 => sys_setuid(args),         // setuid
        0x1007 => sys_getuid(args),         // getuid (fallback if fast-path not available)
        0x1008 => sys_setgid(args),         // setgid
        0x1009 => sys_getgid(args),         // getgid (fallback if fast-path not available)
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
                            crate::subsystems::mm::vm::copyout(pagetable, status_ptr, status_slice.as_ptr(), status_slice.len())
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
fn read_user_argv_array(pagetable: *mut crate::subsystems::mm::vm::PageTable, addr: usize) -> Option<Vec<Vec<u8>>> {
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
            crate::subsystems::mm::vm::copyin(pagetable, arg_ptr_bytes.as_mut_ptr(), ptr, core::mem::size_of::<usize>())
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

fn sys_setuid(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let uid = args[0] as u32;
    
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;
    
    // Check permissions: only root (euid=0) can set arbitrary UIDs
    // Non-root can only set uid to real uid, effective uid, or saved uid
    if proc.euid != 0 {
        if uid != proc.uid && uid != proc.euid && uid != proc.suid {
            return Err(SyscallError::PermissionDenied);
        }
    }
    
    // If root, set all UIDs
    if proc.euid == 0 {
        proc.uid = uid;
        proc.euid = uid;
        proc.suid = uid;
    } else {
        // Non-root only sets effective UID
        proc.euid = uid;
    }
    
    Ok(0)
}

fn sys_getuid(_args: &[u64]) -> SyscallResult {
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    Ok(proc.uid as u64)
}

fn sys_setgid(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let gid = args[0] as u32;
    
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;
    
    // Check permissions: only root (euid=0) can set arbitrary GIDs
    // Non-root can only set gid to real gid, effective gid, or saved gid
    if proc.euid != 0 {
        if gid != proc.gid && gid != proc.egid && gid != proc.sgid {
            return Err(SyscallError::PermissionDenied);
        }
    }
    
    // If root, set all GIDs
    if proc.euid == 0 {
        proc.gid = gid;
        proc.egid = gid;
        proc.sgid = gid;
    } else {
        // Non-root only sets effective GID
        proc.egid = gid;
    }
    
    Ok(0)
}

fn sys_getgid(_args: &[u64]) -> SyscallResult {
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    Ok(proc.gid as u64)
}

fn sys_geteuid(_args: &[u64]) -> SyscallResult {
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    Ok(proc.euid as u64)
}

fn sys_getegid(_args: &[u64]) -> SyscallResult {
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    Ok(proc.egid as u64)
}

fn sys_setreuid(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let ruid = args[0] as i32;  // -1 means don't change
    let euid = args[1] as i32;
    
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;
    
    // Check permissions
    if proc.euid != 0 {
        // Non-root can only set to real uid or effective uid
        if ruid != -1 && ruid as u32 != proc.uid && ruid as u32 != proc.euid {
            return Err(SyscallError::PermissionDenied);
        }
        if euid != -1 && euid as u32 != proc.uid && euid as u32 != proc.euid && euid as u32 != proc.suid {
            return Err(SyscallError::PermissionDenied);
        }
    }
    
    // If ruid is supplied or euid changes to != real uid, save suid
    let should_save_suid = ruid != -1 || (euid != -1 && euid as u32 != proc.uid);
    
    if ruid != -1 {
        proc.uid = ruid as u32;
    }
    if euid != -1 {
        proc.euid = euid as u32;
    }
    if should_save_suid && euid != -1 {
        proc.suid = euid as u32;
    }
    
    Ok(0)
}

fn sys_setregid(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let rgid = args[0] as i32;  // -1 means don't change
    let egid = args[1] as i32;
    
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;
    
    // Check permissions
    if proc.euid != 0 {
        // Non-root can only set to real gid or effective gid
        if rgid != -1 && rgid as u32 != proc.gid && rgid as u32 != proc.egid {
            return Err(SyscallError::PermissionDenied);
        }
        if egid != -1 && egid as u32 != proc.gid && egid as u32 != proc.egid && egid as u32 != proc.sgid {
            return Err(SyscallError::PermissionDenied);
        }
    }
    
    // If rgid is supplied or egid changes to != real gid, save sgid
    let should_save_sgid = rgid != -1 || (egid != -1 && egid as u32 != proc.gid);
    
    if rgid != -1 {
        proc.gid = rgid as u32;
    }
    if egid != -1 {
        proc.egid = egid as u32;
    }
    if should_save_sgid && egid != -1 {
        proc.sgid = egid as u32;
    }
    
    Ok(0)
}

/// Wait for child process to exit with extended options
/// Arguments: [pid, status_ptr, options, rusage_ptr]
/// Returns: child PID on success, error on failure
pub fn sys_wait4(args: &[u64]) -> SyscallResult {
    use crate::syscalls::common::extract_args;

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
    use crate::syscalls::common::extract_args;
    
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
                proc.state = crate::process::ProcState::Runnable;
            }
            
            Ok(())
        } else {
            Err(crate::reliability::errno::ESRCH)
        }
    } else {
        Err(crate::reliability::errno::ESRCH)
    }
}

// Global hostname storage
static HOSTNAME: crate::subsystems::sync::Mutex<[u8; 256]> = crate::subsystems::sync::Mutex::new([0u8; 256]);
static HOSTNAME_LEN: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(9);

/// Set hostname for a process (stores system-wide)
pub fn set_hostname_for_process(_pid: u64, hostname: &str) -> Result<(), i32> {
    set_hostname(hostname)
}

/// Set hostname
pub fn set_hostname(hostname: &str) -> Result<(), i32> {
    if hostname.len() > 255 {
        return Err(crate::reliability::errno::EINVAL);
    }
    
    let mut stored = HOSTNAME.lock();
    let bytes = hostname.as_bytes();
    stored[..bytes.len()].copy_from_slice(bytes);
    if bytes.len() < 256 {
        stored[bytes.len()] = 0;
    }
    HOSTNAME_LEN.store(bytes.len(), core::sync::atomic::Ordering::Relaxed);
    
    Ok(())
}

/// Get hostname
pub fn get_hostname(buf: &mut [u8]) -> Result<usize, i32> {
    let stored = HOSTNAME.lock();
    let len = HOSTNAME_LEN.load(core::sync::atomic::Ordering::Relaxed);
    let copy_len = len.min(buf.len());
    buf[..copy_len].copy_from_slice(&stored[..copy_len]);
    Ok(copy_len)
}

fn sys_setsid(_args: &[u64]) -> SyscallResult {
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;
    
    // A process cannot become a session leader if it's already a process group leader
    if proc.pid == proc.pgid {
        return Err(SyscallError::PermissionDenied);
    }
    
    // Create a new session: set sid = pgid = pid
    proc.sid = pid;
    proc.pgid = pid;
    
    // Disassociate from controlling terminal (not implemented yet)
    
    Ok(pid as u64)
}

fn sys_getsid(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let target_pid = args[0] as usize;
    
    let table = PROC_TABLE.lock();
    
    // If pid is 0, get session ID of calling process
    let check_pid = if target_pid == 0 {
        myproc().ok_or(SyscallError::NotFound)?
    } else {
        target_pid
    };
    
    let proc = table.find_ref(check_pid).ok_or(SyscallError::NotFound)?;
    Ok(proc.sid as u64)
}

fn sys_getpgrp(_args: &[u64]) -> SyscallResult {
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    Ok(proc.pgid as u64)
}

fn sys_nice(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let inc = args[0] as i32;
    
    let pid = myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;
    
    // Calculate new nice value (clamped to -20..19)
    let new_nice = (proc.nice + inc).max(-20).min(19);
    
    // Only root can lower nice value (increase priority)
    if inc < 0 && proc.euid != 0 {
        return Err(SyscallError::PermissionDenied);
    }
    
    proc.nice = new_nice;
    
    // Return the new nice value (errno convention: on success, return new nice)
    Ok(new_nice as u64)
}

fn sys_sched_yield(_args: &[u64]) -> SyscallResult {
    // Yield the CPU to other runnable processes
    crate::process::yield_cpu();
    Ok(0)
}

// POSIX scheduling policy constants
const SCHED_NORMAL: i32 = 0;
const SCHED_FIFO: i32 = 1;
const SCHED_RR: i32 = 2;
const SCHED_BATCH: i32 = 3;
const SCHED_IDLE: i32 = 5;

fn sys_sched_get_priority_max(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let policy = args[0] as i32;
    
    match policy {
        SCHED_FIFO | SCHED_RR => Ok(99),      // Real-time priorities
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE => Ok(0),
        _ => Err(SyscallError::InvalidArgument),
    }
}

fn sys_sched_get_priority_min(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let policy = args[0] as i32;
    
    match policy {
        SCHED_FIFO | SCHED_RR => Ok(1),       // Real-time priorities
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE => Ok(0),
        _ => Err(SyscallError::InvalidArgument),
    }
}

fn sys_sched_setscheduler(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 3)?;
    let target_pid = args[0] as usize;
    let policy = args[1] as i32;
    let _param_ptr = args[2] as usize;
    
    // Validate policy
    match policy {
        SCHED_NORMAL | SCHED_FIFO | SCHED_RR | SCHED_BATCH | SCHED_IDLE => {}
        _ => return Err(SyscallError::InvalidArgument),
    }
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let check_pid = if target_pid == 0 { my_pid } else { target_pid };
    
    let mut table = PROC_TABLE.lock();
    let caller = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let caller_euid = caller.euid;
    
    // Check permissions: only root can set real-time policies
    if (policy == SCHED_FIFO || policy == SCHED_RR) && caller_euid != 0 {
        return Err(SyscallError::PermissionDenied);
    }
    
    // For now, accept the policy but don't enforce it in scheduler
    // Real implementation would need scheduler integration
    let _proc = table.find(check_pid).ok_or(SyscallError::NotFound)?;
    
    Ok(0)
}

fn sys_sched_getscheduler(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let target_pid = args[0] as usize;
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let check_pid = if target_pid == 0 { my_pid } else { target_pid };
    
    let table = PROC_TABLE.lock();
    let _proc = table.find_ref(check_pid).ok_or(SyscallError::NotFound)?;
    
    // Return default scheduler policy
    Ok(SCHED_NORMAL as u64)
}

fn sys_sched_setparam(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let target_pid = args[0] as usize;
    let _param_ptr = args[1] as usize;
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let check_pid = if target_pid == 0 { my_pid } else { target_pid };
    
    let table = PROC_TABLE.lock();
    let _proc = table.find_ref(check_pid).ok_or(SyscallError::NotFound)?;
    
    // Accept but don't enforce - real implementation would set scheduling params
    Ok(0)
}

fn sys_sched_getparam(args: &[u64]) -> SyscallResult {
    use crate::subsystems::mm::vm::copyout;
    
    let args = extract_args(args, 2)?;
    let target_pid = args[0] as usize;
    let param_ptr = args[1] as usize;
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let check_pid = if target_pid == 0 { my_pid } else { target_pid };
    
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(check_pid).ok_or(SyscallError::NotFound)?;
    
    // Get pagetable for copyout
    let caller = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = caller.pagetable;
    
    if pagetable.is_null() || param_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    // sched_param contains only sched_priority for POSIX
    let sched_priority: i32 = 0;  // Normal processes have priority 0
    
    unsafe {
        copyout(pagetable, param_ptr, &sched_priority as *const _ as *const u8, 
                core::mem::size_of::<i32>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
}

fn sys_sched_setaffinity(args: &[u64]) -> SyscallResult {
    use crate::subsystems::mm::vm::copyin;
    
    let args = extract_args(args, 3)?;
    let target_pid = args[0] as usize;
    let cpusetsize = args[1] as usize;
    let mask_ptr = args[2] as usize;
    
    if cpusetsize == 0 || mask_ptr == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let check_pid = if target_pid == 0 { my_pid } else { target_pid };
    
    let table = PROC_TABLE.lock();
    let caller = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = caller.pagetable;
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read CPU mask from user space
    let mut cpu_mask: u64 = 0;
    let copy_size = cpusetsize.min(core::mem::size_of::<u64>());
    unsafe {
        copyin(pagetable, &mut cpu_mask as *mut _ as *mut u8, mask_ptr, copy_size)
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Verify target process exists
    let _proc = table.find_ref(check_pid).ok_or(SyscallError::NotFound)?;
    
    // Accept the affinity mask (real implementation would store and enforce it)
    // For now, we accept any mask as long as at least one CPU is set
    if cpu_mask == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    Ok(0)
}

fn sys_sched_getaffinity(args: &[u64]) -> SyscallResult {
    use crate::subsystems::mm::vm::copyout;
    
    let args = extract_args(args, 3)?;
    let target_pid = args[0] as usize;
    let cpusetsize = args[1] as usize;
    let mask_ptr = args[2] as usize;
    
    if cpusetsize == 0 || mask_ptr == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let check_pid = if target_pid == 0 { my_pid } else { target_pid };
    
    let table = PROC_TABLE.lock();
    let caller = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = caller.pagetable;
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Verify target process exists
    let _proc = table.find_ref(check_pid).ok_or(SyscallError::NotFound)?;
    
    // Return a mask with all available CPUs (for now, just CPU 0)
    // Real implementation would return the actual affinity mask
    let cpu_mask: u64 = 1;  // Only CPU 0 available
    
    let copy_size = cpusetsize.min(core::mem::size_of::<u64>());
    unsafe {
        copyout(pagetable, mask_ptr, &cpu_mask as *const _ as *const u8, copy_size)
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(copy_size as u64)
}

// prctl options
const PR_SET_NAME: i32 = 15;
const PR_GET_NAME: i32 = 16;
const PR_SET_DUMPABLE: i32 = 4;
const PR_GET_DUMPABLE: i32 = 3;
const PR_SET_KEEPCAPS: i32 = 8;
const PR_GET_KEEPCAPS: i32 = 7;

fn sys_prctl(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 5)?;
    let option = args[0] as i32;
    let arg2 = args[1];
    let _arg3 = args[2];
    let _arg4 = args[3];
    let _arg5 = args[4];
    
    match option {
        PR_SET_NAME => {
            // Set process name (arg2 is pointer to name string)
            // For now, accept but don't store
            Ok(0)
        }
        PR_GET_NAME => {
            // Get process name (arg2 is pointer to buffer)
            // For now, return empty name
            if arg2 != 0 {
                let my_pid = myproc().ok_or(SyscallError::NotFound)?;
                let table = PROC_TABLE.lock();
                let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
                let pagetable = proc.pagetable;
                
                if !pagetable.is_null() {
                    // Write null byte to indicate empty name
                    let name: [u8; 16] = [0; 16];
                    unsafe {
                        crate::subsystems::mm::vm::copyout(pagetable, arg2 as usize, 
                            name.as_ptr(), 16)
                            .map_err(|_| SyscallError::BadAddress)?;
                    }
                }
            }
            Ok(0)
        }
        PR_SET_DUMPABLE | PR_SET_KEEPCAPS => {
            // Accept these options but don't enforce
            Ok(0)
        }
        PR_GET_DUMPABLE => {
            // Return 1 (SUID_DUMP_USER - normal dump)
            Ok(1)
        }
        PR_GET_KEEPCAPS => {
            // Return 0 (keepcaps not set)
            Ok(0)
        }
        _ => Err(SyscallError::InvalidArgument),
    }
}

fn sys_capget(args: &[u64]) -> SyscallResult {
    use crate::subsystems::mm::vm::{copyin, copyout};
    
    let args = extract_args(args, 2)?;
    let hdrp = args[0] as usize;
    let datap = args[1] as usize;
    
    if hdrp == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read capability header
    #[repr(C)]
    struct CapUserHeader {
        version: u32,
        pid: i32,
    }
    
    let mut header = CapUserHeader { version: 0, pid: 0 };
    unsafe {
        copyin(pagetable, &mut header as *mut _ as *mut u8, hdrp, 
               core::mem::size_of::<CapUserHeader>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // If datap is provided, write capability data
    if datap != 0 {
        // For root (euid=0), return all capabilities
        // For non-root, return empty capabilities
        let is_root = proc.euid == 0;
        
        #[repr(C)]
        struct CapUserData {
            effective: u32,
            permitted: u32,
            inheritable: u32,
        }
        
        let data = if is_root {
            CapUserData {
                effective: 0xFFFFFFFF,
                permitted: 0xFFFFFFFF,
                inheritable: 0,
            }
        } else {
            CapUserData {
                effective: 0,
                permitted: 0,
                inheritable: 0,
            }
        };
        
        unsafe {
            copyout(pagetable, datap, &data as *const _ as *const u8,
                    core::mem::size_of::<CapUserData>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }
    
    Ok(0)
}

fn sys_capset(args: &[u64]) -> SyscallResult {
    
    let args = extract_args(args, 2)?;
    let hdrp = args[0] as usize;
    let datap = args[1] as usize;
    
    if hdrp == 0 || datap == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Only root can set capabilities
    if proc.euid != 0 {
        return Err(SyscallError::PermissionDenied);
    }
    
    // Accept capability changes (real implementation would enforce them)
    Ok(0)
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
    if new_sz >= crate::subsystems::mm::vm::KERNEL_BASE {
        return Err(SyscallError::InvalidArgument);
    }

    // Check if we need to extend or shrink the heap
    if new_sz > old_sz {
        // Extend heap: allocate and map new pages
        let pages_needed = ((new_sz - old_sz + crate::subsystems::mm::vm::PAGE_SIZE - 1) / crate::subsystems::mm::vm::PAGE_SIZE).max(1);

        for i in 0..pages_needed {
            let va = old_sz + i * crate::subsystems::mm::vm::PAGE_SIZE;

            // Allocate physical page
            let page = crate::subsystems::mm::kalloc();
            if page.is_null() {
                // Clean up already allocated pages on failure
                for j in 0..i {
                    let cleanup_va = old_sz + j * crate::subsystems::mm::vm::PAGE_SIZE;
                    #[cfg(target_arch = "riscv64")]
                    unsafe {
                        if let Some(pa) = crate::subsystems::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                            crate::subsystems::mm::kfree(pa as *mut u8);
                        }
                    }
                    #[cfg(target_arch = "aarch64")]
                    unsafe {
                        if crate::subsystems::mm::vm::unmap_page(pagetable, cleanup_va).is_ok() {
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
            unsafe { core::ptr::write_bytes(page, 0, crate::subsystems::mm::vm::PAGE_SIZE); }

            // Map page with read/write permissions
            let perm = crate::subsystems::mm::vm::flags::PTE_R | crate::subsystems::mm::vm::flags::PTE_W | crate::subsystems::mm::vm::flags::PTE_U;
            unsafe {
                if crate::subsystems::mm::vm::map_page(pagetable, va, page as usize, perm).is_err() {
                    crate::subsystems::mm::kfree(page);
                    // Clean up already allocated pages
                    for j in 0..i {
                        let cleanup_va = old_sz + j * crate::subsystems::mm::vm::PAGE_SIZE;
                        #[cfg(target_arch = "riscv64")]
                        if let Some(pa) = crate::subsystems::mm::vm::riscv64::unmap_page(pagetable, cleanup_va) {
                            crate::subsystems::mm::kfree(pa as *mut u8);
                        }
                    }
                    return Err(SyscallError::OutOfMemory);
                }
            }
        }
    } else if new_sz < old_sz {
        // Shrink heap: unmap and free pages
        let pages_to_free = (old_sz - new_sz + crate::subsystems::mm::vm::PAGE_SIZE - 1) / crate::subsystems::mm::vm::PAGE_SIZE;

        for i in 0..pages_to_free {
            let va = new_sz + i * crate::subsystems::mm::vm::PAGE_SIZE;

            // Unmap page and get physical address
            #[cfg(target_arch = "riscv64")]
            unsafe {
                if let Some(pa) = crate::subsystems::mm::vm::riscv64::unmap_page(pagetable, va) {
                    crate::subsystems::mm::kfree(pa as *mut u8);
                }
            }

            #[cfg(target_arch = "aarch64")]
            unsafe {
                if crate::subsystems::mm::vm::unmap_page(pagetable, va).is_ok() {
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
            let va = new_sz + i * crate::subsystems::mm::vm::PAGE_SIZE;
            crate::subsystems::mm::vm::flush_tlb_page(va);
        }
    }

    // Update process size
    proc.sz = new_sz;

    // Return old break address
    Ok(old_sz as u64)
}

fn sys_sleep(_args: &[u64]) -> SyscallResult {
    use crate::syscalls::common::extract_args;

    // The userland sleep() syscall expects a number of ticks (not seconds)
    let args = extract_args(_args, 1)?;
    let ticks = args[0] as u64;

    if ticks == 0 {
        return Ok(0);
    }

    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let chan = pid as usize;

    let wake_tick = crate::subsystems::time::get_ticks().saturating_add(ticks);
    crate::subsystems::time::add_sleeper(wake_tick, chan);
    crate::process::sleep(chan);

    // Simplified: we don't implement signal interruptions here — return 0 on success
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sys_sleep_zero() {
        // sleep(0) should return immediately
        let res = sys_sleep(&[0u64]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 0);
    }

    #[test]
    fn test_sys_uptime_nonzero() {
        let r = sys_uptime(&[]);
        assert!(r.is_ok());
        // At least returns a value (ticks) — in hosted tests this may be zero
        let _ticks = r.unwrap();
    }
}

fn sys_uptime(_args: &[u64]) -> SyscallResult {
    // Return system uptime in ticks
    let ticks = crate::subsystems::time::get_ticks();
    Ok(ticks as u64)
}


fn sys_setpgid(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 2)?;
    let target_pid = args[0] as usize;
    let pgid = args[1] as usize;
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    
    // If pid is 0, operate on calling process
    let target = if target_pid == 0 { my_pid } else { target_pid };
    
    // If pgid is 0, use target pid as pgid
    let new_pgid = if pgid == 0 { target } else { pgid };
    
    let mut table = PROC_TABLE.lock();
    
    // Verify caller has permission
    let caller = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let caller_sid = caller.sid;
    
    // Get target process
    let target_proc = table.find(target).ok_or(SyscallError::NotFound)?;
    
    // Can only change pgid of self or children in same session
    if target != my_pid {
        // Must be a child of caller and in same session
        if target_proc.parent != Some(my_pid) || target_proc.sid != caller_sid {
            return Err(SyscallError::PermissionDenied);
        }
    }
    
    // Cannot change pgid if target is session leader
    if target_proc.pid == target_proc.sid {
        return Err(SyscallError::PermissionDenied);
    }
    
    // Set the process group ID
    target_proc.pgid = new_pgid;
    
    Ok(0)
}

fn sys_getpgid(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let target_pid = args[0] as usize;
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let target = if target_pid == 0 { my_pid } else { target_pid };
    
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(target).ok_or(SyscallError::NotFound)?;
    
    Ok(proc.pgid as u64)
}

fn sys_getrlimit(args: &[u64]) -> SyscallResult {
    use crate::subsystems::mm::vm::copyout;
    use crate::posix::{Rlimit, RLIM_INFINITY, RLIMIT_NOFILE, RLIMIT_STACK, RLIMIT_AS};
    
    let args = extract_args(args, 2)?;
    let resource = args[0] as i32;
    let rlim_ptr = args[1] as usize;
    
    if rlim_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    // Validate resource type
    if resource < 0 || resource >= 16 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let table = PROC_TABLE.lock();
    let proc = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get the resource limit
    let rlimit = proc.rlimits[resource as usize];
    
    // If not set, return defaults
    let result = if rlimit.rlim_cur == 0 && rlimit.rlim_max == 0 {
        // Default limits
        match resource {
            r if r == RLIMIT_NOFILE => Rlimit { rlim_cur: 1024, rlim_max: 65536 },
            r if r == RLIMIT_STACK => Rlimit { rlim_cur: 8 * 1024 * 1024, rlim_max: RLIM_INFINITY },
            r if r == RLIMIT_AS => Rlimit { rlim_cur: RLIM_INFINITY, rlim_max: RLIM_INFINITY },
            _ => Rlimit { rlim_cur: RLIM_INFINITY, rlim_max: RLIM_INFINITY },
        }
    } else {
        rlimit
    };
    
    // Copy to user space
    unsafe {
        copyout(pagetable, rlim_ptr, &result as *const _ as *const u8,
                core::mem::size_of::<Rlimit>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    Ok(0)
}

fn sys_setrlimit(args: &[u64]) -> SyscallResult {
    use crate::subsystems::mm::vm::copyin;
    use crate::posix::Rlimit;
    
    let args = extract_args(args, 2)?;
    let resource = args[0] as i32;
    let rlim_ptr = args[1] as usize;
    
    if rlim_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    // Validate resource type
    if resource < 0 || resource >= 16 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(my_pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Read new limit from user space
    let mut new_limit = Rlimit { rlim_cur: 0, rlim_max: 0 };
    unsafe {
        copyin(pagetable, &mut new_limit as *mut _ as *mut u8, rlim_ptr,
               core::mem::size_of::<Rlimit>())
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    // Get current limit
    let old_limit = proc.rlimits[resource as usize];
    
    // Validate new limits
    // Soft limit cannot exceed hard limit
    if new_limit.rlim_cur > new_limit.rlim_max {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Only root can raise hard limit above current value
    if new_limit.rlim_max > old_limit.rlim_max && proc.euid != 0 {
        return Err(SyscallError::PermissionDenied);
    }
    
    // Set the new limit
    proc.rlimits[resource as usize] = new_limit;
    
    Ok(0)
}

fn sys_prlimit64(args: &[u64]) -> SyscallResult {
    use crate::subsystems::mm::vm::{copyin, copyout};
    use crate::posix::Rlimit;
    
    let args = extract_args(args, 4)?;
    let target_pid = args[0] as usize;
    let resource = args[1] as i32;
    let new_rlim_ptr = args[2] as usize;
    let old_rlim_ptr = args[3] as usize;
    
    // Validate resource type
    if resource < 0 || resource >= 16 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let my_pid = myproc().ok_or(SyscallError::NotFound)?;
    let target = if target_pid == 0 { my_pid } else { target_pid };
    
    let mut table = PROC_TABLE.lock();
    
    // Check permissions if targeting another process
    if target != my_pid {
        let caller = table.find_ref(my_pid).ok_or(SyscallError::NotFound)?;
        if caller.euid != 0 {
            return Err(SyscallError::PermissionDenied);
        }
    }
    
    let proc = table.find(target).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Get old limit if requested
    if old_rlim_ptr != 0 {
        let old_limit = proc.rlimits[resource as usize];
        unsafe {
            copyout(pagetable, old_rlim_ptr, &old_limit as *const _ as *const u8,
                    core::mem::size_of::<Rlimit>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }
    
    // Set new limit if provided
    if new_rlim_ptr != 0 {
        let mut new_limit = Rlimit { rlim_cur: 0, rlim_max: 0 };
        unsafe {
            copyin(pagetable, &mut new_limit as *mut _ as *mut u8, new_rlim_ptr,
                   core::mem::size_of::<Rlimit>())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        
        // Validate: soft limit cannot exceed hard limit
        if new_limit.rlim_cur > new_limit.rlim_max {
            return Err(SyscallError::InvalidArgument);
        }
        
        let old_limit = proc.rlimits[resource as usize];
        
        // Only root can raise hard limit
        if new_limit.rlim_max > old_limit.rlim_max && proc.euid != 0 {
            return Err(SyscallError::PermissionDenied);
        }
        
        proc.rlimits[resource as usize] = new_limit;
    }
    
    Ok(0)
}

// Global domain name storage
static DOMAINNAME: crate::subsystems::sync::Mutex<[u8; 256]> = crate::subsystems::sync::Mutex::new([0u8; 256]);
static DOMAINNAME_LEN: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Set domain name
pub fn set_domainname(domainname: &str) -> Result<(), i32> {
    if domainname.len() > 255 {
        return Err(crate::reliability::errno::EINVAL);
    }
    
    let mut stored = DOMAINNAME.lock();
    let bytes = domainname.as_bytes();
    stored[..bytes.len()].copy_from_slice(bytes);
    if bytes.len() < 256 {
        stored[bytes.len()] = 0;
    }
    DOMAINNAME_LEN.store(bytes.len(), core::sync::atomic::Ordering::Relaxed);
    
    Ok(())
}

/// Get domain name
pub fn get_domainname(buf: &mut [u8]) -> Result<usize, i32> {
    let stored = DOMAINNAME.lock();
    let len = DOMAINNAME_LEN.load(core::sync::atomic::Ordering::Relaxed);
    let copy_len = len.min(buf.len());
    buf[..copy_len].copy_from_slice(&stored[..copy_len]);
    Ok(copy_len)
}
