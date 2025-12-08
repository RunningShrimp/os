//! memfd_create system call test cases

use super::common::{SyscallError, SyscallResult};
use crate::syscalls::glib::{memfd_flags, fcntl_seals, get_memfd_instance};
use crate::fs::file;
use crate::process;

/// Test memfd_create basic functionality
pub fn test_memfd_create_basic() -> SyscallResult {
    crate::println!("Testing memfd_create basic functionality...");
    
    // Test 1: Create memfd with name and no flags
    let name = "test_memfd";
    let name_ptr = name.as_ptr() as usize;
    
    // Call memfd_create syscall (syscall ID 0xB001)
    let fd = super::dispatch(0xB001, &[name_ptr as u64, 0u64])?;
    
    crate::println!("Created memfd with fd: {}", fd);
    
    // Test 2: Verify file descriptor is valid
    if fd >= 0 && fd < file::NOFILE as u64 {
        crate::println!("✓ File descriptor is valid");
    } else {
        crate::println!("✗ File descriptor is invalid");
        return Err(SyscallError::InvalidArgument);
    }
    
    // Test 3: Check if file is readable and writable
    let file_idx = process::fdlookup(fd as i32).ok_or(SyscallError::BadFileDescriptor)?;
    {
        let file_table = file::FILE_TABLE.lock();
        if let Some(file) = file_table.get(file_idx) {
            if file.readable && file.writable {
                crate::println!("✓ File is readable and writable");
            } else {
                crate::println!("✗ File is not readable or writable");
                return Err(SyscallError::InvalidArgument);
            }
        } else {
            return Err(SyscallError::BadFileDescriptor);
        }
    }
    
    // Test 4: Try to write to memfd
    let test_data = b"Hello, memfd!";
    let test_ptr = test_data.as_ptr() as usize;
    let test_len = test_data.len();
    
    // Call write syscall
    let bytes_written = super::dispatch(0x2003, &[fd, test_ptr as u64, test_len as u64])?;
    crate::println!("Written {} bytes to memfd", bytes_written);
    
    if bytes_written != test_len as u64 {
        crate::println!("✗ Write operation failed");
        return Err(SyscallError::IoError);
    } else {
        crate::println!("✓ Write operation succeeded");
    }
    
    // Test 5: Try to read from memfd
    let mut read_buffer = [0u8; 64];
    let read_ptr = read_buffer.as_mut_ptr() as usize;
    
    // Call read syscall
    let bytes_read = super::dispatch(0x2002, &[fd, read_ptr as u64, 64u64])?;
    crate::println!("Read {} bytes from memfd", bytes_read);
    
    if bytes_read != test_len as u64 {
        crate::println!("✗ Read operation failed");
        return Err(SyscallError::IoError);
    } else {
        crate::println!("✓ Read operation succeeded");
    }
    
    // Test 6: Verify data integrity
    if &read_buffer[..test_len] == test_data {
        crate::println!("✓ Data integrity verified");
    } else {
        crate::println!("✗ Data integrity check failed");
        return Err(SyscallError::IoError);
    }
    
    // Close the file descriptor
    super::dispatch(0x2001, &[fd])?;
    crate::println!("✓ File descriptor closed");
    
    crate::println!("memfd_create basic test passed!");
    Ok(0)
}

/// Test memfd_create with MFD_CLOEXEC flag
pub fn test_memfd_cloexec() -> SyscallResult {
    crate::println!("Testing memfd_create with MFD_CLOEXEC flag...");
    
    let name = "test_cloexec";
    let name_ptr = name.as_ptr() as usize;
    let flags = memfd_flags::MFD_CLOEXEC;
    
    // Create memfd with CLOEXEC flag
    let fd = super::dispatch(0xB001, &[name_ptr as u64, flags as u64])?;
    crate::println!("Created memfd with CLOEXEC flag, fd: {}", fd);
    
    // Verify the flag was set by checking file status
    let file_idx = process::fdlookup(fd as i32).ok_or(SyscallError::BadFileDescriptor)?;
    {
        let file_table = file::FILE_TABLE.lock();
        if let Some(file) = file_table.get(file_idx) {
            if (file.status_flags & crate::posix::O_CLOEXEC) != 0 {
                crate::println!("✓ O_CLOEXEC flag is set");
            } else {
                crate::println!("✗ O_CLOEXEC flag is not set");
                return Err(SyscallError::InvalidArgument);
            }
        } else {
            return Err(SyscallError::BadFileDescriptor);
        }
    }
    
    // Close the file descriptor
    super::dispatch(0x2001, &[fd])?;
    
    crate::println!("memfd_create CLOEXEC test passed!");
    Ok(0)
}

/// Test memfd_create with MFD_ALLOW_SEALING flag
pub fn test_memfd_sealing() -> SyscallResult {
    crate::println!("Testing memfd_create with MFD_ALLOW_SEALING flag...");
    
    let name = "test_sealing";
    let name_ptr = name.as_ptr() as usize;
    let flags = memfd_flags::MFD_ALLOW_SEALING;
    
    // Create memfd with ALLOW_SEALING flag
    let fd = super::dispatch(0xB001, &[name_ptr as u64, flags as u64])?;
    crate::println!("Created memfd with ALLOW_SEALING flag, fd: {}", fd);
    
    // Write some test data
    let test_data = b"Test data for sealing";
    let test_ptr = test_data.as_ptr() as usize;
    let test_len = test_data.len();
    
    super::dispatch(0x2003, &[fd, test_ptr as u64, test_len as u64])?;
    crate::println!("Written {} bytes to memfd", test_len);
    
    // Test 1: Get current seals (should be 0 initially)
    let file_idx = process::fdlookup(fd as i32).ok_or(SyscallError::BadFileDescriptor)?;
    let current_seals = super::dispatch(0x200A, &[fd, 0, 0])?; // F_GET_SEALS
    crate::println!("Current seals: {}", current_seals);
    
    if current_seals != 0 {
        crate::println!("✗ Initial seals should be 0");
        return Err(SyscallError::InvalidArgument);
    }
    
    // Test 2: Add write seal
    let seals_to_add = fcntl_seals::F_SEAL_WRITE;
    let new_seals = super::dispatch(0x200A, &[fd, seals_to_add as u64, 0])?; // F_ADD_SEALS
    crate::println!("Added write seal, new seals: {}", new_seals);
    
    // Test 3: Try to write after sealing (should fail)
    let write_result = super::dispatch(0x2003, &[fd, test_ptr as u64, test_len as u64]);
    match write_result {
        Ok(_) => {
            crate::println!("✗ Write succeeded after sealing (should have failed)");
            return Err(SyscallError::InvalidArgument);
        }
        Err(SyscallError::PermissionDenied) => {
            crate::println!("✓ Write correctly failed after sealing");
        }
        Err(e) => {
            crate::println!("✗ Unexpected error: {:?}", e);
            return Err(e);
        }
    }
    
    // Test 4: Try to read after sealing (should succeed)
    let mut read_buffer = [0u8; 64];
    let read_ptr = read_buffer.as_mut_ptr() as usize;
    let bytes_read = super::dispatch(0x2002, &[fd, read_ptr as u64, 64u64])?;
    
    if bytes_read != test_len as u64 {
        crate::println!("✗ Read failed after sealing");
        return Err(SyscallError::IoError);
    } else {
        crate::println!("✓ Read succeeded after sealing");
    }
    
    // Close the file descriptor
    super::dispatch(0x2001, &[fd])?;
    
    crate::println!("memfd_create sealing test passed!");
    Ok(0)
}

/// Test memfd_create with invalid flags
pub fn test_memfd_invalid_flags() -> SyscallResult {
    crate::println!("Testing memfd_create with invalid flags...");
    
    let name = "test_invalid";
    let name_ptr = name.as_ptr() as usize;
    let invalid_flags = 0x12345678; // Invalid flags
    
    // Try to create memfd with invalid flags (should fail)
    match super::dispatch(0xB001, &[name_ptr as u64, invalid_flags as u64]) {
        Ok(_) => {
            crate::println!("✗ memfd_create succeeded with invalid flags (should have failed)");
            return Err(SyscallError::InvalidArgument);
        }
        Err(SyscallError::InvalidArgument) => {
            crate::println!("✓ memfd_create correctly failed with invalid flags");
        }
        Err(e) => {
            crate::println!("✗ Unexpected error: {:?}", e);
            return Err(e);
        }
    }
    
    crate::println!("memfd_create invalid flags test passed!");
    Ok(0)
}

/// Test memfd_create with empty name
pub fn test_memfd_empty_name() -> SyscallResult {
    crate::println!("Testing memfd_create with empty name...");
    
    let empty_name = "";
    let name_ptr = empty_name.as_ptr() as usize;
    
    // Create memfd with empty name (should use default name)
    let fd = super::dispatch(0xB001, &[name_ptr as u64, 0u64])?;
    crate::println!("Created memfd with empty name, fd: {}", fd);
    
    // Close the file descriptor
    super::dispatch(0x2001, &[fd])?;
    
    crate::println!("memfd_create empty name test passed!");
    Ok(0)
}

/// Run all memfd_create tests
pub fn run_all_memfd_tests() -> SyscallResult {
    crate::println!("=== Starting memfd_create test suite ===");
    
    // Run all test cases
    test_memfd_create_basic()?;
    test_memfd_cloexec()?;
    test_memfd_sealing()?;
    test_memfd_invalid_flags()?;
    test_memfd_empty_name()?;
    
    crate::println!("=== All memfd_create tests completed ===");
    Ok(0)
}