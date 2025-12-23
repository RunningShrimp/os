//! Error registry
//!
//! This module provides error registration and lookup functionality.

use nos_api::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;


use crate::Error;
use crate::Result;
use spin::Mutex;

/// Error registry
pub struct ErrorRegistry {
    /// Registered errors
    errors: BTreeMap<u32, ErrorInfo>,
    /// Errors by name
    errors_by_name: BTreeMap<String, u32>,
    /// Next available error ID
    next_id: u32,
}

impl Default for ErrorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorRegistry {
    /// Create a new error registry
    pub fn new() -> Self {
        Self {
            errors: BTreeMap::new(),
            errors_by_name: BTreeMap::new(),
            next_id: 1000, // Start from 1000 to avoid conflicts
        }
    }

    /// Register an error
    pub fn register(&mut self, name: &str, info: ErrorInfo) -> Result<u32> {
        // Check if error already exists
        if self.errors_by_name.contains_key(name) {
            return Err(Error::InvalidArgument("Error already exists".to_string()));
        }
        
        let id = self.next_id;
        self.next_id += 1;
        
        self.errors.insert(id, info);
        self.errors_by_name.insert(name.into(), id);
        
        Ok(id)
    }

    /// Get an error by ID
    pub fn get(&self, id: u32) -> Option<&ErrorInfo> {
        self.errors.get(&id)
    }

    /// Get an error by name
    pub fn get_by_name(&self, name: &str) -> Option<&ErrorInfo> {
        self.errors_by_name.get(name)
            .and_then(|id| self.errors.get(id))
    }

    /// List all registered errors
    pub fn list(&self) -> Vec<&ErrorInfo> {
        self.errors.values().collect()
    }

    /// Initialize the registry
    pub fn init(&mut self) -> Result<()> {
        // TODO: Initialize error registry
        Ok(())
    }

    /// Shutdown the registry
    pub fn shutdown(&mut self) -> Result<()> {
        // TODO: Shutdown error registry
        Ok(())
    }
}

/// Error information
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// Error name
    pub name: String,
    /// Error description
    pub description: String,
    /// Error category
    pub category: crate::types::ErrorCategory,
    /// Default severity
    pub default_severity: crate::types::ErrorSeverity,
    /// Default recovery strategy
    pub default_recovery_strategy: crate::types::RecoveryStrategy,
    /// Error metadata
    pub metadata: BTreeMap<String, String>,
}

/// Global error registry
static GLOBAL_REGISTRY: spin::Once<Mutex<ErrorRegistry>> = spin::Once::new();

/// Initialize the global error registry
pub fn init_registry() -> Result<()> {
    GLOBAL_REGISTRY.call_once(|| {
        Mutex::new(ErrorRegistry::new())
    });
    Ok(())
}

/// Get the global error registry
pub fn get_registry() -> &'static Mutex<ErrorRegistry> {
    GLOBAL_REGISTRY.get().expect("Error registry not initialized")
}



/// Shutdown the global error registry
pub fn shutdown_registry() -> Result<()> {
    // Note: spin::Once doesn't provide a way to reset, so we just return Ok(())
    // In a real implementation, you might want to provide a different approach
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_registry() {
        let mut registry = ErrorRegistry::new();
        
        // Register a test error
        let info = ErrorInfo {
            name: "test_error".to_string(),
            description: "Test error".to_string(),
            category: crate::types::ErrorCategory::System,
            default_severity: crate::types::ErrorSeverity::Error,
            default_recovery_strategy: crate::types::RecoveryStrategy::Retry,
            metadata: BTreeMap::new(),
        };
        let id = registry.register("test_error", info).unwrap();
        
        // Get error
        let retrieved = registry.get(id).unwrap();
        assert_eq!(retrieved.name, "test_error");
        
        // Get by name
        let retrieved = registry.get_by_name("test_error").unwrap();
        assert_eq!(retrieved.name, "test_error");
    }
}