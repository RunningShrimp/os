//! # Memory Management Module
//!
//! This module provides memory management functionality for the NOS kernel,
//! including both traditional memory management and virtualization support.

pub mod virt_mem;

pub use virt_mem::{
    EptContext, EptMemoryType, EptPermissions, EptPde, EptPdpte, EptPml4Entry, EptPte,
    EptTable, EptViolation, VirtMemError, VpmlTable,
};
