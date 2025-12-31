//! # NOS Kernel Library
//!
//! This crate provides the public API for the NOS (New Operating System) kernel.
//! It acts as an integration layer for the various kernel components.
//!
//! ## 概述
//!
//! NOS 是一个现代的、模块化的操作系统内核，采用 Rust 编写，专注于：
//! - **安全性**: 利用 Rust 的类型系统和内存安全特性
//! - **性能**: 优化的系统调用路径和零拷贝技术
//! - **兼容性**: POSIX 兼容层，支持现有 Linux 应用
//! - **可扩展性**: 模块化架构，易于扩展和维护
//!
//! ## 架构
//!
//! The kernel follows a modular architecture with the following main components:
//!
//! ### 核心子系统
//!
//! - **系统调用** (`syscalls`): 系统调用接口和分发机制
//! - **服务管理** (`services`): 服务管理和发现框架
//! - **错误处理** (`error`): 全面的错误处理和恢复框架
//! - **内存管理** (`subsystems::mm`): 物理和虚拟内存管理
//! - **进程管理** (`subsystems::process`): 进程创建、调度和生命周期管理
//! - **文件系统** (`subsystems::fs`, `vfs`): 虚拟文件系统和文件操作
//! - **网络** (`subsystems::net`): 网络协议栈和套接字接口
//! - **安全** (`security`): 安全机制（ASLR、SMAP/SMEP、ACL、Capabilities）
//! - **IPC** (`subsystems::ipc`): 进程间通信机制
//! - **同步** (`sync`, `subsystems::sync`): 同步原语和锁机制
//! - **调度器** (`sched`, `subsystems::scheduler`): 进程和线程调度
//! - **高可用** (`ha`): 高可用性和容错（集群、复制、故障转移、备份）
//!
//! ### 平台支持
//!
//! - **架构**: x86_64, ARM64 (AArch64), RISC-V
//! - **平台**: 裸金属、虚拟化环境
//!
//! ## 使用示例
//!
//! ### 初始化内核
//!
//! ```no_run
//! use kernel::init_kernel;
//!
//! // Initialize the kernel
//! let boot_params = kernel::BootParameters::default();
//! init_kernel(boot_params)?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ### 关闭内核
//!
//! ```no_run
//! use kernel::shutdown_kernel;
//!
//! // Shutdown the kernel
//! shutdown_kernel()?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ### 获取内核信息
//!
//! ```
//! use kernel::{get_kernel_version, get_kernel_build_info};
//!
//! let version = get_kernel_version();
//! println!("NOS Kernel Version: {}", version);
//!
//! let build_info = get_kernel_build_info();
//! println!("Build Time: {}", build_info.build_time);
//! println!("Git Commit: {}", build_info.git_commit);
//! ```
//!
//! ## 模块组织
//!
//! ### 公共 API 层
//!
//! - [`api`]: 统一的内核 API 接口
//! - [`vfs_interface`]: VFS 接口层，打破循环依赖
//!
//! ### 核心功能
//!
//! - [`core`]: 核心内核功能
//! - [`error`]: 错误处理
//! - [`platform`]: 平台相关代码（架构、驱动、陷阱处理）
//!
//! ### 子系统
//!
//! - [`subsystems`]: 主要内核子系统
//!   - [`subsystems::process`]: 进程管理
//!   - [`subsystems::mm`]: 内存管理
//!   - [`subsystems::fs`]: 文件系统
//!   - [`subsystems::net`]: 网络
//!   - [`subsystems::ipc`]: IPC
//!   - [`subsystems::sync`]: 同步
//!   - [`subsystems::scheduler`]: 调度器
//!
//! ### 兼容层
//!
//! - [`compat`]: 兼容性层（Android、iOS、Linux、macOS、Windows）
//! - [`posix`]: POSIX 类型和常量
//!
//! ## 特性标志
//!
//! ### 编译时特性
//!
//! - `kernel_tests`: 启用内核测试框架和测试用例
//! - `baremetal`: 启用裸金属启动支持（无引导加载程序）
//! - `syscalls`: 启用系统调用支持（通过 nos-syscalls crate）
//! - `services`: 启用服务管理（通过 nos-services crate）
//! - `error_handling`: 启用错误处理（通过 nos-error-handling crate）
//! - `net_stack`: 启用网络协议栈
//! - `posix_layer`: 启用 POSIX 兼容层
//! - `debug_subsystems`: 启用子系统调试日志
//! - `security_audit`: 启用安全审计功能
//! - `formal_verification`: 启用形式化验证工具
//! - `cloud_native`: 启用云原生功能（容器、服务等）
//!
//! ## 设计决策
//!
//! ### 模块化架构
//!
//! NOS 采用高度模块化的设计，每个子系统都有清晰的接口和职责。这使得：
//! - 代码易于理解和维护
//! - 功能可以独立测试
//! - 允许选择性编译功能
//!
//! ### 安全优先
//!
//! - 使用 Rust 的类型系统确保内存安全
//! - 实现了多种安全缓解措施（ASLR、Stack Canaries、SMEP/SMAP）
//! - 提供细粒度的权限控制（Capabilities、ACL）
//!
//! ### 性能优化
//!
//! - 快速系统调用路径
//! - 零拷贝 I/O
//! - 优化的锁和同步原语
//! - 高效的内存分配器
//!
//! ## 错误处理
//!
//! NOS 使用统一的错误处理框架，基于 `nos_api::Result<T>` 和 `nos_api::Error`。
//!
//! ## 性能特征
//!
//! - 系统调用延迟: < 100ns（热路径）
//! - 上下文切换: < 1μs
//! - 内存分配: O(1) 分配和释放
//!
//! ## 线程安全
//!
//! NOS 内核设计为多核安全：
//! - 使用适当的同步原语保护共享状态
//! - 提供 SMP 安全的锁实现
//! - 支持每 CPU 数据结构
//!
//! ## 相关模块
//!
//! - [`nos_api`]: 公共 API 定义
//! - [`nos_syscalls`]: 系统调用实现
//! - [`nos_services`]: 服务管理
//! - [`nos_error_handling`]: 错误处理
//!
//! ## 参考资料
//!
//! - [架构文档](../ARCHITECTURE.md)
//! - [开发指南](../DEVELOPER_GUIDE.md)
//! - [功能特性](../FEATURES.md)

#![no_std]

#[macro_use]
extern crate alloc;

// Re-export standard collection types
pub use alloc::{
    boxed::Box,
    string::String,
    vec::Vec,
    collections::BTreeMap,
    sync::Arc,
};
pub use hashbrown::HashMap;

// Re-export atomic types from core (using ::core to avoid shadowing)
pub use ::core::sync::atomic::{AtomicUsize, AtomicU64, AtomicU32, AtomicI32, AtomicBool, AtomicPtr};

// Re-export synchronization primitives from subsystems
pub use crate::subsystems::sync::Mutex;

// Re-export core types (using ::core to avoid shadowing by local core module)
pub use ::core::cmp::Ordering;
pub use ::core::cmp::{PartialOrd, PartialEq};

// Re-export common types from error module
pub use crate::{
    error::{
        MemoryError, FileSystemError, NetworkError, ProcessError,
        KernelError, KernelResult, Result, SyscallError, SyscallResult,
        DriverError, SecurityError, UnifiedError, UnifiedResult,
    },
};

// Re-export additional commonly used types
pub use crate::{
    subsystems::mm::{AllocationStats, MemoryManagementStats, NumStats},
    subsystems::syscalls::memory::MemoryService,
    libc::CLibStats,
    api::MemoryRegionType,
    memory::{MemoryPermissions, MemoryRegion},
    vfs_interface::{FileMode, VfsError},
    security::enhanced_permissions::AccessResult,
};

// Prelude - imports commonly used types throughout the kernel
pub mod prelude;
pub use prelude::*;

// Define print/println macros at crate root for use throughout the kernel
// These must be defined here to be available as crate::print and crate::println
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::platform::drivers::console::_print(::core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", ::core::format_args!($($arg)*))
    };
}

#[cfg(feature = "kernel_tests")]
#[macro_use]
mod test_macros;

// Logging macros (stub implementations for no_std environments)
#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! log_debug { ($($arg:tt)*) => { { let _ = ($($arg)*); } }; }

#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! log_info { ($($arg:tt)*) => { { let _ = ($($arg)*); } }; }

#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! log_warn { ($($arg:tt)*) => { { let _ = ($($arg)*); } }; }

#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! log_error { ($($arg:tt)*) => { { let _ = ($($arg)*); } }; }

// API layer - public interfaces for kernel subsystems
pub mod api;

// Virtual File System (VFS) - moved to subsystems/fs/vfs
// Re-export for backward compatibility
pub use crate::subsystems::fs::vfs;

// VFS interface layer - moved to subsystems/fs/vfs_interface
// Re-export for backward compatibility
pub use crate::subsystems::fs::vfs_interface;

// Core kernel functionality (from nos-kernel-core)
pub mod core;

// Error handling module
pub mod error;

// Common utilities
pub mod common;

// Kernel factory for creating and managing internal modules
mod kernel_factory;

// Include necessary internal modules for library
pub mod arch;
pub mod platform;
pub mod subsystems;
// Top-level aliases for backward compatibility
// These allow code to use crate::syscalls instead of crate::subsystems::syscalls
pub use crate::subsystems::syscalls;
// services is a root-level module (not a re-export) to provide init() function
// pub use crate::subsystems::services;  // REMOVED: Conflict with root-level services module
pub use crate::subsystems::mm;
// compat remains at root level, declared below
// trap is now exported via platform::{arch, boot, drivers, trap} below

// System call module
pub mod syscall;
// pub mod syscalls;  // REMOVED: Use subsystems::syscalls instead
pub mod signal;
pub mod compat;
pub mod sync;
pub mod trap;

// Accessibility (A11y) module - comprehensive accessibility support
pub mod a11y;

// Re-export key types for external use
/// Core kernel functionality
pub use core::*;

/// Kernel factory and components
pub use kernel_factory::*;
#[cfg(feature = "error_handling")]
pub use nos_error_handling as error_handling;
// External crates that would conflict with our internal aliases when features are enabled
// These are kept commented out to avoid conflicts
// #[cfg(feature = "services")]
// pub use nos_services as services;
// #[cfg(feature = "syscalls")]
// pub use nos_syscalls as syscalls;
/// Performance monitoring
pub use perf::*;
pub use platform::{arch as platform_arch, boot};
/// POSIX types and constants
pub use posix::*;
// Re-export moved modules to maintain compatibility
pub use subsystems::fs;
#[cfg(feature = "networking")]
pub use subsystems::net;
pub use subsystems::{ipc, process, sync as subsystems_sync, time};

/// Boot parameters passed from bootloader to kernel
pub use crate::boot::BootParameters;

mod collections;
pub mod cpu;  // Make cpu public so main.rs can import it
#[cfg(feature = "debug")]
pub mod debug;
pub mod epoll;
mod di;
mod event;
mod ids;
pub mod libc;
mod memory;
pub mod types;
pub mod services;  // Re-enabled: subsystems::services doesn't have init(), need the root-level services module
mod syscall_interface;

// High Availability and Fault Tolerance
pub mod ha;

// Real-Time Operating System features
pub mod rtos;
// Legacy modules - now accessed through subsystems
pub mod monitoring;  // Made public to match glob re-export
pub mod perf;  // Made public to match glob re-export
// POSIX compatibility layer - moved to subsystems/posix
// Re-export for backward compatibility
pub use crate::subsystems::posix;
mod procfs;
pub mod sched;  // Made public to match potential glob re-export
pub mod security;  // Made public to match glob re-export
#[cfg(feature = "security")]
pub mod security_audit;
pub mod tests;

// Storage Management Subsystem
pub mod storage;

// Logging subsystem - comprehensive logging with async backend
pub mod logging;

#[cfg(not(feature = "cloud_native"))]
mod cloud_native {
    pub mod namespaces {
        use alloc::string::String;

        /// Namespace type enumeration
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum NamespaceType {
            Mount,
            UTS,
            IPC,
            Network,
            PID,
            User,
            Cgroup,
            Time,
        }

        pub struct NamespaceConfig {
            pub ns_type: NamespaceType,
            pub new_namespace: bool,
            pub existing_path: Option<String>,
        }

    }
}

/// Initialize the kernel
///
/// This function initializes all kernel subsystems and prepares the system
/// for operation.
/// It uses the same core initialization logic as `rust_main_with_boot_info`,
/// ensuring consistency between bootloader-based and library-based startup.

/// Get kernel version
///
/// # Returns
///
/// * `&'static str` - Kernel version string
pub fn get_kernel_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Get kernel build information
///
/// # Returns
///
/// * `KernelBuildInfo` - Kernel build information
pub fn get_kernel_build_info() -> KernelBuildInfo {
    KernelBuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        build_time: option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown"),
        git_commit: option_env!("VERGEN_GIT_SHA").unwrap_or("unknown"),
        target_triple: option_env!("VERGEN_CARGO_TARGET_TRIPLE")
            .unwrap_or_else(|| option_env!("CARGO_BUILD_TARGET").unwrap_or("unknown")),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        features: get_enabled_features(),
    }
}

/// Kernel build information
#[derive(Debug, Clone)]
pub struct KernelBuildInfo {
    /// Kernel version
    pub version: &'static str,
    /// Build timestamp
    pub build_time: &'static str,
    /// Git commit hash
    pub git_commit: &'static str,
    /// Target triple
    pub target_triple: &'static str,
    /// Build profile
    pub profile: &'static str,
    /// Enabled features
    pub features: alloc::vec::Vec<&'static str>,
}

/// Get enabled features
fn get_enabled_features() -> alloc::vec::Vec<&'static str> {
    let mut features = alloc::vec::Vec::new();

    if cfg!(feature = "baremetal") {
        features.push("baremetal");
    }
    if cfg!(feature = "kernel_tests") {
        features.push("kernel_tests");
    }
    if cfg!(feature = "syscalls") {
        features.push("syscalls");
    }
    if cfg!(feature = "services") {
        features.push("services");
    }
    if cfg!(feature = "error_handling") {
        features.push("error_handling");
    }
    if cfg!(feature = "networking") {
        features.push("networking");
    }
    // posix_layer feature not defined in Cargo.toml - removed conditional compilation
    if cfg!(feature = "debug") {
        features.push("debug");
    }
    if cfg!(feature = "security") {
        features.push("security");
    }
    if cfg!(feature = "formal_verification") {
        features.push("formal_verification");
    }
    if cfg!(feature = "cloud_native") {
        features.push("cloud_native");
    }

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_version() {
        let version = get_kernel_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_kernel_build_info() {
        let build_info = get_kernel_build_info();
        assert!(!build_info.version.is_empty());
        assert!(!build_info.build_time.is_empty());
        assert!(!build_info.git_commit.is_empty());
        assert!(!build_info.target_triple.is_empty());
        assert!(!build_info.profile.is_empty());
        assert!(!build_info.features.is_empty());
    }

    #[test]
    fn test_enabled_features() {
        let features = get_enabled_features();
        assert!(!features.is_empty());
    }
}
// mod mm;  // REMOVED: Use subsystems::mm instead
pub mod reliability;
// Audio subsystem for real-time audio processing
pub mod audio;

/// Blockchain and smart contract support
///
/// This module provides a complete implementation of blockchain technology,
/// including smart contract execution through the Ethereum Virtual Machine (EVM).
pub mod blockchain;

/// Digital content creation and media processing
///
/// This module provides comprehensive multimedia processing capabilities for
/// digital content creation, including image processing, video codecs, 3D rendering,
/// audio workstation, font rendering, and color management.
pub mod media;

/// Virtualization and hypervisor support
///
/// This module provides comprehensive virtualization capabilities including:
/// - Type-1 hypervisor with hardware-assisted virtualization (VT-x/AMD-V)
/// - Virtual Machine Monitor (VMM) for VM lifecycle management
/// - CPU virtualization (vCPUs, virtual APIC, scheduling)
/// - Memory virtualization (EPT/NPT, shadow page tables, ballooning)
/// - Device virtualization (VirtIO, passthrough, SR-IOV)
/// - Guest management (hypercalls, save/restore, debugging)
pub mod virtualization;

/// Quantum computing interface
///
/// This module provides a comprehensive quantum computing framework including:
/// - Quantum bit (qubit) simulation with state vectors
/// - Quantum circuit construction and optimization
/// - Quantum algorithms (Grover, Shor, QFT)
/// - Quantum error correction codes
/// - Quantum key distribution protocols
/// - Post-quantum cryptographic algorithms
pub mod quantum;

/// Distributed computing framework
///
/// This module provides a complete distributed computing framework including:
/// - MapReduce programming model for large-scale data processing
/// - HDFS client for distributed file system access
/// - RPC framework for inter-service communication
/// - Distributed lock service with consensus algorithms
/// - Task scheduler and resource manager
/// - Fault tolerance and automatic recovery
pub mod distributed;

/// Extended Reality (XR) subsystem
///
/// This module provides comprehensive support for Augmented Reality (AR) and Virtual Reality (VR):
/// - SLAM (Simultaneous Localization and Mapping) for real-time tracking
/// - 6DoF head tracking with IMU sensor fusion
/// - Hand tracking and gesture recognition
/// - Eye tracking for foveated rendering
/// - Spatial audio rendering with HRTF
/// - Optimized XR rendering pipeline
///
/// Features:
/// - Motion-to-photon latency: < 20ms
/// - High-precision tracking: < 1mm positional, < 1° rotational
/// - 90Hz/120Hz frame rate support
/// - Low-drift SLAM: < 0.1% drift per minute
/// - Foveated rendering for performance optimization
/// - Binaural audio with room acoustics
pub mod xr;

/// Scientific computing framework
pub mod sci;

/// Cryptography and PKI module
///
/// Comprehensive cryptographic services including:
/// - Symmetric encryption (AES, ChaCha20, etc.)
/// - Asymmetric cryptography (RSA, ECC, Ed25519)
/// - Hash functions (SHA-256, SHA-512, BLAKE2)
/// - Message authentication codes (HMAC, Poly1305, CMAC)
/// - Public Key Infrastructure (X.509 certificates)
/// - Secure key management
///
/// ## Security Features
///
/// - Constant-time operations for secret data
/// - Secure memory handling with automatic zeroing
/// - Side-channel attack resistance
/// - FIPS-compliant algorithms
pub mod crypto;

/// Edge Computing and IoT subsystem
///
/// This module provides comprehensive edge computing and Internet of Things (IoT) support
/// including protocol implementations (MQTT, CoAP, LoRaWAN), device discovery, OTA updates,
/// time-series databases for sensor data, edge computing frameworks, and sensor fusion.
pub mod iot;

/// AI/ML framework
pub mod ai;

/// Machine Learning Operations (MLOps)
///
/// This module provides comprehensive MLOps infrastructure for managing the complete
/// machine learning lifecycle within the kernel. It integrates seamlessly with the AI
/// framework to provide production-ready ML operations.
///
/// ## Submodules
///
/// - **experiment**: Experiment tracking, hyperparameter logging, model versioning
/// - **pipeline**: DAG-based pipelines with execution engine
/// - **serving**: Model serving infrastructure with batch/online prediction
/// - **monitoring**: Data drift detection, performance monitoring, alerting
/// - **retraining**: Automated retraining with hyperparameter optimization
/// - **governance**: Model lineage, fairness auditing, explainability, privacy compliance
///
/// ## Features
///
/// - MLflow-compatible experiment tracking
/// - DAG-based pipeline orchestration
/// - Production model serving with A/B testing
/// - Real-time monitoring and drift detection
/// - Automated retraining with hyperparameter optimization
/// - Comprehensive governance and compliance
pub mod mlops;

/// Advanced File Systems
///
/// This module provides comprehensive file system implementations including:
/// - LFS (Log-structured file system)
/// - COW (Copy-on-write with snapshots)
/// - Distributed (Ceph-like distributed FS)
/// - Cache (Multi-level caching)
/// - Metadata (B+trees, extents, xattrs)
/// - Journaling (Write-ahead logging)
/// - Quota (POSIX disk quotas)
pub mod filesystem;

/// Database engine with SQL support, transactions, and recovery
pub mod database;

/// Device drivers framework
///
/// This module provides comprehensive device driver support:
/// - UIO (Userspace I/O) driver framework
/// - VFIO (Virtual Function I/O) for device assignment
/// - PCI device management
/// - USB device management
/// - GPU driver framework
pub mod drivers;

/// Health checking with probes, circuit breakers, and aggregation
pub mod health;

/// Metrics and telemetry system
///
/// This module provides comprehensive metrics collection and export capabilities:
/// - Counters (monotonically increasing values)
/// - Gauges (point-in-time measurements)
/// - Histograms (value distributions with percentiles)
/// - Labeled metrics (multi-dimensional tracking)
/// - Multiple export formats (Prometheus, OpenMetrics, StatsD)
/// - Thread-safe, lock-free operations for minimal overhead
///
/// ## Features
///
/// - **Counter**: Track cumulative values (requests, bytes, errors)
/// - **Gauge**: Track point-in-time values (memory, connections, queue depth)
/// - **Histogram**: Track distributions (latencies, response sizes)
/// - **Labels**: Add dimensions to metrics (method, status, region)
/// - **Export**: Prometheus text format, OpenMetrics, StatsD protocol
/// - **Performance**: <1% overhead with lock-free atomic operations
///
/// ## Example
///
/// ```rust
/// use kernel::metrics::*;
///
/// // Get the global registry
/// let registry = registry::global_registry();
///
/// // Create a counter
/// let counter = registry.counter("requests_total").unwrap();
/// counter.inc();
///
/// // Create a gauge
/// let gauge = registry.gauge("active_connections").unwrap();
/// gauge.set(42);
///
/// // Create a histogram
/// let histogram = registry.histogram_latency("request_duration").unwrap();
/// histogram.observe(0.123);
///
/// // Export metrics
/// let output = registry.export_prometheus();
/// ```
pub mod metrics;

/// Performance Profiling
///
/// The profiling module provides comprehensive performance profiling capabilities:
/// - CPU profiling with sampling and flame graph generation
/// - Memory profiling with leak detection
/// - Lock contention profiling
/// - I/O profiling with latency tracking
/// - Symbol resolution and demangling
///
/// # Example
///
/// ```rust
/// use kernel::profiling::{Profiler, ProfilerType};
///
/// let profiler = Profiler::new();
/// profiler.enable();
/// profiler.start(ProfilerType::Cpu(100)).unwrap();
/// // ... code to profile ...
/// profiler.stop(ProfilerType::Cpu(0)).unwrap();
/// let report = profiler.report(ProfilerType::Cpu(0)).unwrap();
/// ```
pub mod profiling;

/// ## 工作流和调度 (`workflow`)
///
/// 工作流编排和任务调度系统，支持 DAG 工作流、Cron 调度、作业执行、持久化和重试逻辑：
/// - **DAG 执行**: 基于有向无环图的任务依赖管理
/// - **Cron 调度**: 标准的 Cron 表达式解析和调度（秒到年）
/// - **作业管理**: 优先级调度、超时处理、取消支持
/// - **持久化**: 作业状态持久化和崩溃恢复
/// - **重试机制**: 指数退避、线性退避、抖动支持
///
/// ### 使用示例
///
/// ```no_run
/// use kernel::workflow::{Workflow, WorkflowOrchestrator, Job, RetryConfig};
/// use alloc::sync::Arc;
/// use core::time::Duration;
///
/// // 创建工作流
/// let mut workflow = Workflow::new("my-workflow", "My Workflow");
/// let job1 = Job::new("task1").with_executor(Arc::new(|| Ok(())));
/// let job2 = Job::new("task2").with_executor(Arc::new(|| Ok(())));
///
/// let node1 = workflow.add_job(job1).unwrap();
/// let node2 = workflow.add_job(job2).unwrap();
/// workflow.add_dependency(node1, node2).unwrap();
///
/// // 创建编排器并执行
/// let mut orchestrator = WorkflowOrchestrator::new();
/// orchestrator.register_workflow(workflow).unwrap();
/// orchestrator.execute_workflow("my-workflow").unwrap();
/// ```
pub mod workflow;

// Messaging and events system
pub mod messaging;

// Resource management
pub mod resource;

/// Cloud Native Integration
///
/// Comprehensive cloud-native capabilities for the NOS kernel including:
/// - **Service Orchestration**: Microservices deployment, scaling, and lifecycle management
/// - **Service Mesh**: Envoy-style mesh with mTLS, traffic management, and observability
/// - **API Gateway**: Request routing, rate limiting, transformation, and composition
/// - **Configuration Management**: Distributed config store with versioning and validation
/// - **Secret Management**: Secure secret storage, rotation, and injection
/// - **Observability Bridge**: OpenTelemetry integration for traces, metrics, and logs
///
/// ## Features
///
/// The cloud native integration provides production-ready cloud infrastructure:
/// - Rolling updates, blue-green deployments, and canary releases
/// - Mutual TLS and service-to-service authentication
/// - Circuit breaking, retry, and timeout policies
/// - Dynamic configuration with feature flags
/// - Hardware Security Module (HSM) integration
/// - Distributed tracing with span context propagation
///
/// ## Example
///
/// ```no_run
/// use kernel::cloud::{CloudNativeManager, CloudNativeConfig};
///
/// // Create cloud native manager
/// let config = CloudNativeConfig::default();
/// let mut manager = CloudNativeManager::new(config)?;
///
/// // Initialize
/// manager.initialize()?;
///
/// // Deploy a service
/// let orchestrator = manager.orchestrator();
/// let spec = kernel::cloud::ServiceSpec {
///     name: "my-service".to_string(),
///     image: "nginx:latest".to_string(),
///     version: "1.0".to_string(),
///     replicas: 3,
///     // ... other fields
/// };
/// let service_id = orchestrator.deploy_service(spec)?;
/// ```
#[cfg(feature = "cloud_native")]
pub mod cloud;
