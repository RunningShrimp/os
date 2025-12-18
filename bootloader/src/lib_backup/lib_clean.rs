//! NOS Bootloader Library - P0 Phase 2 Focus
//! 
//! This library contains the essential P0 Phase 2 modules:
//! - bios_realmode: Real mode BIOS interrupt handling framework
//! - error_recovery: Boot-time error handling and diagnostics  
//! - kernel_handoff: Boot information and kernel handoff mechanism
//!
//! This is a clean, focused implementation without the legacy bootloader code

#![no_std]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// P0 Phase 2 Core Modules - The focus of this development session
pub mod bios_realmode;
pub mod error_recovery;  
pub mod kernel_handoff;

// Re-export commonly used types
pub use bios_realmode::{RealModeContext, RealModeExecutor};
pub use error_recovery::{BootError, ErrorRecovery};
pub use kernel_handoff::{BootInformation, BootProtocol, KernelHandoff, MemoryMapEntry, LoadedModule};

// Panic handler - integrates with error_recovery for consistent boot failure handling
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error_recovery::panic_handler(info)
}

// Allocator error handler for no_std environment
// Logs allocation failure information and enters panic state
#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    log::error!("Memory allocation failed: requested {} bytes with alignment {}", 
        layout.size(), layout.align());
    // In a real implementation, this would attempt recovery or graceful shutdown
    // For now, we enter an infinite loop (system must be reset)
    loop {
        // Memory allocation failure - system cannot continue
        core::hint::spin_loop();
    }
}
