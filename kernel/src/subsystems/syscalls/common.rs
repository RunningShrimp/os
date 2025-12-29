//! Common types and utilities for system calls

pub use crate::api::SyscallError;

/// Result type for system calls
pub type SyscallResult<T> = Result<T, SyscallError>;

/// System call arguments
pub type SyscallArgs = [u64; 6];

/// Extract system call arguments from the argument array
///
/// # Arguments
/// * `args` - Raw argument array from syscall interface
/// * `start` - Starting index to extract from
/// * `count` - Number of arguments to extract
///
/// # Returns
/// * `&[u64]` - Slice of arguments
pub fn extract_args(args: &[u64], start: usize, count: usize) -> &[u64] {
    &args[start..start + count.min(args.len() - start)]
}

/// Extract all arguments
pub fn extract_all_args(args: &[u64]) -> &[u64] {
    args
}