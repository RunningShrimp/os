//! Thread management for kernel
//!
//! Implements threading support that extends the existing process infrastructure.
//! Provides kernel threads, POSIX threads (pthreads), and thread scheduling
//! integration with the existing process scheduler.

pub mod types;
pub mod thread_impl;
pub mod table;
pub mod scheduling;
pub mod api;

// Re-export all public types
pub use types::*;
pub use types::Thread;
pub use table::{ThreadTable, THREAD_TABLE};
pub use scheduling::*;
pub use api::*;
pub use api::ThreadError;
