//! C-SKY system call interface
//!
//! This module provides the system call interface for C-SKY processors.

/// System call numbers (Linux ABI compatible)
pub mod syscall_numbers {
    // File operations
    pub const SYS_OPEN: u32 = 2;
    pub const SYS_CLOSE: u32 = 3;
    pub const SYS_READ: u32 = 63;
    pub const SYS_WRITE: u32 = 64;

    // Process operations
    pub const SYS_EXECVE: u32 = 221;
    pub const SYS_EXIT: u32 = 93;
    pub const SYS_GETPID: u32 = 172;
}

/// System call entry point
#[naked]
unsafe extern "C" fn syscall_entry() {
    unsafe {
        core::arch::asm!(
            // Save registers
            "subi.l sp, 256",
            "stm.a r4-r15, (sp)",
            // Get syscall number from a7
            "mov r4, a7",
            // Call Rust handler
            "bl {syscall_handler}",
            // Restore registers
            "ldm.a r4-r15, (sp)",
            "addi.l sp, 256",
            // Return to userspace
            "rte",
            syscall_handler = sym handle_syscall,
            options(noreturn)
        );
    }
}

/// System call handler
#[no_mangle]
unsafe extern "C" fn handle_syscall() -> usize {
    // TODO: Get syscall number from a7 register
    let syscall_number = 0u32;

    match syscall_number {
        syscall_numbers::SYS_OPEN => handle_open(),
        syscall_numbers::SYS_CLOSE => handle_close(),
        syscall_numbers::SYS_READ => handle_read(),
        syscall_numbers::SYS_WRITE => handle_write(),
        syscall_numbers::SYS_EXIT => handle_exit(),
        syscall_numbers::SYS_GETPID => handle_getpid(),
        _ => {
            crate::println!("Unknown syscall: {}", syscall_number);
            usize::MAX // Error
        }
    }
}

// System call implementations
unsafe fn handle_open() -> usize { 0 }
unsafe fn handle_close() -> usize { 0 }
unsafe fn handle_read() -> usize { 0 }
unsafe fn handle_write() -> usize { 0 }
unsafe fn handle_exit() -> usize { 0 }
unsafe fn handle_getpid() -> usize { 1 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_numbers() {
        assert_eq!(syscall_numbers::SYS_OPEN, 2);
    }
}
