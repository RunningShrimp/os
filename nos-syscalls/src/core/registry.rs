//! System call registry
//!
//! This module provides the system call registry functionality.

use nos_api::Result;
#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;
#[cfg(feature = "alloc")]
use alloc::string::{String, ToString};
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use spin::{Mutex, Once};

/// System call registry
pub struct SyscallRegistry {
    /// Registered system calls
    #[cfg(feature = "alloc")]
    syscalls: BTreeMap<u32, SyscallInfo>,
    #[cfg(not(feature = "alloc"))]
    syscalls: core::marker::PhantomData<u32>,
    /// Next available system call ID
    next_id: u32,
}

impl SyscallRegistry {
    /// Create a new system call registry
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "alloc")]
            syscalls: BTreeMap::new(),
            #[cfg(not(feature = "alloc"))]
            syscalls: core::marker::PhantomData,
            next_id: 1000, // Start from 1000 to avoid conflicts
        }
    }

    /// Register a system call
    #[cfg(feature = "alloc")]
    pub fn register(&mut self, name: &str, handler: Box<dyn SyscallHandler>) -> Result<u32> {
        let id = self.next_id;
        self.next_id += 1;
        
        let info = SyscallInfo {
            id,
            name: name.to_string(),
            handler,
        };
        
        self.syscalls.insert(id, info);
        Ok(id)
    }

    // No-alloc version doesn't have a register method that uses Box
    // since Box requires alloc feature

    /// Get system call info by ID
    #[cfg(feature = "alloc")]
    pub fn get(&self, id: u32) -> Option<&SyscallInfo> {
        self.syscalls.get(&id)
    }

    /// Get system call handler by ID (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    pub fn get(&self, _id: u32) -> Option<&dyn SyscallHandler> {
        // TODO: Implement for no-alloc
        None
    }

    /// Get system call info by name
    #[cfg(feature = "alloc")]
    pub fn get_by_name(&self, name: &str) -> Option<&SyscallInfo> {
        self.syscalls.values().find(|info| info.name == name)
    }

    /// Get system call handler by name (no-alloc version)
    #[cfg(not(feature = "alloc"))]
    pub fn get_by_name(&self, _name: &str) -> Option<&dyn SyscallHandler> {
        // TODO: Implement for no-alloc
        None
    }

    /// List all registered system calls
    #[cfg(feature = "alloc")]
    pub fn list(&self) -> alloc::vec::Vec<&SyscallInfo> {
        self.syscalls.values().collect()
    }
}

/// System call information
#[cfg(feature = "alloc")]
pub struct SyscallInfo {
    /// System call ID
    pub id: u32,
    /// System call name
    pub name: String,
    /// System call handler
    pub handler: Box<dyn SyscallHandler>,
}

/// System call handler trait
pub trait SyscallHandler: Send + Sync {
    /// Execute the system call
    fn execute(&self, args: &[usize]) -> Result<isize>;
    
    /// Get the system call name
    fn name(&self) -> &str;
}

/// Global system call registry
static GLOBAL_REGISTRY: Once<Mutex<SyscallRegistry>> = Once::new();

/// Initialize the global system call registry
pub fn init_registry() -> Result<()> {
    GLOBAL_REGISTRY.call_once(|| Mutex::new(SyscallRegistry::new()));
    Ok(())
}

/// Get the global system call registry
pub fn get_registry() -> &'static Mutex<SyscallRegistry> {
    GLOBAL_REGISTRY.get().expect("Registry not initialized")
}

/// Register a system call
#[cfg(feature = "alloc")]
pub fn register_syscall(name: &str, handler: Box<dyn SyscallHandler>) -> Result<u32> {
    let mut registry = get_registry().lock();
    registry.register(name, handler)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        name: &'static str,
        result: isize,
    }

    impl SyscallHandler for TestHandler {
        fn execute(&self, _args: &[usize]) -> Result<isize> {
            Ok(self.result)
        }
        
        fn name(&self) -> &str {
            self.name
        }
    }

    #[test]
    fn test_registry() {
        let mut registry = SyscallRegistry::new();
        
        // Register a test system call
        let handler = TestHandler {
            name: "test_syscall",
            result: 42,
        };
        let id = registry.register("test_syscall", Box::new(handler)).unwrap();
        
        // Get the system call
        let info = registry.get(id).unwrap();
        assert_eq!(info.name, "test_syscall");
        
        // Get by name
        let info = registry.get_by_name("test_syscall").unwrap();
        assert_eq!(info.id, id);
    }
}