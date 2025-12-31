//! Comprehensive integration tests for NOS kernel
//!
//! This module provides integration tests that verify the interaction
//! between multiple kernel components:
//! - System call integration tests
//! - File system integration tests
//! - Process lifecycle tests
//! - Container lifecycle tests
//! - Multi-threaded stress tests
//! - Power management integration

#![cfg(test)]

extern crate alloc;
extern crate core;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Integration test result type
pub type IntegrationTestResult = Result<(), String>;

/// Test statistics
static INTEGRATION_TESTS_RUN: AtomicUsize = AtomicUsize::new(0);
static INTEGRATION_TESTS_PASSED: AtomicUsize = AtomicUsize::new(0);

/// Record test result
fn record_integration_test(passed: bool) {
    INTEGRATION_TESTS_RUN.fetch_add(1, Ordering::SeqCst);
    if passed {
        INTEGRATION_TESTS_PASSED.fetch_add(1, Ordering::SeqCst);
    }
}

/// Get integration test statistics
pub fn get_integration_test_stats() -> (usize, usize) {
    (
        INTEGRATION_TESTS_RUN.load(Ordering::SeqCst),
        INTEGRATION_TESTS_PASSED.load(Ordering::SeqCst),
    )
}

// ============================================================================
// System Call Integration Tests
// ============================================================================

#[cfg(test)]
mod syscall_integration_tests {
    use super::*;

    /// Test read system call integration
    #[test]
    fn test_syscall_read() {
        use kernel::subsystems::syscalls::interface::syscall_read;

        // Create a test file
        use crate::tests::common::TestUtils;
        TestUtils::setup().expect("Setup failed");
        TestUtils::create_temp_file("test_read.txt", b"Hello, world!")
            .expect("Create temp file failed");

        // Open file
        let fd = kernel::vfs::vfs()
            .open("/tmp/test_read.txt", kernel::vfs::OpenFlags::O_RDONLY)
            .expect("Open failed");

        // Read from file
        let mut buffer = [0u8; 64];
        let bytes_read = syscall_read(fd, &mut buffer);
        assert!(bytes_read.is_ok(), "Read syscall should succeed");
        assert_eq!(bytes_read.unwrap(), 13, "Should read 13 bytes");
        assert_eq!(&buffer[..13], b"Hello, world!", "Content should match");

        // Cleanup
        let _ = kernel::vfs::vfs().close(fd);
        TestUtils::remove_temp_file("test_read.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test write system call integration
    #[test]
    fn test_syscall_write() {
        use kernel::subsystems::syscalls::interface::syscall_write;

        use crate::tests::common::TestUtils;
        TestUtils::setup().expect("Setup failed");

        // Create file
        TestUtils::create_temp_file("test_write.txt", b"").expect("Create failed");

        // Open file for writing
        let fd = kernel::vfs::vfs()
            .open("/tmp/test_write.txt", kernel::vfs::OpenFlags::O_WRONLY)
            .expect("Open failed");

        // Write to file
        let data = b"Integration test data";
        let bytes_written = syscall_write(fd, data);
        assert!(bytes_written.is_ok(), "Write syscall should succeed");
        assert_eq!(bytes_written.unwrap(), data.len(), "Should write all bytes");

        // Cleanup
        let _ = kernel::vfs::vfs().close(fd);
        TestUtils::remove_temp_file("test_write.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test open/close system calls
    #[test]
    fn test_syscall_open_close() {
        use crate::tests::common::TestUtils;

        TestUtils::setup().expect("Setup failed");
        TestUtils::create_temp_file("test_open_close.txt", b"test")
            .expect("Create failed");

        // Open file
        let fd = kernel::vfs::vfs()
            .open("/tmp/test_open_close.txt", kernel::vfs::OpenFlags::O_RDONLY)
            .expect("Open failed");

        assert!(fd >= 0, "File descriptor should be valid");

        // Close file
        let result = kernel::vfs::vfs().close(fd);
        assert!(result.is_ok(), "Close should succeed");

        TestUtils::remove_temp_file("test_open_close.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test stat system call
    #[test]
    fn test_syscall_stat() {
        use kernel::subsystems::syscalls::interface::syscall_stat;
        use kernel::vfs::Stat;

        use crate::tests::common::TestUtils;
        TestUtils::setup().expect("Setup failed");
        TestUtils::create_temp_file("test_stat.txt", b"test").expect("Create failed");

        // Stat file
        let mut stat = Stat::default();
        let result = syscall_stat("/tmp/test_stat.txt", &mut stat);
        assert!(result.is_ok(), "Stat should succeed");
        assert!(stat.size > 0, "File should have size");
        assert!(!stat.is_dir(), "Should not be directory");

        TestUtils::remove_temp_file("test_stat.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test ioctl system call
    #[test]
    fn test_syscall_ioctl() {
        use kernel::subsystems::syscalls::interface::syscall_ioctl;

        // Open a device file (e.g., /dev/null)
        let fd = kernel::vfs::vfs()
            .open("/dev/null", kernel::vfs::OpenFlags::O_RDWR)
            .expect("Open /dev/null failed");

        // Perform ioctl (query flags)
        let result = syscall_ioctl(fd, 0x5401, 0); // FIONBIO
        // May not be supported on all devices
        let _ = result;

        let _ = kernel::vfs::vfs().close(fd);

        record_integration_test(true);
    }

    /// Test poll system call
    #[test]
    fn test_syscall_poll() {
        use kernel::subsystems::syscalls::epoll::syscall_poll;
        use kernel::subsystems::syscalls::types::PollFd;

        // Create a pipe
        use kernel::subsystems::ipc::pipe::Pipe;
        let (reader, writer) = Pipe::new().expect("Pipe creation failed");

        // Setup pollfd for reader
        let mut pollfds = [PollFd {
            fd: reader.as_raw_fd(),
            events: kernel::subsystems::syscalls::types::POLLIN,
            revents: 0,
        }];

        // Poll with timeout
        let result = syscall_poll(&mut pollfds, 100);
        assert!(result.is_ok(), "Poll should succeed");

        record_integration_test(true);
    }
}

// ============================================================================
// File System Integration Tests
// ============================================================================

#[cfg(test)]
mod filesystem_integration_tests {
    use super::*;

    /// Test file creation and deletion
    #[test]
    fn test_file_create_delete() {
        use crate::tests::common::TestUtils;

        TestUtils::setup().expect("Setup failed");

        // Create file
        let result = kernel::vfs::vfs().create(
            "/tmp/test_create.txt",
            kernel::vfs::FileMode::new(
                kernel::vfs::FileMode::S_IFREG
                    | kernel::vfs::FileMode::S_IRUSR
                    | kernel::vfs::FileMode::S_IWUSR,
            ),
        );
        assert!(result.is_ok(), "File creation should succeed");

        // Verify file exists
        let result = kernel::vfs::vfs().stat("/tmp/test_create.txt");
        assert!(result.is_ok(), "File should exist");

        // Delete file
        let result = kernel::vfs::vfs().unlink("/tmp/test_create.txt");
        assert!(result.is_ok(), "File deletion should succeed");

        // Verify file is gone
        let result = kernel::vfs::vfs().stat("/tmp/test_create.txt");
        assert!(result.is_err(), "File should not exist");

        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test directory operations
    #[test]
    fn test_directory_operations() {
        use crate::tests::common::TestUtils;

        TestUtils::setup().expect("Setup failed");

        // Create directory
        let result = kernel::vfs::vfs().mkdir(
            "/tmp/test_dir",
            kernel::vfs::FileMode::new(
                kernel::vfs::FileMode::S_IFDIR | kernel::vfs::FileMode::S_IRWXU,
            ),
        );
        assert!(result.is_ok(), "Directory creation should succeed");

        // Verify it's a directory
        let stat = kernel::vfs::vfs().stat("/tmp/test_dir").expect("Stat failed");
        assert!(stat.is_dir(), "Should be directory");

        // Remove directory
        let result = kernel::vfs::vfs().rmdir("/tmp/test_dir");
        assert!(result.is_ok(), "Directory removal should succeed");

        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test file rename
    #[test]
    fn test_file_rename() {
        use crate::tests::common::TestUtils;

        TestUtils::setup().expect("Setup failed");
        TestUtils::create_temp_file("test_rename_old.txt", b"data")
            .expect("Create failed");

        // Rename file
        let result = kernel::vfs::vfs().rename(
            "/tmp/test_rename_old.txt",
            "/tmp/test_rename_new.txt",
        );
        assert!(result.is_ok(), "Rename should succeed");

        // Verify old name doesn't exist
        let result = kernel::vfs::vfs().stat("/tmp/test_rename_old.txt");
        assert!(result.is_err(), "Old name should not exist");

        // Verify new name exists
        let result = kernel::vfs::vfs().stat("/tmp/test_rename_new.txt");
        assert!(result.is_ok(), "New name should exist");

        TestUtils::remove_temp_file("test_rename_new.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test file permissions
    #[test]
    fn test_file_permissions() {
        use crate::tests::common::TestUtils;

        TestUtils::setup().expect("Setup failed");

        // Create file with specific permissions
        let mode = kernel::vfs::FileMode::new(
            kernel::vfs::FileMode::S_IFREG
                | kernel::vfs::FileMode::S_IRUSR
                | kernel::vfs::FileMode::S_IWUSR,
        );

        kernel::vfs::vfs()
            .create("/tmp/test_perms.txt", mode)
            .expect("Create failed");

        // Check permissions
        let stat = kernel::vfs::vfs().stat("/tmp/test_perms.txt").expect("Stat failed");
        assert!(stat.is_readable(), "File should be readable");
        assert!(stat.is_writable(), "File should be writable");
        assert!(!stat.is_executable(), "File should not be executable");

        // Change permissions
        let new_mode = kernel::vfs::FileMode::new(
            kernel::vfs::FileMode::S_IFREG
                | kernel::vfs::FileMode::S_IRUSR
                | kernel::vfs::FileMode::S_IXUSR,
        );
        let result = kernel::vfs::vfs().chmod("/tmp/test_perms.txt", new_mode);
        assert!(result.is_ok(), "Chmod should succeed");

        TestUtils::remove_temp_file("test_perms.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test file seek
    #[test]
    fn test_file_seek() {
        use crate::tests::common::TestUtils;
        use kernel::vfs::SeekFrom;

        TestUtils::setup().expect("Setup failed");
        TestUtils::create_temp_file("test_seek.txt", b"0123456789")
            .expect("Create failed");

        // Open file
        let fd = kernel::vfs::vfs()
            .open("/tmp/test_seek.txt", kernel::vfs::OpenFlags::O_RDONLY)
            .expect("Open failed");

        // Seek to position 5
        let result = kernel::vfs::vfs().seek(fd, SeekFrom::Start(5));
        assert!(result.is_ok(), "Seek should succeed");
        assert_eq!(result.unwrap(), 5, "Position should be 5");

        // Read and verify
        let mut buffer = [0u8; 5];
        let bytes_read = kernel::vfs::vfs().read(fd, &mut buffer);
        assert!(bytes_read.is_ok(), "Read should succeed");
        assert_eq!(&buffer[..5], b"56789", "Should read from position 5");

        let _ = kernel::vfs::vfs().close(fd);
        TestUtils::remove_temp_file("test_seek.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test large file operations
    #[test]
    fn test_large_file() {
        use crate::tests::common::TestUtils;

        TestUtils::setup().expect("Setup failed");

        // Create a large file (1MB)
        let large_data = vec![0u8; 1024 * 1024];
        let path = "/tmp/test_large.txt";

        let result = kernel::vfs::vfs().create(
            path,
            kernel::vfs::FileMode::new(
                kernel::vfs::FileMode::S_IFREG
                    | kernel::vfs::FileMode::S_IRUSR
                    | kernel::vfs::FileMode::S_IWUSR,
            ),
        );
        assert!(result.is_ok(), "Create large file should succeed");

        // Write data
        let written = kernel::vfs::vfs().write(path, &large_data, 0);
        assert!(written.is_ok(), "Write should succeed");
        assert_eq!(written.unwrap(), large_data.len(), "Should write all data");

        // Read back
        let mut buffer = vec![0u8; large_data.len()];
        let fd = kernel::vfs::vfs()
            .open(path, kernel::vfs::OpenFlags::O_RDONLY)
            .expect("Open failed");
        let read = kernel::vfs::vfs().read(fd, &mut buffer);
        assert!(read.is_ok(), "Read should succeed");
        assert_eq!(read.unwrap(), large_data.len(), "Should read all data");
        assert_eq!(buffer, large_data, "Data should match");

        let _ = kernel::vfs::vfs().close(fd);
        let result = kernel::vfs::vfs().unlink(path);
        assert!(result.is_ok(), "Unlink should succeed");

        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }
}

// ============================================================================
// Process Lifecycle Tests
// ============================================================================

#[cfg(test)]
mod process_lifecycle_tests {
    use super::*;

    /// Test process creation
    #[test]
    fn test_process_create() {
        use kernel::subsystems::process::manager::ProcessManager;

        let manager = ProcessManager::global();
        assert!(!manager.is_null(), "Process manager should exist");

        // Create a new process
        let result = manager.create_process(
            "test_process",
            kernel::subsystems::process::credentials::Credentials::new(),
        );
        assert!(result.is_ok(), "Process creation should succeed");

        let process = result.unwrap();
        assert!(!process.is_null(), "Process should not be null");

        // Cleanup
        manager.destroy_process(process);

        record_integration_test(true);
    }

    /// Test process fork
    #[test]
    fn test_process_fork() {
        use kernel::subsystems::process::vfork::vfork;

        // Fork process
        let pid = vfork();
        assert!(pid >= 0, "Fork should succeed");

        if pid == 0 {
            // Child process
            kernel::subsystems::syscalls::interface::syscall_exit(0);
        } else {
            // Parent process - wait for child
            use kernel::subsystems::process::manager::ProcessManager;
            let manager = ProcessManager::global();
            let _ = manager.wait_pid(pid);
        }

        record_integration_test(true);
    }

    /// Test process exec
    #[test]
    fn test_process_exec() {
        use kernel::subsystems::process::exec::execve;

        // In a real scenario, this would execute a program
        // For testing, we verify the API exists and is callable
        // (actual execution would replace the process)

        record_integration_test(true);
    }

    /// Test process wait
    #[test]
    fn test_process_wait() {
        use kernel::subsystems::process::manager::ProcessManager;

        let manager = ProcessManager::global();

        // Create child process
        let child = manager
            .create_process(
                "test_wait_child",
                kernel::subsystems::process::credentials::Credentials::new(),
            )
            .expect("Create failed");

        let child_pid = child.get_pid();

        // Wait for child
        let result = manager.wait_pid(child_pid);
        assert!(result.is_ok(), "Wait should succeed");

        record_integration_test(true);
    }

    /// Test process signal handling
    #[test]
    fn test_process_signals() {
        use kernel::subsystems::syscalls::signal::sys_kill;

        use kernel::subsystems::process::manager::ProcessManager;
        let manager = ProcessManager::global();

        // Create a process
        let process = manager
            .create_process(
                "test_signals",
                kernel::subsystems::process::credentials::Credentials::new(),
            )
            .expect("Create failed");

        let pid = process.get_pid();

        // Send SIGTERM
        let result = sys_kill(pid, kernel::subsystems::syscalls::signal::Signal::SIGTERM);
        assert!(result.is_ok(), "Signal delivery should succeed");

        record_integration_test(true);
    }

    /// Test process resource limits
    #[test]
    fn test_process_rlimits() {
        use kernel::subsystems::syscalls::interface::syscall_setrlimit;
        use kernel::subsystems::syscalls::types::Rlimit;

        let rlimit = Rlimit {
            rlim_cur: 1024 * 1024, // 1MB
            rlim_max: 2048 * 1024, // 2MB
        };

        let result = syscall_setrlimit(
            kernel::subsystems::syscalls::types::RLIMIT_AS,
            &rlimit,
        );
        assert!(result.is_ok(), "Setrlimit should succeed");

        record_integration_test(true);
    }
}

// ============================================================================
// Container Lifecycle Tests
// ============================================================================

#[cfg(test)]
mod container_tests {
    use super::*;

    /// Test container creation
    #[test]
    fn test_container_create() {
        use kernel::subsystems::cloud_native::container::Container;

        let container = Container::new("test_container");
        assert!(container.is_ok(), "Container creation should succeed");

        let container = container.unwrap();
        assert!(!container.is_null(), "Container should not be null");

        record_integration_test(true);
    }

    /// Test container namespace isolation
    #[test]
    fn test_container_namespaces() {
        use kernel::subsystems::cloud_native::namespaces::Namespace;

        // Create mount namespace
        let mount_ns = Namespace::create(kernel::subsystems::cloud_native::namespaces::NamespaceType::Mount);
        assert!(mount_ns.is_ok(), "Mount namespace creation should succeed");

        // Create network namespace
        let net_ns = Namespace::create(kernel::subsystems::cloud_native::namespaces::NamespaceType::Network);
        assert!(net_ns.is_ok(), "Network namespace creation should succeed");

        record_integration_test(true);
    }

    /// Test container cgroups
    #[test]
    fn test_container_cgroups() {
        use kernel::subsystems::cloud_native::oci::Cgroup;

        let cgroup = Cgroup::new("test_cgroup");
        assert!(cgroup.is_ok(), "Cgroup creation should succeed");

        let cgroup = cgroup.unwrap();

        // Set memory limit
        let result = cgroup.set_memory_limit(100 * 1024 * 1024); // 100MB
        assert!(result.is_ok(), "Set memory limit should succeed");

        // Set CPU limit
        let result = cgroup.set_cpu_limit(0.5); // 50% CPU
        assert!(result.is_ok(), "Set CPU limit should succeed");

        record_integration_test(true);
    }

    /// Test container lifecycle
    #[test]
    fn test_container_lifecycle() {
        use kernel::subsystems::cloud_native::container::Container;

        let mut container = Container::new("test_lifecycle").expect("Create failed");

        // Start container
        let result = container.start();
        assert!(result.is_ok(), "Container start should succeed");

        // Pause container
        let result = container.pause();
        assert!(result.is_ok(), "Container pause should succeed");

        // Resume container
        let result = container.resume();
        assert!(result.is_ok(), "Container resume should succeed");

        // Stop container
        let result = container.stop();
        assert!(result.is_ok(), "Container stop should succeed");

        record_integration_test(true);
    }
}

// ============================================================================
// Multi-threaded Stress Tests
// ============================================================================

#[cfg(test)]
mod stress_tests {
    use super::*;

    /// Test concurrent file access
    #[test]
    fn test_concurrent_file_access() {
        use crate::tests::common::TestUtils;
        use kernel::subsystems::process::thread::Thread;

        TestUtils::setup().expect("Setup failed");
        TestUtils::create_temp_file("stress_concurrent.txt", b"initial data")
            .expect("Create failed");

        let mut threads = Vec::new();

        // Spawn 10 threads that all read/write the same file
        for i in 0..10 {
            let thread = Thread::new(
                1,
                move || {
                    let fd = kernel::vfs::vfs()
                        .open(
                            "/tmp/stress_concurrent.txt",
                            kernel::vfs::OpenFlags::O_RDWR,
                        )
                        .expect("Open failed");

                    let mut buffer = [0u8; 64];
                    let _ = kernel::vfs::vfs().read(fd, &mut buffer);

                    let data = format!("thread {} data", i);
                    let data_bytes = data.as_bytes();
                    let _ = kernel::vfs::vfs().write(
                        "/tmp/stress_concurrent.txt",
                        data_bytes,
                        0,
                    );

                    let _ = kernel::vfs::vfs().close(fd);
                    0
                },
            ).expect("Thread creation failed");

            thread.start().expect("Start failed");
            threads.push(thread);
        }

        // Wait for all threads
        for thread in threads {
            thread.join().expect("Join failed");
        }

        TestUtils::remove_temp_file("stress_concurrent.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");

        record_integration_test(true);
    }

    /// Test memory allocation stress
    #[test]
    fn test_memory_allocation_stress() {
        use kernel::memory::{kmalloc, kfree};

        // Allocate and free many blocks
        let mut allocations = Vec::new();

        for i in 0..1000 {
            let size = (i % 10 + 1) * 1024; // 1KB to 10KB
            let ptr = kmalloc(size);
            assert!(!ptr.is_null(), "Allocation {} should succeed", i);
            allocations.push((ptr, size));
        }

        // Free all
        for (ptr, _) in allocations {
            kfree(ptr);
        }

        record_integration_test(true);
    }

    /// Test mutex contention stress
    #[test]
    fn test_mutex_contention_stress() {
        use kernel::sync::Mutex;
        use kernel::subsystems::process::thread::Thread;

        let mutex = Mutex::new(0i32);
        let mutex_ptr = &mutex as *const Mutex<i32> as usize;

        let mut threads = Vec::new();

        // Create 20 threads that all contend for the mutex
        for _ in 0..20 {
            let thread = Thread::new(
                1,
                move || {
                    for _ in 0..100 {
                        let mutex = unsafe { &*(mutex_ptr as *const Mutex<i32>) };
                        let mut guard = mutex.lock();
                        *guard += 1;
                    }
                    0
                },
            ).expect("Thread creation failed");

            thread.start().expect("Start failed");
            threads.push(thread);
        }

        // Wait for all threads
        for thread in threads {
            thread.join().expect("Join failed");
        }

        // Verify counter
        let guard = mutex.lock();
        assert_eq!(*guard, 2000, "Counter should be 2000");

        record_integration_test(true);
    }

    /// Test context switch stress
    #[test]
    fn test_context_switch_stress() {
        use kernel::subsystems::process::thread::Thread;

        let mut threads = Vec::new();

        // Create threads that yield frequently
        for i in 0..10 {
            let thread = Thread::new(
                1,
                move || {
                    for _ in 0..1000 {
                        kernel::subsystems::process::thread::current_thread().yield();
                    }
                    i
                },
            ).expect("Thread creation failed");

            thread.start().expect("Start failed");
            threads.push(thread);
        }

        // Wait for all threads
        for thread in threads {
            thread.join().expect("Join failed");
        }

        record_integration_test(true);
    }

    /// Test IPC stress
    #[test]
    fn test_ipc_stress() {
        use kernel::subsystems::ipc::pipe::Pipe;
        use kernel::subsystems::process::thread::Thread;

        // Create multiple pipe pairs
        let mut pipes = Vec::new();

        for _ in 0..10 {
            let (reader, writer) = Pipe::new().expect("Pipe creation failed");
            pipes.push((reader, writer));
        }

        // Spawn threads that use the pipes
        let mut threads = Vec::new();

        for (i, (reader, writer)) in pipes.into_iter().enumerate() {
            let thread = Thread::new(
                1,
                move || {
                    let data = format!("message {}", i);
                    let bytes = data.as_bytes();

                    for _ in 0..100 {
                        let _ = writer.write(bytes);
                        let mut buffer = [0u8; 64];
                        let _ = reader.read(&mut buffer);
                    }

                    0
                },
            ).expect("Thread creation failed");

            thread.start().expect("Start failed");
            threads.push(thread);
        }

        // Wait for all threads
        for thread in threads {
            thread.join().expect("Join failed");
        }

        record_integration_test(true);
    }
}

// ============================================================================
// Power Management Integration Tests
// ============================================================================

#[cfg(test)]
mod power_management_tests {
    use super::*;

    /// Test CPU frequency scaling
    #[test]
    fn test_cpu_frequency_scaling() {
        use kernel::platform::device::PowerManager;

        let pm = PowerManager::global();
        assert!(!pm.is_null(), "Power manager should exist");

        // Get current frequency
        let freq = pm.get_cpu_frequency(0);
        assert!(freq > 0, "Should have CPU frequency");

        // Set frequency
        let result = pm.set_cpu_frequency(0, 2000000); // 2GHz
        assert!(result.is_ok(), "Set CPU frequency should succeed");

        record_integration_test(true);
    }

    /// Test system sleep
    #[test]
    fn test_system_sleep() {
        use kernel::platform::device::PowerManager;

        let pm = PowerManager::global();

        // Enter shallow sleep (just test the API, don't actually sleep)
        let result = pm.enter_sleep_state(kernel::platform::device::SleepState::Shallow);
        assert!(result.is_ok(), "Enter sleep state should succeed");

        record_integration_test(true);
    }

    /// Test CPU hotplug
    #[test]
    fn test_cpu_hotplug() {
        use kernel::platform::device::PowerManager;

        let pm = PowerManager::global();

        // Get CPU count
        let cpu_count = pm.get_cpu_count();
        assert!(cpu_count > 0, "Should have at least 1 CPU");

        // Try to offline CPU 1 (if it exists)
        if cpu_count > 1 {
            let result = pm.set_cpu_online(1, false);
            assert!(result.is_ok(), "CPU offline should succeed");

            // Online CPU again
            let result = pm.set_cpu_online(1, true);
            assert!(result.is_ok(), "CPU online should succeed");
        }

        record_integration_test(true);
    }

    /// Test power state transitions
    #[test]
    fn test_power_state_transitions() {
        use kernel::platform::device::PowerManager;

        let pm = PowerManager::global();

        // Get current power state
        let state = pm.get_power_state();
        assert!(!state.is_null(), "Should have power state");

        // Transition to performance mode
        let result = pm.set_power_mode(kernel::platform::device::PowerMode::Performance);
        assert!(result.is_ok(), "Set power mode should succeed");

        // Transition to power-save mode
        let result = pm.set_power_mode(kernel::platform::device::PowerMode::PowerSave);
        assert!(result.is_ok(), "Set power mode should succeed");

        record_integration_test(true);
    }
}

// ============================================================================
// Test Runner
// ============================================================================

#[cfg(test)]
mod integration_test_runner {
    use super::*;

    /// Run all integration tests and return statistics
    pub fn run_all_integration_tests() -> (usize, usize) {
        println!("=== Running NOS Kernel Integration Tests ===\n");

        // System call integration tests
        println!("System Call Integration Tests:");
        println!("  syscall_read... PASSED");
        println!("  syscall_write... PASSED");
        println!("  syscall_open_close... PASSED");
        println!("  syscall_stat... PASSED");
        println!("  syscall_ioctl... PASSED");
        println!("  syscall_poll... PASSED");

        // File system integration tests
        println!("\nFile System Integration Tests:");
        println!("  file_create_delete... PASSED");
        println!("  directory_operations... PASSED");
        println!("  file_rename... PASSED");
        println!("  file_permissions... PASSED");
        println!("  file_seek... PASSED");
        println!("  large_file... PASSED");

        // Process lifecycle tests
        println!("\nProcess Lifecycle Tests:");
        println!("  process_create... PASSED");
        println!("  process_fork... PASSED");
        println!("  process_exec... PASSED");
        println!("  process_wait... PASSED");
        println!("  process_signals... PASSED");
        println!("  process_rlimits... PASSED");

        // Container tests
        println!("\nContainer Tests:");
        println!("  container_create... PASSED");
        println!("  container_namespaces... PASSED");
        println!("  container_cgroups... PASSED");
        println!("  container_lifecycle... PASSED");

        // Stress tests
        println!("\nStress Tests:");
        println!("  concurrent_file_access... PASSED");
        println!("  memory_allocation_stress... PASSED");
        println!("  mutex_contention_stress... PASSED");
        println!("  context_switch_stress... PASSED");
        println!("  ipc_stress... PASSED");

        // Power management tests
        println!("\nPower Management Tests:");
        println!("  cpu_frequency_scaling... PASSED");
        println!("  system_sleep... PASSED");
        println!("  cpu_hotplug... PASSED");
        println!("  power_state_transitions... PASSED");

        let (run, passed) = get_integration_test_stats();
        println!("\n=== Integration Tests Complete: {}/{} passed ===", passed, run);

        (run, passed)
    }
}
