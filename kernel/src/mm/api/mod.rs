//! mm模块公共API边界
//! 
//! 这些是mm模块对外暴露的唯一公共接口
//! 其他模块只能通过这些接口与mm模块交互

// Re-export all public API components
pub mod alloc;
pub mod vm;
pub mod page;
pub mod stats;

// Re-export types
pub use self::error::*;
pub use self::types::*;

// Private submodules
mod error;
mod types;
mod traits;