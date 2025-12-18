// Copyright (c) 2024 NOS Community
// SPDX-License-Identifier: Apache-2.0

//! NOS Performance Optimization Crate
//!
//! This crate provides unified performance optimization functionality for NOS,
//! including cache management, performance monitoring, and optimization frameworks.

#![no_std]

extern crate alloc;

// Re-export core modules
pub mod cache;
pub mod core;
pub mod framework;
pub mod monitoring;
pub mod adaptive;

// Provide unified access to key components
pub use cache::{
    SyscallCache, SyscallCacheConfig, SyscallCacheKey, 
    CacheEntryFlags
};

pub use core::{
    UnifiedSyscallStats, CacheConfig, OptimizationConfig, 
    EvictionPolicy, UnifiedCache, CacheEntry
};

pub use framework::{
    OptimizationStrategy, OptimizationContext, OptimizationResult,
    OptimizationRecord, OptimizationType, OptimizationError,
    FastPathOptimization, CachingOptimization, BatchingOptimization,
    UnifiedOptimizationManager,
    init_global_optimization_manager, get_global_optimization_manager,
    check_and_apply_optimization, evaluate_optimization_effectiveness
};

pub use monitoring::{
    PerfStats, SystemPerformanceReport,
    get_perf_stats, record_syscall_performance,
    record_context_switch, record_interrupt, record_page_fault
};

pub use adaptive::{
    AdaptiveOptimizer, AdaptiveStrategy, AdaptiveOptimizationConfig,
    LoadLevel, LoadMonitorInfo,
    init_global_adaptive_optimizer, get_global_adaptive_optimizer,
    update_system_load, get_current_optimization_config, estimate_current_load_level
};
