#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! Container Support
//!
//! This module implements container support for NOS:
//! - Container namespace isolation
//! - cgroup resource limits
//! - Container lifecycle management
//! - Container networking
//! - Container storage
//!
//! Features:
//! - Multiple namespace types (mount, net, pid, etc.)
//! - Cgroup v2 support
//! - Container image management
//! - Container runtime
//! - Container statistics

pub mod namespace;
pub mod cgroup;
pub mod runtime;
pub mod image;
pub mod networking;
pub mod storage;

// Re-export common container types
pub use namespace::*;
pub use cgroup::*;
pub use runtime::*;
pub use image::*;
pub use networking::*;
pub use storage::*;
