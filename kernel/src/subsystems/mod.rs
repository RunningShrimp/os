pub mod fs;
pub mod net;
pub mod ipc;
pub mod process;

// Flattened modules from deep nesting
pub mod process_service;
pub mod glib_memory;

// Re-exports for convenience
pub use self::fs as vfs; // vfs alias if needed