/// Bootloader Initialization - Stack and Memory Setup
///
/// Provides early initialization code for the bootloader.
/// - Sets up stack
/// - Initializes heap
/// - Prepares memory for kernel loading

/// Bootloader stack size (64 KB)
pub const BOOTLOADER_STACK_SIZE: usize = 64 * 1024;

/// Bootloader heap size (256 KB)
pub const BOOTLOADER_HEAP_SIZE: usize = 256 * 1024;

/// Bootloader stack base address (grows downward)
/// This would typically be set by the boot protocol,
/// but for standalone bootloader we allocate statically.
pub const BOOTLOADER_STACK_BASE: u64 = 0x7FFF0000;

/// Bootloader heap base address
pub const BOOTLOADER_HEAP_BASE: u64 = 0x7FFE0000;

/// Initialize bootloader memory layout
///
/// # Safety
///
/// - Must be called once at bootloader startup
/// - Must be called before any memory allocations
pub unsafe fn init_memory_layout() -> Result<(), &'static str> {
    // Stack grows downward from BOOTLOADER_STACK_BASE
    // Heap grows upward from BOOTLOADER_HEAP_BASE
    
    // In real implementation would:
    // 1. Set stack pointer (RSP) if not already done
    // 2. Clear BSS section
    // 3. Initialize heap allocator
    
    Ok(())
}

/// Bootloader initialization stage enum
#[derive(Debug, Clone, Copy)]
pub enum InitStage {
    PreStack,      // Before stack initialization
    StackReady,    // Stack initialized
    HeapReady,     // Heap initialized
    Complete,      // All initialization complete
}

/// Bootloader initialization state tracker
pub struct InitState {
    stage: InitStage,
}

impl InitState {
    /// Create new initialization state
    pub fn new() -> Self {
        Self {
            stage: InitStage::PreStack,
        }
    }

    /// Get current initialization stage
    pub fn stage(&self) -> InitStage {
        self.stage
    }

    /// Mark stack as ready
    pub fn mark_stack_ready(&mut self) {
        self.stage = InitStage::StackReady;
    }

    /// Mark heap as ready
    pub fn mark_heap_ready(&mut self) {
        self.stage = InitStage::HeapReady;
    }

    /// Mark initialization complete
    pub fn mark_complete(&mut self) {
        self.stage = InitStage::Complete;
    }

    /// Check if initialization is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.stage, InitStage::Complete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_state_creation() {
        let state = InitState::new();
        assert!(matches!(state.stage(), InitStage::PreStack));
        assert!(!state.is_complete());
    }

    #[test]
    fn test_init_state_transitions() {
        let mut state = InitState::new();
        state.mark_stack_ready();
        assert!(matches!(state.stage(), InitStage::StackReady));
        
        state.mark_heap_ready();
        assert!(matches!(state.stage(), InitStage::HeapReady));
        
        state.mark_complete();
        assert!(state.is_complete());
    }
}
