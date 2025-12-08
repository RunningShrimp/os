//! Common test utilities for kernel integration tests

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Integration test result type
pub type IntegrationTestResult = Result<(), String>;

/// Test utilities for integration tests
pub struct TestUtils;

/// Helper macros for integration tests
#[macro_export]
macro_rules! integration_test_assert {
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

#[macro_export]
macro_rules! integration_test_assert_eq {
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
    ($left:expr, $right:expr, $msg:expr) => {
        if $left != $right {
            return Err(alloc::format!(
                "Assertion failed: {} == {} - {} (left: {:?}, right: {:?})",
                stringify!($left),
                stringify!($right),
                $msg,
                $left,
                $right
            ));
        }
    };
}

impl TestUtils {
    /// Setup function for integration tests
    pub fn setup() -> IntegrationTestResult {
        // Initialize kernel components needed for testing
        // This would include setting up memory management, process tables, etc.
        Ok(())
    }

    /// Cleanup function for integration tests
    pub fn cleanup() -> IntegrationTestResult {
        // Clean up after tests
        Ok(())
    }

    /// Helper to create a temporary file for testing
    pub fn create_temp_file(name: &str, content: &[u8]) -> IntegrationTestResult {
        // Create /tmp if missing then create the requested file and write content
        let path = alloc::format!("/tmp/{}", name);

        // Ensure tmp directory exists (best effort)
        let _ = crate::vfs::vfs().mkdir("/tmp", crate::vfs::FileMode::new(crate::vfs::FileMode::S_IFDIR | crate::vfs::FileMode::S_IRWXU));

        match crate::vfs::vfs().create(&path, crate::vfs::FileMode::new(crate::vfs::FileMode::S_IFREG | crate::vfs::FileMode::S_IRUSR | crate::vfs::FileMode::S_IWUSR)) {
            Ok(_) => {
                // write content at offset 0
                let _ = crate::vfs::vfs().write(&path, content, 0);
                Ok(())
            }
            Err(e) => Err(alloc::format!("failed to create temp file {}: {:?}", path, e)),
        }
    }

    /// Helper to remove a temporary file
    pub fn remove_temp_file(name: &str) -> IntegrationTestResult {
        let path = alloc::format!("/tmp/{}", name);
        match crate::vfs::vfs().unlink(&path) {
            Ok(_) => Ok(()),
            Err(e) => Err(alloc::format!("failed to remove temp file {}: {:?}", path, e)),
        }
    }
}

/// Test fixture for setting up and tearing down test environments
pub struct TestFixture {
    pub name: String,
    setup_fn: Option<fn() -> IntegrationTestResult>,
    cleanup_fn: Option<fn() -> IntegrationTestResult>,
}

impl TestFixture {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            setup_fn: None,
            cleanup_fn: None,
        }
    }

    pub fn with_setup(mut self, setup: fn() -> IntegrationTestResult) -> Self {
        self.setup_fn = Some(setup);
        self
    }

    pub fn with_cleanup(mut self, cleanup: fn() -> IntegrationTestResult) -> Self {
        self.cleanup_fn = Some(cleanup);
        self
    }

    pub fn run_test<F>(&self, test_fn: F) -> IntegrationTestResult
    where
        F: FnOnce() -> IntegrationTestResult,
    {
        // Run setup
        if let Some(setup) = self.setup_fn {
            setup()?;
        }

        // Run the test
        let result = test_fn();

        // Run cleanup
        if let Some(cleanup) = self.cleanup_fn {
            let _ = cleanup(); // Ignore cleanup errors
        }

        result
    }
}

/// Performance measurement utilities
pub struct PerformanceTimer {
    start_time: u64,
    name: String,
}

impl PerformanceTimer {
    pub fn new(name: &str) -> Self {
        Self {
            start_time: crate::time::get_ticks(),
            name: name.to_string(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        let current = crate::time::get_ticks();
        // Convert ticks to milliseconds (assuming some tick frequency)
        (current - self.start_time) / 1000
    }

    pub fn print_elapsed(&self) {
        let elapsed = self.elapsed_ms();
        crate::println!("Performance [{}]: {} ms", self.name, elapsed);
    }
}

impl Drop for PerformanceTimer {
    fn drop(&mut self) {
        self.print_elapsed();
    }
}