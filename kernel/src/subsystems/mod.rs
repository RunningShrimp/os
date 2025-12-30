pub mod drivers;
pub mod fs;
pub mod ipc;
pub mod microkernel;
pub mod net;
pub mod perf;
pub mod posix;
pub mod process;
pub mod scheduler;
pub mod services;
pub mod syscalls;

// Flattened modules from deep nesting

// Migrated infrastructure modules
pub mod mm;
pub mod sync;
pub mod time;

// Optional subsystems (feature-gated)
pub mod cloud_native;

#[cfg(feature = "formal_verification")]
pub mod formal_verification;
