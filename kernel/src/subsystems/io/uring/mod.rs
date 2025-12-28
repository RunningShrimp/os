#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! io_uring High-Performance Async I/O
//!
//! This module implements io_uring for NOS:
//! - Submission and completion queues
//! - Fixed and ring buffers
//! - Async I/O operations
//! - io_uring context management
//! - Polled and interrupt-driven completion
//!
//! Features:
//! - io_uring v2 interface
//! - Multi-queue support
//! - Async file operations
//! - Async network operations
//! - Batch I/O
//! - Zero-copy transfers

pub mod queue;
pub mod buffers;
pub mod ops;
pub mod context;

// Re-export common io_uring types
pub use queue::*;
pub use buffers::*;
pub use ops::*;
pub use context::*;
