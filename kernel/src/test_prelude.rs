//! Test prelude - provides common imports for test modules
//!
//! This module re-exports all test macros and helper functions
//! that are commonly needed across test modules.

// Re-export test macros at module level for use in tests
pub use crate::test_assert;
pub use crate::test_assert_eq;
pub use crate::test_assert_ne;
pub use crate::test_err;
pub use crate::test_ok;
pub use crate::test_macros::TestResult;
