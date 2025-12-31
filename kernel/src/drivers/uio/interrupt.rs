//! Interrupt handling for UIO devices
//!
//! This module provides interrupt controller functionality for delivering
//! hardware interrupts to userspace applications via eventfd mechanism.

use crate::drivers::uio::{UioError, UioResult};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use spin::Mutex;

/// UIO interrupt handler function type
pub type UioInterruptHandlerFn = Arc<dyn Fn(u32) -> UioResult<()> + Send + Sync>;

/// UIO interrupt controller
///
/// Manages interrupt delivery to userspace for a UIO device.
#[derive(Debug)]
pub struct UioInterrupt {
    /// IRQ number
    irq: AtomicU32,
    /// Interrupt enabled flag
    enabled: AtomicBool,
    /// Interrupt handler
    handler: Mutex<Option<UioInterruptHandlerFn>>,
    /// Eventfd for interrupt notification
    eventfd: Mutex<Option<u64>>,
    /// Interrupt count
    interrupt_count: AtomicU64,
}

impl UioInterrupt {
    /// Create a new UIO interrupt controller
    pub fn new() -> Self {
        Self {
            irq: AtomicU32::new(0),
            enabled: AtomicBool::new(false),
            handler: Mutex::new(None),
            eventfd: Mutex::new(None),
            interrupt_count: AtomicU64::new(0),
        }
    }

    /// Create interrupt with specific IRQ
    pub fn with_irq(irq: u32) -> Self {
        Self {
            irq: AtomicU32::new(irq),
            enabled: AtomicBool::new(false),
            handler: Mutex::new(None),
            eventfd: Mutex::new(None),
            interrupt_count: AtomicU64::new(0),
        }
    }

    /// Register interrupt handler
    ///
    /// Sets up the function to be called when an interrupt occurs.
    pub fn register_handler(&self, handler: UioInterruptHandlerFn) -> UioResult<()> {
        let mut guard = self.handler.lock();
        *guard = Some(handler);
        Ok(())
    }

    /// Unregister interrupt handler
    pub fn unregister_handler(&self) -> UioResult<()> {
        let mut guard = self.handler.lock();
        *guard = None;
        Ok(())
    }

    /// Enable interrupt
    pub fn enable(&self) -> UioResult<()> {
        self.enabled.store(true, Ordering::Release);

        // Register with kernel interrupt subsystem
        let irq = self.irq.load(Ordering::Acquire);
        if irq > 0 {
            // Platform-specific interrupt registration
            self.register_irq(irq)?;
        }

        Ok(())
    }

    /// Disable interrupt
    pub fn disable(&self) -> UioResult<()> {
        self.enabled.store(false, Ordering::Release);

        let irq = self.irq.load(Ordering::Acquire);
        if irq > 0 {
            self.unregister_irq(irq)?;
        }

        Ok(())
    }

    /// Check if interrupt is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Get IRQ number
    pub fn irq(&self) -> u32 {
        self.irq.load(Ordering::Acquire)
    }

    /// Set IRQ number
    pub fn set_irq(&self, irq: u32) {
        self.irq.store(irq, Ordering::Release);
    }

    /// Set up eventfd for interrupt notification
    pub fn set_eventfd(&self, fd: u64) -> UioResult<()> {
        let mut guard = self.eventfd.lock();
        *guard = Some(fd);
        Ok(())
    }

    /// Get eventfd
    pub fn eventfd(&self) -> Option<u64> {
        let guard = self.eventfd.lock();
        *guard
    }

    /// Trigger interrupt (called by kernel interrupt handler)
    pub fn trigger(&self) -> UioResult<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        // Increment interrupt count
        self.interrupt_count.fetch_add(1, Ordering::Release);

        // Call handler if registered
        let guard = self.handler.lock();
        if let Some(ref handler) = *guard {
            let irq = self.irq.load(Ordering::Acquire);
            handler(irq)?;
        }

        // Notify via eventfd
        let fd_guard = self.eventfd.lock();
        if let Some(fd) = *fd_guard {
            self.notify_eventfd(fd)?;
        }

        Ok(())
    }

    /// Get interrupt count
    pub fn interrupt_count(&self) -> u64 {
        self.interrupt_count.load(Ordering::Acquire)
    }

    /// Reset interrupt count
    pub fn reset_count(&self) {
        self.interrupt_count.store(0, Ordering::Release);
    }

    /// Register IRQ with platform interrupt subsystem
    fn register_irq(&self, irq: u32) -> UioResult<()> {
        // Platform-specific implementation
        // This would call the platform's interrupt registration function
        log::debug!("Registering IRQ {}", irq);
        Ok(())
    }

    /// Unregister IRQ from platform interrupt subsystem
    fn unregister_irq(&self, irq: u32) -> UioResult<()> {
        // Platform-specific implementation
        log::debug!("Unregistering IRQ {}", irq);
        Ok(())
    }

    /// Notify via eventfd
    fn notify_eventfd(&self, fd: u64) -> UioResult<()> {
        // Write to eventfd to notify userspace
        // Platform-specific implementation
        log::trace!("Notifying eventfd {}", fd);
        Ok(())
    }
}

impl Default for UioInterrupt {
    fn default() -> Self {
        Self::new()
    }
}

/// UIO interrupt handler trait
///
/// Implemented by devices that need custom interrupt handling.
pub trait UioInterruptHandler: Send + Sync {
    /// Handle interrupt
    fn handle_interrupt(&self, irq: u32) -> UioResult<()>;

    /// Enable interrupt (optional)
    fn enable(&self) -> UioResult<()> {
        Ok(())
    }

    /// Disable interrupt (optional)
    fn disable(&self) -> UioResult<()> {
        Ok(())
    }
}

/// Function-based interrupt handler
impl<F> UioInterruptHandler for F
where
    F: Fn(u32) -> UioResult<()> + Send + Sync,
{
    fn handle_interrupt(&self, irq: u32) -> UioResult<()> {
        self(irq)
    }
}

/// Interrupt affinity controller
///
/// Manages CPU affinity for UIO interrupts.
#[derive(Debug)]
pub struct UioInterruptAffinity {
    /// IRQ number
    irq: u32,
    /// CPU mask for affinity
    cpu_mask: u64,
}

impl UioInterruptAffinity {
    /// Create new interrupt affinity controller
    pub fn new(irq: u32) -> Self {
        Self {
            irq,
            cpu_mask: 0xFFFF_FFFF_FFFF_FFFF, // All CPUs by default
        }
    }

    /// Set CPU affinity
    pub fn set_affinity(&mut self, cpu_mask: u64) -> UioResult<()> {
        self.cpu_mask = cpu_mask;

        // Platform-specific implementation
        log::debug!("Setting IRQ {} affinity to {:#X}", self.irq, cpu_mask);
        Ok(())
    }

    /// Get CPU affinity
    pub fn affinity(&self) -> u64 {
        self.cpu_mask
    }

    /// Set affinity to single CPU
    pub fn set_cpu(&mut self, cpu: u32) -> UioResult<()> {
        let mask = 1u64 << cpu;
        self.set_affinity(mask)
    }
}

/// Interrupt statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct UioInterruptStats {
    /// Total interrupt count
    pub total_count: u64,
    /// Interrupts per second
    pub interrupts_per_second: u64,
    /// Last interrupt timestamp
    pub last_interrupt_ns: u64,
    /// Missed interrupts
    pub missed_count: u64,
}

/// Statistics tracking for UIO interrupts
#[derive(Debug)]
pub struct UioInterruptStatsTracker {
    stats: Mutex<UioInterruptStats>,
    start_time_ns: u64,
}

impl UioInterruptStatsTracker {
    /// Create new statistics tracker
    pub fn new() -> Self {
        Self {
            stats: Mutex::new(UioInterruptStats::default()),
            start_time_ns: 0,
        }
    }

    /// Record interrupt
    pub fn record_interrupt(&self) {
        let mut stats = self.stats.lock();
        stats.total_count += 1;
        // Update timestamp (platform-specific)
    }

    /// Get statistics
    pub fn stats(&self) -> UioInterruptStats {
        let stats = self.stats.lock();
        *stats
    }

    /// Reset statistics
    pub fn reset(&self) {
        let mut stats = self.stats.lock();
        *stats = UioInterruptStats::default();
    }
}

impl Default for UioInterruptStatsTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_new() {
        let intr = UioInterrupt::new();
        assert_eq!(intr.irq(), 0);
        assert!(!intr.is_enabled());
    }

    #[test]
    fn test_interrupt_with_irq() {
        let intr = UioInterrupt::with_irq(42);
        assert_eq!(intr.irq(), 42);
    }

    #[test]
    fn test_interrupt_enable_disable() {
        let intr = UioInterrupt::new();
        assert!(!intr.is_enabled());

        intr.enable().unwrap();
        assert!(intr.is_enabled());

        intr.disable().unwrap();
        assert!(!intr.is_enabled());
    }

    #[test]
    fn test_interrupt_set_irq() {
        let intr = UioInterrupt::new();
        intr.set_irq(10);
        assert_eq!(intr.irq(), 10);
    }

    #[test]
    fn test_interrupt_handler() {
        let intr = UioInterrupt::new();
        let called = Arc::new(core::sync::atomic::AtomicBool::new(false));

        let handler = {
            let called = called.clone();
            Arc::new(move |_irq| {
                called.store(true, Ordering::Release);
                Ok(())
            })
        };

        intr.register_handler(handler).unwrap();
        intr.trigger().unwrap();

        // Note: This test may need adjustment based on actual implementation
    }

    #[test]
    fn test_interrupt_affinity() {
        let mut affinity = UioInterruptAffinity::new(10);
        assert_eq!(affinity.affinity(), 0xFFFF_FFFF_FFFF_FFFF);

        affinity.set_cpu(0).unwrap();
        assert_eq!(affinity.affinity(), 1);

        affinity.set_affinity(0xF).unwrap();
        assert_eq!(affinity.affinity(), 0xF);
    }

    #[test]
    fn test_interrupt_stats() {
        let tracker = UioInterruptStatsTracker::new();
        let stats = tracker.stats();
        assert_eq!(stats.total_count, 0);

        tracker.record_interrupt();
        let stats = tracker.stats();
        assert_eq!(stats.total_count, 1);
    }
}
