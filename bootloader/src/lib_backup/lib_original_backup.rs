//! NOS Bootloader Library - P0 Phase 2 Core Implementation
//! 
//! Minimal bootloader library focused exclusively on P0 Phase 2 deliverables:
//! - Real mode BIOS interrupt execution (bios_realmode)
//! - Boot error recovery and diagnostics (error_recovery)
//! - Kernel handoff and boot information passing (kernel_handoff)
//!
//! This stripped-down version eliminates dependencies on legacy bootloader
//! modules that have incomplete implementations and circular dependencies.

#![no_std]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// ============================================================================
// REQUIRED MODULES FOR P0 PHASE 2
// ============================================================================

/// Memory allocator (required by alloc crate)
pub mod allocator;

/// VGA text mode support (required by error_recovery)
pub mod bios_complete;

/// Real mode BIOS interrupt handler framework
pub mod bios_realmode;

/// Boot error recovery and diagnostics
pub mod error_recovery;

/// Kernel handoff mechanism and boot information
pub mod kernel_handoff;

/// E820 memory detection via BIOS
pub mod e820_detection;

/// Disk I/O operations via INT 0x13
pub mod disk_io;

/// Boot flow orchestration
pub mod boot_orchestrator;

/// Real mode mode switching for BIOS calls
pub mod realmode_switcher;

/// VGA text mode output
pub mod vga;

/// Bootloader initialization
pub mod init;

/// Boot sequence configuration
pub mod boot_sequence;

/// BIOS call wrappers
pub mod bios_calls;

/// BIOS interrupt executor
pub mod bios_int_executor;

/// Hardware initialization
pub mod hw_init;

/// Boot loader state and execution
pub mod boot_loader;

/// Boot diagnostics and verification
pub mod boot_diagnostics;

/// Complete boot execution coordinator
pub mod boot_executor;

/// Boot preparation and kernel readiness
pub mod boot_preparation;

/// Advanced boot manager - complete integration
pub mod boot_manager;

/// Final boot validation and system check
pub mod boot_validation;

/// Advanced boot control and execution
pub mod boot_control;

/// Real kernel loader implementation
pub mod kernel_loader_impl;

/// Mode transition (real/protected/long mode)
pub mod mode_transition;

/// Boot information builder
pub mod boot_info_builder;

/// Kernel entry handler
pub mod kernel_entry;

/// Multiboot2 protocol executor
pub mod multiboot2_executor;

/// Master Boot Record (MBR) handler
pub mod mbr_handler;

/// GUID Partition Table (GPT) handler
pub mod gpt_handler;

/// Real disk I/O implementation
pub mod disk_reader;

/// Boot loader integration with disk support
pub mod boot_loader_integration;

/// Advanced boot flow manager
pub mod boot_flow_manager;

/// Secure boot handler and management
pub mod secure_boot_handler;

/// Secure boot signature verification framework
pub mod secure_boot_verifier;

/// Fallback boot device management
pub mod fallback_boot;

/// Boot recovery and error handling
pub mod boot_recovery;

/// System validation and hardware capability checking
pub mod system_validation;

/// Advanced boot protocol support
pub mod advanced_boot_protocol;

/// Boot optimization and performance profiling
pub mod boot_optimization;

/// Firmware integrity verification
pub mod firmware_integrity;

/// Boot coordinator - master boot orchestration
pub mod boot_coordinator;

/// Kernel bootstrap - ELF64 environment setup
pub mod kernel_bootstrap;

/// Boot finalization - final system checks
pub mod boot_finalization;

/// Complete boot diagnostics and analysis
pub mod boot_diagnostics_complete;

/// Multiprocessor initialization (P1)
pub mod multiprocessor_init;

/// ACPI parser and table support (P1)
pub mod acpi_parser;

/// Advanced memory management (P1)
pub mod advanced_memory_mgmt;

/// Boot parallelization (P2)
pub mod boot_parallelization;

/// Lazy loading optimization (P2)
pub mod lazy_loading;

/// Cache optimization (P2)
pub mod cache_optimization;

/// Error mitigation (P3)
pub mod error_mitigation;

/// Performance profiling (P3)
pub mod performance_profiling;

/// IDT manager (P4)
pub mod idt_manager;

/// Exception handler (P4)
pub mod exception_handler;

/// Interrupt routing (P4)
pub mod interrupt_routing;

/// UART driver (P5)
pub mod uart_driver;

/// Timer driver (P5)
pub mod timer_driver;

/// Device enumeration (P5)
pub mod device_enumeration;

/// TPM driver (P6)
pub mod tpm_driver;

/// Secure Boot framework (P6)
pub mod secure_boot_framework;

/// Boot measurement (P6)
pub mod boot_measurement;

/// Virtualization detection (P7)
pub mod virtualization_detect;

/// Hypervisor initialization (P7)
pub mod hypervisor_init;

/// Virtual machine management (P7)
pub mod virtual_machine;

/// ACPI power domains (P8)
pub mod acpi_power_domains;

/// DVFS scaling (P8)
pub mod dvfs_scaling;

/// Sleep/wake handler (P8)
pub mod sleep_wake_handler;

/// Memory hotplug (P9)
pub mod memory_hotplug;

/// Memory ECC (P9)
pub mod memory_ecc;

/// Memory mirroring (P9)
pub mod memory_mirroring;

/// Hardware scan (P10)
pub mod hardware_scan;

/// Boot timing analysis (P10)
pub mod boot_timing_analysis;

/// Boot failure logger (P10)
pub mod boot_failure_logger;

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub use bios_realmode::{RealModeContext, RealModeExecutor};
pub use error_recovery::{BootError, ErrorRecovery};
pub use kernel_handoff::{BootInformation, BootProtocol, KernelHandoff, MemoryMapEntry, LoadedModule};

// ============================================================================
// PANIC HANDLER
// ============================================================================

// Only define panic handler for no_std builds (not tests)
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Route to error_recovery panic handler
    error_recovery::panic_handler(info)
}
