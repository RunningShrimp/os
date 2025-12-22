//! NOS Bootloader Library - Modular Architecture
//! 
//! Comprehensive bootloader implementation organized into logical subsystems.
//! Features both new hierarchical structure and legacy flat imports for backward compatibility.

#![no_std]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// ============================================================================
// STRUCTURED MODULE HIERARCHY (14 Functional Tiers)
// ============================================================================

/// Domain layer - Core bootloader domain concepts (DDD)
pub mod domain;

/// Infrastructure layer - Hardware abstraction and external services
pub mod infrastructure;

/// Application layer - Use case orchestration
pub mod application;

/// Core bootloader framework and initialization (P0)
pub mod core;

/// BIOS layer - Real mode BIOS interrupt handling (P0)
pub mod bios;

/// Firmware interface - BIOS, UEFI, Multiboot2 protocols (P0)
pub mod firmware;

/// Memory management - Layout, paging, hotplug, ECC, mirroring (P1, P9)
pub mod memory_mgmt;

/// CPU initialization - Multiprocessor, modes, power, interrupts (P1, P8)
pub mod cpu_init;

/// Device drivers - UART, timer, enumeration, TPM, display (P5-P7)
pub mod drivers;

/// Security - Secure boot, TPM, measurement, verification, integrity (P6)
pub mod security;

/// Kernel interface - Loading, handoff, bootstrap, protocol (P0, P4)
pub mod kernel_if;

/// Boot protocol - Protocol definitions and handling
pub mod protocol;

/// Boot orchestration - Flow control, manager, executor, preparation, validation (P0+)
pub mod boot_stage;

/// Boot menu - Unified UI for selection (Graphical/Text/Serial modes)
pub mod boot_menu;

/// Graphics rendering - ARGB8888 framebuffer operations with double buffering
pub mod graphics;

/// Diagnostics - Hardware scanning, timing, logging, profiling (P10)
pub mod diagnostics;

/// Optimization - Parallelization, lazy loading, caching, error mitigation (P2, P3, P10)
pub mod optimization;

/// ACPI support - ACPI parsing, power management (P1, P8)
pub mod acpi_support;

/// Platform abstraction - Console, system info, validation (Utilities)
pub mod platform;

/// Utility library - Error handling, logging, memory utilities, MMIO (Utilities)
pub mod utils;

// Core bootloader traits for dependency injection
pub use utils::boot_traits::{
    MemoryManager, KernelLoader, BootValidator, 
    BootInfoProvider, BootExecutor, DiagnosticReporter
};

// ============================================================================
// MODULE HIERARCHY NOTE
// ============================================================================
//
// This bootloader uses a strict hierarchical module structure.
// All imports should use the full module path (e.g., crate::core::allocator)
// rather than flat imports. This ensures clear dependency relationships
// and maintains architectural consistency.
//
// For migration from flat imports, see the migration guide in
// BOOTLOADER_CODE_REFACTORING_PLAN.md

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================
//
// Core bootloader types that are commonly used across the codebase.
// These are carefully selected to provide essential functionality while
// maintaining the hierarchical structure.

pub use utils::error_recovery::ErrorRecoveryManager;
pub use bios::bios_realmode::{RealModeContext, RealModeExecutor};
pub use kernel_if::kernel_handoff::{BootInformation, BootProtocol};

// ============================================================================
// PANIC HANDLER
// ============================================================================

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    utils::error_recovery::panic_handler(info)
}
