//! LoongArch system call interface
//!
//! This module provides the system call interface for LoongArch64,
//! including system call entry point and number mapping.

/// System call numbers for LoongArch
///
/// LoongArch uses the same system call numbers as the Linux ABI
/// for compatibility with existing Linux binaries.
pub mod syscall_numbers {
    // File operations
    pub const SYS_OPEN: u64 = 2;
    pub const SYS_CLOSE: u64 = 3;
    pub const SYS_READ: u64 = 63;
    pub const SYS_WRITE: u64 = 64;
    pub const SYS_STAT: u64 = 4;
    pub const SYS_FSTAT: u64 = 5;
    pub const SYS_LSTAT: u64 = 6;
    pub const SYS_POLL: u64 = 7;
    pub const SYS_LSEEK: u64 = 8;
    pub const SYS_MMAP: u64 = 9;
    pub const SYS_MPROTECT: u64 = 10;
    pub const SYS_MUNMAP: u64 = 11;
    pub const SYS_BRK: u64 = 12;
    pub const SYS_IOCTL: u64 = 15;
    pub const SYS_ACCESS: u64 = 21;

    // Process operations
    pub const SYS_EXECVE: u64 = 221;
    pub const SYS_EXIT: u64 = 93;
    pub const SYS_EXIT_GROUP: u64 = 94;
    pub const SYS_WAIT4: u64 = 61;
    pub const SYS_KILL: u64 = 62;
    pub const SYS_GETPID: u64 = 172;
    pub const SYS_GETPPID: u64 = 173;
    pub const SYS_GETTID: u64 = 174;

    // Thread operations
    pub const SYS_CLONE: u64 = 220;
    pub const SYS_FORK: u64 = 220;
    pub const SYS_VFORK: u64 = 220;

    // Signal operations
    pub const SYS_RT_SIGACTION: u64 = 13;
    pub const SYS_RT_SIGPROCMASK: u64 = 14;
    pub const SYS_RT_SIGRETURN: u64 = 19;

    // Socket operations
    pub const SYS_SOCKET: u64 = 198;
    pub const SYS_BIND: u64 = 199;
    pub const SYS_CONNECT: u64 = 200;
    pub const SYS_LISTEN: u64 = 201;
    pub const SYS_ACCEPT: u64 = 202;
    pub const SYS_SENDTO: u64 = 206;
    pub const SYS_RECVFROM: u64 = 207;

    // Memory operations
    pub const SYS_MSYNC: u64 = 18;
    pub const SYS_MLOCK: u64 = 20;
    pub const SYS_MUNLOCK: u64 = 21;

    // Time operations
    pub const SYS_CLOCK_GETTIME: u64 = 113;
    pub const SYS_GETTIMEOFDAY: u64 = 114;

    // Scheduler operations
    pub const SYS_SCHED_YIELD: u64 = 124;
    pub const SYS_SCHED_GETAFFINITY: u64 = 123;
    pub const SYS_SCHED_SETAFFINITY: u64 = 122;
}

/// System call entry point from userspace
///
/// This function is called when a userspace process makes a system call
/// using the `syscall` instruction.
///
/// # Calling Convention
///
/// LoongArch system calls use the following convention:
/// - a7: System call number
/// - a0-a3: Arguments (up to 6 arguments)
/// - Return value: a0
///
/// # Safety
///
/// This function must only be called from assembly trampoline code.
#[naked]
unsafe extern "C" fn syscall_entry() {
    unsafe {
        core::arch::asm!(
            // Save all registers
            "addi.d $sp, $sp, -256",
            "st.d $ra, $sp, 0",
            "st.d $tp, $sp, 8",
            "st.d $a0, $sp, 16",  // Return value / arg 1
            "st.d $a1, $sp, 24",  // arg 2
            "st.d $a2, $sp, 32",  // arg 3
            "st.d $a3, $sp, 40",  // arg 4
            "st.d $a4, $sp, 48",  // arg 5
            "st.d $a5, $sp, 56",  // arg 6
            "st.d $a6, $sp, 64",
            "st.d $a7, $sp, 72",  // syscall number
            "st.d $t0, $sp, 80",
            "st.d $t1, $sp, 88",
            "st.d $t2, $sp, 96",
            "st.d $t3, $sp, 104",
            "st.d $t4, $sp, 112",
            "st.d $t5, $sp, 120",
            "st.d $t6, $sp, 128",
            "st.d $t7, $sp, 136",
            "st.d $t8, $sp, 144",
            // Call Rust syscall handler
            "bl {syscall_handler}",
            // Restore registers and return
            "ld.d $ra, $sp, 0",
            "ld.d $tp, $sp, 8",
            "ld.d $a0, $sp, 16",  // Return value
            "ld.d $a1, $sp, 24",
            "ld.d $a2, $sp, 32",
            "ld.d $a3, $sp, 40",
            "ld.d $a4, $sp, 48",
            "ld.d $a5, $sp, 56",
            "ld.d $a6, $sp, 64",
            "ld.d $a7, $sp, 72",
            "ld.d $t0, $sp, 80",
            "ld.d $t1, $sp, 88",
            "ld.d $t2, $sp, 96",
            "ld.d $t3, $sp, 104",
            "ld.d $t4, $sp, 112",
            "ld.d $t5, $sp, 120",
            "ld.d $t6, $sp, 128",
            "ld.d $t7, $sp, 136",
            "ld.d $t8, $sp, 144",
            "addi.d $sp, $sp, 256",
            // Return to userspace
            "sysret 0",
            syscall_handler = sym handle_syscall,
            options(noreturn)
        );
    }
}

/// System call handler (Rust implementation)
///
/// # Arguments
///
/// All arguments are passed according to LoongArch ABI:
/// - a7: System call number
/// - a0-a5: System call arguments
///
/// # Returns
///
/// System call return value in a0
#[no_mangle]
unsafe extern "C" fn handle_syscall() -> usize {
    // System call number is in a7
    let syscall_number: u64;

    // Read syscall number from register
    // (In actual implementation, this would be passed differently)
    syscall_number = 0; // Placeholder

    match syscall_number {
        // File operations
        syscall_numbers::SYS_OPEN => handle_open(),
        syscall_numbers::SYS_CLOSE => handle_close(),
        syscall_numbers::SYS_READ => handle_read(),
        syscall_numbers::SYS_WRITE => handle_write(),
        syscall_numbers::SYS_STAT => handle_stat(),

        // Process operations
        syscall_numbers::SYS_EXIT => handle_exit(),
        syscall_numbers::SYS_GETPID => handle_getpid(),

        // Unknown system call
        _ => {
            crate::println!("Unknown syscall: {}", syscall_number);
            usize::MAX // Return error
        }
    }
}

// System call implementations

unsafe fn handle_open() -> usize {
    // TODO: Implement actual open syscall
    0 // Return file descriptor
}

unsafe fn handle_close() -> usize {
    // TODO: Implement actual close syscall
    0 // Success
}

unsafe fn handle_read() -> usize {
    // TODO: Implement actual read syscall
    0 // Bytes read
}

unsafe fn handle_write() -> usize {
    // TODO: Implement actual write syscall
    0 // Bytes written
}

unsafe fn handle_stat() -> usize {
    // TODO: Implement actual stat syscall
    0 // Success
}

unsafe fn handle_exit() -> usize {
    // TODO: Implement actual exit syscall
    0 // Never returns
}

unsafe fn handle_getpid() -> usize {
    // TODO: Return actual PID
    1 // Current process ID
}

/// Get system call name from number
pub fn syscall_name(number: u64) -> &'static str {
    match number {
        syscall_numbers::SYS_OPEN => "open",
        syscall_numbers::SYS_CLOSE => "close",
        syscall_numbers::SYS_READ => "read",
        syscall_numbers::SYS_WRITE => "write",
        syscall_numbers::SYS_EXIT => "exit",
        syscall_numbers::SYS_GETPID => "getpid",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_numbers() {
        assert_eq!(syscall_numbers::SYS_OPEN, 2);
        assert_eq!(syscall_numbers::SYS_CLOSE, 3);
        assert_eq!(syscall_numbers::SYS_READ, 63);
    }

    #[test]
    fn test_syscall_names() {
        assert_eq!(syscall_name(syscall_numbers::SYS_OPEN), "open");
        assert_eq!(syscall_name(syscall_numbers::SYS_EXIT), "exit");
    }
}
