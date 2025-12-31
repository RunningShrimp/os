//! Kernel Module Framework
//!
//! Provides the framework and trait definitions for kernel modules,
//! including the module declaration macro and lifecycle management.

use alloc::string::String;

/// Result type for module operations
pub type ModuleResult<T> = core::result::Result<T, ModuleError>;

/// Errors that can occur in module operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleError {
    /// Initialization failed
    InitFailed,
    /// Cleanup failed
    CleanupFailed,
    /// Invalid module
    InvalidModule,
    /// Module already initialized
    AlreadyInitialized,
    /// Module not initialized
    NotInitialized,
}

/// Trait that all kernel modules must implement
pub trait Module {
    /// Initialize the module
    ///
    /// This is called when the module is loaded. It should set up any
    /// necessary data structures, register handlers, etc.
    fn init(&mut self) -> ModuleResult<()>;

    /// Clean up the module
    ///
    /// This is called when the module is unloaded. It should release all
    /// resources and unregister any handlers.
    fn cleanup(&mut self) -> ModuleResult<()>;

    /// Get the module name
    fn name(&self) -> &str;

    /// Get the module version
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// Get the module author
    fn author(&self) -> &str {
        "Unknown"
    }

    /// Get the module description
    fn description(&self) -> &str {
        ""
    }

    /// Get module parameters (for configuration)
    fn parameters(&self) -> &[ModuleParameter] {
        &[]
    }

    /// Handle a module parameter change
    fn set_parameter(&mut self, _name: &str, _value: &str) -> ModuleResult<()> {
        Ok(())
    }
}

/// Module parameter for runtime configuration
#[derive(Debug, Clone)]
pub struct ModuleParameter {
    pub name: String,
    pub value: String,
    pub param_type: ParameterType,
    pub description: String,
    pub readonly: bool,
}

/// Parameter types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterType {
    String,
    Integer,
    Boolean,
    Hex,
}

/// Declare a kernel module
///
/// This macro generates the necessary boilerplate for a kernel module,
/// including the `_module_init` and `_module_cleanup` functions that are
/// called by the module loader.
///
/// # Example
///
/// ```rust
/// declare_module!(MyModule);
///
/// struct MyModule;
///
/// impl Module for MyModule {
///     fn init(&mut self) -> ModuleResult<()> {
///         Ok(())
///     }
///
///     fn cleanup(&mut self) -> ModuleResult<()> {
///         Ok(())
///     }
///
///     fn name(&self) -> &str {
///         "mymodule"
///     }
/// }
/// ```
#[macro_export]
macro_rules! declare_module {
    ($module_type:ty) => {
        #[no_mangle]
        pub extern "C" fn _module_init() -> *mut u8 {
            use alloc::boxed::Box;
            use $crate::module::framework::Module;

            let module: Box<$module_type> = Box::new(<$module_type>::default());
            let mut module = Box::into_raw(module);

            unsafe {
                if let Err(_) = (*module).init() {
                    // Init failed, clean up
                    let _ = Box::from_raw(module);
                    return core::ptr::null_mut();
                }
            }

            module as *mut u8
        }

        #[no_mangle]
        pub extern "C" fn _module_cleanup(ptr: *mut u8) {
            use alloc::boxed::Box;
            use $crate::module::framework::Module;

            if ptr.is_null() {
                return;
            }

            unsafe {
                let module = Box::from_raw(ptr as *mut $module_type);
                let _ = module.cleanup();
            }
        }

        #[no_mangle]
        pub extern "C" fn _module_name() -> *const u8 {
            use $crate::module::framework::Module;
            use alloc::string::String;

            let temp: <$module_type> = <$module_type>::default();
            let name = temp.name();
            let name_string = String::from(name);

            // Leak the string to make it static
            Box::leak(name_string.into_boxed_str()).as_ptr()
        }

        #[no_mangle]
        pub extern "C" fn _module_version() -> *const u8 {
            use $crate::module::framework::Module;
            use alloc::string::String;

            let temp: <$module_type> = <$module_type>::default();
            let version = temp.version();
            let version_string = String::from(version);

            Box::leak(version_string.into_boxed_str()).as_ptr()
        }
    };
}

/// Export a symbol for use by modules
///
/// This makes a kernel function or variable available to dynamically loaded modules.
///
/// # Safety
///
/// The symbol must be valid and have a stable address.
///
/// Note: This is a placeholder function. In a real implementation, this would
/// interface with the global module manager.
pub fn export_symbol(_name: &str, _addr: u64) {
    // Placeholder: Would export to global module manager in real implementation
    // This requires static mutable state or proper synchronization
}

/// Import a symbol from the kernel or another module
///
/// Returns the address of the symbol if found, None otherwise.
///
/// Note: This is a placeholder function. In a real implementation, this would
/// interface with the global module manager.
pub fn import_symbol(_name: &str) -> Option<u64> {
    // Placeholder: Would import from global module manager in real implementation
    None
}

/// Module context - provides access to kernel services
pub trait ModuleContext {
    /// Log a message
    fn log(&self, message: &str);

    /// Allocate memory
    fn allocate(&self, size: usize) -> Option<*mut u8>;

    /// Free memory
    fn deallocate(&self, ptr: *mut u8, size: usize);

    /// Register a callback
    fn register_callback(&mut self, name: String, callback: extern "C" fn()) -> ModuleResult<()>;

    /// Unregister a callback
    fn unregister_callback(&mut self, name: String) -> ModuleResult<()>;
}

/// Simple module context implementation
pub struct SimpleModuleContext;

impl ModuleContext for SimpleModuleContext {
    fn log(&self, message: &str) {
        // In a real implementation, this would use the kernel logger
        let _ = message;
    }

    fn allocate(&self, size: usize) -> Option<*mut u8> {
        unsafe {
            let layout = core::alloc::Layout::from_size_align(size, 8).ok()?;
            let ptr = alloc::alloc::alloc(layout);
            if ptr.is_null() {
                None
            } else {
                Some(ptr)
            }
        }
    }

    fn deallocate(&self, ptr: *mut u8, size: usize) {
        unsafe {
            let layout = core::alloc::Layout::from_size_align_unchecked(size, 8);
            alloc::alloc::dealloc(ptr, layout);
        }
    }

    fn register_callback(&mut self, _name: String, _callback: extern "C" fn()) -> ModuleResult<()> {
        Ok(())
    }

    fn unregister_callback(&mut self, _name: String) -> ModuleResult<()> {
        Ok(())
    }
}

/// Helper macro for creating module parameters
#[macro_export]
macro_rules! module_param {
    (string, $name:expr, $default:expr, $desc:expr) => {
        $crate::module::framework::ModuleParameter {
            name: String::from($name),
            value: String::from($default),
            param_type: $crate::module::framework::ParameterType::String,
            description: String::from($desc),
            readonly: false,
        }
    };

    (int, $name:expr, $default:expr, $desc:expr) => {
        $crate::module::framework::ModuleParameter {
            name: String::from($name),
            value: String::from($default),
            param_type: $crate::module::framework::ParameterType::Integer,
            description: String::from($desc),
            readonly: false,
        }
    };

    (bool, $name:expr, $default:expr, $desc:expr) => {
        $crate::module::framework::ModuleParameter {
            name: String::from($name),
            value: String::from($default),
            param_type: $crate::module::framework::ParameterType::Boolean,
            description: String::from($desc),
            readonly: false,
        }
    };

    (hex, $name:expr, $default:expr, $desc:expr) => {
        $crate::module::framework::ModuleParameter {
            name: String::from($name),
            value: String::from($default),
            param_type: $crate::module::framework::ParameterType::Hex,
            description: String::from($desc),
            readonly: false,
        }
    };

    (readonly string, $name:expr, $default:expr, $desc:expr) => {
        $crate::module::framework::ModuleParameter {
            name: String::from($name),
            value: String::from($default),
            param_type: $crate::module::framework::ParameterType::String,
            description: String::from($desc),
            readonly: true,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestModule;

    impl Default for TestModule {
        fn default() -> Self {
            TestModule
        }
    }

    impl Module for TestModule {
        fn init(&mut self) -> ModuleResult<()> {
            Ok(())
        }

        fn cleanup(&mut self) -> ModuleResult<()> {
            Ok(())
        }

        fn name(&self) -> &str {
            "testmodule"
        }

        fn version(&self) -> &str {
            "0.1.0"
        }

        fn author(&self) -> &str {
            "Test Author"
        }
    }

    #[test]
    fn test_module_trait() {
        let module = TestModule;
        assert_eq!(module.name(), "testmodule");
        assert_eq!(module.version(), "0.1.0");
        assert_eq!(module.author(), "Test Author");
    }

    #[test]
    fn test_module_parameter_macro() {
        let param = module_param!(string, "test_param", "default", "Test parameter");
        assert_eq!(param.name, "test_param");
        assert_eq!(param.value, "default");
        assert!(!param.readonly);
    }

    #[test]
    fn test_simple_context() {
        let ctx = SimpleModuleContext;
        let ptr = ctx.allocate(16);
        assert!(ptr.is_some());
        if let Some(p) = ptr {
            ctx.deallocate(p, 16);
        }
    }
}
