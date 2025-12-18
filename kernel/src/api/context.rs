//! Kernel Context Module
//!
//! This module provides the KernelContext structure that manages core kernel components.
//! It implements dependency injection to replace global state and break circular dependencies.

use alloc::sync::Arc;
use crate::sync::Mutex;
use crate::api::syscall::{SyscallDispatcher, SyscallContext};
use crate::api::process::{ProcessManager, ThreadManager};
use crate::api::memory::{MemoryManager, ProcessMemoryManager};
use crate::api::error::KernelError;

/// Kernel context
///
/// This structure manages all core kernel components and provides
/// a centralized point for dependency injection.
pub struct KernelContext {
    /// System call dispatcher
    syscall_dispatcher: Arc<dyn SyscallDispatcher>,
    /// Process manager
    process_manager: Arc<dyn ProcessManager>,
    /// Thread manager
    thread_manager: Arc<dyn ThreadManager>,
    /// Memory manager
    memory_manager: Arc<dyn MemoryManager>,
    /// Process memory manager
    process_memory_manager: Arc<dyn ProcessMemoryManager>,
    /// System call context
    syscall_context: Arc<dyn SyscallContext>,
}

impl KernelContext {
    /// Create a new kernel context
    ///
    /// # Arguments
    /// * `syscall_dispatcher` - System call dispatcher
    /// * `process_manager` - Process manager
    /// * `thread_manager` - Thread manager
    /// * `memory_manager` - Memory manager
    /// * `process_memory_manager` - Process memory manager
    /// * `syscall_context` - System call context
    ///
    /// # Returns
    /// * `KernelContext` - New kernel context
    pub fn new(
        syscall_dispatcher: Arc<dyn SyscallDispatcher>,
        process_manager: Arc<dyn ProcessManager>,
        thread_manager: Arc<dyn ThreadManager>,
        memory_manager: Arc<dyn MemoryManager>,
        process_memory_manager: Arc<dyn ProcessMemoryManager>,
        syscall_context: Arc<dyn SyscallContext>,
    ) -> Self {
        Self {
            syscall_dispatcher,
            process_manager,
            thread_manager,
            memory_manager,
            process_memory_manager,
            syscall_context,
        }
    }

    /// Get the system call dispatcher
    ///
    /// # Returns
    /// * `Arc<dyn SyscallDispatcher>` - System call dispatcher
    pub fn get_syscall_dispatcher(&self) -> Arc<dyn SyscallDispatcher> {
        Arc::clone(&self.syscall_dispatcher)
    }

    /// Get the process manager
    ///
    /// # Returns
    /// * `Arc<dyn ProcessManager>` - Process manager
    pub fn get_process_manager(&self) -> Arc<dyn ProcessManager> {
        Arc::clone(&self.process_manager)
    }

    /// Get the thread manager
    ///
    /// # Returns
    /// * `Arc<dyn ThreadManager>` - Thread manager
    pub fn get_thread_manager(&self) -> Arc<dyn ThreadManager> {
        Arc::clone(&self.thread_manager)
    }

    /// Get the memory manager
    ///
    /// # Returns
    /// * `Arc<dyn MemoryManager>` - Memory manager
    pub fn get_memory_manager(&self) -> Arc<dyn MemoryManager> {
        Arc::clone(&self.memory_manager)
    }

    /// Get the process memory manager
    ///
    /// # Returns
    /// * `Arc<dyn ProcessMemoryManager>` - Process memory manager
    pub fn get_process_memory_manager(&self) -> Arc<dyn ProcessMemoryManager> {
        Arc::clone(&self.process_memory_manager)
    }

    /// Get the system call context
    ///
    /// # Returns
    /// * `Arc<dyn SyscallContext>` - System call context
    pub fn get_syscall_context(&self) -> Arc<dyn SyscallContext> {
        Arc::clone(&self.syscall_context)
    }
}

/// Global kernel context
///
/// This is a global reference to the kernel context.
/// It is used to provide access to kernel components throughout the system.
static mut KERNEL_CONTEXT: Option<KernelContext> = None;
static KERNEL_CONTEXT_INIT: Mutex<bool> = Mutex::new(false);

/// Initialize the global kernel context
///
/// # Arguments
/// * `context` - Kernel context to initialize
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(KernelError)` - Initialization error
pub fn init_kernel_context(context: KernelContext) -> Result<(), KernelError> {
    let mut init_guard = KERNEL_CONTEXT_INIT.lock();
    
    if *init_guard {
        return Err(KernelError::AlreadyExists);
    }
    
    unsafe {
        KERNEL_CONTEXT = Some(context);
    }
    
    *init_guard = true;
    Ok(())
}

/// Get the global kernel context
///
/// # Returns
/// * `Option<&'static KernelContext>` - Global kernel context if initialized
pub fn get_kernel_context() -> Option<&'static KernelContext> {
    unsafe { KERNEL_CONTEXT.as_ref() }
}

/// Get the system call dispatcher from the global context
///
/// # Returns
/// * `Option<Arc<dyn SyscallDispatcher>>` - System call dispatcher if context is initialized
pub fn get_syscall_dispatcher() -> Option<Arc<dyn SyscallDispatcher>> {
    get_kernel_context().map(|ctx| ctx.get_syscall_dispatcher())
}

/// Get the process manager from the global context
///
/// # Returns
/// * `Option<Arc<dyn ProcessManager>>` - Process manager if context is initialized
pub fn get_process_manager() -> Option<Arc<dyn ProcessManager>> {
    get_kernel_context().map(|ctx| ctx.get_process_manager())
}

/// Get the thread manager from the global context
///
/// # Returns
/// * `Option<Arc<dyn ThreadManager>>` - Thread manager if context is initialized
pub fn get_thread_manager() -> Option<Arc<dyn ThreadManager>> {
    get_kernel_context().map(|ctx| ctx.get_thread_manager())
}

/// Get the memory manager from the global context
///
/// # Returns
/// * `Option<Arc<dyn MemoryManager>>` - Memory manager if context is initialized
pub fn get_memory_manager() -> Option<Arc<dyn MemoryManager>> {
    get_kernel_context().map(|ctx| ctx.get_memory_manager())
}

/// Get the process memory manager from the global context
///
/// # Returns
/// * `Option<Arc<dyn ProcessMemoryManager>>` - Process memory manager if context is initialized
pub fn get_process_memory_manager() -> Option<Arc<dyn ProcessMemoryManager>> {
    get_kernel_context().map(|ctx| ctx.get_process_memory_manager())
}

/// Get the system call context from the global context
///
/// # Returns
/// * `Option<Arc<dyn SyscallContext>>` - System call context if context is initialized
pub fn get_syscall_context() -> Option<Arc<dyn SyscallContext>> {
    get_kernel_context().map(|ctx| ctx.get_syscall_context())
}

/// Check if the kernel context is initialized
///
/// # Returns
/// * `bool` - True if initialized, false otherwise
pub fn is_kernel_context_initialized() -> bool {
    *KERNEL_CONTEXT_INIT.lock()
}

/// Reset the global kernel context
///
/// This function is primarily used for testing.
pub fn reset_kernel_context() {
    let mut init_guard = KERNEL_CONTEXT_INIT.lock();
    unsafe {
        KERNEL_CONTEXT = None;
    }
    *init_guard = false;
}

/// Kernel context builder
///
/// This builder provides a convenient way to create a kernel context.
pub struct KernelContextBuilder {
    syscall_dispatcher: Option<Arc<dyn SyscallDispatcher>>,
    process_manager: Option<Arc<dyn ProcessManager>>,
    thread_manager: Option<Arc<dyn ThreadManager>>,
    memory_manager: Option<Arc<dyn MemoryManager>>,
    process_memory_manager: Option<Arc<dyn ProcessMemoryManager>>,
    syscall_context: Option<Arc<dyn SyscallContext>>,
}

impl KernelContextBuilder {
    /// Create a new kernel context builder
    ///
    /// # Returns
    /// * `KernelContextBuilder` - New kernel context builder
    pub fn new() -> Self {
        Self {
            syscall_dispatcher: None,
            process_manager: None,
            thread_manager: None,
            memory_manager: None,
            process_memory_manager: None,
            syscall_context: None,
        }
    }

    /// Set the system call dispatcher
    ///
    /// # Arguments
    /// * `dispatcher` - System call dispatcher
    ///
    /// # Returns
    /// * `Self` - Builder with system call dispatcher set
    pub fn with_syscall_dispatcher(mut self, dispatcher: Arc<dyn SyscallDispatcher>) -> Self {
        self.syscall_dispatcher = Some(dispatcher);
        self
    }

    /// Set the process manager
    ///
    /// # Arguments
    /// * `manager` - Process manager
    ///
    /// # Returns
    /// * `Self` - Builder with process manager set
    pub fn with_process_manager(mut self, manager: Arc<dyn ProcessManager>) -> Self {
        self.process_manager = Some(manager);
        self
    }

    /// Set the thread manager
    ///
    /// # Arguments
    /// * `manager` - Thread manager
    ///
    /// # Returns
    /// * `Self` - Builder with thread manager set
    pub fn with_thread_manager(mut self, manager: Arc<dyn ThreadManager>) -> Self {
        self.thread_manager = Some(manager);
        self
    }

    /// Set the memory manager
    ///
    /// # Arguments
    /// * `manager` - Memory manager
    ///
    /// # Returns
    /// * `Self` - Builder with memory manager set
    pub fn with_memory_manager(mut self, manager: Arc<dyn MemoryManager>) -> Self {
        self.memory_manager = Some(manager);
        self
    }

    /// Set the process memory manager
    ///
    /// # Arguments
    /// * `manager` - Process memory manager
    ///
    /// # Returns
    /// * `Self` - Builder with process memory manager set
    pub fn with_process_memory_manager(mut self, manager: Arc<dyn ProcessMemoryManager>) -> Self {
        self.process_memory_manager = Some(manager);
        self
    }

    /// Set the system call context
    ///
    /// # Arguments
    /// * `context` - System call context
    ///
    /// # Returns
    /// * `Self` - Builder with system call context set
    pub fn with_syscall_context(mut self, context: Arc<dyn SyscallContext>) -> Self {
        self.syscall_context = Some(context);
        self
    }

    /// Build the kernel context
    ///
    /// # Returns
    /// * `Result<KernelContext, KernelError>` - Kernel context or error
    pub fn build(self) -> Result<KernelContext, KernelError> {
        let syscall_dispatcher = self.syscall_dispatcher
            .ok_or(KernelError::InvalidArgument)?;
        let process_manager = self.process_manager
            .ok_or(KernelError::InvalidArgument)?;
        let thread_manager = self.thread_manager
            .ok_or(KernelError::InvalidArgument)?;
        let memory_manager = self.memory_manager
            .ok_or(KernelError::InvalidArgument)?;
        let process_memory_manager = self.process_memory_manager
            .ok_or(KernelError::InvalidArgument)?;
        let syscall_context = self.syscall_context
            .ok_or(KernelError::InvalidArgument)?;

        Ok(KernelContext::new(
            syscall_dispatcher,
            process_manager,
            thread_manager,
            memory_manager,
            process_memory_manager,
            syscall_context,
        ))
    }
}

impl Default for KernelContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}