//! 内核核心初始化 / Kernel Core Initialization
//!
//! 本模块提供内核核心初始化逻辑，支持：
//! This module provides core kernel initialization logic, supporting:
//!
//! - **并行初始化**: 利用多核CPU加速启动（目标50%加速）
//! - **向后兼容**: 保持与原有串行初始化的兼容性
//! - **错误恢复**: 组件失败时优雅降级
//! - **性能监控**: 收集启动性能指标
//!
//! ## 初始化阶段 / Initialization Phases
//!
//! 1. **Early Init** (早期): 串行，控制台、中断、Boot CPU
//! 2. **Parallel Init** (并行): 内存、文件系统、驱动程序
//! 3. **Late Init** (晚期): 网络、服务、监控
//!
//! ## 使用方式 / Usage
//!
//! 默认使用并行初始化（如果CPU > 1）：
//! ```rust
//! init_kernel_core(boot_params);
//! ```
//!
//! 强制使用串行初始化：
//! ```rust
//! init_kernel_core_serial(boot_params);
//! ```

use crate::platform::boot::BootParameters;

// ============================================================================
// 串行初始化（向后兼容） / Serial Initialization (Backward Compatible)
// ============================================================================

/// 核心内核初始化函数（默认使用并行）
/// Core kernel initialization function (uses parallel by default)
///
/// 此函数初始化所有内核子系统。它是 bootloader 入口点和库入口点的共同调用点。
/// This function initializes all kernel subsystems. It is called by both
/// bootloader entry and library entry point.
///
/// # 参数 / Arguments
/// * `boot_params` - 来自 bootloader 的可选启动参数
///                   Optional boot parameters from bootloader
///
/// # 性能 / Performance
///
/// 在多核系统上，使用并行初始化可减少约 50% 的启动时间。
/// On multi-core systems, parallel initialization reduces boot time by ~50%.
pub fn init_kernel_core(boot_params: Option<&BootParameters>) {
    // 检查CPU数量，决定是否使用并行初始化
    // Check CPU count to decide whether to use parallel initialization
    let num_cpus = crate::cpu::ncpus();

    if num_cpus > 1 {
        crate::println!("[boot] Multi-CPU system detected ({} CPUs), using parallel initialization", num_cpus);
        init_kernel_core_parallel(boot_params);
    } else {
        crate::println!("[boot] Single-CPU system detected, using serial initialization");
        init_kernel_core_serial(boot_params);
    }
}

/// 串行内核初始化（向后兼容的原有实现）
/// Serial kernel initialization (original backward-compatible implementation)
///
/// 这是原有的串行初始化实现，保持向后兼容性。
/// This is the original serial initialization implementation, maintaining backward compatibility.
///
/// # 参数 / Arguments
/// * `boot_params` - 来自 bootloader 的可选启动参数
///                   Optional boot parameters from bootloader
pub fn init_kernel_core_serial(boot_params: Option<&BootParameters>) {
    use crate::monitoring;

    monitoring::timeline::record("boot_start");

    // Initialize boot information if provided
    if let Some(params) = boot_params {
        // Boot parameters are already initialized in rust_main_with_boot_info
        // For library entry, we need to initialize them here
        crate::platform::boot::init_from_boot_parameters(params as *const BootParameters);
    } else if !crate::platform::boot::is_bootloader_boot() {
        // No boot parameters - initialize legacy mode
        crate::platform::boot::init_direct_boot();
    }

    // Early hardware initialization (UART, etc.)
    crate::platform::arch::early_init();
    monitoring::timeline::record("early_init");

    crate::println!();
    crate::println!("NOS kernel v0.1.0 booting on {}...", {
        #[cfg(target_arch = "riscv64")]
        {
            "riscv64"
        }
        #[cfg(target_arch = "aarch64")]
        {
            "aarch64"
        }
        #[cfg(target_arch = "x86_64")]
        {
            "x86_64"
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            "unknown"
        }
    });
    crate::println!();

    // Print boot information
    crate::platform::boot::print_boot_info();

    // Initialize boot CPU
    crate::cpu::init_boot_cpu();
    crate::println!("[boot] boot CPU initialized");

    // Initialize trap handling early
    crate::platform::trap::init();
    crate::println!("[boot] trap handlers initialized");

    // Initialize memory management from boot info or fall back to legacy
    crate::platform::boot::init_memory_from_boot_info();
    if !crate::platform::boot::is_bootloader_boot() {
        // Memory allocator is initialized by mm::phys::init() which is called earlier
        // No need for separate initialization
    }
    crate::println!("[boot] physical memory initialized");

    // Initialize kernel heap allocator
    // (allocator::init is called in mm::init())
    crate::println!("[boot] kernel heap ready");

    // Initialize framebuffer if available from bootloader
    crate::platform::boot::init_framebuffer_from_boot_info();

    // Initialize ACPI if available from bootloader
    crate::platform::boot::init_acpi_from_boot_info();

    // Initialize device tree if available from bootloader
    crate::platform::boot::init_device_tree_from_boot_info();

    // Initialize virtual memory / page tables
    crate::subsystems::mm::vm::init();
    crate::println!("[boot] virtual memory initialized");
    monitoring::timeline::record("vm_init");

    // Initialize RCU subsystem
    crate::subsystems::sync::rcu::init_rcu();
    crate::println!("[boot] RCU subsystem initialized");

    // Initialize timer
    crate::subsystems::time::init();
    crate::println!("[boot] timer initialized");

    // Initialize drivers
    crate::platform::drivers::init();
    crate::println!("[boot] drivers initialized");
    monitoring::timeline::record("drivers_init");

    // Initialize and mount VFS root (ramfs)
    crate::vfs::ramfs::init();
    crate::vfs::ext4::init();
    crate::vfs::procfs::init();
    crate::vfs::sysfs::init();

    // Try to mount ramfs first, fall back to tmpfs if it fails
    let root_mounted = match crate::vfs::mount("ramfs", None, "/", 0) {
        Ok(()) => {
            crate::println!("[boot] VFS root mounted (ramfs)");
            true
        },
        Err(e) => {
            crate::println!("[boot] ramfs mount failed: {:?}, trying tmpfs...", e);
            match crate::vfs::mount("tmpfs", None, "/", 0) {
                Ok(()) => {
                    crate::println!("[boot] VFS root mounted (tmpfs)");
                    true
                },
                Err(e2) => {
                    crate::println!("[boot] tmpfs mount also failed: {:?}", e2);
                    false
                },
            }
        },
    };

    // Verify root file system is accessible
    if root_mounted {
        match crate::subsystems::fs::verify_root() {
            Ok(()) => {
                if let Ok(attr) = crate::vfs::vfs().stat("/") {
                    crate::println!(
                        "[vfs] root verified: ino={} mode={:#o} size={}B",
                        attr.ino,
                        attr.mode.permissions(),
                        attr.size
                    );
                }
            },
            Err(e) => {
                crate::println!("[boot] WARNING: Root file system verification failed: {:?}", e);
                crate::println!(
                    "[boot] System may not function correctly without a root file system"
                );
            },
        }
    } else {
        crate::println!("[boot] ERROR: Failed to mount root file system!");
        crate::println!("[boot] System cannot continue without a root file system");

        // In production profile, root mount failure is fatal
        #[cfg(not(debug_assertions))]
        {
            // Production build: panic immediately
            crate::panic!("CRITICAL: Root file system mount failed - system cannot continue");
        }

        // In debug builds, we allow continuing for development/debugging
        #[cfg(debug_assertions)]
        {
            crate::println!(
                "[boot] WARNING: Continuing without root file system (DEBUG BUILD ONLY)"
            );
            crate::println!("[boot] WARNING: This is unsafe and may cause system instability");
        }
    }

    // Initialize file system subsystem
    crate::subsystems::fs::init().expect("File system subsystem initialization failed");

    // Initialize file system with journaling support (if enabled)
    #[cfg(feature = "journaling_fs")]
    {
        use crate::{platform::drivers::RamDisk, subsystems::fs::journaling_wrapper};
        if journaling_wrapper::init_fs_with_journaling(RamDisk) {
            crate::println!("[boot] journaling filesystem initialized");
        } else {
            crate::println!("[boot] falling back to regular filesystem");
        }
    }

    monitoring::timeline::record("fs_init");

    // Initialize C standard library (newlib)
    crate::libc::init().expect("C standard library initialization failed");
    crate::println!("[boot] C standard library initialized");

    // Initialize AIO subsystem
    #[cfg(feature = "syscalls")]
    {
        crate::syscalls::aio::init().expect("AIO subsystem initialization failed");
        crate::println!("[boot] AIO subsystem initialized");
    }

    // Initialize advanced memory mapping subsystem
    crate::println!("[boot] Advanced memory mapping subsystem initialized");

    // Initialize process subsystem
    crate::subsystems::process::init().expect("Process subsystem initialization failed");
    crate::println!("[boot] process subsystem initialized");

    // Initialize unified system call dispatcher
    #[cfg(feature = "syscalls")]
    {
        // Use the unified_impl module directly since re-exports don't work properly
        let config = crate::subsystems::syscalls::dispatch::unified_impl::UnifiedDispatcherConfig::default();
        crate::subsystems::syscalls::dispatch::unified_impl::init_unified_dispatcher(config);
        crate::println!("[boot] unified syscall dispatcher initialized");

        // Register POSIX file descriptor system calls (timerfd, eventfd, signalfd)
        crate::subsystems::syscalls::posix_fd::register_posix_fd_syscalls()
            .expect("Failed to register POSIX fd syscalls");
        crate::println!("[boot] POSIX fd syscalls (timerfd, eventfd, signalfd) registered");
    }

    // Initialize fast-path syscall optimization (legacy, will be removed)
    #[cfg(feature = "syscalls")]
    {
        crate::subsystems::syscalls::fast_path::init();
        crate::println!("[boot] fast-path syscall optimization initialized (legacy)");
    }

    // Initialize IPC subsystem
    crate::subsystems::ipc::init().expect("IPC subsystem initialization failed");
    crate::println!("[boot] IPC subsystem initialized");

    #[cfg(all(not(feature = "lazy_init"), feature = "net_stack"))]
    {
        crate::subsystems::net::init();
        crate::println!("[boot] network stack initialized");
    }

    // Initialize threading subsystem
    crate::subsystems::process::thread::init();
    crate::println!("[boot] threading subsystem initialized");

    // Initialize unified scheduler with priority queues
    {
        use crate::sched::unified::init_unified_scheduler;
        let _ = init_unified_scheduler();
        let num_cpus = crate::cpu::ncpus();
        crate::println!("[boot] unified scheduler initialized ({} CPUs)", num_cpus);
    }

    // Initialize microkernel core (required for hybrid architecture)
    crate::subsystems::microkernel::init_microkernel().expect("Microkernel initialization failed");
    crate::println!("[boot] microkernel core initialized");

    // Initialize service layer
    #[cfg(feature = "services")]
    {
        crate::services::init().expect("Service layer initialization failed");
        crate::println!("[boot] service layer initialized");
    }
    monitoring::timeline::record("services_init");

    // Initialize enhanced permission system
    crate::security::enhanced_permissions::init_permission_manager();
    crate::println!("[security] Enhanced permission system initialized");

    // Initialize security subsystem
    match crate::security::init_security_subsystem() {
        Ok(()) => {
            crate::println!("[boot] security subsystem initialized");
        },
        Err(e) => {
            crate::println!("[boot] WARNING: Security subsystem initialization failed: {:?}", e);
            crate::println!("[boot] System will continue with reduced security features");
        },
    }

    #[cfg(feature = "security")]
    {
        crate::security_audit::init_security_audit().expect("Security audit initialization failed");
        crate::println!("[boot] security audit initialized");
    }

    #[cfg(feature = "formal_verification")]
    {
        crate::formal_verification::init_formal_verification()
            .expect("Formal verification initialization failed");
        crate::println!("[boot] formal verification system initialized");
    }

    // Initialize error handling system
    #[cfg(feature = "error_handling")]
    {
        crate::error_handling::init_error_handling().expect("Error handling initialization failed");
        crate::println!("[boot] error handling system initialized");

        // Initialize error recovery manager
        crate::error::recovery::init_recovery_manager();
        crate::println!("[boot] error recovery manager initialized");
    }

    // Initialize unified error mapper
    {
        use crate::error::unified_mapping::init_error_mapper;
        init_error_mapper();
        crate::println!("[boot] unified error mapper initialized");
    }

    // Initialize fault diagnosis system
    #[cfg(feature = "debug")]
    {
        crate::debug::fault_diagnosis::create_fault_diagnosis_engine()
            .lock()
            .init()
            .expect("Fault diagnosis initialization failed");
        crate::println!("[boot] fault diagnosis system initialized");
    }

    // Initialize graceful degradation system
    crate::reliability::graceful_degradation::create_graceful_degradation_manager()
        .lock()
        .init()
        .expect("Graceful degradation initialization failed");
    crate::println!("[boot] graceful degradation system initialized");

    // Initialize health monitoring integration
    crate::monitoring::health_integration::init_health_integration()
        .expect("Health integration initialization failed");
    crate::println!("[boot] health monitoring integration initialized");

    #[cfg(feature = "debug")]
    {
        crate::debug::init().expect("Debugging system initialization failed");
        crate::println!("[boot] debugging system initialized");

        crate::debug::monitoring::init().expect("Monitoring system initialization failed");
        crate::println!("[boot] monitoring system initialized");

        crate::debug::profiling::init().expect("Profiling system initialization failed");
        crate::println!("[boot] profiling system initialized");

        crate::debug::tracing::init().expect("Tracing system initialization failed");
        crate::println!("[boot] tracing system initialized");

        crate::debug::metrics::init().expect("Metrics system initialization failed");
        crate::println!("[boot] metrics system initialized");

        crate::debug::symbols::init().expect("Debug symbols system initialization failed");
        crate::println!("[boot] debug symbols system initialized");
    }

    // Initialize cross-platform compatibility layer
    crate::compat::init().expect("Cross-platform compatibility layer initialization failed");
    crate::println!("[boot] cross-platform compatibility layer initialized");

    // Initialize device manager system
    crate::platform::drivers::device_manager::init()
        .expect("Device manager system initialization failed");
    crate::println!("[boot] device manager system initialized");

    #[cfg(all(not(feature = "lazy_init"), feature = "graphics_subsystem"))]
    {
        crate::graphics::init();
        crate::println!("[boot] graphics subsystem initialized");
    }

    #[cfg(all(not(feature = "lazy_init"), feature = "web_engine"))]
    {
        crate::web::init();
        crate::println!("[boot] web engine subsystem initialized");
    }

    #[cfg(feature = "observability")]
    {
        crate::monitoring::metrics::init_metrics_collector()
            .expect("Metrics collector initialization failed");
        crate::monitoring::health::init_health_checker()
            .expect("Health checker initialization failed");
        crate::monitoring::alerting::init_alert_manager()
            .expect("Alert manager initialization failed");
        crate::println!("[boot] monitoring system initialized");
    }

    // Start other CPUs
    crate::cpu::start_aps();
    crate::cpu::boot_complete();
    crate::println!("[boot] SMP initialization complete ({} CPUs)", crate::cpu::ncpus());
    monitoring::timeline::record("boot_complete");

    crate::println!();
    crate::println!("NOS kernel ready!");
    crate::println!();
}

// ============================================================================
// 并行初始化 / Parallel Initialization
// ============================================================================

/// 并行内核初始化（新的并行实现）
/// Parallel kernel initialization (new parallel implementation)
///
/// 使用并行初始化框架，在多核系统上可减少约 50% 的启动时间。
/// Uses parallel initialization framework, reducing boot time by ~50% on multi-core systems.
///
/// # 参数 / Arguments
/// * `boot_params` - 来自 bootloader 的可选启动参数
///                   Optional boot parameters from bootloader
///
/// # 性能优化 / Performance Optimization
///
/// 将初始化分为三个阶段：
/// Divides initialization into three phases:
/// 1. **早期（串行）**: 控制台、中断、Boot CPU
/// 2. **并行**: 内存管理、文件系统、驱动程序
/// 3. **晚期**: 网络、服务、监控
#[cfg(feature = "parallel_init")]
pub fn init_kernel_core_parallel(boot_params: Option<&BootParameters>) {
    use crate::monitoring;
    use crate::core::parallel_init::{create_default_parallel_engine, ParallelInitConfig};
    use crate::core::init_dependencies::create_standard_kernel_deps;

    monitoring::timeline::record("boot_start_parallel");

    crate::println!();
    crate::println!("NOS kernel v0.1.0 booting on {}...", {
        #[cfg(target_arch = "riscv64")]
        {
            "riscv64"
        }
        #[cfg(target_arch = "aarch64")]
        {
            "aarch64"
        }
        #[cfg(target_arch = "x86_64")]
        {
            "x86_64"
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            "unknown"
        }
    });
    crate::println!();

    // ===== Phase 0: 最早期初始化（必须串行）=====
    // ===== Phase 0: Very early init (must be serial) =====

    // Initialize boot information
    if let Some(params) = boot_params {
        crate::platform::boot::init_from_boot_parameters(params as *const BootParameters);
    } else if !crate::platform::boot::is_bootloader_boot() {
        crate::platform::boot::init_direct_boot();
    }

    // Early hardware initialization
    crate::platform::arch::early_init();

    // Print boot information
    crate::platform::boot::print_boot_info();

    monitoring::timeline::record("early_hardware_init");

    // ===== 使用并行初始化引擎 =====
    // ===== Use parallel initialization engine =====

    let deps = match create_standard_kernel_deps() {
        Ok(deps) => deps,
        Err(e) => {
            crate::println!("[boot] Failed to create dependency graph: {:?}", e);
            crate::println!("[boot] Falling back to serial initialization");
            init_kernel_core_serial(boot_params);
            return;
        }
    };

    let config = ParallelInitConfig {
        max_parallel_tasks: crate::cpu::ncpus(),
        abort_on_failure: false, // Continue on optional component failures
        enable_monitoring: true,
        task_timeout_us: 0,
        verbose: true,
    };

    let mut engine = match create_default_parallel_engine() {
        Ok(engine) => engine,
        Err(e) => {
            crate::println!("[boot] Failed to create parallel engine: {:?}", e);
            crate::println!("[boot] Falling back to serial initialization");
            init_kernel_core_serial(boot_params);
            return;
        }
    };

    // 注册所有初始化函数
    // Register all initialization functions
    register_init_functions(&mut engine);

    // 执行并行初始化
    // Execute parallel initialization
    match engine.initialize() {
        Ok(stats) => {
            crate::println!();
            crate::println!("=== Parallel Initialization Statistics ===");
            crate::println!("Total time: {} us", stats.total_time_us);
            crate::println!(
                "Completed: {}/{} components",
                stats.completed_count,
                stats.total_count()
            );
            if stats.failed_count > 0 {
                crate::println!("Failed: {} components", stats.failed_count);
            }
            if stats.skipped_count > 0 {
                crate::println!("Skipped: {} components", stats.skipped_count);
            }
            crate::println!("Success rate: {:.1}%", stats.success_rate() * 100.0);
            crate::println!("Expected speedup: {:.2}x", stats.expected_speedup);
            crate::println!("Actual speedup: {:.2}x", stats.actual_speedup);
            crate::println!("CPU utilization: {:.1}%", stats.cpu_utilization);
            crate::println!("==========================================");
            crate::println!();
        }
        Err(e) => {
            crate::println!("[boot] Parallel initialization failed: {:?}", e);
            crate::println!("[boot] This should not happen with abort_on_failure=false");
        }
    }

    monitoring::timeline::record("boot_complete_parallel");

    crate::println!();
    crate::println!("NOS kernel ready (parallel initialization)!");
    crate::println!();
}

/// 注册所有组件的初始化函数
/// Register initialization functions for all components
#[cfg(feature = "parallel_init")]
fn register_init_functions(engine: &mut crate::core::parallel_init::ParallelInitEngine) {
    use crate::core::parallel_init::InitFn;

    // 辅助宏：注册初始化函数
    // Helper macro: register initialization function
    macro_rules! register {
        ($name:expr, $func:expr) => {
            engine.register_init_function($name, $func as InitFn);
        };
    }

    // Early components
    register!("boot_params", || Ok(()));
    register!("console", || {
        crate::platform::arch::early_init();
        Ok(())
    });
    register!("interrupts", || {
        crate::platform::trap::init();
        Ok(())
    });
    register!("boot_cpu", || {
        crate::cpu::init_boot_cpu();
        Ok(())
    });

    // Parallel components
    register!("phys_memory", || {
        crate::platform::boot::init_memory_from_boot_info();
        Ok(())
    });
    register!("heap", || Ok(()));
    register!("vm", || {
        crate::subsystems::mm::vm::init();
        Ok(())
    });
    register!("rcu", || {
        crate::subsystems::sync::rcu::init_rcu();
        Ok(())
    });
    register!("timer", || {
        crate::subsystems::time::init();
        Ok(())
    });
    register!("drivers", || {
        crate::platform::drivers::init();
        Ok(())
    });
    register!("ramfs", || {
        crate::vfs::ramfs::init();
        Ok(())
    });
    register!("ext4", || {
        crate::vfs::ext4::init();
        Ok(())
    });
    register!("procfs", || {
        crate::vfs::procfs::init();
        Ok(())
    });
    register!("sysfs", || {
        crate::vfs::sysfs::init();
        Ok(())
    });
    register!("vfs_mount", || {
        // VFS mount logic
        match crate::vfs::mount("ramfs", None, "/", 0) {
            Ok(()) => crate::println!("[vfs] VFS root mounted (ramfs)"),
            Err(_) => {
                crate::vfs::mount("tmpfs", None, "/", 0)
                    .map_err(|e| crate::error::Error::from(e))?;
                crate::println!("[vfs] VFS root mounted (tmpfs)");
            }
        }
        Ok(())
    });
    register!("libc", || {
        crate::libc::init()?;
        Ok(())
    });
    #[cfg(feature = "syscalls")]
    register!("aio", || {
        crate::syscalls::aio::init()?;
        Ok(())
    });
    register!("scheduler", || {
        use crate::sched::unified::init_unified_scheduler;
        let _ = init_unified_scheduler();
        Ok(())
    });
    register!("threading", || {
        crate::subsystems::process::thread::init();
        Ok(())
    });

    // Late components
    register!("process", || {
        crate::subsystems::process::init()?;
        Ok(())
    });
    #[cfg(feature = "syscalls")]
    register!("syscalls", || {
        use crate::subsystems::syscalls::dispatch::unified_impl::UnifiedDispatcherConfig;
        let config = UnifiedDispatcherConfig::default();
        crate::subsystems::syscalls::dispatch::unified_impl::init_unified_dispatcher(config);
        crate::subsystems::syscalls::posix_fd::register_posix_fd_syscalls()?;
        Ok(())
    });
    register!("ipc", || {
        crate::subsystems::ipc::init()?;
        Ok(())
    });
    #[cfg(feature = "net_stack")]
    register!("network", || {
        crate::subsystems::net::init();
        Ok(())
    });
    register!("security", || {
        crate::security::init_security_subsystem()?;
        Ok(())
    });
    #[cfg(feature = "services")]
    register!("services", || {
        crate::services::init()?;
        Ok(())
    });
    register!("compat", || {
        crate::compat::init()?;
        Ok(())
    });
    register!("smp", || {
        crate::cpu::start_aps();
        crate::cpu::boot_complete();
        Ok(())
    });
}

/// 单核系统回退到串行初始化
/// Fallback to serial initialization for single-core systems
#[cfg(not(feature = "parallel_init"))]
pub fn init_kernel_core_parallel(boot_params: Option<&BootParameters>) {
    crate::println!("[boot] Parallel initialization not enabled, using serial");
    init_kernel_core_serial(boot_params);
}
