//! Common types and utilities for syscall handlers
//!
//! This module provides shared types and utilities used across all
//! syscall handlers.

use alloc::{string::String, vec::Vec};
use crate::error_handling::unified::KernelError;
use crate::vfs::error::VfsError;


