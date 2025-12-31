//! # 同步原语统一导出层
//!
//! 此模块重新导出 `kernel::subsystems::sync` 的所有同步原语，
//! 保持向后兼容性。
//!
//! ## 架构说明
//!
//! 为了消除循环依赖，同步原语的实现已统一到 `kernel::subsystems::sync` 模块。
//! 此 `kernel::sync` 模块作为兼容层，重新导出所有类型。
//!
//! ## 模块结构
//!
//! - **实现层**: `kernel::subsystems::sync` - 包含所有同步原语的实际实现
//! - **兼容层**: `kernel::sync` (此模块) - 重新导出，保持向后兼容
//!
//! ## 迁移指南
//!
//! 新代码应直接使用 `kernel::subsystems::sync` 中的类型：
//!
//! ```rust
//! // 旧方式（仍然支持）
//! use kernel::sync::SpinLock;
//!
//! // 新方式（推荐）
//! use kernel::subsystems::sync::SpinLock;
//! ```
//!
//! ## 可用的同步原语
//!
//! ### 基础锁
//! - [`SpinLock`]: 自旋锁，用于短期临界区
//! - [`Mutex`]: 互斥锁，可能阻塞
//! - [`RwLock`]: 读写锁，支持并发读
//! - [`Sleeplock`]: 睡眠锁，允许睡眠
//!
//! ### 中断控制
//! - [`SpinLockIrq`]: 禁用中断的自旋锁
//! - [`MutexIrq`]: 禁用中断的互斥锁
//!
//! ### 初始化原语
//! - [`Once`]: 单次初始化
//! - [`OnceLock`]: 带值的单次初始化
//! - [`Lazy`]: 惰性初始化
//!
//! ### 自适应锁
//! - [`AdaptiveSpinlock`]: 自适应自旋锁
//! - [`AdaptiveRwLock`]: 自适应读写锁
//!
//! ### 高级特性
//! - [`Rcu`]: 读-复制-更新机制
//! - [`PriorityMutex`]: 优先级互斥锁
//!
//! ## 相关模块
//!
//! - [`crate::subsystems::sync`]: 同步原语的主要实现

// ============================================================================
// Re-export all synchronization primitives from subsystems::sync
// ============================================================================

// Basic locks
pub use crate::subsystems::sync::{
    RawSpinLock,
    SpinLock,
    SpinLockGuard,
    SpinLockIrq,
    SpinLockIrqGuard,
    Mutex,
    MutexGuard,
    MutexIrq,
    MutexIrqGuard,
    RwLock,
    RwLockReadGuard,
    RwLockWriteGuard,
    Sleeplock,
    SleeplockGuard,
};

// Initialization primitives
pub use crate::subsystems::sync::{
    Once,
    OnceLock,
    Lazy,
};

// Adaptive locks
pub use crate::subsystems::sync::{
    AdaptiveConfig,
    AdaptiveSpinlock,
    AdaptiveSpinlockStats,
    AdaptiveRwLock,
    AdaptiveRwLockStats,
};

// Advanced features
pub use crate::subsystems::sync::{
    Rcu,
    PriorityMutex,
};

// Lock guard utilities
pub use crate::subsystems::sync::LockGuard;

// Interrupt control functions
pub use crate::subsystems::sync::{
    push_off,
    pop_off,
};

// Submodules (keep for backward compatibility)
pub mod adaptive_spinlock {
    pub use crate::subsystems::sync::adaptive_spinlock_legacy::*;
}

pub mod lock_guard {
    pub use crate::subsystems::sync::lock_guard_utils::*;
}

pub mod primitives {
    pub use crate::subsystems::sync::primitives::*;
}

pub mod rcu {
    pub use crate::subsystems::sync::rcu::*;
}

#[cfg(feature = "realtime")]
pub mod realtime {
    pub use crate::subsystems::sync::realtime::*;
}

// Test modules
#[cfg(feature = "kernel_tests")]
pub mod tests {
    pub use crate::subsystems::sync::tests::*;
}

#[cfg(feature = "kernel_tests")]
pub mod futex_tests {
    pub use crate::subsystems::sync::futex_tests::*;
}

// ============================================================================
// Legacy documentation (preserved for compatibility)
// ============================================================================

// Note: Example code for adaptive locks can be found in the
// adaptive_spinlock_legacy module documentation.
