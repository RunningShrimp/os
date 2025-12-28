//! Kernel types module

/// Kernel configuration
pub struct KernelConfig {
    /// Debug mode
    pub debug: bool,
    /// Log level
    pub log_level: u32,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            debug: false,
            log_level: 0,
        }
    }
}