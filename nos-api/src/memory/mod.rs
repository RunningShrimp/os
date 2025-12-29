//! Memory management module

pub mod interface;
pub mod types;

// Re-export commonly used items
pub use interface::*;
// Re-export types from interface to avoid ambiguous glob re-exports
pub use interface::{MemoryAllocation, MemoryErrors, MemoryFragmentation, MemoryUsage};
// Explicitly re-export types to avoid ambiguous glob re-exports
pub use types::MemoryRegion;
