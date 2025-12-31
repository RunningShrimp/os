//! Example Kernel Module
//!
//! This module demonstrates how to create a kernel module using the module framework.

use alloc::vec::Vec;
use crate::module::framework::{Module, ModuleResult, ModuleError, ModuleContext, module_param, ParameterType};

/// Example module that demonstrates the module framework
pub struct ExampleModule {
    /// Module counter
    counter: usize,
    /// Module context
    context: ExampleContext,
    /// Module parameters
    parameters: Vec<ModuleParameter>,
}

/// Context for the example module
pub struct ExampleContext {
    /// Log buffer
    log_buffer: Vec<String>,
}

impl ExampleContext {
    pub fn new() -> Self {
        Self {
            log_buffer: Vec::new(),
        }
    }

    pub fn get_logs(&self) -> &[String] {
        &self.log_buffer
    }
}

impl Default for ExampleModule {
    fn default() -> Self {
        Self {
            counter: 0,
            context: ExampleContext::new(),
            parameters: vec![
                module_param!(int, "counter", "0", "Module counter"),
                module_param!(string, "message", "Hello from module!", "Default message"),
                module_param!(bool, "enabled", "true", "Enable module features"),
            ],
        }
    }
}

impl Module for ExampleModule {
    /// Initialize the module
    fn init(&mut self) -> ModuleResult<()> {
        self.context.log("Example module initializing...");

        // Initialize counter from parameter
        if let Some(param) = self.parameters.iter().find(|p| p.name == "counter") {
            self.counter = param.value.parse().unwrap_or(0);
        }

        // Perform module initialization
        self.counter = self.counter.wrapping_add(1);

        self.context.log(&format!("Module initialized with counter = {}", self.counter));

        // Register callbacks or resources here
        // For example: register_device_handler, create_proc_entry, etc.

        Ok(())
    }

    /// Clean up the module
    fn cleanup(&mut self) -> ModuleResult<()> {
        self.context.log("Example module cleaning up...");

        // Unregister callbacks and release resources
        // For example: unregister_device_handler, remove_proc_entry, etc.

        self.context.log("Module cleanup complete");

        Ok(())
    }

    /// Get module name
    fn name(&self) -> &str {
        "example"
    }

    /// Get module version
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// Get module author
    fn author(&self) -> &str {
        "Kernel Developer"
    }

    /// Get module description
    fn description(&self) -> &str {
        "An example kernel module demonstrating the module framework"
    }

    /// Get module parameters
    fn parameters(&self) -> &[ModuleParameter] {
        &self.parameters
    }

    /// Set a module parameter
    fn set_parameter(&mut self, name: &str, value: &str) -> ModuleResult<()> {
        if let Some(param) = self.parameters.iter_mut().find(|p| p.name == name) {
            if param.readonly {
                return Err(ModuleError::InvalidModule);
            }

            // Validate parameter value based on type
            match param.param_type {
                ParameterType::Integer => {
                    if value.parse::<i64>().is_err() {
                        return Err(ModuleError::InvalidModule);
                    }
                }
                ParameterType::Boolean => {
                    if value != "true" && value != "false" && value != "0" && value != "1" {
                        return Err(ModuleError::InvalidModule);
                    }
                }
                ParameterType::Hex => {
                    if !value.starts_with("0x") && !value.starts_with("0X") {
                        return Err(ModuleError::InvalidModule);
                    }
                    if value[2..].parse::<u64>().is_err() {
                        return Err(ModuleError::InvalidModule);
                    }
                }
                ParameterType::String => {
                    // Any string is valid
                }
            }

            param.value = value.to_string();
            Ok(())
        } else {
            Err(ModuleError::InvalidModule)
        }
    }
}

impl ExampleModule {
    /// Get the current counter value
    pub fn get_counter(&self) -> usize {
        self.counter
    }

    /// Increment the counter
    pub fn increment_counter(&mut self) {
        self.counter = self.counter.wrapping_add(1);
    }

    /// Get the context logs
    pub fn get_logs(&self) -> &[String] {
        self.context.get_logs()
    }

    /// Log a message
    pub fn log(&mut self, message: &str) {
        self.context.log(message);
    }
}

impl ModuleContext for ExampleContext {
    fn log(&self, message: &str) {
        // In a real implementation, this would use the kernel logger
        // For now, we just store it in the buffer
        // Note: This would need interior mutability in a real scenario
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

    fn register_callback(&mut self, name: String, _callback: extern "C" fn()) -> ModuleResult<()> {
        self.log_buffer.push(format!("Registered callback: {}", name));
        Ok(())
    }

    fn unregister_callback(&mut self, name: String) -> ModuleResult<()> {
        self.log_buffer.push(format!("Unregistered callback: {}", name));
        Ok(())
    }
}

/// Module tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_info() {
        let module = ExampleModule::default();
        assert_eq!(module.name(), "example");
        assert_eq!(module.version(), "1.0.0");
        assert_eq!(module.author(), "Kernel Developer");
    }

    #[test]
    fn test_module_init() {
        let mut module = ExampleModule::default();
        assert!(module.init().is_ok());
        assert_eq!(module.counter, 1);
    }

    #[test]
    fn test_module_cleanup() {
        let mut module = ExampleModule::default();
        module.init().unwrap();
        assert!(module.cleanup().is_ok());
    }

    #[test]
    fn test_counter_operations() {
        let mut module = ExampleModule::default();
        assert_eq!(module.get_counter(), 0);
        module.increment_counter();
        assert_eq!(module.get_counter(), 1);
    }

    #[test]
    fn test_parameters() {
        let module = ExampleModule::default();
        let params = module.parameters();
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "counter");
        assert_eq!(params[1].name, "message");
        assert_eq!(params[2].name, "enabled");
    }

    #[test]
    fn test_set_parameter() {
        let mut module = ExampleModule::default();
        assert!(module.set_parameter("counter", "42").is_ok());
        assert_eq!(module.parameters()[0].value, "42");

        // Test invalid parameter
        assert!(module.set_parameter("nonexistent", "value").is_err());
    }

    #[test]
    fn test_parameter_validation() {
        let mut module = ExampleModule::default();

        // Valid integer
        assert!(module.set_parameter("counter", "123").is_ok());

        // Invalid integer
        assert!(module.set_parameter("counter", "abc").is_err());

        // Valid boolean
        assert!(module.set_parameter("enabled", "true").is_ok());
        assert!(module.set_parameter("enabled", "false").is_ok());
        assert!(module.set_parameter("enabled", "1").is_ok());
        assert!(module.set_parameter("enabled", "0").is_ok());

        // Invalid boolean
        assert!(module.set_parameter("enabled", "yes").is_err());

        // Valid hex
        assert!(module.set_parameter("counter", "0x1234").is_ok());

        // Invalid hex
        assert!(module.set_parameter("counter", "0xxyz").is_err());
    }
}
