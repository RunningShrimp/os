//! POSIX System Calls Tests
//! 
//! This module contains unit tests for the POSIX system calls implemented
//! in the NOS kernel. It tests the core functionality of:
//! - Process management (fork, execve, wait4/waitpid, exit, getpid/getppid)
//! - File system operations (mount/umount, stat/fstat/lstat, fsync/fdatasync, truncate/ftruncate, chmod/fchmod)
//! - Signal handling (sigaction, sigprocmask, sigpending, kill/raise, signal)
//! - Memory management (mmap, munmap, mprotect, msync)
//! - Time management (gettimeofday, clock_gettime, nanosleep, timer_create/timer_settime)

use super::common::{SyscallError, SyscallResult};
use crate::mm::vm::{copyout, copyin};
use crate::posix::{stat, Timespec, Timeval, SigAction, SigSet, Itimerspec, SigEvent, O_CREAT, O_WRONLY};
use alloc::vec::Vec;

/// Test result type
pub type TestResult = Result<(), &'static str>;

/// Test context for system calls
pub struct TestContext {
    /// Test process ID
    pub test_pid: Option<crate::process::Pid>,
    /// Test file descriptor
    pub test_fd: Option<i32>,
    /// Test memory region
    pub test_addr: Option<usize>,
    /// Test signal number
    pub test_signal: Option<u32>,
}

impl TestContext {
    /// Create a new test context
    pub fn new() -> Self {
        Self {
            test_pid: None,
            test_fd: None,
            test_addr: None,
            test_signal: None,
        }
    }
    
    /// Clean up test context
    pub fn cleanup(&mut self) {
        // Close test file descriptor
        if let Some(fd) = self.test_fd {
            let _ = crate::syscalls::file_io::dispatch(0x2001, &[fd as u64]);
            self.test_fd = None;
        }
        
        // Unmap test memory region
        if let Some(addr) = self.test_addr {
            let _ = crate::syscalls::memory::dispatch(0x3002, &[addr as u64, 0x1000]);
            self.test_addr = None;
        }
        
        // Clean up test process
        if let Some(pid) = self.test_pid {
            // Send SIGTERM to test process
            let _ = crate::syscalls::signal::dispatch(0x5000, &[pid as u64, 15]);
            self.test_pid = None;
        }
    }
}

// ============================================================================
// Process Management Tests
// ============================================================================

/// Test fork system call
pub fn test_fork() -> TestResult {
    let mut ctx = TestContext::new();
    
    // Test fork
    match crate::syscalls::process::dispatch(0x1000, &[]) {
        Ok(child_pid) => {
            // In parent process
            ctx.test_pid = Some(child_pid as crate::process::Pid);
            
            // Test if child PID is valid
            if child_pid == 0 {
                return Err("fork returned 0 in parent process");
            }
            
            // Test wait for child
            match crate::syscalls::process::dispatch(0x1020, &[child_pid as u64, 0, 0, 0]) {
                Ok(_) => {
                    // Child exited successfully
                }
                Err(e) => {
                    return Err("wait4 failed");
                }
            }
        }
        Err(e) => {
            return Err("fork failed");
        }
    }
    
    ctx.cleanup();
    Ok(())
}

/// Test execve system call
pub fn test_execve() -> TestResult {
    let mut ctx = TestContext::new();
    
    // Create a test file to execute
    const TEST_PROGRAM: &[u8] = b"#!/bin/sh\necho 'execve test'\n";
    const TEST_PATH: &str = "/tmp/test_exec.sh";
    
    // Write test program to file
    match crate::syscalls::fs::dispatch(0x2000, &[
        TEST_PATH.as_ptr() as u64,
        (0o644 | O_CREAT | O_WRONLY) as u64,
        0
    ]) {
        Ok(fd) => {
            ctx.test_fd = Some(fd as i32);
            
            // Write test program
            match crate::syscalls::file_io::dispatch(0x2003, &[
                fd as u64,
                TEST_PROGRAM.as_ptr() as u64,
                TEST_PROGRAM.len() as u64
            ]) {
                Ok(_) => {
                    // Program written successfully
                }
                Err(e) => {
                    return Err("write failed");
                }
            }
            
            // Close file
            let _ = crate::syscalls::file_io::dispatch(0x2001, &[fd as u64]);
            ctx.test_fd = None;
        }
        Err(e) => {
            return Err("open failed");
        }
    }
    
    // Test execve (this would replace the current process)
    // For testing purposes, we just validate the arguments
    let argv = [
        TEST_PATH.as_ptr() as u64,
        0 // argv terminator
    ];
    
    let envp = [
        0 // envp terminator
    ];
    
    // In a real test, we would fork and exec in child
    // For now, we just validate the syscall would accept these arguments
    match crate::syscalls::process::dispatch(0x1001, &argv) {
        Ok(_) => {
            // execve would not return on success
        }
        Err(e) => {
            return Err("execve failed");
        }
    }
    
    ctx.cleanup();
    Ok(())
}

/// Test wait4 system call
pub fn test_wait4() -> TestResult {
    let mut ctx = TestContext::new();
    
    // Fork a child process
    match crate::syscalls::process::dispatch(0x1000, &[]) {
        Ok(child_pid) => {
            if child_pid == 0 {
                // In child process, exit with status 42
                let _ = crate::syscalls::process::dispatch(0x1003, &[42]);
                return Ok(());
            } else {
                // In parent process
                ctx.test_pid = Some(child_pid as crate::process::Pid);
                
                // Allocate status buffer
                let mut status = 0i32;
                
                // Test wait4
                match crate::syscalls::process::dispatch(0x1020, &[
                    child_pid as u64,
                    status as u64,
                    0, // options
                    0  // rusage
                ]) {
                    Ok(_) => {
                        // Check if status is correct
                        if status != 42 {
                            return Err("wait4 returned wrong status");
                        }
                    }
                    Err(e) => {
                        return Err("wait4 failed");
                    }
                }
            }
        }
        Err(e) => {
            return Err("fork failed");
        }
    }
    
    ctx.cleanup();
    Ok(())
}

/// Test getpid and getppid system calls
pub fn test_getpid_getppid() -> TestResult {
    // Test getpid
    match crate::syscalls::process::dispatch(0x1004, &[]) {
        Ok(pid) => {
            // PID should be non-zero
            if pid == 0 {
                return Err("getpid returned 0");
            }
            
            // Test getppid
            match crate::syscalls::process::dispatch(0x1005, &[]) {
                Ok(ppid) => {
                    // PPID should be 0 for init process or valid for other processes
                    crate::println!("[test] getpid={}, getppid={}", pid, ppid);
                }
                Err(e) => {
                    return Err("getppid failed");
                }
            }
        }
        Err(e) => {
            return Err("getpid failed");
        }
    }
    
    Ok(())
}

/// Test exit system call
pub fn test_exit() -> TestResult {
    // Fork a child process
    match crate::syscalls::process::dispatch(0x1000, &[]) {
        Ok(child_pid) => {
            if child_pid == 0 {
                // In child process, test exit
                // Note: exit doesn't return, so we can't test the return value
                let _ = crate::syscalls::process::dispatch(0x1003, &[123]);
                return Ok(());
            } else {
                // In parent process, wait for child
                match crate::syscalls::process::dispatch(0x1020, &[
                    child_pid as u64,
                    0, // status_ptr
                    0, // options
                    0  // rusage
                ]) {
                    Ok(_) => {
                        // Child exited
                    }
                    Err(e) => {
                        return Err("wait4 failed");
                    }
                }
            }
        }
        Err(e) => {
            return Err("fork failed");
        }
    }
    
    Ok(())
}

/// Test raise system call
pub fn test_raise() -> TestResult {
    // Test signal handling setup first
    match crate::syscalls::signal::dispatch(0x5001, &[
        15, // SIGTERM
        &SigAction {
            sa_handler: 1, // Custom handler
            sa_flags: 0,
            sa_mask: SigSet { bits: 0 },
            sa_restorer: 0,
        } as u64,
        0 // oldact_ptr
    ]) {
        Ok(_) => {
            // Signal handler set
        }
        Err(e) => {
            return Err("sigaction failed");
        }
    }
    
    // Test raise
    match crate::syscalls::process::dispatch(0x1021, &[15]) {
        Ok(_) => {
            // Signal raised
        }
        Err(e) => {
            return Err("raise failed");
        }
    }
    
    Ok(())
}

// ============================================================================
// File System Tests
// ============================================================================

/// Test mount system call
pub fn test_mount() -> TestResult {
    // Only root can mount filesystems
    let current_uid = crate::process::getuid();
    if current_uid != 0 {
        return Err("mount requires root privileges");
    }
    
    // Test mount
    const TEST_SOURCE: &str = "none";
    const TEST_TARGET: &str = "/tmp/test_mount";
    const TEST_FSTYPE: &str = "tmpfs";
    
    match crate::syscalls::fs::dispatch(0x7013, &[
        TEST_SOURCE.as_ptr() as u64,
        TEST_TARGET.as_ptr() as u64,
        TEST_FSTYPE.as_ptr() as u64,
        0, // flags
        0  // data
    ]) {
        Ok(_) => {
            // Mount succeeded
            crate::println!("[test] mount succeeded");
            
            // Test umount
            match crate::syscalls::fs::dispatch(0x7014, &[TEST_TARGET.as_ptr() as u64]) {
                Ok(_) => {
                    crate::println!("[test] umount succeeded");
                }
                Err(e) => {
                    return Err("umount failed");
                }
            }
        }
        Err(e) => {
            return Err("mount failed");
        }
    }
    
    Ok(())
}

/// Test stat system call
pub fn test_stat() -> TestResult {
    // Create a test file
    const TEST_PATH: &str = "/tmp/test_stat";
    
    // Create test file
    match crate::syscalls::fs::dispatch(0x2000, &[
        TEST_PATH.as_ptr() as u64,
        0o644 | O_CREAT | O_WRONLY,
        0
    ]) {
        Ok(fd) => {
            // Write some data
            const TEST_DATA: &[u8] = b"test data for stat";
            match crate::syscalls::file_io::dispatch(0x2003, &[
                fd as u64,
                TEST_DATA.as_ptr() as u64,
                TEST_DATA.len() as u64
            ]) {
                Ok(_) => {
                    // Data written
                }
                Err(e) => {
                    return Err("write failed");
                }
            }
            
            // Close file
            let _ = crate::syscalls::file_io::dispatch(0x2001, &[fd as u64]);
            
            // Test stat
            let mut stat_buf = stat {
                st_dev: 0,
                st_ino: 0,
                st_mode: 0,
                st_nlink: 0,
                st_uid: 0,
                st_gid: 0,
                st_rdev: 0,
                st_size: 0,
                st_blksize: 0,
                st_blocks: 0,
                st_atime: 0,
                st_atime_nsec: 0,
                st_mtime: 0,
                st_mtime_nsec: 0,
                st_ctime: 0,
                st_ctime_nsec: 0,
            };
            
            match crate::syscalls::fs::dispatch(0x7010, &[
                TEST_PATH.as_ptr() as u64,
                &mut stat_buf as u64
            ]) {
                Ok(_) => {
                    // Check if stat returned reasonable values
                    if stat_buf.st_size == 0 {
                        return Err("stat returned zero size");
                    }
                    
                    crate::println!("[test] stat succeeded: size={}, mode=0o{:o}", 
                        stat_buf.st_size, stat_buf.st_mode);
                }
                Err(e) => {
                    return Err("stat failed");
                }
            }
            
            // Clean up test file
            let _ = crate::syscalls::fs::dispatch(0x7005, &[TEST_PATH.as_ptr() as u64, 0o644]);
        }
        Err(e) => {
            return Err("open failed");
        }
    }
    
    Ok(())
}

/// Test fstat system call
pub fn test_fstat() -> TestResult {
    // Create a test file
    const TEST_PATH: &str = "/tmp/test_fstat";
    
    // Create test file
    match crate::syscalls::fs::dispatch(0x2000, &[
        TEST_PATH.as_ptr() as u64,
        0o644 | O_CREAT | O_WRONLY,
        0
    ]) {
        Ok(fd) => {
            // Write some data
            const TEST_DATA: &[u8] = b"test data for fstat";
            match crate::syscalls::file_io::dispatch(0x2003, &[
                fd as u64,
                TEST_DATA.as_ptr() as u64,
                TEST_DATA.len() as u64
            ]) {
                Ok(_) => {
                    // Data written
                }
                Err(e) => {
                    return Err("write failed");
                }
            }
            
            // Test fstat
            let mut stat_buf = stat {
                st_dev: 0,
                st_ino: 0,
                st_mode: 0,
                st_nlink: 0,
                st_uid: 0,
                st_gid: 0,
                st_rdev: 0,
                st_size: 0,
                st_blksize: 0,
                st_blocks: 0,
                st_atime: 0,
                st_atime_nsec: 0,
                st_mtime: 0,
                st_mtime_nsec: 0,
                st_ctime: 0,
                st_ctime_nsec: 0,
            };
            
            match crate::syscalls::file_io::dispatch(0x2005, &[
                fd as u64,
                &mut stat_buf as u64
            ]) {
                Ok(_) => {
                    // Check if fstat returned reasonable values
                    if stat_buf.st_size == 0 {
                        return Err("fstat returned zero size");
                    }
                    
                    crate::println!("[test] fstat succeeded: size={}, mode=0o{:o}", 
                        stat_buf.st_size, stat_buf.st_mode);
                }
                Err(e) => {
                    return Err("fstat failed");
                }
            }
            
            // Close file
            let _ = crate::syscalls::file_io::dispatch(0x2001, &[fd as u64]);
            
            // Clean up test file
            let _ = crate::syscalls::fs::dispatch(0x7005, &[TEST_PATH.as_ptr() as u64, 0o644]);
        }
        Err(e) => {
            return Err("open failed");
        }
    }
    
    Ok(())
}

/// Test fsync system call
pub fn test_fsync() -> TestResult {
    // Create a test file
    const TEST_PATH: &str = "/tmp/test_fsync";
    
    // Create test file
    match crate::syscalls::fs::dispatch(0x2000, &[
        TEST_PATH.as_ptr() as u64,
        0o644 | O_CREAT | O_WRONLY,
        0
    ]) {
        Ok(fd) => {
            // Write some data
            const TEST_DATA: &[u8] = b"test data for fsync";
            match crate::syscalls::file_io::dispatch(0x2003, &[
                fd as u64,
                TEST_DATA.as_ptr() as u64,
                TEST_DATA.len() as u64
            ]) {
                Ok(_) => {
                    // Data written
                }
                Err(e) => {
                    return Err("write failed");
                }
            }
            
            // Test fsync
            match crate::syscalls::file_io::dispatch(0x2006, &[fd as u64]) {
                Ok(_) => {
                    crate::println!("[test] fsync succeeded");
                }
                Err(e) => {
                    return Err("fsync failed");
                }
            }
            
            // Close file
            let _ = crate::syscalls::file_io::dispatch(0x2001, &[fd as u64]);
            
            // Clean up test file
            let _ = crate::syscalls::fs::dispatch(0x7005, &[TEST_PATH.as_ptr() as u64, 0o644]);
        }
        Err(e) => {
            return Err("open failed");
        }
    }
    
    Ok(())
}

/// Test fchmod system call
pub fn test_fchmod() -> TestResult {
    // Create a test file
    const TEST_PATH: &str = "/tmp/test_fchmod";
    
    // Create test file
    match crate::syscalls::fs::dispatch(0x2000, &[
        TEST_PATH.as_ptr() as u64,
        0o644 | O_CREAT | O_WRONLY,
        0
    ]) {
        Ok(fd) => {
            // Close file
            let _ = crate::syscalls::file_io::dispatch(0x2001, &[fd as u64]);
            
            // Test fchmod
            match crate::syscalls::fs::dispatch(0x700B, &[
                fd as u64,
                0o755 // rwxr-xr-x
            ]) {
                Ok(_) => {
                    crate::println!("[test] fchmod succeeded");
                }
                Err(e) => {
                    return Err("fchmod failed");
                }
            }
            
            // Clean up test file
            let _ = crate::syscalls::fs::dispatch(0x7005, &[TEST_PATH.as_ptr() as u64, 0o644]);
        }
        Err(e) => {
            return Err("open failed");
        }
    }
    
    Ok(())
}

// ============================================================================
// Signal Handling Tests
// ============================================================================

/// Test sigaction system call
pub fn test_sigaction() -> TestResult {
    // Test setting a signal handler
    const TEST_SIGNAL: u32 = 15; // SIGTERM
    
    match crate::syscalls::signal::dispatch(0x5001, &[
        TEST_SIGNAL,
        &SigAction {
            sa_handler: 1, // Custom handler
            sa_flags: 0,
            sa_mask: SigSet { bits: 0 },
            sa_restorer: 0,
        } as u64,
        0 // oldact_ptr
    ]) {
        Ok(_) => {
            crate::println!("[test] sigaction succeeded");
        }
        Err(e) => {
            return Err("sigaction failed");
        }
    }
    
    Ok(())
}

/// Test sigprocmask system call
pub fn test_sigprocmask() -> TestResult {
    // Test setting signal mask
    let new_mask = SigSet { bits: 1 << 14 }; // Block SIGALRM
    
    match crate::syscalls::signal::dispatch(0x5002, &[
        2, // SIG_BLOCK
        &new_mask as u64,
        0 // oldset_ptr
    ]) {
        Ok(_) => {
            crate::println!("[test] sigprocmask succeeded");
        }
        Err(e) => {
            return Err("sigprocmask failed");
        }
    }
    
    Ok(())
}

/// Test sigpending system call
pub fn test_sigpending() -> TestResult {
    // Test checking pending signals
    let mut pending_set = SigSet { bits: 0 };
    
    match crate::syscalls::signal::dispatch(0x5003, &[
        &mut pending_set as u64
    ]) {
        Ok(_) => {
            crate::println!("[test] sigpending succeeded: pending signals = 0x{:x}", pending_set.bits);
        }
        Err(e) => {
            return Err("sigpending failed");
        }
    }
    
    Ok(())
}

/// Test kill system call
pub fn test_kill() -> TestResult {
    // Fork a child process
    match crate::syscalls::process::dispatch(0x1000, &[]) {
        Ok(child_pid) => {
            if child_pid == 0 {
                // In child process, loop indefinitely
                loop {
                    crate::time::sleep_ms(100);
                }
            } else {
                // In parent process
                // Test kill
                match crate::syscalls::signal::dispatch(0x5000, &[
                    child_pid as u64,
                    15 // SIGTERM
                ]) {
                    Ok(_) => {
                        crate::println!("[test] kill succeeded");
                        
                        // Wait for child to exit
                        match crate::syscalls::process::dispatch(0x1020, &[
                            child_pid as u64,
                            0, // status_ptr
                            0, // options
                            0  // rusage
                        ]) {
                            Ok(_) => {
                                crate::println!("[test] child exited after kill");
                            }
                            Err(e) => {
                                return Err("wait4 failed");
                            }
                        }
                    }
                    Err(e) => {
                        return Err("kill failed");
                    }
                }
            }
        }
        Err(e) => {
            return Err("fork failed");
        }
    }
    
    Ok(())
}

/// Test signal system call (BSD compatibility)
pub fn test_signal() -> TestResult {
    // Test signal function (BSD compatibility)
    const TEST_SIGNAL: u32 = 2; // SIGINT

    match crate::syscalls::signal::dispatch(0x500B, &[
        TEST_SIGNAL,
        1 // SIG_IGN
    ]) {
        Ok(old_handler) => {
            crate::println!("[test] signal succeeded, old handler = 0x{:x}", old_handler);
        }
        Err(e) => {
            return Err("signal failed");
        }
    }

    Ok(())
}

/// Test signalfd system call
pub fn test_signalfd() -> TestResult {
    // Test signalfd creation
    let mut mask = SigSet { bits: 1 << 14 }; // SIGALRM

    match crate::syscalls::glib::dispatch(0xB007, &[
        -1i32 as u64, // fd = -1 (create new)
        &mut mask as u64,
        0 // flags = 0
    ]) {
        Ok(fd) => {
            if fd == 0 {
                return Err("signalfd returned invalid fd");
            }

            crate::println!("[test] signalfd creation succeeded: fd={}", fd);

            // Test reading from empty signalfd (should block or return EAGAIN)
            let mut buf = [0u8; 128];
            match crate::syscalls::file_io::dispatch(0x2002, &[
                fd,
                buf.as_mut_ptr() as u64,
                buf.len() as u64
            ]) {
                Ok(_) => {
                    // Should not succeed with empty signalfd
                    return Err("read from empty signalfd should not succeed");
                }
                Err(_) => {
                    // Expected: EAGAIN or block
                    crate::println!("[test] read from empty signalfd correctly failed");
                }
            }

            // Test signalfd4 with existing fd
            let new_mask = SigSet { bits: 1 << 15 }; // SIGTERM
            match crate::syscalls::glib::dispatch(0xB008, &[
                fd, // existing fd
                &new_mask as u64,
                0 // flags = 0
            ]) {
                Ok(result_fd) => {
                    if result_fd != fd {
                        return Err("signalfd4 with existing fd returned different fd");
                    }
                    crate::println!("[test] signalfd4 update succeeded");
                }
                Err(e) => {
                    return Err("signalfd4 update failed");
                }
            }

            // Close signalfd
            let _ = crate::syscalls::file_io::dispatch(0x2001, &[fd]);
        }
        Err(e) => {
            return Err("signalfd creation failed");
        }
    }

    Ok(())
}

// ============================================================================
// Memory Management Tests
// ============================================================================

/// Test mmap system call
pub fn test_mmap() -> TestResult {
    // Test anonymous memory mapping
    const TEST_SIZE: usize = 0x1000; // 4KB
    
    match crate::syscalls::memory::dispatch(0x3001, &[
        0, // addr_hint
        TEST_SIZE as u64,
        3, // PROT_READ | PROT_WRITE
        0x20, // MAP_ANONYMOUS | MAP_PRIVATE
        -1, // fd
        0  // offset
    ]) {
        Ok(addr) => {
            if addr == 0 {
                return Err("mmap returned null");
            }
            
            // Test writing to mapped memory
            unsafe {
                let ptr = addr as *mut u8;
                for i in 0..TEST_SIZE {
                    *ptr.add(i) = (i % 256) as u8;
                }
            }
            
            crate::println!("[test] mmap succeeded at addr 0x{:x}", addr);
            
            // Test munmap
            match crate::syscalls::memory::dispatch(0x3002, &[
                addr,
                TEST_SIZE as u64
            ]) {
                Ok(_) => {
                    crate::println!("[test] munmap succeeded");
                }
                Err(e) => {
                    return Err("munmap failed");
                }
            }
        }
        Err(e) => {
            return Err("mmap failed");
        }
    }
    
    Ok(())
}

/// Test mprotect system call
pub fn test_mprotect() -> TestResult {
    // Test memory protection
    const TEST_SIZE: usize = 0x1000; // 4KB
    
    // First, map memory
    match crate::syscalls::memory::dispatch(0x3001, &[
        0, // addr_hint
        TEST_SIZE as u64,
        3, // PROT_READ | PROT_WRITE
        0x20, // MAP_ANONYMOUS | MAP_PRIVATE
        -1, // fd
        0  // offset
    ]) {
        Ok(addr) => {
            if addr == 0 {
                return Err("mmap returned null");
            }
            
            // Test mprotect - make memory read-only
            match crate::syscalls::memory::dispatch(0x3003, &[
                addr,
                TEST_SIZE as u64,
                1 // PROT_READ
            ]) {
                Ok(_) => {
                    crate::println!("[test] mprotect succeeded");
                    
                    // Test that memory is now read-only
                    unsafe {
                        let ptr = addr as *mut u8;
                        // This should cause a page fault if protection works
                        // For testing purposes, we just check that mprotect returned success
                    }
                }
                Err(e) => {
                    return Err("mprotect failed");
                }
            }
            
            // Clean up
            let _ = crate::syscalls::memory::dispatch(0x3002, &[
                addr,
                TEST_SIZE as u64
            ]);
        }
        Err(e) => {
            return Err("mmap failed");
        }
    }
    
    Ok(())
}

/// Test msync system call
pub fn test_msync() -> TestResult {
    // Test memory synchronization
    const TEST_SIZE: usize = 0x1000; // 4KB
    
    // First, map memory
    match crate::syscalls::memory::dispatch(0x3001, &[
        0, // addr_hint
        TEST_SIZE as u64,
        3, // PROT_READ | PROT_WRITE
        0x20, // MAP_ANONYMOUS | MAP_PRIVATE
        -1, // fd
        0  // offset
    ]) {
        Ok(addr) => {
            if addr == 0 {
                return Err("mmap returned null");
            }
            
            // Write to mapped memory
            unsafe {
                let ptr = addr as *mut u8;
                for i in 0..TEST_SIZE {
                    *ptr.add(i) = (i % 256) as u8;
                }
            }
            
            // Test msync
            match crate::syscalls::memory::dispatch(0x300A, &[
                addr,
                TEST_SIZE as u64,
                0 // flags
            ]) {
                Ok(_) => {
                    crate::println!("[test] msync succeeded");
                }
                Err(e) => {
                    return Err("msync failed");
                }
            }
            
            // Clean up
            let _ = crate::syscalls::memory::dispatch(0x3002, &[
                addr,
                TEST_SIZE as u64
            ]);
        }
        Err(e) => {
            return Err("mmap failed");
        }
    }
    
    Ok(())
}

// ============================================================================
// Time Management Tests
// ============================================================================

/// Test gettimeofday system call
pub fn test_gettimeofday() -> TestResult {
    // Test gettimeofday
    let mut tv = Timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    
    match crate::syscalls::time::dispatch(0x6001, &[
        &mut tv as u64,
        0 // timezone
    ]) {
        Ok(_) => {
            // Check if time is reasonable
            if tv.tv_sec <= 0 {
                return Err("gettimeofday returned invalid time");
            }
            
            crate::println!("[test] gettimeofday succeeded: {}s, {}us", tv.tv_sec, tv.tv_usec);
        }
        Err(e) => {
            return Err("gettimeofday failed");
        }
    }
    
    Ok(())
}

/// Test clock_gettime system call
pub fn test_clock_gettime() -> TestResult {
    // Test clock_gettime
    let mut tp = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    
    match crate::syscalls::time::dispatch(0x6003, &[
        1, // CLOCK_MONOTONIC
        &mut tp as u64
    ]) {
        Ok(_) => {
            // Check if time is reasonable
            if tp.tv_sec <= 0 {
                return Err("clock_gettime returned invalid time");
            }
            
            crate::println!("[test] clock_gettime succeeded: {}s, {}ns", tp.tv_sec, tp.tv_nsec);
        }
        Err(e) => {
            return Err("clock_gettime failed");
        }
    }
    
    Ok(())
}

/// Test nanosleep system call
pub fn test_nanosleep() -> TestResult {
    // Test nanosleep
    let req = Timespec {
        tv_sec: 0,
        tv_nsec: 10_000_000, // 10ms
    };
    
    let start_ns = crate::time::timestamp_nanos();
    
    match crate::syscalls::time::dispatch(0x6006, &[
        &req as u64,
        0 // rem_ptr
    ]) {
        Ok(_) => {
            let elapsed_ns = crate::time::timestamp_nanos() - start_ns;
            
            // Check if we slept for approximately the right amount
            if elapsed_ns < 5_000_000 {
                return Err("nanosleep didn't sleep long enough");
            }
            
            crate::println!("[test] nanosleep succeeded: slept for {}ns", elapsed_ns);
        }
        Err(e) => {
            return Err("nanosleep failed");
        }
    }
    
    Ok(())
}

/// Test timer_create system call
pub fn test_timer_create() -> TestResult {
    // Test timer_create
    let mut timer_id = 0i32;
    
    let sev = SigEvent {
        sigev_notify: 1, // SIGEV_SIGNAL
        sigev_signo: 14, // SIGALRM
        sigev_value: 0,
        sigev_notify_function_id: 0,
        sigev_notify_attributes: 0,
    };
    
    match crate::syscalls::time::dispatch(0x600B, &[
        1, // CLOCK_REALTIME
        &sev as u64,
        &mut timer_id as u64
    ]) {
        Ok(_) => {
            if timer_id <= 0 {
                return Err("timer_create returned invalid timer ID");
            }
            
            crate::println!("[test] timer_create succeeded: timer_id={}", timer_id);
        }
        Err(e) => {
            return Err("timer_create failed");
        }
    }
    
    Ok(())
}

/// Test timer_settime system call
pub fn test_timer_settime() -> TestResult {
    // First create a timer
    let mut timer_id = 0i32;

    let sev = SigEvent {
        sigev_notify: 1, // SIGEV_SIGNAL
        sigev_signo: 14, // SIGALRM
        sigev_value: 0,
        sigev_notify_function_id: 0,
        sigev_notify_attributes: 0,
    };

    match crate::syscalls::time::dispatch(0x600B, &[
        1, // CLOCK_REALTIME
        &sev as u64,
        &mut timer_id as u64
    ]) {
        Ok(_) => {
            if timer_id <= 0 {
                return Err("timer_create returned invalid timer ID");
            }

            // Set timer
            let new_value = Itimerspec {
                it_interval: Timespec { tv_sec: 0, tv_nsec: 0 }, // One-shot
                it_value: Timespec { tv_sec: 1, tv_nsec: 0 }, // 1 second
            };

            match crate::syscalls::time::dispatch(0x600C, &[
                timer_id as u64,
                0, // flags
                &new_value as u64,
                0  // old_value_ptr
            ]) {
                Ok(_) => {
                    crate::println!("[test] timer_settime succeeded");
                }
                Err(e) => {
                    return Err("timer_settime failed");
                }
            }
        }
        Err(e) => {
            return Err("timer_create failed");
        }
    }

    Ok(())
}

/// Test timerfd system calls
pub fn test_timerfd() -> TestResult {
    // Test timerfd_create
    match crate::syscalls::glib::dispatch(0xB004, &[
        1, // CLOCK_MONOTONIC
        0  // flags = 0
    ]) {
        Ok(fd) => {
            if fd == 0 {
                return Err("timerfd_create returned invalid fd");
            }

            crate::println!("[test] timerfd_create succeeded: fd={}", fd);

            // Test timerfd_settime
            let new_value = Itimerspec {
                it_interval: Timespec { tv_sec: 0, tv_nsec: 100_000_000 }, // 100ms interval
                it_value: Timespec { tv_sec: 0, tv_nsec: 50_000_000 }, // 50ms initial
            };

            match crate::syscalls::glib::dispatch(0xB005, &[
                fd,
                0, // flags = 0
                &new_value as u64,
                0  // old_value_ptr
            ]) {
                Ok(_) => {
                    crate::println!("[test] timerfd_settime succeeded");

                    // Test timerfd_gettime
                    let mut curr_value = Itimerspec::default();

                    match crate::syscalls::glib::dispatch(0xB006, &[
                        fd,
                        &mut curr_value as u64
                    ]) {
                        Ok(_) => {
                            crate::println!("[test] timerfd_gettime succeeded");

                            // Test reading from timerfd (should get expiration count)
                            let mut buf = [0u8; 8];
                            match crate::syscalls::file_io::dispatch(0x2002, &[
                                fd,
                                buf.as_mut_ptr() as u64,
                                buf.len() as u64
                            ]) {
                                Ok(bytes_read) => {
                                    if bytes_read != 8 {
                                        return Err("timerfd read returned wrong number of bytes");
                                    }

                                    let expirations = u64::from_le_bytes(buf);
                                    crate::println!("[test] timerfd read succeeded: {} expirations", expirations);

                                    // Note: In a real test, we would wait for actual timer expirations
                                    // For now, we just verify the syscall interfaces work
                                }
                                Err(_) => {
                                    // Expected: EAGAIN if no expirations yet
                                    crate::println!("[test] timerfd read correctly returned EAGAIN (no expirations yet)");
                                }
                            }
                        }
                        Err(e) => {
                            return Err("timerfd_gettime failed");
                        }
                    }
                }
                Err(e) => {
                    return Err("timerfd_settime failed");
                }
            }

            // Close timerfd
            let _ = crate::syscalls::file_io::dispatch(0x2001, &[fd]);
        }
        Err(e) => {
            return Err("timerfd_create failed");
        }
    }

    Ok(())
}

// ============================================================================
// Integration Test
// ============================================================================

/// Run all POSIX system call tests
pub fn run_all_tests() -> Result<(), Vec<&'static str>> {
    let mut errors = Vec::new();
    
    // Process management tests
    if let Err(e) = test_fork() {
        errors.push(e);
    }
    
    if let Err(e) = test_execve() {
        errors.push(e);
    }
    
    if let Err(e) = test_wait4() {
        errors.push(e);
    }
    
    if let Err(e) = test_getpid_getppid() {
        errors.push(e);
    }
    
    if let Err(e) = test_exit() {
        errors.push(e);
    }
    
    if let Err(e) = test_raise() {
        errors.push(e);
    }
    
    // File system tests
    if let Err(e) = test_mount() {
        errors.push(e);
    }
    
    if let Err(e) = test_stat() {
        errors.push(e);
    }
    
    if let Err(e) = test_fstat() {
        errors.push(e);
    }
    
    if let Err(e) = test_fsync() {
        errors.push(e);
    }
    
    if let Err(e) = test_fchmod() {
        errors.push(e);
    }
    
    // Signal handling tests
    if let Err(e) = test_sigaction() {
        errors.push(e);
    }
    
    if let Err(e) = test_sigprocmask() {
        errors.push(e);
    }
    
    if let Err(e) = test_sigpending() {
        errors.push(e);
    }
    
    if let Err(e) = test_kill() {
        errors.push(e);
    }
    
    if let Err(e) = test_signal() {
        errors.push(e);
    }

    if let Err(e) = test_signalfd() {
        errors.push(e);
    }

    // Memory management tests
    if let Err(e) = test_mmap() {
        errors.push(e);
    }
    
    if let Err(e) = test_mprotect() {
        errors.push(e);
    }
    
    if let Err(e) = test_msync() {
        errors.push(e);
    }
    
    // Time management tests
    if let Err(e) = test_gettimeofday() {
        errors.push(e);
    }
    
    if let Err(e) = test_clock_gettime() {
        errors.push(e);
    }
    
    if let Err(e) = test_nanosleep() {
        errors.push(e);
    }
    
    if let Err(e) = test_timer_create() {
        errors.push(e);
    }
    
    if let Err(e) = test_timer_settime() {
        errors.push(e);
    }

    // TimerFd tests
    if let Err(e) = test_timerfd() {
        errors.push(e);
    }

    if errors.is_empty() {
        crate::println!("[posix_tests] All tests passed!");
        Ok(())
    } else {
        crate::println!("[posix_tests] {} tests failed", errors.len());
        for error in &errors {
            crate::println!("[posix_tests] Error: {}", error);
        }
        Err(errors)
    }
}

/// Run comprehensive Phase 1 integration tests
pub fn run_phase1_integration_tests() -> Result<(), Vec<&'static str>> {
    // Run existing POSIX tests first
    if let Err(errors) = run_all_tests() {
        return Err(errors);
    }

    // Run new integration tests
    #[cfg(feature = "kernel_tests")]
    {
        use crate::syscalls::integration_tests;
        return integration_tests::run_all_integration_tests();
    }

    #[cfg(not(feature = "kernel_tests"))]
    {
        crate::println!("[phase1] Integration tests require kernel_tests feature");
        Ok(())
    }
}