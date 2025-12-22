//! Common test utilities
//!
//! This module provides common utilities for testing NOS components.

/// Test suite trait
pub trait TestSuite {
    /// Run the test suite
    ///
    /// # Returns
    /// * `TestResult` - Test result
    fn run_tests(&self) -> TestResult;
}

/// Test result
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Number of tests passed
    pub passed: usize,
    /// Total number of tests
    pub total: usize,
}

impl TestResult {
    /// Create a new test result
    pub fn new() -> Self {
        Self {
            passed: 0,
            total: 0,
        }
    }
    
    /// Create a test result with values
    pub fn with_values(passed: usize, total: usize) -> Self {
        Self { passed, total }
    }
    
    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.passed == self.total
    }
    
    /// Get pass rate
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.passed as f64 / self.total as f64
        }
    }
}

impl Default for TestResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Test assertion macro
#[macro_export]
macro_rules! assert_test {
    ($condition:expr, $message:expr) => {
        if !($condition) {
            panic!("Test failed: {}", $message);
        }
    };
}

/// Test assertion with message macro
#[macro_export]
macro_rules! assert_eq_test {
    ($left:expr, $right:expr, $message:expr) => {
        if $left != $right {
            panic!("Test failed: {} (expected: {}, got: {})", $message, $right, $left);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_result() {
        let result = TestResult::new();
        assert_eq!(result.passed, 0);
        assert_eq!(result.total, 0);
        assert!(result.all_passed());
        assert_eq!(result.pass_rate(), 1.0);
        
        let result = TestResult::with_values(5, 10);
        assert_eq!(result.passed, 5);
        assert_eq!(result.total, 10);
        assert!(!result.all_passed());
        assert_eq!(result.pass_rate(), 0.5);
    }
}