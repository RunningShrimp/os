//! Interrupt control functions for SMP-safe synchronization

/// Disable interrupts and return previous interrupt state
#[inline]
pub fn push_off() -> bool {
    super::push_off()
}

/// Restore interrupt state
#[inline]
pub fn pop_off(was_enabled: bool) {
    super::pop_off(was_enabled)
}
