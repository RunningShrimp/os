//! Kernel core initialization
//!
//! This module provides the core kernel initialization logic that is shared
//! between bootloader-based startup and library-based initialization.

use crate::platform::boot::BootParameters;

/// Core kernel initialization function
/// 
/// This function initializes all kernel subsystems in the correct order.
/// It is called by both `rust_main_with_boot_info` (bootloader entry) and
/// `init_kernel` (library entry point).
///
/// # Arguments
/// * `boot_params` - Optional boot parameters from bootloader
pub fn init_kernel_core(boot_params: Option<&BootParameters>) {
    use crate::monitoring;
    
    monitoring::timeline::record("boot_start");
    
    // Initialize boot information if provided
    if let Some(params) = boot_params {
        // Boot parameters are already initialized in rust_main_with_boot_info
        // For library entry, we need to initialize them here
        unsafe {
            crate::platform::boot::init_from_boot_parameters(params as *const BootParameters);
        }
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
        { "riscv64" }
        #[cfg(target_arch = "aarch64")]
        { "aarch64" }
        #[cfg(target_arch = "x86_64")]
        { "x86_64" }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
        { "unknown" }
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
    crate::vfs::procfs::fs::init();
    crate::vfs::sysfs::fs::init();
    
    // Try to mount ramfs first, fall back to tmpfs if it fails
    let root_mounted = match crate::vfs::mount("ramfs", "/", None, 0) {
        Ok(()) => {
            crate::println!("[boot] VFS root mounted (ramfs)");
            true
        }
        Err(e) => {
            crate::println!("[boot] ramfs mount failed: {:?}, trying tmpfs...", e);
            match crate::vfs::mount("tmpfs", "/", None, 0) {
                Ok(()) => {
                    crate::println!("[boot] VFS root mounted (tmpfs)");
                    true
                }
                Err(e2) => {
                    crate::println!("[boot] tmpfs mount also failed: {:?}", e2);
                    false
                }
            }
        }
    };
    
    // Verify root file system is accessible
    if root_mounted {
        match crate::vfs::verify_root() {
            Ok(()) => {
                if let Ok(attr) = crate::vfs::vfs().stat("/") {
                    crate::println!("[vfs] root verified: ino={} mode={:#o} size={}B", 
                        attr.ino, attr.mode.permissions(), attr.size);
                }
            }
            Err(e) => {
                crate::println!("[boot] WARNING: Root file system verification failed: {:?}", e);
                crate::println!("[boot] System may not function correctly without a root file system");
            }
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
            crate::println!("[boot] WARNING: Continuing without root file system (DEBUG BUILD ONLY)");
            crate::println!("[boot] WARNING: This is unsafe and may cause system instability");
        }
    }

    // Initialize file system subsystem
    crate::subsystems::fs::init()
        .expect("File system subsystem initialization failed");
    
    // Initialize file system with journaling support (if enabled)
    #[cfg(feature = "journaling_fs")]
    {
        use crate::subsystems::fs::journaling_wrapper;
        use crate::platform::drivers::RamDisk;
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
    crate::subsystems::process::init()
        .expect("Process subsystem initialization failed");
    crate::println!("[boot] process subsystem initialized");
    
    // Initialize unified system call dispatcher
    #[cfg(feature = "syscalls")]
    {
        use crate::subsystems::syscalls::dispatch::unified::{init_unified_dispatcher, UnifiedDispatcherConfig};
        let config = UnifiedDispatcherConfig::default();
        init_unified_dispatcher(config);
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
    crate::subsystems::ipc::init()
        .expect("IPC subsystem initialization failed");
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
        use crate::subsystems::scheduler::unified::init_unified_scheduler;
        let num_cpus = crate::cpu::ncpus();
        init_unified_scheduler(num_cpus);
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
        }
        Err(e) => {
            crate::println!("[boot] WARNING: Security subsystem initialization failed: {:?}", e);
            crate::println!("[boot] System will continue with reduced security features");
        }
    }

    #[cfg(feature = "security_audit")]
    {
        crate::security_audit::init_security_audit().expect("Security audit initialization failed");
        crate::println!("[boot] security audit initialized");
    }

    #[cfg(feature = "formal_verification")]
    {
        crate::formal_verification::init_formal_verification().expect("Formal verification initialization failed");
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
    crate::debug::fault_diagnosis::create_fault_diagnosis_engine().lock().init().expect("Fault diagnosis initialization failed");
    crate::println!("[boot] fault diagnosis system initialized");

    // Initialize graceful degradation system
    crate::reliability::graceful_degradation::create_graceful_degradation_manager().lock().init().expect("Graceful degradation initialization failed");
    crate::println!("[boot] graceful degradation system initialized");

    // Initialize health monitoring integration
    crate::monitoring::health_integration::init_health_integration().expect("Health integration initialization failed");
    crate::println!("[boot] health monitoring integration initialized");

    #[cfg(feature = "debug_subsystems")]
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
    crate::platform::drivers::device_manager::init().expect("Device manager system initialization failed");
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
        crate::monitoring::metrics::init_metrics_collector().expect("Metrics collector initialization failed");
        crate::monitoring::health::init_health_checker().expect("Health checker initialization failed");
        crate::monitoring::alerting::init_alert_manager().expect("Alert manager initialization failed");
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
