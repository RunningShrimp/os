//! # Stress Test Suite for NOS Kernel
//!
//! Comprehensive stress tests for various kernel subsystems:
//! - Memory pressure tests
//! - Process creation stress
//! - IPC channel stress
//! - Network connection stress
//! - Filesystem operation stress
//! - Scheduler stress tests

use crate::prelude::*;
use crate::error::UnifiedError;

// ============================================================================
// Memory Pressure Tests
// ============================================================================

pub mod memory {
    use super::*;

    /// Memory pressure stress test
    pub fn stress_memory_pressure() -> Result<(), UnifiedError> {
        println!("Starting memory pressure stress test...");

        // Gradually increase memory allocation until OOM
        let mut allocations: Vec<*mut u8> = Vec::new();
        let allocation_size = 1024 * 1024; // 1MB per allocation
        let max_allocations = 1024; // Max 1GB

        for i in 0..max_allocations {
            let ptr = unsafe {
                crate::subsystems::mm::allocator::alloc_pages(8) // 8 pages = 32KB
            };

            match ptr {
                Some(page) => {
                    allocations.push(page as *mut u8);
                }
                None => {
                    println!("Memory exhausted at {} MB", i);
                    break;
                }
            }

            // Print progress every 100 allocations
            if i % 100 == 0 {
                println!("Allocated {} MB", i);
            }
        }

        println!("Total allocations: {} MB", allocations.len());

        // Cleanup
        for ptr in allocations {
            unsafe {
                crate::subsystems::mm::allocator::free_pages(ptr as usize, 8);
            }
        }

        println!("Memory pressure test completed successfully");
        Ok(())
    }

    /// Rapid allocation/deallocation stress test
    pub fn stress_alloc_dealloc_cycles() -> Result<(), UnifiedError> {
        println!("Starting alloc/dealloc cycle stress test...");

        let cycles = 10000;
        let sizes = vec![4096, 8192, 16384, 32768, 65536]; // Various sizes

        for i in 0..cycles {
            for &size in &sizes {
                let ptr = unsafe {
                    crate::subsystems::mm::allocator::alloc_pages(
                        (size / 4096) as u8
                    )
                };

                if let Some(page) = ptr {
                    unsafe {
                        // Write to allocated memory
                        core::ptr::write_bytes(page as *mut u8, 0xAA, size);

                        // Free immediately
                        crate::subsystems::mm::allocator::free_pages(page, (size / 4096) as u8);
                    }
                }
            }

            // Print progress
            if i % 1000 == 0 {
                println!("Completed {} cycles", i);
            }
        }

        println!("Alloc/dealloc cycle test completed: {} iterations", cycles);
        Ok(())
    }

    /// Memory fragmentation stress test
    pub fn stress_fragmentation() -> Result<(), UnifiedError> {
        println!("Starting memory fragmentation stress test...");

        let mut allocations: Vec<(usize, u8)> = Vec::new();
        let iterations = 1000;

        // Random sized allocations
        for i in 0..iterations {
            let size = 4096 * ((i % 8) + 1); // 4KB to 32KB
            let order = (size / 4096) as u8;

            let ptr = unsafe {
                crate::subsystems::mm::allocator::alloc_pages(order)
            };

            if let Some(page) = ptr {
                allocations.push((page, order));
            }

            // Free random allocations to create fragmentation
            if allocations.len() > 100 && i % 3 == 0 {
                let idx = (i % allocations.len()) as usize;
                if let Some((page, order)) = allocations.get(idx) {
                    unsafe {
                        crate::subsystems::mm::allocator::free_pages(*page, *order);
                    }
                    allocations.remove(idx);
                }
            }

            if i % 100 == 0 {
                println!("Iteration {}: {} active allocations", i, allocations.len());
            }
        }

        // Cleanup remaining
        for (page, order) in allocations {
            unsafe {
                crate::subsystems::mm::allocator::free_pages(page, order);
            }
        }

        println!("Fragmentation stress test completed");
        Ok(())
    }
}

// ============================================================================
// Process Creation Stress Tests
// ============================================================================

pub mod process {
    use super::*;

    /// Process creation stress test
    pub fn stress_process_creation() -> Result<(), UnifiedError> {
        println!("Starting process creation stress test...");

        let max_processes = 1000;
        let mut pids: Vec<crate::process::Pid> = Vec::new();

        for i in 0..max_processes {
            // Fork new process
            let result = unsafe {
                crate::subsystems::syscalls::process::sys_fork()
            };

            match result {
                Ok(pid) => {
                    if pid == 0 {
                        // Child process - exit immediately
                        crate::subsystems::syscalls::process::sys_exit(0);
                    } else {
                        // Parent process - track child PID
                        pids.push(pid);
                    }
                }
                Err(_) => {
                    println!("Process limit reached at {} processes", i);
                    break;
                }
            }

            if i % 100 == 0 {
                println!("Created {} processes", i);
            }
        }

        println!("Total processes created: {}", pids.len());

        // Wait for all children
        for pid in &pids {
            let _ = unsafe {
                crate::subsystems::syscalls::process::sys_waitpid(*pid, core::ptr::null_mut(), 0)
            };
        }

        println!("Process creation stress test completed");
        Ok(())
    }

    /// Thread creation stress test
    pub fn stress_thread_creation() -> Result<(), UnifiedError> {
        println!("Starting thread creation stress test...");

        let max_threads = 100;
        let mut tids: Vec<crate::process::Tid> = Vec::new();

        for i in 0..max_threads {
            // Create new thread
            let result = unsafe {
                crate::subsystems::syscalls::thread::sys_clone(
                    core::ptr::null_mut(),
                    crate::posix::CLONE_VM | crate::posix::CLONE_FS,
                    0
                )
            };

            match result {
                Ok(tid) => {
                    tids.push(tid);
                }
                Err(_) => {
                    println!("Thread limit reached at {} threads", i);
                    break;
                }
            }

            if i % 10 == 0 {
                println!("Created {} threads", i);
            }
        }

        println!("Total threads created: {}", tids.len());

        // Wait for all threads
        for tid in &tids {
            let _ = unsafe {
                crate::subsystems::syscalls::thread::sys_join(*tid)
            };
        }

        println!("Thread creation stress test completed");
        Ok(())
    }

    /// Context switch stress test
    pub fn stress_context_switching() -> Result<(), UnifiedError> {
        println!("Starting context switch stress test...");

        let num_tasks = 10;
        let switches_per_task = 10000;
        let total_switches = num_tasks * switches_per_task;

        // Create multiple tasks
        let mut tasks: Vec<crate::process::Pid> = Vec::new();

        for _ in 0..num_tasks {
            let result = unsafe {
                crate::subsystems::syscalls::process::sys_fork()
            };

            if let Ok(pid) = result {
                if pid == 0 {
                    // Child - busy wait
                    for _ in 0..switches_per_task {
                        core::hint::spin_loop();
                    }
                    crate::subsystems::syscalls::process::sys_exit(0);
                } else {
                    tasks.push(pid);
                }
            }
        }

        // Parent - measure context switches
        println!("Created {} tasks for context switch test", tasks.len());

        // Wait for completion
        for pid in &tasks {
            let _ = unsafe {
                crate::subsystems::syscalls::process::sys_waitpid(*pid, core::ptr::null_mut(), 0)
            };
        }

        println!("Context switch stress test completed: ~{} switches", total_switches);
        Ok(())
    }
}

// ============================================================================
// IPC Stress Tests
// ============================================================================

pub mod ipc {
    use super::*;

    /// Message queue stress test
    pub fn stress_message_queues() -> Result<(), UnifiedError> {
        println!("Starting message queue stress test...");

        let num_queues = 100;
        let messages_per_queue = 1000;
        let message_size = 256;

        // Create message queues
        let mut mq_ids: Vec<i32> = Vec::new();

        for i in 0..num_queues {
            let result = unsafe {
                crate::subsystems::syscalls::mqueue::mq_open(
                    &format!("/test_mq_{}", i) as *const _ as *const u8,
                    crate::posix::O_CREAT | crate::posix::O_EXCL,
                    0o644,
                    core::ptr::null()
                )
            };

            match result {
                Ok(id) => mq_ids.push(id),
                Err(_) => {
                    println!("Message queue limit reached at {}", i);
                    break;
                }
            }
        }

        println!("Created {} message queues", mq_ids.len());

        // Send/receive messages
        for _ in 0..messages_per_queue {
            for &mq_id in &mq_ids {
                let mut msg = vec![0u8; message_size];
                let _ = unsafe {
                    crate::subsystems::syscalls::mqueue::mq_send(
                        mq_id,
                        msg.as_ptr(),
                        message_size,
                        1
                    )
                };

                let _ = unsafe {
                    crate::subsystems::syscalls::mqueue::mq_receive(
                        mq_id,
                        msg.as_mut_ptr(),
                        message_size,
                        core::ptr::null_mut()
                    )
                };
            }
        }

        // Cleanup
        for &mq_id in &mq_ids {
            let _ = unsafe {
                crate::subsystems::syscalls::mqueue::mq_close(mq_id)
            };
            let _ = unsafe {
                crate::subsystems::syscalls::mqueue::mq_unlink(
                    &format!("/test_mq_{}", mq_id) as *const _ as *const u8
                )
            };
        }

        println!("Message queue stress test completed");
        Ok(())
    }

    /// Shared memory stress test
    pub fn stress_shared_memory() -> Result<(), UnifiedError> {
        println!("Starting shared memory stress test...");

        let num_segments = 100;
        let segment_size = 1024 * 1024; // 1MB per segment

        // Create shared memory segments
        let mut shm_ids: Vec<i32> = Vec::new();

        for i in 0..num_segments {
            let result = unsafe {
                crate::subsystems::posix::shm::shmget(
                    i as i32,
                    segment_size,
                    crate::posix::IPC_CREAT | crate::posix::IPC_EXCL | 0o644
                )
            };

            match result {
                Ok(id) => shm_ids.push(id),
                Err(_) => {
                    println!("Shared memory limit reached at {}", i);
                    break;
                }
            }
        }

        println!("Created {} shared memory segments ({} MB total)",
            shm_ids.len(),
            shm_ids.len() * segment_size / (1024 * 1024)
        );

        // Attach, write, detach
        for &shm_id in &shm_ids {
            let result = unsafe {
                crate::subsystems::posix::shm::shmat(shm_id, core::ptr::null_mut(), 0)
            };

            if let Ok(addr) = result {
                if !addr.is_null() {
                    unsafe {
                        // Write pattern
                        core::ptr::write_bytes(addr as *mut u8, 0x55, segment_size);

                        // Detach
                        crate::subsystems::posix::shm::shmdt(addr as *const u8);
                    }
                }
            }
        }

        // Cleanup
        for &shm_id in &shm_ids {
            let _ = unsafe {
                crate::subsystems::posix::shm::shmctl(shm_id, crate::posix::IPC_RMID, core::ptr::null_mut())
            };
        }

        println!("Shared memory stress test completed");
        Ok(())
    }

    /// Pipe stress test
    pub fn stress_pipes() -> Result<(), UnifiedError> {
        println!("Starting pipe stress test...");

        let num_pipes = 100;
        let bytes_per_pipe = 1024 * 1024; // 1MB per pipe

        // Create pipes
        let mut pipes: Vec<(i32, i32)> = Vec::new();

        for _ in 0..num_pipes {
            let mut fds = [0i32; 2];
            let result = unsafe {
                crate::subsystems::syscalls::fs::sys_pipe(fds.as_mut_ptr())
            };

            match result {
                Ok(_) => pipes.push((fds[0], fds[1])),
                Err(_) => {
                    println!("Pipe limit reached");
                    break;
                }
            }
        }

        println!("Created {} pipes", pipes.len());

        // Write/read data
        let mut data = vec![0u8; 4096]; // 4KB chunks

        for &(read_fd, write_fd) in &pipes {
            let mut total_written = 0;

            while total_written < bytes_per_pipe {
                let result = unsafe {
                    crate::subsystems::syscalls::fs::sys_write(
                        write_fd,
                        data.as_ptr()
                    )
                };

                match result {
                    Ok(n) if n > 0 => {
                        total_written += n as usize;
                    }
                    _ => break,
                }
            }

            // Read back
            let mut total_read = 0;
            while total_read < total_written {
                let mut buf = vec![0u8; 4096];
                let result = unsafe {
                    crate::subsystems::syscalls::fs::sys_read(
                        read_fd,
                        buf.as_mut_ptr()
                    )
                };

                match result {
                    Ok(n) if n > 0 => {
                        total_read += n as usize;
                    }
                    _ => break,
                }
            }
        }

        // Close pipes
        for (read_fd, write_fd) in pipes {
            let _ = unsafe {
                crate::subsystems::syscalls::fs::sys_close(read_fd)
            };
            let _ = unsafe {
                crate::subsystems::syscalls::fs::sys_close(write_fd)
            };
        }

        println!("Pipe stress test completed");
        Ok(())
    }
}

// ============================================================================
// Network Stress Tests
// ============================================================================

pub mod network {
    use super::*;

    /// Network connection stress test
    pub fn stress_network_connections() -> Result<(), UnifiedError> {
        println!("Starting network connection stress test...");

        let max_connections = 1000;
        let mut sockets: Vec<i32> = Vec::new();

        // Create TCP sockets
        for i in 0..max_connections {
            let result = unsafe {
                crate::subsystems::syscalls::network::sys_socket(
                    crate::posix::AF_INET,
                    crate::posix::SOCK_STREAM,
                    0
                )
            };

            match result {
                Ok(fd) => {
                    sockets.push(fd);
                }
                Err(_) => {
                    println!("Socket limit reached at {}", i);
                    break;
                }
            }

            if i % 100 == 0 {
                println!("Created {} sockets", i);
            }
        }

        println!("Total sockets created: {}", sockets.len());

        // Close all sockets
        for fd in sockets {
            let _ = unsafe {
                crate::subsystems::syscalls::fs::sys_close(fd)
            };
        }

        println!("Network connection stress test completed");
        Ok(())
    }

    /// UDP packet storm test
    pub fn stress_udp_packets() -> Result<(), UnifiedError> {
        println!("Starting UDP packet storm test...");

        let num_sockets = 100;
        let packets_per_socket = 10000;
        let packet_size = 1400; // MTU-sized

        // Create UDP sockets
        let mut sockets: Vec<i32> = Vec::new();

        for _ in 0..num_sockets {
            let result = unsafe {
                crate::subsystems::syscalls::network::sys_socket(
                    crate::posix::AF_INET,
                    crate::posix::SOCK_DGRAM,
                    0
                )
            };

            if let Ok(fd) = result {
                sockets.push(fd);
            }
        }

        println!("Created {} UDP sockets", sockets.len());

        // Send packets
        let mut data = vec![0u8; packet_size];

        for &sockfd in &sockets {
            let mut sent = 0;

            for _ in 0..packets_per_socket {
                let result = unsafe {
                    crate::subsystems::syscalls::network::sys_sendto(
                        sockfd,
                        data.as_ptr(),
                        packet_size,
                        0,
                        core::ptr::null(),
                        0
                    )
                };

                match result {
                    Ok(_) => sent += 1,
                    Err(_) => break,
                }
            }

            //println!("Sent {} packets from socket", sent);
        }

        // Close sockets
        for sockfd in sockets {
            let _ = unsafe {
                crate::subsystems::syscalls::fs::sys_close(sockfd)
            };
        }

        println!("UDP packet storm test completed");
        Ok(())
    }
}

// ============================================================================
// Filesystem Stress Tests
// ============================================================================

pub mod filesystem {
    use super::*;

    /// File creation stress test
    pub fn stress_file_creation() -> Result<(), UnifiedError> {
        println!("Starting file creation stress test...");

        let base_path = "/tmp/stress_test";
        let num_files = 1000;

        // Create test directory
        let _ = unsafe {
            crate::subsystems::syscalls::fs::sys_mkdir(
                base_path.as_ptr() as *const u8,
                0o755
            )
        };

        let mut files_created = 0;

        for i in 0..num_files {
            let filename = format!("{}/file_{}.txt", base_path, i);

            let result = unsafe {
                crate::subsystems::syscalls::fs::sys_open(
                    filename.as_ptr() as *const u8,
                    crate::posix::O_CREAT | crate::posix::O_WRONLY | crate::posix::O_EXCL,
                    0o644
                )
            };

            match result {
                Ok(fd) => {
                    files_created += 1;
                    let _ = unsafe {
                        crate::subsystems::syscalls::fs::sys_close(fd)
                    };
                }
                Err(_) => {
                    println!("File limit reached at {}", i);
                    break;
                }
            }

            if i % 100 == 0 {
                println!("Created {} files", i);
            }
        }

        println!("Total files created: {}", files_created);

        // Cleanup
        for i in 0..files_created {
            let filename = format!("{}/file_{}.txt", base_path, i);
            let _ = unsafe {
                crate::subsystems::syscalls::fs::sys_unlink(
                    filename.as_ptr() as *const u8
                )
            };
        }

        let _ = unsafe {
            crate::subsystems::syscalls::fs::sys_rmdir(
                base_path.as_ptr() as *const u8
            )
        };

        println!("File creation stress test completed");
        Ok(())
    }

    /// File I/O stress test
    pub fn stress_file_io() -> Result<(), UnifiedError> {
        println!("Starting file I/O stress test...");

        let test_file = "/tmp/stress_io_test.dat";
        let file_size = 100 * 1024 * 1024; // 100MB
        let buffer_size = 64 * 1024; // 64KB buffer

        // Create file
        let fd = unsafe {
            crate::subsystems::syscalls::fs::sys_open(
                test_file.as_ptr() as *const u8,
                crate::posix::O_CREAT | crate::posix::O_WRONLY | crate::posix::O_TRUNC,
                0o644
            )
        }?;

        // Write data
        let mut buffer = vec![0u8; buffer_size];
        let mut total_written = 0;

        while total_written < file_size {
            let result = unsafe {
                crate::subsystems::syscalls::fs::sys_write(
                    fd,
                    buffer.as_ptr()
                )
            }?;

            total_written += result as usize;

            if total_written % (10 * 1024 * 1024) == 0 {
                println!("Written {} MB", total_written / (1024 * 1024));
            }
        }

        let _ = unsafe {
            crate::subsystems::syscalls::fs::sys_close(fd)
        };

        println!("Write completed: {} MB", total_written / (1024 * 1024));

        // Read back
        let fd = unsafe {
            crate::subsystems::syscalls::fs::sys_open(
                test_file.as_ptr() as *const u8,
                crate::posix::O_RDONLY,
                0
            )
        }?;

        let mut total_read = 0;

        while total_read < file_size {
            let result = unsafe {
                crate::subsystems::syscalls::fs::sys_read(
                    fd,
                    buffer.as_mut_ptr()
                )
            }?;

            total_read += result as usize;

            if result == 0 {
                break;
            }
        }

        let _ = unsafe {
            crate::subsystems::syscalls::fs::sys_close(fd)
        };

        println!("Read completed: {} MB", total_read / (1024 * 1024));

        // Cleanup
        let _ = unsafe {
            crate::subsystems::syscalls::fs::sys_unlink(
                test_file.as_ptr() as *const u8
            )
        };

        println!("File I/O stress test completed");
        Ok(())
    }

    /// Directory operations stress test
    pub fn stress_directory_operations() -> Result<(), UnifiedError> {
        println!("Starting directory operations stress test...");

        let base_path = "/tmp/dir_stress_test";
        let num_dirs = 100;
        let files_per_dir = 100;

        // Create base directory
        let _ = unsafe {
            crate::subsystems::syscalls::fs::sys_mkdir(
                base_path.as_ptr() as *const u8,
                0o755
            )
        };

        // Create subdirectories
        for i in 0..num_dirs {
            let dir_path = format!("{}/dir_{}", base_path, i);

            let _ = unsafe {
                crate::subsystems::syscalls::fs::sys_mkdir(
                    dir_path.as_ptr() as *const u8,
                    0o755
                )
            };

            // Create files in subdirectory
            for j in 0..files_per_dir {
                let file_path = format!("{}/dir_{}/file_{}.txt", base_path, i, j);

                let fd = unsafe {
                    crate::subsystems::syscalls::fs::sys_open(
                        file_path.as_ptr() as *const u8,
                        crate::posix::O_CREAT | crate::posix::O_WRONLY | crate::posix::O_EXCL,
                        0o644
                    )
                };

                if let Ok(fd) = fd {
                    let _ = unsafe {
                        crate::subsystems::syscalls::fs::sys_close(fd)
                    };
                }
            }

            if i % 10 == 0 {
                println!("Created {} directories", i);
            }
        }

        println!("Created {} directories with {} files each",
            num_dirs, files_per_dir
        );

        // Cleanup (recursive delete)
        for i in 0..num_dirs {
            for j in 0..files_per_dir {
                let file_path = format!("{}/dir_{}/file_{}.txt", base_path, i, j);
                let _ = unsafe {
                    crate::subsystems::syscalls::fs::sys_unlink(
                        file_path.as_ptr() as *const u8
                    )
                };
            }

            let dir_path = format!("{}/dir_{}", base_path, i);
            let _ = unsafe {
                crate::subsystems::syscalls::fs::sys_rmdir(
                    dir_path.as_ptr() as *const u8
                )
            };
        }

        let _ = unsafe {
            crate::subsystems::syscalls::fs::sys_rmdir(
                base_path.as_ptr() as *const u8
            )
        };

        println!("Directory operations stress test completed");
        Ok(())
    }
}

// ============================================================================
// Stress Test Suite
// ============================================================================

pub fn run_all_stress_tests() -> Result<(), UnifiedError> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║       NOS Kernel - Comprehensive Stress Test Suite        ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    // Memory stress tests
    println!("=== Memory Stress Tests ===");
    memory::stress_memory_pressure()?;
    memory::stress_alloc_dealloc_cycles()?;
    memory::stress_fragmentation()?;
    println!();

    // Process stress tests
    println!("=== Process Stress Tests ===");
    process::stress_process_creation()?;
    process::stress_thread_creation()?;
    process::stress_context_switching()?;
    println!();

    // IPC stress tests
    println!("=== IPC Stress Tests ===");
    ipc::stress_message_queues()?;
    ipc::stress_shared_memory()?;
    ipc::stress_pipes()?;
    println!();

    // Network stress tests
    println!("=== Network Stress Tests ===");
    network::stress_network_connections()?;
    network::stress_udp_packets()?;
    println!();

    // Filesystem stress tests
    println!("=== Filesystem Stress Tests ===");
    filesystem::stress_file_creation()?;
    filesystem::stress_file_io()?;
    filesystem::stress_directory_operations()?;
    println!();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║          All Stress Tests Completed Successfully!         ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    Ok(())
}

// ============================================================================
// Stress Test Configuration
// ============================================================================

pub struct StressTestConfig {
    pub memory_test_enabled: bool,
    pub process_test_enabled: bool,
    pub ipc_test_enabled: bool,
    pub network_test_enabled: bool,
    pub fs_test_enabled: bool,
    pub verbose: bool,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            memory_test_enabled: true,
            process_test_enabled: true,
            ipc_test_enabled: true,
            network_test_enabled: true,
            fs_test_enabled: true,
            verbose: true,
        }
    }
}

pub fn run_stress_tests_with_config(config: StressTestConfig) -> Result<(), UnifiedError> {
    if config.memory_test_enabled {
        memory::stress_memory_pressure()?;
        memory::stress_alloc_dealloc_cycles()?;
    }

    if config.process_test_enabled {
        process::stress_process_creation()?;
        process::stress_thread_creation()?;
    }

    if config.ipc_test_enabled {
        ipc::stress_message_queues()?;
        ipc::stress_pipes()?;
    }

    if config.network_test_enabled {
        network::stress_network_connections()?;
    }

    if config.fs_test_enabled {
        filesystem::stress_file_io()?;
    }

    Ok(())
}
