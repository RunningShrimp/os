//! Modular System Call Framework
//!
//! This module provides a modular framework for organizing and managing
//! system calls in NOS operating system, improving maintainability.

use alloc::{
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
    string::{String, ToString},
    boxed::Box,
    format,
};
use nos_api::Result;
use crate::SyscallHandler;
use core::sync::atomic::{AtomicU64, Ordering};

/// System call category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyscallCategory {
    /// File system operations
    FileSystem = 0,
    /// Memory management
    Memory = 1,
    /// Process management
    Process = 2,
    /// Network operations
    Network = 3,
    /// Device I/O
    Device = 4,
    /// Time and timers
    Time = 5,
    /// Synchronization primitives
    Sync = 6,
    /// System information
    System = 7,
    /// Security and permissions
    Security = 8,
    /// Performance monitoring
    Performance = 9,
}

impl SyscallCategory {
    /// Get category name
    pub fn name(&self) -> &'static str {
        match self {
            Self::FileSystem => "FileSystem",
            Self::Memory => "Memory",
            Self::Process => "Process",
            Self::Network => "Network",
            Self::Device => "Device",
            Self::Time => "Time",
            Self::Sync => "Sync",
            Self::System => "System",
            Self::Security => "Security",
            Self::Performance => "Performance",
        }
    }
    
    /// Get category description
    pub fn description(&self) -> &'static str {
        match self {
            Self::FileSystem => "File system operations (open, read, write, close, etc.)",
            Self::Memory => "Memory management (mmap, munmap, alloc, etc.)",
            Self::Process => "Process management (fork, exec, wait, etc.)",
            Self::Network => "Network operations (socket, bind, connect, etc.)",
            Self::Device => "Device I/O operations (ioctl, etc.)",
            Self::Time => "Time and timer operations (gettimeofday, etc.)",
            Self::Sync => "Synchronization primitives (mutex, semaphore, etc.)",
            Self::System => "System information (uname, etc.)",
            Self::Security => "Security and permissions (setuid, etc.)",
            Self::Performance => "Performance monitoring and profiling",
        }
    }
}

/// System call metadata
#[derive(Debug, Clone)]
pub struct SyscallMetadata {
    /// System call ID
    pub id: u32,
    /// System call name
    pub name: String,
    /// Category
    pub category: SyscallCategory,
    /// Description
    pub description: String,
    /// Number of arguments
    pub arg_count: u32,
    /// Return type
    pub return_type: String,
    /// Version
    pub version: u32,
    /// Deprecated flag
    pub deprecated: bool,
    /// Security level required
    pub security_level: u32,
}

impl SyscallMetadata {
    /// Create new syscall metadata
    pub fn new(
        id: u32,
        name: String,
        category: SyscallCategory,
        description: String,
        arg_count: u32,
        return_type: String,
    ) -> Self {
        Self {
            id,
            name,
            category,
            description,
            arg_count,
            return_type,
            version: 1,
            deprecated: false,
            security_level: 0,
        }
    }
    
    /// Set version
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }
    
    /// Set deprecated flag
    pub fn with_deprecated(mut self, deprecated: bool) -> Self {
        self.deprecated = deprecated;
        self
    }
    
    /// Set security level
    pub fn with_security_level(mut self, level: u32) -> Self {
        self.security_level = level;
        self
    }
}

/// System call module for organizing related syscalls
pub struct SyscallModule {
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Module description
    pub description: String,
    /// Module category
    pub category: SyscallCategory,
    /// System calls in this module
    #[allow(clippy::arc_with_non_send_sync)]
    pub syscalls: BTreeMap<u32, Arc<dyn SyscallHandler>>,
    /// Module dependencies
    pub dependencies: Vec<String>,
    /// Initialization function
    pub init_fn: Option<Box<dyn Fn() -> Result<()>>>,
    /// Cleanup function
    pub cleanup_fn: Option<Box<dyn Fn() -> Result<()>>>,
}

impl SyscallModule {
    /// Create a new syscall module
    pub fn new(
        name: String,
        version: String,
        description: String,
        category: SyscallCategory,
    ) -> Self {
        Self {
            name,
            version,
            description,
            category,
            syscalls: BTreeMap::new(),
            dependencies: Vec::new(),
            init_fn: None,
            cleanup_fn: None,
        }
    }
    
    /// Add a dependency
    pub fn add_dependency(&mut self, dependency: String) {
        self.dependencies.push(dependency);
    }
    
    /// Register a syscall handler
    pub fn register_syscall(&mut self, handler: Box<dyn SyscallHandler>) -> Result<()> {
        let id = handler.id();
        self.syscalls.insert(id, Arc::from(handler));
        Ok(())
    }
    
    /// Set initialization function
    pub fn set_init_fn(&mut self, init_fn: Box<dyn Fn() -> Result<()>>) {
        self.init_fn = Some(init_fn);
    }
    
    /// Set cleanup function
    pub fn set_cleanup_fn(&mut self, cleanup_fn: Box<dyn Fn() -> Result<()>>) {
        self.cleanup_fn = Some(cleanup_fn);
    }
    
    /// Initialize the module
    pub fn initialize(&self) -> Result<()> {
        if let Some(ref init_fn) = self.init_fn {
            init_fn()?;
        }
        Ok(())
    }
    
    /// Cleanup the module
    pub fn cleanup(&self) -> Result<()> {
        if let Some(ref cleanup_fn) = self.cleanup_fn {
            cleanup_fn()?;
        }
        Ok(())
    }
    
    /// Get syscall by ID
    pub fn get_syscall(&self, id: u32) -> Option<&Arc<dyn SyscallHandler>> {
        self.syscalls.get(&id)
    }
    
    /// Get all syscall IDs
    pub fn get_syscall_ids(&self) -> Vec<u32> {
        self.syscalls.keys().cloned().collect()
    }
    
    /// Get module information
    pub fn get_info(&self) -> ModuleInfo {
        ModuleInfo {
            name: self.name.clone(),
            version: self.version.clone(),
            description: self.description.clone(),
            category: self.category,
            syscall_count: self.syscalls.len(),
            dependencies: self.dependencies.clone(),
        }
    }
}

/// Module information
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module name
    pub name: String,
    /// Module version
    pub version: String,
    /// Module description
    pub description: String,
    /// Module category
    pub category: SyscallCategory,
    /// Number of syscalls in module
    pub syscall_count: usize,
    /// Module dependencies
    pub dependencies: Vec<String>,
}

/// Modular system call dispatcher
#[allow(clippy::should_implement_trait)]
pub struct ModularDispatcher {
    /// Registered modules
    modules: BTreeMap<String, Arc<SyscallModule>>,
    /// Syscall to module mapping
    syscall_to_module: BTreeMap<u32, String>,
    /// Module initialization order
    init_order: Vec<String>,
    /// Dispatcher statistics
    stats: DispatcherStats,
}

/// Dispatcher statistics
#[derive(Debug, Clone)]
#[allow(clippy::should_implement_trait)]
pub struct DispatcherStats {
    /// Total modules registered
    pub total_modules: usize,
    /// Total syscalls registered
    pub total_syscalls: usize,
    /// Syscall call counts
    syscall_counts: BTreeMap<u32, u64>,
    /// Module load times in microseconds
    module_load_times: BTreeMap<String, u64>,
}

impl DispatcherStats {
    /// Create new dispatcher statistics
    pub fn new() -> Self {
        Self {
            total_modules: 0,
            total_syscalls: 0,
            syscall_counts: BTreeMap::new(),
            module_load_times: BTreeMap::new(),
        }
    }
    
    /// Record a syscall execution
    pub fn record_syscall(&mut self, id: u32) {
        let count = self.syscall_counts.entry(id).or_insert(0);
        *count += 1;
    }
    
    /// Record module load time
    pub fn record_module_load(&mut self, name: String, time_us: u64) {
        self.module_load_times.insert(name, time_us);
    }
    
    /// Get most used syscalls
    pub fn get_most_used_syscalls(&self, count: usize) -> Vec<(u32, u64)> {
        let mut syscalls: Vec<_> = self.syscall_counts.iter()
            .map(|(&id, &count)| (id, count))
            .collect();
        syscalls.sort_by(|a, b| b.1.cmp(&a.1));
        syscalls.into_iter().take(count).collect()
    }
}

impl ModularDispatcher {
    /// Create a new modular dispatcher
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            syscall_to_module: BTreeMap::new(),
            init_order: Vec::new(),
            stats: DispatcherStats::new(),
        }
    }
    
    /// Register a module
    pub fn register_module(&mut self, module: Arc<SyscallModule>) -> Result<()> {
        let name = module.name.clone();
        
        // Check for circular dependencies
        self.check_circular_dependencies(&module)?;
        
        // Initialize the module
        let start_time = self.get_time_us();
        module.initialize()?;
        let end_time = self.get_time_us();
        
        // Register module
        self.modules.insert(name.clone(), module.clone());
        self.stats.total_modules += 1;
        self.stats.total_syscalls += module.syscalls.len();
        self.stats.record_module_load(name.clone(), end_time - start_time);
        
        // Map syscalls to module
        for &id in module.syscalls.keys() {
            self.syscall_to_module.insert(id, name.clone());
        }
        
        // Update initialization order
        self.update_init_order(&name)?;
        
        Ok(())
    }
    
    /// Check for circular dependencies
    fn check_circular_dependencies(&self, module: &SyscallModule) -> Result<()> {
        let mut visited = Vec::new();
        self.check_circular_dependencies_recursive(&module.name, &mut visited)
    }
    
    /// Recursive check for circular dependencies
    fn check_circular_dependencies_recursive(
        &self,
        module_name: &str,
        visited: &mut Vec<String>,
    ) -> Result<()> {
        if visited.contains(&module_name.to_string()) {
            return Err(nos_api::Error::InvalidArgument(
                "Circular dependency detected".to_string()
            ));
        }
        
        visited.push(module_name.to_string());
        
        if let Some(module) = self.modules.get(module_name) {
            for dep in &module.dependencies {
                self.check_circular_dependencies_recursive(dep, visited)?;
            }
        }
        
        visited.pop();
        Ok(())
    }
    
    /// Update module initialization order
    fn update_init_order(&mut self, module_name: &str) -> Result<()> {
        if !self.init_order.contains(&module_name.to_string()) {
            // Collect dependencies first to avoid borrowing conflicts
            let deps = if let Some(module) = self.modules.get(module_name) {
                module.dependencies.clone()
            } else {
                Vec::new()
            };
            
            // Add dependencies first
            for dep in &deps {
                self.update_init_order(dep)?;
            }
            
            // Then add the module
            self.init_order.push(module_name.to_string());
        }
        Ok(())
    }
    
    /// Dispatch a syscall
    pub fn dispatch(&mut self, id: u32, args: &[usize]) -> Result<isize> {
        // Record syscall execution
        self.stats.record_syscall(id);

        // Find the module that handles this syscall
        if let Some(module_name) = self.syscall_to_module.get(&id)
            && let Some(module) = self.modules.get(module_name)
            && let Some(handler) = module.syscalls.get(&id) {
            return handler.execute(args);
        }

        Err(nos_api::Error::NotFound(
            "Syscall not found".to_string()
        ))
    }
    
    /// Get module by name
    pub fn get_module(&self, name: &str) -> Option<&Arc<SyscallModule>> {
        self.modules.get(name)
    }
    
    /// Get all modules
    pub fn get_modules(&self) -> Vec<&Arc<SyscallModule>> {
        self.modules.values().collect()
    }
    
    /// Get module for a syscall
    pub fn get_module_for_syscall(&self, id: u32) -> Option<&str> {
        self.syscall_to_module.get(&id).map(|s| s.as_str())
    }
    
    /// Get dispatcher statistics
    pub fn get_stats(&self) -> &DispatcherStats {
        &self.stats
    }
    
    /// Get initialization order
    pub fn get_init_order(&self) -> &[String] {
        &self.init_order
    }
    
    /// Get current time in microseconds
    fn get_time_us(&self) -> u64 {
        // In a real implementation, this would use a high-precision timer
        static TIME_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIME_COUNTER.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Generate module report
    pub fn generate_module_report(&self) -> String {
        let mut report = String::from("=== Module Report ===\n");
        
        report.push_str(&format!("Total modules: {}\n", self.stats.total_modules));
        report.push_str(&format!("Total syscalls: {}\n", self.stats.total_syscalls));
        report.push_str("Initialization order:\n");
        for (i, name) in self.init_order.iter().enumerate() {
            report.push_str(&format!("  {}. {}\n", i + 1, name));
        }
        
        report.push_str("\nModule details:\n");
        for module in self.modules.values() {
            let info = module.get_info();
            report.push_str(&format!(
                "  {} (v{}): {} syscalls, category: {}\n",
                info.name, info.version, info.syscall_count, info.category.name()
            ));
        }
        
        report
    }
    
    /// Generate syscall usage report
    pub fn generate_usage_report(&self) -> String {
        let mut report = String::from("=== Syscall Usage Report ===\n");
        
        let most_used = self.stats.get_most_used_syscalls(10);
        report.push_str("Most used syscalls:\n");
        for (id, count) in most_used {
            if let Some(module_name) = self.syscall_to_module.get(&id)
                && let Some(module) = self.modules.get(module_name)
                && let Some(handler) = module.syscalls.get(&id) {
                report.push_str(&format!(
                    "  {}: {} ({} calls)\n",
                    handler.name(), id, count
                ));
            }
        }

        report
    }
}

/// Module builder for easier module creation
pub struct ModuleBuilder {
    name: String,
    version: String,
    description: String,
    category: SyscallCategory,
}

impl ModuleBuilder {
    /// Create a new module builder
    pub fn new(name: String, version: String, category: SyscallCategory) -> Self {
        Self {
            name,
            version,
            description: String::new(),
            category,
        }
    }
    
    /// Set description
    pub fn description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
    
    /// Build the module
    pub fn build(self) -> SyscallModule {
        SyscallModule::new(
            self.name,
            self.version,
            self.description,
            self.category,
        )
    }
}

/// Register all standard modules
#[allow(clippy::arc_with_non_send_sync)]
pub fn register_standard_modules(dispatcher: &mut ModularDispatcher) -> Result<()> {
    // File system module
    let fs_module = ModuleBuilder::new(
        "filesystem".to_string(),
        "1.0.0".to_string(),
        SyscallCategory::FileSystem,
    )
    .description("File system operations".to_string())
    .build();

    // Memory module
    let mem_module = ModuleBuilder::new(
        "memory".to_string(),
        "1.0.0".to_string(),
        SyscallCategory::Memory,
    )
    .description("Memory management operations".to_string())
    .build();

    // Network module
    let net_module = ModuleBuilder::new(
        "network".to_string(),
        "1.0.0".to_string(),
        SyscallCategory::Network,
    )
    .description("Network operations".to_string())
    .build();

    // Process module
    let proc_module = ModuleBuilder::new(
        "process".to_string(),
        "1.0.0".to_string(),
        SyscallCategory::Process,
    )
    .description("Process management operations".to_string())
    .build();

    // Register modules
    let _ = dispatcher.register_module(Arc::new(fs_module));
    let _ = dispatcher.register_module(Arc::new(mem_module));
    let _ = dispatcher.register_module(Arc::new(net_module));
    let _ = dispatcher.register_module(Arc::new(proc_module));

    Ok(())
}