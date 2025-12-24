//! Architecture abstraction layer
//!
//! This module provides architecture-specific implementations and abstractions
//! for different CPU architectures, allowing to kernel to support multiple
//! architectures with a unified interface.

pub mod memory_layout;
pub mod kpti;
pub mod retpoline;

