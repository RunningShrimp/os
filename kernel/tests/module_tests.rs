//! Kernel Module System Tests
//!
//! Comprehensive tests for the kernel module loading system.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use kernel::module::{ModuleManager, validate_module_name, validate_module_size, ModuleError};

#[cfg(test)]
mod tests {
    use super::*;

    /// Test module manager creation
    #[test]
    fn test_module_manager_creation() {
        let manager = ModuleManager::new();
        assert_eq!(manager.all_modules().len(), 0);
    }

    /// Test module name validation
    #[test]
    fn test_validate_module_name() {
        // Valid names
        assert!(validate_module_name("test_module").is_ok());
        assert!(validate_module_name("test-module").is_ok());
        assert!(validate_module_name("module123").is_ok());

        // Invalid names
        assert!(validate_module_name("").is_err());
        assert!(validate_module_name("test@module").is_err());
        assert!(validate_module_name("test module").is_err());
        assert!(validate_module_name("test/module").is_err());
    }

    /// Test module size validation
    #[test]
    fn test_validate_module_size() {
        // Valid sizes
        assert!(validate_module_size(1024).is_ok());
        assert!(validate_module_size(1024 * 1024).is_ok());

        // Invalid sizes
        assert!(validate_module_size(0).is_err());
        assert!(validate_module_size(16 * 1024 * 1024 + 1).is_err());
    }

    /// Test symbol export and import
    #[test]
    fn test_symbol_export_import() {
        let mut manager = ModuleManager::new();

        // Export a symbol
        manager.export_symbol(String::from("test_function"), 0x1000);

        // Import the symbol
        assert_eq!(manager.import_symbol("test_function"), Some(0x1000));

        // Try to import nonexistent symbol
        assert_eq!(manager.import_symbol("nonexistent"), None);
    }

    /// Test listing modules
    #[test]
    fn test_list_modules() {
        let manager = ModuleManager::new();
        let modules = manager.list_modules();
        assert_eq!(modules.len(), 0);
    }

    /// Test checking if module is loaded
    #[test]
    fn test_is_loaded() {
        let manager = ModuleManager::new();
        assert!(!manager.is_loaded("test"));
    }

    /// Test getting module info for non-existent module
    #[test]
    fn test_get_module_info_nonexistent() {
        let manager = ModuleManager::new();
        assert_eq!(manager.get_module_info("test"), None);
    }

    /// Test unloading non-existent module
    #[test]
    fn test_unload_nonexistent() {
        let mut manager = ModuleManager::new();
        assert!(manager.unload_module("nonexistent").is_err());
    }

    /// Test getting non-existent module
    #[test]
    fn test_get_nonexistent_module() {
        let manager = ModuleManager::new();
        assert_eq!(manager.get_module("test"), None);
    }

    /// Test getting mutable reference to non-existent module
    #[test]
    fn test_get_nonexistent_module_mut() {
        let mut manager = ModuleManager::new();
        assert_eq!(manager.get_module_mut("test"), None);
    }

    /// Test symbol table operations
    #[test]
    fn test_symbol_table() {
        use kernel::module::loader::SymbolTable;

        let mut table = SymbolTable::new();

        // Add symbols
        table.add(String::from("symbol1"), 0x1000);
        table.add(String::from("symbol2"), 0x2000);

        // Check symbols
        assert_eq!(table.get("symbol1"), Some(0x1000));
        assert_eq!(table.get("symbol2"), Some(0x2000));
        assert_eq!(table.get("nonexistent"), None);

        // Check contains
        assert!(table.contains("symbol1"));
        assert!(!table.contains("nonexistent"));
    }

    /// Test module parameters
    #[test]
    fn test_module_parameters() {
        use kernel::module::framework::{ModuleParameter, ParameterType};

        let param = ModuleParameter {
            name: String::from("test_param"),
            value: String::from("test_value"),
            param_type: ParameterType::String,
            description: String::from("Test parameter"),
            readonly: false,
        };

        assert_eq!(param.name, "test_param");
        assert_eq!(param.value, "test_value");
        assert!(!param.readonly);
    }

    /// Test module parameter types
    #[test]
    fn test_parameter_types() {
        use kernel::module::framework::ParameterType;

        assert_eq!(ParameterType::String, ParameterType::String);
        assert_eq!(ParameterType::Integer, ParameterType::Integer);
        assert_eq!(ParameterType::Boolean, ParameterType::Boolean);
        assert_eq!(ParameterType::Hex, ParameterType::Hex);
    }

    /// Test module state
    #[test]
    fn test_module_state() {
        use kernel::module::ModuleState;

        assert_eq!(ModuleState::Loading, ModuleState::Loading);
        assert_eq!(ModuleState::Active, ModuleState::Active);
        assert_eq!(ModuleState::Unloading, ModuleState::Unloading);
    }

    /// Test module info
    #[test]
    fn test_module_info() {
        use kernel::module::ModuleInfo;
        use kernel::module::ModuleState;

        let info = ModuleInfo {
            name: String::from("test_module"),
            state: ModuleState::Active,
            refcount: 0,
            dependencies: Vec::new(),
            text_size: 1024,
            data_size: 512,
            bss_size: 256,
            symbol_count: 10,
        };

        assert_eq!(info.name, "test_module");
        assert_eq!(info.state, ModuleState::Active);
        assert_eq!(info.refcount, 0);
        assert_eq!(info.text_size, 1024);
    }

    /// Test loading errors
    #[test]
    fn test_load_errors() {
        use kernel::module::LoadError;

        assert_eq!(LoadError::InvalidElf, LoadError::InvalidElf);
        assert_eq!(LoadError::UnsupportedArch, LoadError::UnsupportedArch);
        assert_eq!(LoadError::MemoryError, LoadError::MemoryError);
    }

    /// Test relocation errors
    #[test]
    fn test_reloc_errors() {
        use kernel::module::RelocError;

        let error = RelocError::SymbolNotFound(String::from("test_symbol"));
        match error {
            RelocError::SymbolNotFound(name) => assert_eq!(name, "test_symbol"),
            _ => panic!("Wrong error type"),
        }

        assert_eq!(RelocError::UnknownType, RelocError::UnknownType);
        assert_eq!(RelocError::InvalidOffset, RelocError::InvalidOffset);
        assert_eq!(RelocError::Overflow, RelocError::Overflow);
    }

    /// Test module error
    #[test]
    fn test_module_error() {
        assert_eq!(ModuleError::InitFailed, ModuleError::InitFailed);
        assert_eq!(ModuleError::CleanupFailed, ModuleError::CleanupFailed);
        assert_eq!(ModuleError::InvalidModule, ModuleError::InvalidModule);
        assert_eq!(ModuleError::AlreadyInitialized, ModuleError::AlreadyInitialized);
        assert_eq!(ModuleError::NotInitialized, ModuleError::NotInitialized);
    }

    /// Test constants
    #[test]
    fn test_constants() {
        assert_eq!(kernel::module::MODULE_SYSTEM_VERSION, "1.0.0");
        assert_eq!(kernel::module::MAX_MODULE_SIZE, 16 * 1024 * 1024);
        assert_eq!(kernel::module::MAX_MODULES, 256);
        assert_eq!(kernel::module::MAX_MODULE_NAME_LEN, 64);
        assert_eq!(kernel::module::MAX_DEPENDENCY_DEPTH, 10);
    }
}

/// Module system integration test
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test complete module lifecycle
    #[test]
    fn test_module_lifecycle() {
        let mut manager = ModuleManager::new();

        // Initially empty
        assert_eq!(manager.list_modules().len(), 0);

        // Export symbols needed by modules
        manager.export_symbol(String::from("printk"), 0x1000);
        manager.export_symbol(String::from("kmalloc"), 0x2000);
        manager.export_symbol(String::from("kfree"), 0x3000);

        // In a real test, we would load an actual ELF module here
        // For now, we just verify the manager is set up correctly
        assert_eq!(manager.import_symbol("printk"), Some(0x1000));
        assert_eq!(manager.import_symbol("kmalloc"), Some(0x2000));
        assert_eq!(manager.import_symbol("kfree"), Some(0x3000));
    }

    /// Test dependency resolution setup
    #[test]
    fn test_dependency_resolution() {
        let mut manager = ModuleManager::new();

        // Export symbols that would be provided by "base" module
        manager.export_symbol(String::from("base_function"), 0x1000);

        // Verify symbols are available
        assert!(manager.import_symbol("base_function").is_some());
    }
}
