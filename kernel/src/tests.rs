//! Kernel Test Framework
//!
//! Provides test infrastructure for kernel testing in both hosted and bare-metal
//! environments. Tests are organized by module and can be run with the
//! `kernel_tests` feature.
//!
//! # Usage
//! ```
//! #[cfg(feature = "kernel_tests")]
//! mod tests {
//!     use super::*;
//!     use crate::tests::{test_case, TestResult};
//!     
//!     test_case!(test_example, {
//!         assert_eq!(1 + 1, 2);
//!         Ok(())
//!     });
//! }
//! ```

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Test result type
pub type TestResult = Result<(), String>;

/// Global test counters
static TESTS_PASSED: AtomicUsize = AtomicUsize::new(0);
static TESTS_FAILED: AtomicUsize = AtomicUsize::new(0);
static TESTS_SKIPPED: AtomicUsize = AtomicUsize::new(0);

/// Test runner state
pub struct TestRunner {
    tests: Vec<(&'static str, fn() -> TestResult)>,
}

impl TestRunner {
    /// Create a new test runner
    pub const fn new() -> Self {
        TestRunner { tests: Vec::new() }
    }

    /// Register a test case
    pub fn add_test(&mut self, name: &'static str, test_fn: fn() -> TestResult) {
        self.tests.push((name, test_fn));
    }

    /// Run all registered tests
    pub fn run_all(&self) -> (usize, usize, usize) {
        TESTS_PASSED.store(0, Ordering::SeqCst);
        TESTS_FAILED.store(0, Ordering::SeqCst);
        TESTS_SKIPPED.store(0, Ordering::SeqCst);

        crate::println!();
        crate::println!("==== Running {} tests ====", self.tests.len());
        crate::println!();

        for (name, test_fn) in &self.tests {
            crate::print!("  {}: ", name);
            match test_fn() {
                Ok(()) => {
                    TESTS_PASSED.fetch_add(1, Ordering::SeqCst);
                    crate::println!("\x1b[32mPASSED\x1b[0m");
                }
                Err(msg) => {
                    TESTS_FAILED.fetch_add(1, Ordering::SeqCst);
                    crate::println!("\x1b[31mFAILED\x1b[0m: {}", msg);
                }
            }
        }

        let passed = TESTS_PASSED.load(Ordering::SeqCst);
        let failed = TESTS_FAILED.load(Ordering::SeqCst);
        let skipped = TESTS_SKIPPED.load(Ordering::SeqCst);

        crate::println!();
        crate::println!("==== Test Results ====");
        crate::println!("  Passed:  {}", passed);
        crate::println!("  Failed:  {}", failed);
        crate::println!("  Skipped: {}", skipped);
        crate::println!();

        (passed, failed, skipped)
    }
}

/// Mark a test as skipped
pub fn skip_test(reason: &str) -> TestResult {
    TESTS_SKIPPED.fetch_add(1, Ordering::SeqCst);
    Err(alloc::format!("SKIPPED: {}", reason))
}

/// Helper macro for creating test cases
#[macro_export]
macro_rules! test_case {
    ($name:ident, $body:block) => {
        pub fn $name() -> $crate::tests::TestResult {
            $body
        }
    };
}

/// Helper macro for assertions with better error messages
#[macro_export]
macro_rules! test_assert {
    ($cond:expr) => {
        if !$cond {
            return Err(alloc::format!("Assertion failed: {}", stringify!($cond)));
        }
    };
    ($cond:expr, $msg:expr) => {
        if !$cond {
            return Err(alloc::format!("Assertion failed: {} - {}", stringify!($cond), $msg));
        }
    };
}

/// Helper macro for equality assertions
#[macro_export]
macro_rules! test_assert_eq {
    ($left:expr, $right:expr) => {
        if $left != $right {
            return Err(alloc::format!(
                "Assertion failed: {} == {} (left: {:?}, right: {:?})",
                stringify!($left),
                stringify!($right),
                $left,
                $right
            ));
        }
    };
}

// ============================================================================
// Alloc module tests
// ============================================================================

pub mod alloc_tests {
    use super::*;
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    /// Test basic Box allocation
    pub fn test_box_alloc() -> TestResult {
        let b = Box::new(42u64);
        test_assert_eq!(*b, 42u64);
        Ok(())
    }

    /// Test Vec with growth
    pub fn test_vec_growth() -> TestResult {
        let mut v: Vec<i32> = Vec::new();
        for i in 0..1000 {
            v.push(i);
        }
        test_assert_eq!(v.len(), 1000);
        test_assert_eq!(v.iter().sum::<i32>(), (0..1000).sum::<i32>());
        Ok(())
    }

    /// Test large allocation
    pub fn test_large_alloc() -> TestResult {
        let v: Vec<u8> = alloc::vec![0xAA; 65536];
        test_assert_eq!(v.len(), 65536);
        test_assert!(v.iter().all(|&x| x == 0xAA));
        Ok(())
    }

    /// Test multiple allocations
    pub fn test_multiple_allocs() -> TestResult {
        let mut boxes: Vec<Box<[u8; 256]>> = Vec::new();
        for i in 0..100 {
            let mut data = [0u8; 256];
            data[0] = i as u8;
            boxes.push(Box::new(data));
        }
        for (i, b) in boxes.iter().enumerate() {
            test_assert_eq!(b[0], i as u8);
        }
        Ok(())
    }

    /// Test allocation and deallocation cycle
    pub fn test_alloc_dealloc_cycle() -> TestResult {
        for _ in 0..10 {
            let mut v: Vec<Box<u64>> = Vec::new();
            for i in 0..100 {
                v.push(Box::new(i as u64));
            }
            // Deallocate by dropping
        }
        Ok(())
    }
}

// ============================================================================
// Sync module tests
// ============================================================================

pub mod sync_tests {
    use super::*;
    use crate::sync::{SpinLock, Mutex};

    /// Test SpinLock basic operations
    pub fn test_spinlock_basic() -> TestResult {
        let sl = SpinLock::new();
        test_assert!(!sl.is_locked());
        sl.lock();
        test_assert!(sl.is_locked());
        sl.unlock();
        test_assert!(!sl.is_locked());
        Ok(())
    }

    /// Test Mutex with data
    pub fn test_mutex_data() -> TestResult {
        let mutex: Mutex<i32> = Mutex::new(0);
        {
            let mut guard = mutex.lock();
            *guard = 100;
        }
        test_assert_eq!(*mutex.lock(), 100);
        Ok(())
    }

    /// Test Mutex modification
    pub fn test_mutex_modify() -> TestResult {
        let mutex: Mutex<Vec<i32>> = Mutex::new(Vec::new());
        {
            let mut guard = mutex.lock();
            for i in 0..10 {
                guard.push(i);
            }
        }
        test_assert_eq!(mutex.lock().len(), 10);
        Ok(())
    }
}

// ============================================================================
// File system tests
// ============================================================================

pub mod file_tests {
    use super::*;
    use crate::file;

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
}

// ============================================================================
// Pipe tests
// ============================================================================

pub mod pipe_tests {
    use super::*;
    use crate::pipe;
    use crate::file;

    /// Test basic pipe operations
    pub fn test_pipe_basic() -> TestResult {
        let result = pipe::pipe_alloc();
        if let Some((read_fd, write_fd)) = result {
            let data = b"hello";
            let written = pipe::pipe_write(write_fd, data);
            test_assert_eq!(written, 5);

            let mut buf = [0u8; 16];
            let read = pipe::pipe_read(read_fd, &mut buf);
            test_assert_eq!(read, 5);
            test_assert_eq!(&buf[..5], b"hello");

            file::file_close(read_fd);
            file::file_close(write_fd);
            Ok(())
        } else {
            skip_test("pipe_alloc not available")
        }
    }

    /// Test pipe with non-blocking mode
    pub fn test_pipe_nonblock() -> TestResult {
        use crate::posix::O_NONBLOCK;
        
        if let Some((rfd_idx, wfd_idx)) = pipe::pipe_alloc() {
            {
                let mut table = file::FILE_TABLE.lock();
                if let Some(f) = table.get_mut(rfd_idx) {
                    f.status_flags |= O_NONBLOCK;
                }
            }
            let mut buf = [0u8; 4];
            let ret = file::file_read(rfd_idx, &mut buf);
            test_assert_eq!(ret, crate::errno::errno_neg(crate::errno::EAGAIN));
            file::file_close(rfd_idx);
            file::file_close(wfd_idx);
            Ok(())
        } else {
            skip_test("pipe_alloc not available")
        }
    }

    /// Test pipe close behavior
    pub fn test_pipe_close_read() -> TestResult {
        if let Some((rfd_idx, wfd_idx)) = pipe::pipe_alloc() {
            file::file_close(rfd_idx);
            let buf = [0xBBu8; 16];
            let n = file::file_write(wfd_idx, &buf);
            test_assert_eq!(n, crate::errno::errno_neg(crate::errno::EPIPE));
            file::file_close(wfd_idx);
            Ok(())
        } else {
            skip_test("pipe_alloc not available")
        }
    }
}

// ============================================================================
// VFS tests
// ============================================================================

pub mod vfs_tests {
    use super::*;
    use crate::vfs::{FileMode, vfs};

    /// Test VFS create and write
    pub fn test_vfs_create_write() -> TestResult {
        let path = "/test_create";
        let mut f = match vfs().create(path, FileMode::new(FileMode::S_IFREG | 0o644)) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("create failed: {:?}", e)),
        };
        
        let msg = b"test content";
        match f.write(msg.as_ptr() as usize, msg.len()) {
            Ok(n) => test_assert_eq!(n, msg.len()),
            Err(e) => return Err(alloc::format!("write failed: {:?}", e)),
        }
        
        // Cleanup
        let _ = vfs().unlink(path);
        Ok(())
    }

    /// Test VFS read
    pub fn test_vfs_read() -> TestResult {
        let path = "/test_read";
        let msg = b"hello world";
        
        // Create and write
        let mut f = match vfs().create(path, FileMode::new(FileMode::S_IFREG | 0o644)) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("create failed: {:?}", e)),
        };
        let _ = f.write(msg.as_ptr() as usize, msg.len());
        
        // Open and read
        let mut f2 = match vfs().open(path, 0) {
            Ok(f) => f,
            Err(e) => return Err(alloc::format!("open failed: {:?}", e)),
        };
        
        let mut buf = [0u8; 32];
        match f2.read(buf.as_mut_ptr() as usize, msg.len()) {
            Ok(n) => {
                test_assert_eq!(n, msg.len());
                test_assert_eq!(&buf[..msg.len()], msg);
            }
            Err(e) => return Err(alloc::format!("read failed: {:?}", e)),
        }
        
        // Cleanup
        let _ = vfs().unlink(path);
        Ok(())
    }

    /// Test VFS mkdir
    pub fn test_vfs_mkdir() -> TestResult {
        let path = "/test_dir";
        
        match vfs().mkdir(path, FileMode::new(FileMode::S_IFDIR | 0o755)) {
            Ok(_) => {}
            Err(e) => return Err(alloc::format!("mkdir failed: {:?}", e)),
        }
        
        // Verify it's a directory
        match vfs().stat(path) {
            Ok(attr) => {
                let mode = FileMode::new(attr.mode.0);
                test_assert!(mode.file_type() == crate::vfs::FileType::Directory);
            }
            Err(e) => return Err(alloc::format!("stat failed: {:?}", e)),
        }
        
        // Cleanup
        let _ = vfs().rmdir(path);
        Ok(())
    }
}

// ============================================================================
// Process tests
// ============================================================================

pub mod process_tests {
    use super::*;
    use crate::process;

    /// Test process getpid
    pub fn test_getpid() -> TestResult {
        let pid = process::getpid();
        // PID should be non-negative
        test_assert!(pid >= 0);
        Ok(())
    }
}

// ============================================================================
// Test runner registration
// ============================================================================

/// Run all kernel tests
pub fn run_all_tests() -> (usize, usize, usize) {
    let mut runner = TestRunner::new();
    
    // Alloc tests
    runner.add_test("alloc::box_alloc", alloc_tests::test_box_alloc);
    runner.add_test("alloc::vec_growth", alloc_tests::test_vec_growth);
    runner.add_test("alloc::large_alloc", alloc_tests::test_large_alloc);
    runner.add_test("alloc::multiple_allocs", alloc_tests::test_multiple_allocs);
    runner.add_test("alloc::alloc_dealloc_cycle", alloc_tests::test_alloc_dealloc_cycle);
    
    // Sync tests
    runner.add_test("sync::spinlock_basic", sync_tests::test_spinlock_basic);
    runner.add_test("sync::mutex_data", sync_tests::test_mutex_data);
    runner.add_test("sync::mutex_modify", sync_tests::test_mutex_modify);
    
    // File tests
    runner.add_test("file::fd_alloc", file_tests::test_fd_alloc);
    runner.add_test("file::multiple_fd_alloc", file_tests::test_multiple_fd_alloc);
    
    // Pipe tests
    runner.add_test("pipe::basic", pipe_tests::test_pipe_basic);
    runner.add_test("pipe::nonblock", pipe_tests::test_pipe_nonblock);
    runner.add_test("pipe::close_read", pipe_tests::test_pipe_close_read);
    
    // VFS tests
    runner.add_test("vfs::create_write", vfs_tests::test_vfs_create_write);
    runner.add_test("vfs::read", vfs_tests::test_vfs_read);
    runner.add_test("vfs::mkdir", vfs_tests::test_vfs_mkdir);
    
    // Process tests
    runner.add_test("process::getpid", process_tests::test_getpid);
    
    runner.run_all()
}
