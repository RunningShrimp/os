//! Atomic Operation Optimization

use core::fmt;

/// Errors from atomic operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicError {
    CasFailed,
    InvalidOrdering,
    NullPointer,
    AllocationFailed,
    NotSupported,
    ABAProblem,
    InvalidAlignment,
}

impl fmt::Display for AtomicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomicError::CasFailed => write!(f, "CAS operation failed"),
            AtomicError::InvalidOrdering => write!(f, "Invalid memory ordering"),
            AtomicError::NullPointer => write!(f, "Null pointer"),
            AtomicError::AllocationFailed => write!(f, "Allocation failed"),
            AtomicError::NotSupported => write!(f, "Operation not supported"),
            AtomicError::ABAProblem => write!(f, "ABA problem detected"),
            AtomicError::InvalidAlignment => write!(f, "Invalid alignment"),
        }
    }
}

pub type AtomicResult<T> = core::result::Result<T, AtomicError>;
