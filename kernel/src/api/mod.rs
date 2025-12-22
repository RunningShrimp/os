//! API Layer Module
//!
//! This module contains the public API definitions for the kernel.
//! It provides trait definitions and interfaces that are used by
//! other modules to interact with kernel subsystems.
//!
//! The API layer separates the interface from the implementation,
//! allowing for better modularity and testability.

pub mod syscall;
pub mod process;
pub mod memory;
pub mod error;
pub mod context;
pub mod interfaces;

// Re-export common types for convenience
// 从syscall模块导出类型，但排除KernelError以避免与error模块冲突
pub use syscall::{SyscallError, KernelErrorExt};
// 明确导出process模块中的类型，避免与memory模块中的MemoryRegion冲突
pub use process::{ProcessManager, Process, ThreadManager, Thread, ProcessState, ThreadState, ProcessConfig, ThreadConfig, WaitOptions, ExitStatus, ProcessStats, FileDescriptorTable, FileDescriptor, MemoryMap, MemoryPermissions, ProcessError, ThreadError};
// 明确导出memory模块中的类型，避免与process模块中的MemoryRegion冲突
pub use memory::{MemoryManager, ProcessMemoryManager, PhysicalAddress, VirtualAddress, AllocationFlags, MappingFlags, ProtectionFlags, MemoryAdvice, MemoryStats, MemoryError, ProcessMemoryUsage};
// 分别从两个模块导出MemoryRegion和MemoryRegionType，并使用别名区分
pub use process::{MemoryRegion as ProcessMemoryRegion, MemoryRegionType as ProcessMemoryRegionType};
pub use memory::{MemoryRegion as MemoryRegion, MemoryRegionType as MemoryRegionType};
// 从error模块导出所有类型，包括KernelError
pub use error::*;
pub use context::*;
pub use interfaces::*;