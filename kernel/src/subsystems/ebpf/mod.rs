//! Extended Berkeley Packet Filter (eBPF) Support
//!
//! This module implements full eBPF support for NOS:
//! - eBPF virtual machine
//! - eBPF instruction set
//! - eBPF program loading and execution
//! - eBPF JIT compiler
//! - eBPF hooks and event system
//! - eBPF helper functions
//!
//! Features:
//! - Full eBPF ISA (64-bit)
//! - Kernel-space eBPF execution
//! - User-space eBPF program loading
//! - eBPF maps (per-CPU and hash)
//! - eBPF hooks for system calls, networking, etc.
//! - eBPF helper functions (kernel API for eBPF programs)
//! - eBPF JIT compilation to native code

pub mod vm;
pub mod instructions;
pub mod loader;
pub mod hooks;
pub mod helpers;
pub mod maps;

// Re-export common eBPF types
pub use vm::*;
pub use instructions::*;
pub use loader::*;
pub use hooks::*;
pub use helpers::*;
pub use maps::*;
