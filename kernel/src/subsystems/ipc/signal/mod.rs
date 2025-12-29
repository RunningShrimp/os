//! Signal Module
//!
//! Provides signal-related functionality

pub mod types;

pub use types::SignalState;

/// Signal notification values
pub const SIGEV_SIGNAL: i32 = 0;
pub const SIGEV_NONE: i32 = 1;
