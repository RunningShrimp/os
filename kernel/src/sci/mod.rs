//! # Scientific Computing Framework
//!
//! A comprehensive scientific computing library providing:
//! - Numerical optimization algorithms
//! - Linear algebra operations
//! - FFT and signal processing
//! - ODE and PDE solvers
//! - Parallel computing framework
//!
//! ## Features
//! - High-precision numerical computing
//! - SIMD optimizations
//! - Parallel computation support
//! - Standard library compatibility
//! - Comprehensive test coverage

// Import Vec from alloc for no_std compatibility
extern crate alloc;

pub mod optimization;
pub mod linalg;
pub mod fft;
pub mod ode;
pub mod pde;
pub mod parallel;

// Re-export Vec for use in submodules
pub use alloc::vec::Vec;

pub use optimization::*;
pub use linalg::*;
pub use fft::*;
pub use ode::*;
pub use pde::*;
pub use parallel::*;

/// Scientific computing framework version
pub const SCI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default precision for floating-point operations
pub type SciFloat = f64;

/// Default precision for complex numbers
pub type SciComplex = num_complex::Complex<SciFloat>;

/// Result type for scientific computing operations
pub type SciResult<T> = core::result::Result<T, SciError>;

/// Errors in scientific computing operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SciError {
    /// Matrix is singular or near-singular
    SingularMatrix,
    /// Iteration did not converge
    NotConverged,
    /// Invalid input dimensions
    InvalidDimensions,
    /// Numerical overflow or underflow
    NumericalError,
    /// Invalid parameters
    InvalidParameters,
    /// Index out of bounds
    IndexOutOfBounds,
}

impl core::fmt::Display for SciError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SciError::SingularMatrix => write!(f, "Matrix is singular or near-singular"),
            SciError::NotConverged => write!(f, "Iteration did not converge"),
            SciError::InvalidDimensions => write!(f, "Invalid input dimensions"),
            SciError::NumericalError => write!(f, "Numerical overflow or underflow"),
            SciError::InvalidParameters => write!(f, "Invalid parameters"),
            SciError::IndexOutOfBounds => write!(f, "Index out of bounds"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", SciError::SingularMatrix),
            "Matrix is singular or near-singular"
        );
        assert_eq!(
            format!("{}", SciError::NotConverged),
            "Iteration did not converge"
        );
    }
}
