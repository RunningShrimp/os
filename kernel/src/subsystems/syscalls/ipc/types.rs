//! IPC Type Definitions
//!
//! Common types for IPC operations

/// Message queue flags
pub const IPC_CREAT: i32 = 0o1000;
pub const IPC_EXCL: i32 = 0o2000;

/// IPC result type
pub type IpcResult<T> = nos_api::Result<T>;
