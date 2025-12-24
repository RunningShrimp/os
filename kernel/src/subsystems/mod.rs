pub mod fs;
pub mod net;
pub mod ipc;
pub mod process;
pub mod microkernel;
pub mod perf;

// Flattened modules from deep nesting

// Migrated infrastructure modules
pub mod mm;
pub mod sync;
pub mod time;

// Optional subsystems (feature-gated)
#[cfg(feature = "cloud_native")]
pub mod cloud_native;

#[cfg(feature = "formal_verification")]
pub mod formal_verification;

// Re-exports for convenience
pub use self::fs as vfs; // vfs alias if needed