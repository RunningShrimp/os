//! Memory Management Module
//!
//! Re-exports memory management functionality from subsystems::mm
//! and virtualization memory management from the mm module directory

// Re-export traditional memory management from subsystems
pub use crate::subsystems::mm::*;

// Re-export virtualization memory management from mm module
pub use crate::mm::virt_mem::*;
