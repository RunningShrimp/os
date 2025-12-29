//! API Layer Module
//!
//! This module contains the public API definitions for the kernel.
//! It provides trait definitions and interfaces that are used by
//! other modules to interact with kernel subsystems.
//!
//! The API layer separates the interface from the implementation,
//! allowing for better modularity and testability.

pub mod adapter;
pub mod context;
pub mod error;
pub mod interfaces;
pub mod memory;
pub mod process;
pub mod syscall;
pub mod syscall_id;

// Re-export common types for convenience
// 从adapter模块导出API适配器类型（优先级最高）
pub use adapter::*;

// 从syscall模块导出类型，但排除KernelError以避免与error模块冲突
pub use context::*;
// 从error模块导出所有类型，包括KernelError
pub use error::*;
pub use interfaces::*;
// 明确导出memory模块中的类型，避免与process模块中的MemoryRegion冲突
pub use memory::{
    AllocationFlags, MappingFlags, MemoryAdvice, MemoryError, MemoryManager, MemoryStats,
    PhysicalAddress, ProcessMemoryManager, ProcessMemoryUsage, ProtectionFlags, VirtualAddress,
};
pub use memory::{MemoryRegion, MemoryRegionType};
// 明确导出process模块中的类型，避免与memory模块中的MemoryRegion冲突
pub use process::{
    ExitStatus, FileDescriptor, FileDescriptorTable, MemoryMap, MemoryPermissions, Process,
    ProcessConfig, ProcessError, ProcessManager, ProcessState, ProcessStats, Thread, ThreadConfig,
    ThreadError, ThreadManager, ThreadState, WaitOptions,
};
// 分别从两个模块导出MemoryRegion和MemoryRegionType，并使用别名区分
pub use process::{
    MemoryRegion as ProcessMemoryRegion, MemoryRegionType as ProcessMemoryRegionType,
};
pub use syscall::{KernelErrorExt, SyscallError};
pub use syscall_id as kernel_syscall_id;
