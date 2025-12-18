//! Deferred initialization - Lazy loading for performance optimization
//!
//! Reduces boot time by deferring non-critical initialization:
//! - Graphics initialization (deferred until needed)
//! - Network initialization (optional)
//! - Diagnostic systems (optional)

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

/// Deferred initialization state
pub struct DeferredInit<T> {
    initialized: AtomicBool,
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Send> Send for DeferredInit<T> {}
unsafe impl<T: Send + Sync> Sync for DeferredInit<T> {}

impl<T> DeferredInit<T> {
    /// Create new deferred initialization slot
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            value: UnsafeCell::new(None),
        }
    }

    /// Initialize the value (one-time only)
    pub fn init(&self, value: T) -> Result<(), T> {
        if self.initialized.compare_exchange(
            false,
            true,
            Ordering::Release,
            Ordering::Relaxed,
        ).is_ok() {
            // SAFETY: We just verified no other thread has initialized
            unsafe {
                *self.value.get() = Some(value);
            }
            Ok(())
        } else {
            // Already initialized
            Err(value)
        }
    }

    /// Get reference to initialized value
    pub fn get(&self) -> Option<&T> {
        if self.initialized.load(Ordering::Acquire) {
            // SAFETY: We verified initialization and it's immutable after
            unsafe {
                (*self.value.get()).as_ref()
            }
        } else {
            None
        }
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }
}

/// Boot phase tracker for selective initialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase {
    /// Hardware initialization phase
    Hardware,
    /// Memory setup phase
    Memory,
    /// Drivers phase
    Drivers,
    /// Graphics phase (deferred)
    Graphics,
    /// Network phase (optional)
    Network,
    /// Kernel loading phase
    KernelLoad,
    /// Final validation phase
    Validation,
    /// Ready to jump to kernel
    Ready,
}

/// Boot phase controller for conditional initialization
pub struct BootPhaseController {
    current_phase: BootPhase,
    skip_graphics: bool,
    skip_network: bool,
}

impl BootPhaseController {
    /// Create new boot phase controller
    pub fn new() -> Self {
        Self {
            current_phase: BootPhase::Hardware,
            skip_graphics: false,
            skip_network: false,
        }
    }

    /// Set to skip graphics initialization
    pub fn skip_graphics(&mut self) {
        self.skip_graphics = true;
    }

    /// Set to skip network initialization
    pub fn skip_network(&mut self) {
        self.skip_network = true;
    }

    /// Advance to next phase
    pub fn advance(&mut self) -> BootPhase {
        self.current_phase = match self.current_phase {
            BootPhase::Hardware => BootPhase::Memory,
            BootPhase::Memory => BootPhase::Drivers,
            BootPhase::Drivers => {
                if self.skip_graphics {
                    BootPhase::KernelLoad
                } else {
                    BootPhase::Graphics
                }
            }
            BootPhase::Graphics => {
                if self.skip_network {
                    BootPhase::KernelLoad
                } else {
                    BootPhase::Network
                }
            }
            BootPhase::Network => BootPhase::KernelLoad,
            BootPhase::KernelLoad => BootPhase::Validation,
            BootPhase::Validation => BootPhase::Ready,
            BootPhase::Ready => BootPhase::Ready,
        };
        self.current_phase
    }

    /// Get current phase
    pub fn current(&self) -> BootPhase {
        self.current_phase
    }

    /// Check if should initialize graphics
    pub fn should_init_graphics(&self) -> bool {
        !self.skip_graphics
    }

    /// Check if should initialize network
    pub fn should_init_network(&self) -> bool {
        !self.skip_network
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deferred_init() {
        let slot: DeferredInit<i32> = DeferredInit::new();
        assert!(!slot.is_initialized());
        
        assert!(slot.init(42).is_ok());
        assert!(slot.is_initialized());
        assert_eq!(slot.get(), Some(&42));
    }

    #[test]
    fn test_boot_phase() {
        let mut controller = BootPhaseController::new();
        assert_eq!(controller.current(), BootPhase::Hardware);
        
        controller.advance();
        assert_eq!(controller.current(), BootPhase::Memory);
    }
}
