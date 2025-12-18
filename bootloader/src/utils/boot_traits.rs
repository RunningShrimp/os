//! Core bootloader traits for dependency injection
//! 
//! Defines interfaces that modules must implement to reduce coupling
//! and enable loose dependency between components.

use alloc::vec::Vec;

/// Memory management interface
pub trait MemoryManager {
    fn detect_memory(&mut self) -> Result<(), &'static str>;
    fn get_memory_map(&self) -> Vec<(u64, u64)>;
    fn validate_memory(&self) -> bool;
}

/// Kernel loading interface
pub trait KernelLoader {
    fn load_kernel(&mut self) -> Result<u64, &'static str>;
    fn validate_kernel(&self) -> bool;
    fn get_kernel_size(&self) -> u64;
}

/// Boot validation interface
pub trait BootValidator {
    fn validate_all(&self) -> Result<(), &'static str>;
    fn validate_memory(&self) -> bool;
    fn validate_kernel(&self) -> bool;
}

/// Boot information provider
pub trait BootInfoProvider {
    fn build_boot_info(&mut self) -> Result<(), &'static str>;
    fn is_ready(&self) -> bool;
}

/// Boot executor trait
pub trait BootExecutor {
    fn execute(&mut self) -> Result<(), &'static str>;
    fn is_ready(&self) -> bool;
}

/// Diagnostic reporter
pub trait DiagnosticReporter {
    fn report(&self) -> &'static str;
    fn log_event(&mut self, event: &'static str);
}
