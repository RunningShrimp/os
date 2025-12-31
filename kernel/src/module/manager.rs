//! Kernel Module Manager
//!
//! Manages the lifecycle of kernel modules including loading, unloading,
//! dependency resolution, and reference counting.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic {{AtomicUsize,, Ordering}, Ordering};
use crate::module::loader::{ElfModule, LoadError, SymbolTable, Error};

// Import ToString trait for string conversions
use alloc::string::ToString;

/// Module state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// Module is being loaded
    Loading,
    /// Module is active and running
    Active,
    /// Module is being unloaded
    Unloading,
}

/// A loaded kernel module
#[derive(Debug)]
pub struct LoadedModule {
    /// The ELF module
    pub module: ElfModule,
    /// Reference count for dependencies
    pub refcount: AtomicUsize,
    /// Current state
    pub state: ModuleState,
    /// Module dependencies
    pub dependencies: Vec<String>,
    /// Init function address
    pub init_fn: Option<u64>,
    /// Cleanup function address
    pub cleanup_fn: Option<u64>,
}

/// Kernel Module Manager
#[derive(Debug)]
pub struct ModuleManager {
    /// Loaded modules indexed by name
    pub modules: BTreeMap<String, LoadedModule>,
    /// Global kernel symbol table
    pub symbols: SymbolTable,
    /// Dependency graph: module -> list of modules that depend on it
    reverse_deps: BTreeMap<String, Vec<String>>,
}

impl ModuleManager {
    /// Create a new module manager
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            symbols: Self::init_kernel_symbols(),
            reverse_deps: BTreeMap::new(),
        }
    }

    /// Initialize kernel symbol table with exported functions
    fn init_kernel_symbols() -> SymbolTable {
        let mut symbols = SymbolTable::new();

        // Add common kernel functions that modules might need
        // In a real implementation, these would be extracted from the kernel
        symbols.add(String::from("printk"), 0);
        symbols.add(String::from("kmalloc"), 0);
        symbols.add(String::from("kfree"), 0);
        symbols.add(String::from("memcpy"), 0);
        symbols.add(String::from("memset"), 0);
        symbols.add(String::from("strlen"), 0);
        symbols.add(String::from("strcmp"), 0);
        symbols.add(String::from("strcpy"), 0);

        symbols
    }

    /// Load a module from ELF data
    pub fn load_module(&mut self, name: &str, data: &[u8]) -> Result<(), Error> {
        // Check if module is already loaded
        if self.modules.contains_key(name) {
            return Err(Error::Load(LoadError::InvalidElf));
        }

        // Load ELF module
        let elf_module = ElfModule::load(data, name.to_string())?;

        // Resolve symbols against kernel
        elf_module.resolve_symbols(&self.symbols)?;

        // Resolve dependencies
        let dependencies = self.resolve_dependencies(&elf_module)?;

        // Increment reference counts for dependencies
        for dep in &dependencies {
            if let Some(dep_module) = self.modules.get_mut(dep) {
                dep_module.refcount.fetch_add(1, Ordering::SeqCst);

                // Update reverse dependency graph
                self.reverse_deps
                    .entry(dep.clone())
                    .or_insert_with(Vec::new)
                    .push(name.to_string());
            } else {
                return Err(Error::Load(LoadError::MissingSections));
            }
        }

        // Get init and cleanup functions
        let init_fn = elf_module.get_init_fn();
        let cleanup_fn = elf_module.get_cleanup_fn();

        // Create loaded module
        let loaded_module = LoadedModule {
            init_fn,
            cleanup_fn,
            module: elf_module,
            refcount: AtomicUsize::new(0),
            state: ModuleState::Loading,
            dependencies,
        };

        // Insert into module list
        self.modules.insert(name.to_string(), loaded_module);

        // Call init function if present
        if let Some(module) = self.modules.get_mut(name) {
            if let Some(init_addr) = module.init_fn {
                unsafe {
                    let init_fn: extern "C" fn() -> i32 = core::mem::transmute(init_addr);
                    if init_fn() != 0 {
                        // Init failed, clean up
                        self.unload_module_cleanup(name);
                        return Err(Error::Load(LoadError::MemoryError));
                    }
                }
            }
            module.state = ModuleState::Active;
        }

        Ok(())
    }

    /// Unload a module
    pub fn unload_module(&mut self, name: &str) -> Result<(), Error> {
        // Check if module exists
        if !self.modules.contains_key(name) {
            return Err(Error::Load(LoadError::InvalidElf));
        }

        let module = self.modules.get(name).unwrap();

        // Check reference count
        if module.refcount.load(Ordering::SeqCst) > 0 {
            return Err(Error::Load(LoadError::MemoryError));
        }

        // Check for dependent modules
        if let Some(deps) = self.reverse_deps.get(name) {
            if !deps.is_empty() {
                return Err(Error::Load(LoadError::MemoryError));
            }
        }

        // Call cleanup function if present
        if let Some(cleanup_addr) = module.cleanup_fn {
            unsafe {
                let cleanup_fn: extern "C" fn() = core::mem::transmute(cleanup_addr);
                cleanup_fn();
            }
        }

        self.unload_module_cleanup(name);

        Ok(())
    }

    /// Internal cleanup for module unloading
    fn unload_module_cleanup(&mut self, name: &str) {
        if let Some(module) = self.modules.remove(name) {
            // Decrement reference counts for dependencies
            for dep in &module.dependencies {
                if let Some(dep_module) = self.modules.get_mut(dep) {
                    dep_module.refcount.fetch_sub(1, Ordering::SeqCst);

                    // Remove from reverse dependency graph
                    if let Some(reverse) = self.reverse_deps.get_mut(dep) {
                        reverse.retain(|m| m != name);
                    }
                }
            }

            // Remove from reverse dependency graph
            self.reverse_deps.remove(name);

            // Module memory is freed when ElfModule is dropped
        }
    }

    /// Resolve module dependencies
    pub fn resolve_dependencies(&self, module: &ElfModule) -> Result<Vec<String>, Error> {
        let mut dependencies = Vec::new();

        // Check for undefined symbols that might be in other modules
        for symbol in &module.symbols {
            if symbol.value == 0 && !symbol.name.is_empty() {
                // Symbol is undefined, check if it's in another module
                let mut found = false;

                // Check kernel symbols first
                if self.symbols.contains(&symbol.name) {
                    found = true;
                }

                // Check loaded modules
                if !found {
                    for (mod_name, loaded_module) in &self.modules {
                        if loaded_module.module.get_symbol(&symbol.name).is_some() {
                            dependencies.push(mod_name.clone());
                            found = true;
                            break;
                        }
                    }
                }

                if !found {
                    return Err(Error::Load(LoadError::MissingSections));
                }
            }
        }

        // Remove duplicates
        dependencies.sort();
        dependencies.dedup();

        Ok(dependencies)
    }

    /// Get a loaded module by name
    pub fn get_module(&self, name: &str) -> Option<&LoadedModule> {
        self.modules.get(name)
    }

    /// Get a mutable reference to a loaded module
    pub fn get_module_mut(&mut self, name: &str) -> Option<&mut LoadedModule> {
        self.modules.get_mut(name)
    }

    /// Check if a module is loaded
    pub fn is_loaded(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// Get list of all loaded module names
    pub fn list_modules(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    /// Get module information
    pub fn get_module_info(&self, name: &str) -> Option<ModuleInfo> {
        self.modules.get(name).map(|m| ModuleInfo {
            name: name.to_string(),
            state: m.state,
            refcount: m.refcount.load(Ordering::SeqCst),
            dependencies: m.dependencies.clone(),
            text_size: m.module.text_size,
            data_size: m.module.data_size,
            bss_size: m.module.bss_size,
            symbol_count: m.module.symbols.len(),
        })
    }

    /// Export a kernel symbol
    pub fn export_symbol(&mut self, name: String, addr: u64) {
        self.symbols.add(name, addr);
    }

    /// Import a symbol (resolve address)
    pub fn import_symbol(&self, name: &str) -> Option<u64> {
        self.symbols.get(name)
    }

    /// Get all modules
    pub fn all_modules(&self) -> &BTreeMap<String, LoadedModule> {
        &self.modules
    }
}

/// Information about a loaded module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub state: ModuleState,
    pub refcount: usize,
    pub dependencies: Vec<String>,
    pub text_size: usize,
    pub data_size: usize,
    pub bss_size: usize,
    pub symbol_count: usize,
}

impl Default for ModuleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_manager_creation() {
        let manager = ModuleManager::new();
        assert_eq!(manager.modules.len(), 0);
        assert!(!manager.is_loaded("test"));
    }

    #[test]
    fn test_module_list() {
        let manager = ModuleManager::new();
        let modules = manager.list_modules();
        assert_eq!(modules.len(), 0);
    }

    #[test]
    fn test_symbol_export_import() {
        let mut manager = ModuleManager::new();
        manager.export_symbol(String::from("test_symbol"), 0x2000);

        assert_eq!(manager.import_symbol("test_symbol"), Some(0x2000));
        assert_eq!(manager.import_symbol("nonexistent"), None);
    }

    #[test]
    fn test_unload_nonexistent() {
        let mut manager = ModuleManager::new();
        assert!(manager.unload_module("nonexistent").is_err());
    }

    #[test]
    fn test_get_module_info() {
        let manager = ModuleManager::new();
        assert_eq!(manager.get_module_info("test"), None);
    }
}
