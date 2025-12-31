//! # System Call Integration Tests
//!
//! Comprehensive integration tests for system call dispatch and handling

use crate::prelude::*;
use crate::subsystems::syscalls::*;
use crate::posix::*;

// ============================================================================
// System Call Dispatch Integration Tests
// ============================================================================

#[cfg(test)]
mod syscall_dispatch_tests {
    use super::*;

    /// Test syscall dispatch with zero args
    #[test]
    fn test_syscall_dispatch_zero_args() {
        let syscall_num = SYS_getpid;
        let args = [0u64; 6];

        let result = unsafe { syscall_dispatch(syscall_num, &args) };

        assert!(result.is_ok(), "getpid syscall should succeed");

        let pid = result.unwrap();
        assert!(pid > 0, "PID should be positive");
    }

    /// Test syscall dispatch with one arg
    #[test]
    fn test_syscall_dispatch_one_arg() {
        let syscall_num = SYS_close;
        let args = [3u64, 0, 0, 0, 0, 0]; // Close fd 3

        // May fail if fd 3 not open, but should not crash
        let _result = unsafe { syscall_dispatch(syscall_num, &args) };
    }

    /// Test syscall dispatch with three args
    #[test]
    fn test_syscall_dispatch_three_args() {
        let syscall_num = SYS_openat;
        let path = "/tmp/test.txt\0";
        let args = [
            AT_FDCWD as u64,
            path.as_ptr() as u64,
            (O_CREAT | O_WRONLY) as u64,
            0o644u64,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(syscall_num, &args) };

        assert!(result.is_ok(), "openat syscall should succeed");

        let fd = result.unwrap() as i32;
        assert!(fd >= 0, "fd should be valid");

        // Cleanup
        let _ = unsafe { syscall_dispatch(SYS_close, &[fd as u64, 0, 0, 0, 0, 0]) };
        let _ = unsafe { syscall_dispatch(SYS_unlinkat, &[AT_FDCWD as u64, path.as_ptr() as u64, 0, 0, 0, 0]) };
    }

    /// Test invalid syscall number
    #[test]
    fn test_invalid_syscall() {
        let invalid_syscall = 9999;
        let args = [0u64; 6];

        let result = unsafe { syscall_dispatch(invalid_syscall, &args) };

        assert!(result.is_err(), "Invalid syscall should fail");
    }

    /// Test syscall with null pointer arg
    #[test]
    fn test_syscall_null_pointer() {
        let syscall_num = SYS_write;
        let args = [1u64, 0, 100, 0, 0, 0]; // Write to fd 1, null buffer

        let result = unsafe { syscall_dispatch(syscall_num, &args) };

        // Should fail with EFAULT or similar
        assert!(result.is_err(), "Write with null buffer should fail");
    }
}

// ============================================================================
// Filesystem Syscall Integration Tests
// ============================================================================

#[cfg(test)]
mod fs_syscall_tests {
    use super::*;

    /// Test open-read-write-close sequence
    #[test]
    fn test_full_file_lifecycle() {
        let path = "/tmp/test_lifecycle.txt";
        let data = b"Integration test data\0";

        // Open
        let open_args = [
            AT_FDCWD as u64,
            path.as_ptr() as u64,
            (O_CREAT | O_RDWR) as u64,
            0o644u64,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_openat, &open_args) };
        assert!(result.is_ok(), "Open should succeed");

        let fd = result.unwrap();

        // Write
        let write_args = [fd, data.as_ptr() as u64, data.len() as u64, 0, 0, 0];
        let result = unsafe { syscall_dispatch(SYS_write, &write_args) };
        assert!(result.is_ok(), "Write should succeed");

        // Seek back to start
        let seek_args = [fd, 0, SEEK_SET as u64];
        let result = unsafe { syscall_dispatch(SYS_lseek, &seek_args) };
        assert!(result.is_ok(), "Seek should succeed");

        // Read
        let mut read_buf = [0u8; 64];
        let read_args = [fd, read_buf.as_ptr() as u64, 64, 0, 0, 0];
        let result = unsafe { syscall_dispatch(SYS_read, &read_args) };
        assert!(result.is_ok(), "Read should succeed");

        // Close
        let close_args = [fd, 0, 0, 0, 0, 0];
        let result = unsafe { syscall_dispatch(SYS_close, &close_args) };
        assert!(result.is_ok(), "Close should succeed");

        // Unlink
        let unlink_args = [AT_FDCWD as u64, path.as_ptr() as u64, 0, 0, 0, 0];
        let _ = unsafe { syscall_dispatch(SYS_unlinkat, &unlink_args) };
    }

    /// Test stat syscall
    #[test]
    fn test_stat_syscall() {
        let path = "/tmp\0";

        let mut stat = Stat::default();

        let fstatat_args = [
            AT_FDCWD as u64,
            path.as_ptr() as u64,
            &mut stat as *mut Stat as u64,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_fstatat, &fstatat_args) };

        assert!(result.is_ok(), "fstatat should succeed");
    }

    /// Test mkdir-rmdir sequence
    #[test]
    fn test_mkdir_rmdir_sequence() {
        let path = "/tmp/test_mkdir_rmdir\0";

        // mkdir
        let mkdir_args = [
            AT_FDCWD as u64,
            path.as_ptr() as u64,
            0o755u64,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_mkdirat, &mkdir_args) };
        assert!(result.is_ok(), "mkdir should succeed");

        // rmdir
        let unlink_args = [
            AT_FDCWD as u64,
            path.as_ptr() as u64,
            0,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_unlinkat, &unlink_args) };
        assert!(result.is_ok(), "rmdir should succeed");
    }
}

// ============================================================================
// Process Syscall Integration Tests
// ============================================================================

#[cfg(test)]
mod process_syscall_tests {
    use super::*;

    /// Test fork-exec-wait sequence
    #[test]
    fn test_fork_exec_wait() {
        let fork_args = [SIGCHLD as u64, 0, 0, 0, 0, 0];

        let result = unsafe { syscall_dispatch(SYS_fork, &fork_args) };

        match result {
            Ok(0) => {
                // Child process
                // Try to exec something simple
                let program = "/bin/true\0";
                let args = [
                    program.as_ptr() as u64,
                    0, // null terminator
                    0,
                    0,
                    0,
                    0,
                ];

                let _ = unsafe { syscall_dispatch(SYS_execve, &args) };
                unsafe { syscall_dispatch(SYS_exit, &[0, 0, 0, 0, 0, 0]) };
            }
            Ok(child_pid) => {
                // Parent - wait for child
                let mut status: i32 = 0;
                let wait_args = [
                    child_pid,
                    &mut status as *mut i32 as u64,
                    WNOHANG as u64,
                    0,
                    0,
                    0,
                ];

                // Try non-blocking wait first
                let result = unsafe { syscall_dispatch(SYS_wait4, &wait_args) };

                // Then blocking wait
                let wait_args = [
                    child_pid,
                    &mut status as *mut i32 as u64,
                    0,
                    0,
                    0,
                    0,
                ];

                let result = unsafe { syscall_dispatch(SYS_wait4, &wait_args) };
                assert!(result.is_ok(), "Wait should succeed");
            }
            Err(_) => {
                panic!("Fork should succeed");
            }
        }
    }

    /// Test getpid and getppid
    #[test]
    fn test_process_ids() {
        let result = unsafe { syscall_dispatch(SYS_getpid, &[0, 0, 0, 0, 0, 0]) };
        assert!(result.is_ok(), "getpid should succeed");

        let pid = result.unwrap();
        assert!(pid > 0, "PID should be positive");

        let result = unsafe { syscall_dispatch(SYS_getppid, &[0, 0, 0, 0, 0, 0]) };
        assert!(result.is_ok(), "getppid should succeed");

        let ppid = result.unwrap();
        assert!(ppid > 0, "PPID should be positive");
    }

    /// Test exit syscall
    #[test]
    fn test_exit_syscall() {
        let pid = unsafe { syscall_dispatch(SYS_fork, &[SIGCHLD as u64, 0, 0, 0, 0, 0]) };

        if let Ok(0) = pid {
            // Child - exit with code 42
            unsafe { syscall_dispatch(SYS_exit, &[42, 0, 0, 0, 0, 0]) };
        } else if let Ok(child_pid) = pid {
            // Parent - wait and check exit code
            let mut status: i32 = 0;
            let wait_args = [
                child_pid,
                &mut status as *mut i32 as u64,
                0,
                0,
                0,
                0,
            ];

            let _ = unsafe { syscall_dispatch(SYS_wait4, &wait_args) };

            assert_eq!(WEXITSTATUS(status), 42, "Exit code should be 42");
        }
    }
}

// ============================================================================
// Memory Syscall Integration Tests
// ============================================================================

#[cfg(test)]
mod memory_syscall_tests {
    use super::*;

    /// Test mmap-munmap sequence
    #[test]
    fn test_mmap_munmap() {
        let size = 4096u64;

        let mmap_args = [
            0, // addr (null = let kernel choose)
            size,
            (PROT_READ | PROT_WRITE) as u64,
            (MAP_PRIVATE | MAP_ANONYMOUS) as u64,
            !0u64, // fd (-1)
            0,      // offset
        ];

        let result = unsafe { syscall_dispatch(SYS_mmap, &mmap_args) };

        assert!(result.is_ok(), "mmap should succeed");

        let addr = result.unwrap();

        // Write to mapped memory
        unsafe {
            *(addr as *mut u32) = 0xDEADBEEF;
        }

        // Read back
        let value = unsafe {
            *(addr as *mut u32)
        };

        assert_eq!(value, 0xDEADBEEF, "Should read back written value");

        // munmap
        let munmap_args = [addr, size, 0, 0, 0, 0];
        let result = unsafe { syscall_dispatch(SYS_munmap, &munmap_args) };

        assert!(result.is_ok(), "munmap should succeed");
    }

    /// Test mprotect
    #[test]
    fn test_mprotect() {
        let size = 8192u64;

        let mmap_args = [
            0,
            size,
            PROT_READ as u64,
            (MAP_PRIVATE | MAP_ANONYMOUS) as u64,
            !0u64,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_mmap, &mmap_args) };
        assert!(result.is_ok());

        let addr = result.unwrap();

        // Change to read-write
        let mprotect_args = [
            addr,
            size,
            (PROT_READ | PROT_WRITE) as u64,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_mprotect, &mprotect_args) };

        assert!(result.is_ok(), "mprotect should succeed");

        // Cleanup
        let munmap_args = [addr, size, 0, 0, 0, 0];
        let _ = unsafe { syscall_dispatch(SYS_munmap, &munmap_args) };
    }

    /// Test brk
    #[test]
    fn test_brk() {
        // Get current break
        let result = unsafe { syscall_dispatch(SYS_brk, &[0, 0, 0, 0, 0, 0]) };

        assert!(result.is_ok(), "brk(0) should succeed");

        let original_break = result.unwrap();

        // Increase break
        let new_break = original_break + 4096;
        let result = unsafe { syscall_dispatch(SYS_brk, &[new_break, 0, 0, 0, 0, 0]) };

        // May succeed or fail depending on memory availability
        let _ = result;

        // Restore original break
        let _ = unsafe { syscall_dispatch(SYS_brk, &[original_break, 0, 0, 0, 0, 0]) };
    }
}

// ============================================================================
// Network Syscall Integration Tests
// ============================================================================

#[cfg(test)]
mod network_syscall_tests {
    use super::*;

    /// Test socket creation
    #[test]
    fn test_socket_creation() {
        let socket_args = [
            AF_INET as u64,
            SOCK_STREAM as u64,
            0, // protocol
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_socket, &socket_args) };

        assert!(result.is_ok(), "socket should succeed");

        let fd = result.unwrap() as i32;

        // Close
        let close_args = [fd as u64, 0, 0, 0, 0, 0];
        let result = unsafe { syscall_dispatch(SYS_close, &close_args) };

        assert!(result.is_ok(), "close should succeed");
    }

    /// Test socket options
    #[test]
    fn test_socket_options() {
        // Create socket
        let socket_args = [
            AF_INET as u64,
            SOCK_DGRAM as u64,
            0,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_socket, &socket_args) };
        assert!(result.is_ok());

        let sockfd = result.unwrap();

        // Set socket option
        let optval: i32 = 1;
        let setsockopt_args = [
            sockfd,
            SOL_SOCKET as u64,
            SO_REUSEADDR as u64,
            &optval as *const i32 as u64,
            core::mem::size_of::<i32>() as u64,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_setsockopt, &setsockopt_args) };
        // May succeed or fail

        // Close
        let close_args = [sockfd, 0, 0, 0, 0, 0];
        let _ = unsafe { syscall_dispatch(SYS_close, &close_args) };
    }

    /// Test bind
    #[test]
    fn test_bind() {
        // Create socket
        let socket_args = [
            AF_INET as u64,
            SOCK_STREAM as u64,
            0,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_socket, &socket_args) };
        assert!(result.is_ok());

        let sockfd = result.unwrap();

        // Bind to address
        let addr = SockaddrIn {
            sin_family: AF_INET as u16,
            sin_port: 0, // Any port
            sin_addr: INADDR_ANY,
            ..Default::default()
        };

        let bind_args = [
            sockfd,
            &addr as *const SockaddrIn as u64,
            core::mem::size_of::<SockaddrIn>() as u64,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_bind, &bind_args) };
        // May fail if port in use, but should not crash

        // Close
        let close_args = [sockfd, 0, 0, 0, 0, 0];
        let _ = unsafe { syscall_dispatch(SYS_close, &close_args) };
    }
}

// ============================================================================
// Signal Syscall Integration Tests
// ============================================================================

#[cfg(test)]
mod signal_syscall_tests {
    use super::*;

    /// Test kill syscall
    #[test]
    fn test_kill_self() {
        let pid = unsafe { syscall_dispatch(SYS_getpid, &[0, 0, 0, 0, 0, 0]) };
        assert!(pid.is_ok());

        let pid_val = pid.unwrap();

        // Send SIGUSR1 to self
        let kill_args = [pid_val, SIGUSR1 as u64, 0, 0, 0, 0];

        let result = unsafe { syscall_dispatch(SYS_kill, &kill_args) };

        assert!(result.is_ok(), "kill should succeed");
    }

    /// Test sigaction
    #[test]
    fn test_sigaction_setup() {
        extern "C" fn handler(_signo: i32) {}

        let mut sa = Sigaction {
            sa_handler: Some(handler),
            sa_mask: 0,
            sa_flags: 0,
            sa_restorer: None,
        };

        let sigaction_args = [
            SIGUSR2 as u64,
            &sa as *const Sigaction as u64,
            0, // old_action (null)
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_rt_sigaction, &sigaction_args) };

        assert!(result.is_ok(), "sigaction should succeed");
    }

    /// Test signal mask
    #[test]
    fn test_sigprocmask() {
        let mut old_mask: u64 = 0;
        let mask = 1u64 << (SIGUSR1 as u64);

        let sigprocmask_args = [
            SIG_BLOCK as u64,
            &mask as *const u64 as u64,
            &mut old_mask as *mut u64 as u64,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_rt_sigprocmask, &sigprocmask_args) };

        assert!(result.is_ok(), "sigprocmask should succeed");
    }
}

// ============================================================================
// Time Syscall Integration Tests
// ============================================================================

#[cfg(test)]
mod time_syscall_tests {
    use super::*;

    /// Test gettimeofday
    #[test]
    fn test_gettimeofday() {
        let mut tv = Timeval {
            tv_sec: 0,
            tv_usec: 0,
        };

        let args = [
            &mut tv as *mut Timeval as u64,
            0, // timezone (null)
            0,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_gettimeofday, &args) };

        assert!(result.is_ok(), "gettimeofday should succeed");

        assert!(tv.tv_sec > 0, "Seconds should be positive");
        assert!(tv.tv_usec >= 0 && tv.tv_usec < 1_000_000, "Microseconds should be valid");
    }

    /// Test clock_gettime
    #[test]
    fn test_clock_gettime() {
        let mut ts = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        let args = [
            CLOCK_REALTIME as u64,
            &mut ts as *mut Timespec as u64,
            0,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_clock_gettime, &args) };

        assert!(result.is_ok(), "clock_gettime should succeed");

        assert!(ts.tv_sec > 0, "Seconds should be positive");
        assert!(ts.tv_nsec >= 0 && ts.tv_nsec < 1_000_000_000, "Nanoseconds should be valid");
    }

    /// Test nanosleep
    #[test]
    fn test_nanosleep() {
        let mut req = Timespec {
            tv_sec: 0,
            tv_nsec: 10_000_000, // 10ms
        };

        let mut rem = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        let args = [
            &req as *const Timespec as u64,
            &mut rem as *mut Timespec as u64,
            0,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_nanosleep, &args) };

        assert!(result.is_ok(), "nanosleep should succeed");
    }
}

// ============================================================================
// Thread Syscall Integration Tests
// ============================================================================

#[cfg(test)]
mod thread_syscall_tests {
    use super::*;

    /// Test clone (thread creation)
    #[test]
    fn test_clone_thread() {
        let pid = unsafe { syscall_dispatch(SYS_fork, &[SIGCHLD as u64, 0, 0, 0, 0, 0]) };

        if let Ok(0) = pid {
            // Child - create thread
            let stack = vec![0u8; 65536];
            let stack_ptr = stack.as_ptr() as usize + stack.len();

            let clone_flags = CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND;
            let clone_args = [
                clone_flags,
                stack_ptr as u64,
                0, // parent_tidptr
                0, // child_tidptr
                0, // tls
                0, // ctid
            ];

            let result = unsafe { syscall_dispatch(SYS_clone, &clone_args) };

            match result {
                Ok(0) => {
                    // Thread
                    unsafe { syscall_dispatch(SYS_exit, &[0, 0, 0, 0, 0, 0]) };
                }
                Ok(_) => {
                    // Parent - thread created
                    unsafe { syscall_dispatch(SYS_exit, &[0, 0, 0, 0, 0, 0]) };
                }
                Err(_) => {
                    unsafe { syscall_dispatch(SYS_exit, &[1, 0, 0, 0, 0, 0]) };
                }
            }
        } else if let Ok(child_pid) = pid {
            // Original parent
            let mut status: i32 = 0;
            let wait_args = [
                child_pid,
                &mut status as *mut i32 as u64,
                0,
                0,
                0,
                0,
            ];

            let _ = unsafe { syscall_dispatch(SYS_wait4, &wait_args) };
        }
    }

    /// Test gettid
    #[test]
    fn test_gettid() {
        let result = unsafe { syscall_dispatch(SYS_gettid, &[0, 0, 0, 0, 0, 0]) };

        assert!(result.is_ok(), "gettid should succeed");

        let tid = result.unwrap();
        assert!(tid > 0, "TID should be positive");
    }
}

// ============================================================================
// Info Syscall Integration Tests
// ============================================================================

#[cfg(test)]
mod info_syscall_tests {
    use super::*;

    /// Test sysinfo
    #[test]
    fn test_sysinfo() {
        let mut info = SysInfo::default();

        let args = [
            &mut info as *mut SysInfo as u64,
            0,
            0,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_sysinfo, &args) };

        assert!(result.is_ok(), "sysinfo should succeed");

        assert!(info.uptime > 0, "Uptime should be positive");
        assert!(info.totalram > 0, "Total RAM should be positive");
    }

    /// Test uname
    #[test]
    fn test_uname() {
        let mut uts = UtsName::default();

        let args = [
            &mut uts as *mut UtsName as u64,
            0,
            0,
            0,
            0,
            0,
        ];

        let result = unsafe { syscall_dispatch(SYS_uname, &args) };

        assert!(result.is_ok(), "uname should succeed");

        // Check that sysname is set
        assert!(uts.sysname[0] != 0, "sysname should be set");
    }
}

// ============================================================================
// Placeholder Types and Constants
// ============================================================================

// System call numbers
const SYS_getpid: u64 = 39;
const SYS_getppid: u64 = 110;
const SYS_fork: u64 = 57;
const SYS_exit: u64 = 60;
const SYS_wait4: u64 = 61;
const SYS_execve: u64 = 221;
const SYS_close: u64 = 57;
const SYS_read: u64 = 63;
const SYS_write: u64 = 64;
const SYS_openat: u64 = 257;
const SYS_unlinkat: u64 = 263;
const SYS_mkdirat: u64 = 258;
const SYS_fstatat: u64 = 262;
const SYS_lseek: u64 = 62;
const SYS_mmap: u64 = 222;
const SYS_munmap: u64 = 10;
const SYS_mprotect: u64 = 226;
const SYS_brk: u64 = 214;
const SYS_socket: u64 = 41;
const SYS_bind: u64 = 49;
const SYS_setsockopt: u64 = 54;
const SYS_listen: u64 = 50;
const SYS_accept: u64 = 43;
const SYS_kill: u64 = 129;
const SYS_rt_sigaction: u64 = 13;
const SYS_rt_sigprocmask: u64 = 14;
const SYS_gettimeofday: u64 = 96;
const SYS_clock_gettime: u64 = 228;
const SYS_nanosleep: u64 = 101;
const SYS_clone: u64 = 56;
const SYS_gettid: u64 = 186;
const SYS_sysinfo: u64 = 99;
const SYS_uname: u64 = 63;

// Other constants
const AT_FDCWD: i32 = -100;
const SIGCHLD: i32 = 17;
const SIGUSR1: i32 = 10;
const SIGUSR2: i32 = 12;
const WNOHANG: i32 = 1;
const AF_INET: i32 = 2;
const SOCK_STREAM: i32 = 1;
const SOCK_DGRAM: i32 = 2;
const SOL_SOCKET: i32 = 1;
const SO_REUSEADDR: i32 = 2;
const INADDR_ANY: u32 = 0;
const CLONE_VM: i32 = 0x100;
const CLONE_FS: i32 = 0x200;
const CLONE_FILES: i32 = 0x400;
const CLONE_SIGHAND: i32 = 0x800;
const SIG_BLOCK: i32 = 0;
const CLOCK_REALTIME: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 2;
const MAP_ANONYMOUS: i32 = 32;

// Structs
struct Stat {
    st_dev: u64,
    st_ino: u64,
    st_mode: u32,
    st_nlink: u64,
    st_uid: u32,
    st_gid: u32,
    st_rdev: u64,
    st_size: i64,
    st_blksize: i64,
    st_blocks: i64,
}

impl Default for Stat {
    fn default() -> Self {
        Stat {
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
        }
    }
}

struct Sigaction {
    sa_handler: Option<extern "C" fn(i32)>,
    sa_mask: u64,
    sa_flags: u64,
    sa_restorer: Option<extern "C" fn()>,
}

struct Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

struct SysInfo {
    uptime: i64,
    loads: [u64; 3],
    totalram: u64,
    freeram: u64,
    sharedram: u64,
    bufferram: u64,
    totalswap: u64,
    freeswap: u64,
    procs: u16,
    pad: u16,
    totalhigh: u64,
    freehigh: u64,
    mem_unit: u32,
}

impl Default for SysInfo {
    fn default() -> Self {
        SysInfo {
            uptime: 0,
            loads: [0, 0, 0],
            totalram: 0,
            freeram: 0,
            sharedram: 0,
            bufferram: 0,
            totalswap: 0,
            freeswap: 0,
            procs: 0,
            pad: 0,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 0,
        }
    }
}

struct UtsName {
    sysname: [i8; 65],
    nodename: [i8; 65],
    release: [i8; 65],
    version: [i8; 65],
    machine: [i8; 65],
    domainname: [i8; 65],
}

impl Default for UtsName {
    fn default() -> Self {
        UtsName {
            sysname: [0; 65],
            nodename: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
            domainname: [0; 65],
        }
    }
}

#[repr(C)]
struct SockaddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
}

impl Default for SockaddrIn {
    fn default() -> Self {
        SockaddrIn {
            sin_family: 0,
            sin_port: 0,
            sin_addr: 0,
            sin_zero: [0; 8],
        }
    }
}

// Helper functions
fn WEXITSTATUS(status: i32) -> u8 {
    ((status >> 8) & 0xFF) as u8
}
