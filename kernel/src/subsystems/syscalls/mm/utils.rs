//! 内存管理工具函数

use crate::api::KernelError;

/// Extract arguments with validation
///
/// Extracts `n` arguments from the argument slice, returning an error if insufficient arguments are provided.
pub fn extract_args(args: &[u64], n: usize) -> Result<&[u64], KernelError> {
    if args.len() < n {
        return Err(KernelError::InvalidArgument);
    }
    Ok(&args[0..n])
}
