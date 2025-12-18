//! Minimal bootloader library for P0 Phase 2
//! Contains only the new P0 Phase 2 modules
//! 
//! This is a simplified version focused on getting Phase 2 modules working

#![no_std]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// P0 Phase 2 Core Modules
pub mod bios_realmode;
pub mod error_recovery;
pub mod kernel_handoff;

// Re-export common types
pub use error_recovery::{BootError, ErrorRecovery};
pub use kernel_handoff::{KernelHandoff, BootInformation, BootProtocol};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error_recovery::panic_handler(info)
}
