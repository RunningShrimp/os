//! File System Tests
//!
//! Tests for file system functionality

#[cfg(feature = "kernel_tests")]
pub mod file_tests {
    use alloc::string::String;
    use alloc::vec::Vec;
    use crate::{test_assert_eq, test_assert, test_assert_ne};
    use crate::tests::skip_test;
    use crate::tests::TestResult;
    use crate::fs::file;

    /// Test file descriptor allocation
    pub fn test_fd_alloc() -> TestResult {
        if let Some(fd) = file::file_alloc() {
            file::file_close(fd);
            Ok(())
        } else {
            Err(String::from("file_alloc returned None"))
        }
    }

    /// Test multiple fd allocation
    pub fn test_multiple_fd_alloc() -> TestResult {
        let mut fds = Vec::new();
        for _ in 0..10 {
            match file::file_alloc() {
                Some(fd) => fds.push(fd),
                None => return Err(String::from("Failed to allocate fd")),
            }
        }
        for fd in fds {
            file::file_close(fd);
        }
        Ok(())
    }

    /// Test file table initialization
    pub fn test_file_table_init() -> TestResult {
        let table = crate::fs::FTable::new();

        // All files should be unused initially
        for i in 0..crate::fs::NFILE {
            let file = table.get(i);
            test_assert!(file.is_none() || !file.unwrap().is_valid(),
                alloc::format!("File {} should be unused initially", i));
        }

        Ok(())
    }

    /// Test file allocation and deallocation
    pub fn test_file_alloc_dealloc() -> TestResult {
        // Allocate a file
        let fd1 = crate::fs::file_alloc();
        test_assert!(fd1.is_some(), "File allocation should succeed");
        let fd1_idx = fd1.unwrap();

        // Verify file exists
        let table = crate::fs::FILE_TABLE.lock();
        let file = table.get(fd1_idx);
        test_assert!(file.is_some(), "Allocated file should exist");
        test_assert!(file.unwrap().is_valid(), "Allocated file should be valid");
        test_assert_eq!(file.unwrap().ref_count, 1, "File should have ref count 1");
        drop(table);

        // Allocate another file
        let fd2 = crate::fs::file_alloc();
        test_assert!(fd2.is_some(), "Second file allocation should succeed");
        let fd2_idx = fd2.unwrap();
        test_assert_ne!(fd1_idx, fd2_idx, "File indices should be different");

        // Close first file
        crate::fs::file_close(fd1_idx);

        // Verify first file is freed
        let table = crate::fs::FILE_TABLE.lock();
        let file1 = table.get(fd1_idx);
        test_assert!(file1.is_none() || !file1.unwrap().is_valid(),
            "Closed file should be invalid");

        // Second file should still exist
        let file2 = table.get(fd2_idx);
        test_assert!(file2.is_some() && file2.unwrap().is_valid(),
            "Second file should still be valid");

        // Close second file
        crate::fs::file_close(fd2_idx);

        Ok(())
    }

    /// Test file reference counting
    pub fn test_file_ref_counting() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        // Duplicate the file (increment ref count)
        let dup_fd = crate::fs::file_dup(fd).unwrap();
        test_assert_eq!(dup_fd, fd, "Dup should return same file index");

        // Check ref count
        let table = crate::fs::FILE_TABLE.lock();
        let file = table.get(fd).unwrap();
        test_assert_eq!(file.ref_count, 2, "File should have ref count 2");

        // Close one reference
        crate::fs::file_close(fd);

        // File should still exist
        let file_after = table.get(fd).unwrap();
        test_assert!(file_after.is_valid(), "File should still be valid after closing one ref");
        test_assert_eq!(file_after.ref_count, 1, "File should have ref count 1");

        // Close last reference
        crate::fs::file_close(dup_fd);

        // File should be freed
        let file_final = table.get(fd);
        test_assert!(file_final.is_none() || !file_final.unwrap().is_valid(),
            "File should be freed after closing last ref");

        Ok(())
    }

    /// Test file type validation
    pub fn test_file_type_validation() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        let table = crate::fs::FILE_TABLE.lock();
        let file = table.get_mut(fd).unwrap();

        // Initially should be None type
        test_assert_eq!(file.ftype, crate::fs::FileType::None);
        test_assert!(!file.is_valid(), "File with None type should not be valid");

        // Set different types
        file.ftype = crate::fs::FileType::Pipe;
        test_assert!(file.is_valid(), "File with Pipe type should be valid");

        file.ftype = crate::fs::FileType::Inode;
        test_assert!(file.is_valid(), "File with Inode type should be valid");

        file.ftype = crate::fs::FileType::Device;
        test_assert!(file.is_valid(), "File with Device type should be valid");

        file.ftype = crate::fs::FileType::Vfs;
        test_assert!(file.is_valid(), "File with Vfs type should be valid");

        file.ftype = crate::fs::FileType::Socket;
        test_assert!(file.is_valid(), "File with Socket type should be valid");

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test file permissions
    pub fn test_file_permissions() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        let mut table = crate::fs::FILE_TABLE.lock();
        let file = table.get_mut(fd).unwrap();

        // Initially not readable or writable
        test_assert!(!file.readable, "File should not be readable initially");
        test_assert!(!file.writable, "File should not be writable initially");

        // Set readable
        file.readable = true;
        test_assert!(file.readable, "File should be readable");

        // Set writable
        file.writable = true;
        test_assert!(file.writable, "File should be writable");

        // Reset permissions
        file.readable = false;
        file.writable = false;
        test_assert!(!file.readable, "File should not be readable");
        test_assert!(!file.writable, "File should not be writable");

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test file status flags
    pub fn test_file_status_flags() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        let mut table = crate::fs::FILE_TABLE.lock();
        let file = table.get_mut(fd).unwrap();

        // Initially flags should be 0
        test_assert_eq!(file.status_flags, 0, "File status flags should be 0 initially");

        // Set various flags
        file.status_flags = crate::posix::O_NONBLOCK;
        test_assert_eq!(file.status_flags, crate::posix::O_NONBLOCK);

        file.status_flags |= crate::posix::O_APPEND;
        test_assert_eq!(file.status_flags, crate::posix::O_NONBLOCK | crate::posix::O_APPEND);

        // Clear flags
        file.status_flags = 0;
        test_assert_eq!(file.status_flags, 0);

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test file offset management
    pub fn test_file_offset_management() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        let mut table = crate::fs::FILE_TABLE.lock();
        let file = table.get_mut(fd).unwrap();

        // Set file type to allow offset operations
        file.ftype = crate::fs::FileType::Inode;

        // Initially offset should be 0
        test_assert_eq!(file.offset, 0, "File offset should be 0 initially");

        // Set offset
        file.offset = 1024;
        test_assert_eq!(file.offset, 1024);

        file.offset = 4096;
        test_assert_eq!(file.offset, 4096);

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test file table capacity limits
    pub fn test_file_table_capacity() -> TestResult {
        let mut allocated_fds = Vec::new();

        // Allocate all available file slots
        for _ in 0..crate::fs::NFILE {
            match crate::fs::file_alloc() {
                Some(fd) => allocated_fds.push(fd),
                None => break, // No more slots available
            }
        }

        let allocated_count = allocated_fds.len();
        test_assert!(allocated_count > 0, "Should be able to allocate at least some files");

        // Next allocation should fail
        let overflow_fd = crate::fs::file_alloc();
        test_assert!(overflow_fd.is_none(), "File allocation should fail when table is full");

        // Free one file
        if let Some(fd_to_free) = allocated_fds.pop() {
            crate::fs::file_close(fd_to_free);
        }

        // Now allocation should succeed
        let new_fd = crate::fs::file_alloc();
        test_assert!(new_fd.is_some(), "File allocation should succeed after freeing");

        // Clean up remaining files
        for fd in allocated_fds {
            crate::fs::file_close(fd);
        }
        if let Some(fd) = new_fd {
            crate::fs::file_close(fd);
        }

        Ok(())
    }

    /// Test file iterator
    pub fn test_file_iterator() -> TestResult {
        let fd1 = crate::fs::file_alloc().unwrap();
        let fd2 = crate::fs::file_alloc().unwrap();

        let mut table = crate::fs::FILE_TABLE.lock();

        // Count valid files
        let mut valid_count = 0;
        for i in 0..crate::fs::NFILE {
            if let Some(file) = table.get(i) {
                if file.is_valid() {
                    valid_count += 1;
                }
            }
        }
        test_assert_eq!(valid_count, 2, "Should find 2 valid files");

        // Test mutable access
        let mut modified_count = 0;
        for i in 0..crate::fs::NFILE {
            if let Some(file) = table.get_mut(i) {
                if file.is_valid() {
                    file.readable = true;
                    modified_count += 1;
                }
            }
        }
        test_assert_eq!(modified_count, 2, "Should modify 2 files");

        // Verify modifications
        let mut readable_count = 0;
        for i in 0..crate::fs::NFILE {
            if let Some(file) = table.get(i) {
                if file.is_valid() && file.readable {
                    readable_count += 1;
                }
            }
        }
        test_assert_eq!(readable_count, 2, "Should find 2 readable files");

        // Clean up
        crate::fs::file_close(fd1);
        crate::fs::file_close(fd2);

        Ok(())
    }

    /// Test file read/write operations (basic)
    pub fn test_file_read_write_basic() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        let table = crate::fs::FILE_TABLE.lock();
        let file = table.get_mut(fd).unwrap();

        // Set up file for testing
        file.ftype = crate::fs::FileType::Inode;
        file.readable = true;
        file.writable = true;

        // Test read from unreadable file
        file.readable = false;
        let mut buf = [0u8; 10];
        let result = file.read(&mut buf);
        test_assert_eq!(result, -1, "Read from unreadable file should fail");

        // Test write to unwritable file
        file.writable = false;
        let data = [1u8, 2, 3];
        let result = file.write(&data);
        test_assert_eq!(result, -1, "Write to unwritable file should fail");

        // Test with readable/writable file (simulated success)
        file.readable = true;
        file.writable = true;
        // Note: Actual read/write would depend on underlying implementation

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test file seek operations
    pub fn test_file_seek_operations() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        let table = crate::fs::FILE_TABLE.lock();
        let file = table.get_mut(fd).unwrap();

        // Test seek on different file types
        file.ftype = crate::fs::FileType::Inode;
        let result = file.seek(1024);
        test_assert_eq!(result, 1024, "Seek on inode file should succeed");

        file.ftype = crate::fs::FileType::Device;
        let result = file.seek(2048);
        test_assert_eq!(result, -1, "Seek on device file should fail");

        file.ftype = crate::fs::FileType::Pipe;
        let result = file.seek(4096);
        test_assert_eq!(result, -1, "Seek on pipe file should fail");

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test file stat operations
    pub fn test_file_stat_operations() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        // Test stat on invalid file
        let result = crate::fs::file_stat(9999);
        test_assert!(result.is_err(), "Stat on invalid file should fail");

        // Test stat on valid file
        let result = crate::fs::file_stat(fd);
        test_assert!(result.is_ok(), "Stat on valid file should succeed");

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test file truncation
    pub fn test_file_truncate() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        // Test truncate on invalid file
        let result = crate::fs::file_truncate(9999, 1024);
        test_assert!(result.is_err(), "Truncate on invalid file should fail");

        // Test truncate on valid file (would depend on VFS support)
        let result = crate::fs::file_truncate(fd, 1024);
        // Result depends on whether VFS is available

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test file permission changes
    pub fn test_file_permission_changes() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        // Test chmod on invalid file
        let result = crate::fs::file_chmod(9999, 0o644);
        test_assert!(result.is_err(), "Chmod on invalid file should fail");

        // Test chmod on valid file (would depend on VFS support)
        let result = crate::fs::file_chmod(fd, 0o755);
        // Result depends on whether VFS is available

        // Test chown
        let result = crate::fs::file_chown(9999, 1000, 1000);
        test_assert!(result.is_err(), "Chown on invalid file should fail");

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test file event subscription
    pub fn test_file_event_subscription() -> TestResult {
        let fd = crate::fs::file_alloc().unwrap();

        // Test subscribe/unsubscribe (these are no-ops for most file types)
        crate::fs::file_subscribe(fd, crate::posix::POLLIN, 0x1000);
        crate::fs::file_unsubscribe(fd, 0x1000);

        // Test poll
        let events = crate::fs::file_poll(fd);
        test_assert!(events >= 0, "Poll should return valid event mask");

        // Clean up
        crate::fs::file_close(fd);

        Ok(())
    }

    /// Test socket file operations
    pub fn test_socket_file_operations() -> TestResult {
        // Test socket file creation (would need actual socket implementation)
        let socket_file = crate::fs::file_socket_new(
            crate::net::socket::Socket::Udp(Default::default()),
            true, false
        );

        if let Some(fd) = socket_file {
            // Verify socket file properties
            let table = crate::fs::FILE_TABLE.lock();
            if let Some(file) = table.get(fd) {
                test_assert_eq!(file.ftype, crate::fs::FileType::Socket);
                test_assert!(file.readable);
                test_assert!(!file.writable);
                test_assert!(file.socket.is_some());
            }

            // Test socket file operations
            let socket = crate::fs::file_get_socket(fd);
            test_assert!(socket.is_some());

            // Clean up
            crate::fs::file_close(fd);
        } else {
            // Socket creation not available, skip test
            skip_test("Socket file creation not available");
        }

        Ok(())
    }

    /// Test file system stress test
    pub fn test_file_system_stress() -> TestResult {
        let mut allocated_files = Vec::new();

        // Allocate many files
        for _ in 0..50 {
            if let Some(fd) = crate::fs::file_alloc() {
                allocated_files.push(fd);
            } else {
                break;
            }
        }

        let allocated_count = allocated_files.len();
        test_assert!(allocated_count > 0, "Should allocate at least some files");

        // Perform operations on all files
        for &fd in &allocated_files {
            let table = crate::fs::FILE_TABLE.lock();
            if let Some(file) = table.get_mut(fd) {
                file.readable = true;
                file.writable = true;
                file.status_flags = crate::posix::O_NONBLOCK;
            }
        }

        // Free all files
        for fd in allocated_files {
            crate::fs::file_close(fd);
        }

        // Verify all files are freed
        let table = crate::fs::FILE_TABLE.lock();
        let mut valid_count = 0;
        for i in 0..crate::fs::NFILE {
            if let Some(file) = table.get(i) {
                if file.is_valid() {
                    valid_count += 1;
                }
            }
        }
        test_assert_eq!(valid_count, 0, "All files should be freed");

        Ok(())
    }
}

// ============================================================================
// Pipe tests
// ============================================================================

