//! Test Module
//!
//! Provides testing utilities and benchmarking framework

pub mod benchmark_final;
pub mod integration;

// Re-export common testing types
pub use integration::{IntegrationTestResult, TestStep};

/// Test result type
pub type TestResult = Result<(), String>;
