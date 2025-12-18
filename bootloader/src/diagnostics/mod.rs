//! Diagnostics - Hardware scanning, timing, logging, profiling (P10)

pub mod hardware_scan;
pub mod boot_timing_analysis;
pub mod boot_failure_logger;
pub mod performance_profiling;
pub mod boot_log;
pub mod boot_timer;
pub mod post;
pub mod collector;

// Re-export key diagnostics components
pub use collector::{BootDiagnostics, HardwareInfo, GraphicsStatus};
