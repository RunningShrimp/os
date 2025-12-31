//! Kernel Module System
//!
//! This module provides a complete dynamic kernel module loading system,
//! allowing the kernel to load and unload code at runtime. It includes:
//!
//! - ELF module loading and parsing
//! - Symbol resolution and dependency management
//! - Module lifecycle management
//! - Safe module initialization and cleanup
//!
//! # Architecture
//!
//! The module system is split into several components:
//!
//! - **Loader**: Handles ELF parsing, relocation, and symbol resolution
//! - **Manager**: Manages module lifecycle, dependencies, and reference counting
//! - **Framework**: Provides the trait and macro definitions for modules
//!
//! # Example
//!
//! ```rust
//! use kernel::module::ModuleManager;
//!
//! let mut manager = ModuleManager::new();
//! let module_data = std::fs::read("mymodule.ko")?;
//! manager.load_module("mymodule", &module_data)?;
//! ```

pub mod loader;
pub mod manager;
pub mod framework;

pub use loader::{ElfModule, Symbol, SymbolTable, LoadError, RelocError};
pub use manager::{ModuleManager, LoadedModule, ModuleState, ModuleInfo};
pub use framework::{Module, ModuleContext, ModuleError, ModuleResult, ModuleParameter, ParameterType};

/// Kernel module system version
pub const MODULE_SYSTEM_VERSION: &str = "1.0.0";

/// Maximum module size (16MB)
pub const MAX_MODULE_SIZE: usize = 16 * 1024 * 1024;

/// Maximum number of modules
pub const MAX_MODULES: usize = 256;

/// Maximum module name length
pub const MAX_MODULE_NAME_LEN: usize = 64;

/// Maximum dependency depth
pub const MAX_DEPENDENCY_DEPTH: usize = 10;

/// Initialize the module system
///
/// This function should be called during kernel initialization to set up
/// the global module manager and export kernel symbols.
pub fn init_module_system() -> Result<ModuleManager, ModuleError> {
    let manager = ModuleManager::new();

    // Export essential kernel symbols
    export_kernel_symbols(&manager);

    Ok(manager)
}

/// Export kernel symbols for use by modules
fn export_kernel_symbols(_manager: &ModuleManager) {
    // In a real implementation, this would export all necessary kernel symbols
    // For now, this is a placeholder

    // Common symbols that modules might need:
    // - Memory allocation functions
    // - String manipulation functions
    // - Logging functions
    // - Synchronization primitives
    // - File system operations
    // - Network operations
    // etc.
}

/// Validate module name
pub fn validate_module_name(name: &str) -> Result<(), ModuleError> {
    if name.is_empty() {
        return Err(ModuleError::InvalidModule);
    }

    if name.len() > MAX_MODULE_NAME_LEN {
        return Err(ModuleError::InvalidModule);
    }

    // Check for valid characters (alphanumeric and underscore)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(ModuleError::InvalidModule);
    }

    Ok(())
}

/// Check if module size is within limits
pub fn validate_module_size(size: usize) -> Result<(), ModuleError> {
    if size > MAX_MODULE_SIZE {
        return Err(ModuleError::InvalidModule);
    }

    if size == 0 {
        return Err(ModuleError::InvalidModule);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_module_name() {
        assert!(validate_module_name("test_module").is_ok());
        assert!(validate_module_name("test-module").is_ok());
        assert!(validate_module_name("").is_err());
        assert!(validate_module_name("test@module").is_err());
    }

    #[test]
    fn test_validate_module_size() {
        assert!(validate_module_size(1024).is_ok());
        assert!(validate_module_size(0).is_err());
        assert!(validate_module_size(MAX_MODULE_SIZE + 1).is_err());
    }

    #[test]
    fn test_constants() {
        assert_eq!(MODULE_SYSTEM_VERSION, "1.0.0");
        assert_eq!(MAX_MODULE_SIZE, 16 * 1024 * 1024);
        assert_eq!(MAX_MODULES, 256);
    }
}
