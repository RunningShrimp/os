//! 内存管理系统调用处理模块
//!
//! 本模块包含内存管理相关系统调用的具体实现逻辑，包括：
//! - 内存映射和取消映射
//! - 内存保护操作
//! - 内存分配和释放
//! - 虚拟内存管理

pub mod mmap;
pub mod munmap;
pub mod mprotect;
pub mod madvise;
pub mod mlock;
pub mod brk;
pub mod shm;
pub mod numa;
pub mod utils;

// 重新导出所有公共接口
pub use mmap::*;
pub use munmap::*;
pub use mprotect::*;
pub use madvise::*;
pub use mlock::*;
pub use brk::*;
pub use shm::*;
pub use numa::*;
