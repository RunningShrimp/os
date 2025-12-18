//! NOS Test Suite
//!
//! This module provides a unified test suite for the NOS operating system.
//! It includes unit tests, integration tests, benchmarks, and compatibility tests.

pub mod unit;
pub mod integration;
pub mod compatibility;

/// Common test utilities
pub mod common;

/// Run all tests
///
/// This function runs all available test suites.
///
/// # Returns
/// * `TestResult` - Overall test result
pub fn run_all_tests() -> TestResult {
    println!("Running NOS test suite...");
    
    // Run unit tests
    println!("Running unit tests...");
    let unit_result = unit::run_unit_tests();
    
    // Run integration tests
    println!("Running integration tests...");
    let integration_result = integration::run_integration_tests();
    
    // Run compatibility tests
    println!("Running compatibility tests...");
    let compatibility_result = compatibility::run_compatibility_tests();
    
    // Combine results
    let overall_result = TestResult {
        unit_passed: unit_result.unit_passed,
        unit_total: unit_result.unit_total,
        integration_passed: integration_result.integration_passed,
        integration_total: integration_result.integration_total,
        compatibility_passed: compatibility_result.compatibility_passed,
        compatibility_total: compatibility_result.compatibility_total,
    };
    
    // Print summary
    println!("Test Summary:");
    println!("  Unit tests: {}/{}", overall_result.unit_passed, overall_result.unit_total);
    println!("  Integration tests: {}/{}", overall_result.integration_passed, overall_result.integration_total);
    println!("  Compatibility tests: {}/{}", overall_result.compatibility_passed, overall_result.compatibility_total);
    
    let total_passed = overall_result.unit_passed + overall_result.integration_passed + 
                     overall_result.compatibility_passed;
    let total_tests = overall_result.unit_total + overall_result.integration_total + 
                    overall_result.compatibility_total;
    
    println!("  Overall: {}/{} tests passed ({:.1}%)", 
             total_passed, total_tests, 
             (total_passed as f64 / total_tests as f64) * 100.0);
    
    overall_result
}

/// Test result
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Number of unit tests passed
    pub unit_passed: usize,
    /// Total number of unit tests
    pub unit_total: usize,
    /// Number of integration tests passed
    pub integration_passed: usize,
    /// Total number of integration tests
    pub integration_total: usize,
    /// Number of compatibility tests passed
    pub compatibility_passed: usize,
    /// Total number of compatibility tests
    pub compatibility_total: usize,
}

impl TestResult {
    /// Create a new test result
    pub fn new() -> Self {
        Self {
            unit_passed: 0,
            unit_total: 0,
            integration_passed: 0,
            integration_total: 0,
            compatibility_passed: 0,
            compatibility_total: 0,
        }
    }
    
    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.unit_passed == self.unit_total &&
        self.integration_passed == self.integration_total &&
        self.compatibility_passed == self.compatibility_total
    }
}

impl Default for TestResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_result() {
        let result = TestResult::new();
        assert_eq!(result.unit_passed, 0);
        assert_eq!(result.unit_total, 0);
        assert!(result.all_passed());
    }
}