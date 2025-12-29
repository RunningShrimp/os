//! mm模块公共API边界
//!
//! 这些是mm模块对外暴露的唯一公共接口
//! 其他模块只能通过这些接口与mm模块交互

// Re-export all public API components
pub mod alloc;
pub mod page;
pub mod stats;
pub mod vm;

// Re-export types
pub use self::{error::*, types::*};

// Private submodules
mod error;
mod traits;
mod types;
