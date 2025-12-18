//! NOS Bootloader Library - Modular Architecture
//! 
//! Comprehensive bootloader implementation organized into logical subsystems.
//! 
//! # Module Organization
//! 
//! ## Core (P0)
//! - Foundational bootloader framework and initialization
//! 
//! ## Firmware Interface (P0)
//! - BIOS interaction, disk handling, boot protocols
//! - Includes: BIOS layer, firmware protocols, multiboot support
//! 
//! ## Memory Management (P1, P9)
//! - Memory layout, paging, hotplug, ECC, mirroring
//! 
//! ## CPU Initialization (P1, P8)
//! - Multiprocessor support, mode transitions, power management, interrupts
//! 
//! ## Device Drivers (P5, P6, P7)
//! - UART, timer, device enumeration, TPM, display
//! 
//! ## Security (P6)
//! - Secure boot, TPM, measurement, verification, integrity
//! 
//! ## Kernel Interface
//! - Kernel loading, handoff, bootstrap, protocol
//! 
//! ## Boot Stage (Orchestration)
//! - Flow control, manager, executor, preparation, validation, finalization, recovery
//! 
//! ## Diagnostics (P10)
//! - Hardware scanning, timing analysis, failure logging, profiling
//! 
//! ## Optimization (P2, P3)
//! - Parallelization, lazy loading, cache optimization, error mitigation
//! 
//! ## ACPI Support (P1, P8)
//! - ACPI parsing, power management
//! 
//! ## Platform Abstraction
//! - Console, system info, device detection
//! 
//! ## Utilities
//! - Error handling, logging, memory utilities, MMIO

#![no_std]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

// CORE BOOTLOADER FRAMEWORK (P0)
pub mod core;

// FIRMWARE INTERFACE LAYER
pub mod bios_layer;
pub mod firmware;

// MEMORY MANAGEMENT (P1, P9)
pub mod memory_mgmt;

// CPU INITIALIZATION (P1, P8)
pub mod cpu_init;

// DEVICE DRIVERS (P5, P6, P7)
pub mod driver_layer;

// SECURITY SUBSYSTEM (P6)
pub mod security;

// KERNEL INTERFACE
pub mod kernel_if;

// BOOT STAGE ORCHESTRATION
pub mod boot_stage;

// DIAGNOSTICS (P10)
pub mod diagnostics;

// OPTIMIZATION (P2, P3)
pub mod optimization;

// ACPI SUPPORT (P1, P8)
pub mod acpi_support;

// PLATFORM ABSTRACTION
pub mod platform;

// UTILITY LIBRARY
pub mod utils_lib;

// ============================================================================
// LEGACY SUPPORT (Backward Compatibility)
// ============================================================================
// These modules are re-exported at top level for backward compatibility
// with existing code that imports directly from bootloader crate.

// Core modules
pub mod allocator;
pub mod init;
pub mod version;

// BIOS/Firmware modules
pub mod bios_realmode;
pub mod bios_calls;
pub mod bios_int_executor;
pub mod bios_complete;
pub mod e820_detection;
pub mod disk_io;
pub mod disk_reader;
pub mod mbr_handler;
pub mod gpt_handler;
pub mod uefi_boot_services;
pub mod uefi_loader;
pub mod uefi_loader_v2;
pub mod multiboot2_executor;
pub mod multiboot_loader;

// Memory modules
pub mod memory_init;
pub mod memory_mapping;
pub mod paging;
pub mod paging_setup;
pub mod memory_hotplug;
pub mod memory_ecc;
pub mod memory_mirroring;
pub mod advanced_memory_mgmt;

// CPU modules
pub mod multiprocessor_init;
pub mod mode_transition;
pub mod realmode_switcher;
pub mod acpi_power_domains;
pub mod dvfs_scaling;
pub mod sleep_wake_handler;
pub mod idt_manager;
pub mod exception_handler;
pub mod interrupt_routing;

// Driver modules
pub mod uart_driver;
pub mod timer_driver;
pub mod boot_timer;
pub mod device_enumeration;
pub mod device_detect;
pub mod tpm_driver;
pub mod vga;
pub mod console;
pub mod console_vga;

// Security modules
pub mod secure_boot_framework;
pub mod secure_boot_handler;
pub mod secure_boot_verifier;
pub mod boot_measurement;
pub mod firmware_integrity;
pub mod boot_security;

// Kernel modules
pub mod kernel_handoff;
pub mod kernel_loader;
pub mod kernel_loader_impl;
pub mod kernel_bootstrap;
pub mod kernel_entry;
pub mod elf_loader;
pub mod elf_loader_v2;
pub mod elf_loader_hardened;
pub mod elf64;
pub mod boot_info_builder;
pub mod boot_handoff;

// Boot modules (legacy flat structure)
pub mod boot_orchestrator;
pub mod boot_sequence;
pub mod boot_loader;
pub mod boot_diagnostics;
pub mod boot_executor;
pub mod boot_preparation;
pub mod boot_manager;
pub mod boot_validation;
pub mod boot_control;
pub mod boot_loader_integration;
pub mod boot_flow_manager;
pub mod boot_recovery;
pub mod boot_finalization;
pub mod boot_diagnostics_complete;
pub mod boot_coordinator;
pub mod system_validation;
pub mod advanced_boot_protocol;
pub mod boot_optimization;
pub mod fallback_boot;
pub mod boot_flow;
pub mod boot_orchestration;
pub mod boot_parallelization;
pub mod lazy_loading;
pub mod cache_optimization;
pub mod boot_verification;

// Diagnostic modules
pub mod hardware_scan;
pub mod boot_timing_analysis;
pub mod boot_failure_logger;
pub mod performance_profiling;

// Utility modules
pub mod error_recovery;
pub mod error;
pub mod error_handling;
pub mod recovery;
pub mod boot_log;
pub mod mem_util;
pub mod mmio;

// Other modules
pub mod acpi_parser;
pub mod error_mitigation;
pub mod system_info;
pub mod hw_init;
pub mod virtualization_detect;
pub mod hypervisor_init;
pub mod virtual_machine;

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================
// Most commonly used types and traits from organized module hierarchy

// Core
pub use core::{BootState, BootPhase};

// BIOS/Firmware
pub use bios_realmode::{RealModeContext, RealModeExecutor};
pub use bios_layer::BiosCalls;

// Memory
pub use memory_mgmt::{MemoryLayout, MemoryManager};

// CPU
pub use cpu_init::{CpuPowerManager, InterruptManager};

// Drivers
pub use driver_layer::{UartDriver, TimerDriver, DisplayDriver};

// Security
pub use security::SecureBootManager;

// Kernel
pub use kernel_if::{KernelHandoff, BootInformation};

// Boot
pub use boot_stage::BootManager;

// Diagnostics
pub use diagnostics::{HardwareScanner, BootTimingAnalyzer, BootFailureLogger};

// Error handling
pub use error_recovery::{BootError, ErrorRecovery};

// ============================================================================
// PANIC HANDLER
// ============================================================================

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error_recovery::panic_handler(info)
}
