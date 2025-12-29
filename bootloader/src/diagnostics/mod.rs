//! Diagnostics - Hardware scanning, timing, logging, profiling (P10)

pub mod boot_failure_logger;
pub mod boot_log;
pub mod boot_timer;
pub mod boot_timing_analysis;
pub mod collector;
pub mod hardware_scan;
pub mod performance_profiling;
pub mod post;

// Re-export key diagnostics components
pub use collector::{BootDiagnostics, GraphicsStatus, HardwareInfo};
