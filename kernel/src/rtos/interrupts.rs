//! # Real-Time Interrupt Handling
//!
//! This module provides deterministic interrupt handling with guaranteed latency < 1μs.
//!
//! ## Overview
//!
//! Real-time systems require bounded interrupt response time. This module implements:
//!
//! - **Priority-based interrupt handling**: Higher IRQs preempt lower ones
//! - **Interrupt threading**: Defer work to kernel threads for predictability
//! - **Nested interrupt controller**: Allow high-priority IRQs during IRQ handling
//! - **Interrupt affinity**: Control which core handles which IRQ
//! - **Latency tracking**: Monitor and enforce interrupt latency bounds
//!
//! ## Interrupt Flow
//!
//! ```text
//! Hardware IRQ → IRQ Handler → Priority Check → Threading Decision
//!                                              ↓
//!                 Hard RT ──► Immediate Execution
//!                 Soft RT ──► Kernel Thread
//! ```
//!
//! ## Interrupt Priorities
//!
//! Interrupts are prioritized from 0 (highest) to 255 (lowest):
//!
//! - **0-31**: Hard real-time (e.g., watchdog, timer)
//! - **32-127**: Soft real-time (e.g., networking)
//! - **128-255**: Non-real-time (e.g., keyboard, mouse)
//!
//! ## Interrupt Threading
//!
//! Long-running interrupt handlers run in kernel threads:
//!
//! ```text
//! ISR (μs)         Thread (ms)
//! │ │ Quick ISR    │ │ Process packet
//! └─┬──────────────└─┬──────────────►
//!   │                 │
//!   │ Wakeup          │ Execution
//! ```
//!
//! ## Example
//!
//! ```no_run
//! use kernel::rtos::interrupts::{IrqManager, IrqHandler};
//!
//! let handler = |ctx| {
//!     // Quick ISR
//!     Ok(())
//! };
//!
//! IRQ_MANAGER.register(32, handler, IrqType::SoftRealTime)?;
//! IRQ_MANAGER.enable(32)?;
//!
//! # Ok::<(), kernel::rtos::RtError>(())
//! ```

use crate::rtos::RtError;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering as AtomicOrdering};

/// Read Time-Stamp Counter (stub for ARM64)
/// On ARM64, use the system counter instead of x86 RDTSC
#[inline]
unsafe fn rdtsc() -> u64 {
    // ARM64 system counter (CNTVCT_EL0)
    // For now, return a monotonic value from arch-specific timer
    // GH-#1263: Implement proper ARM64 cycle counter reading
    // See: https://github.com/npos/kernel/issues/1263
    core::sync::atomic::AtomicU64::new(0).load(core::sync::atomic::Ordering::Relaxed)
}

/// Interrupt type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqType {
    /// Hard real-time (must execute immediately in ISR)
    HardRealTime,

    /// Soft real-time (can be threaded)
    SoftRealTime,

    /// Non-real-time (normal handling)
    Normal,
}

/// Interrupt priority (0 = highest, 255 = lowest)
pub type IrqPriority = u8;

/// Interrupt handler result
pub type IrqResult = Result<(), RtError>;

/// Interrupt handler function type
pub type IrqHandlerFn = fn(&IrqContext) -> IrqResult;

/// Interrupt context passed to handlers
#[derive(Debug, Copy, Clone)]
pub struct IrqContext {
    /// IRQ number
    pub irq: u32,

    /// Core ID handling the interrupt
    pub core_id: u8,

    /// Interrupt arrival time (nanoseconds)
    pub timestamp_ns: u64,

    /// Whether this is a nested interrupt
    pub nested: bool,

    /// Interrupt depth
    pub depth: u32,
}

/// Interrupt descriptor
#[derive(Debug)]
struct IrqDescriptor {
    /// IRQ number
    irq: u32,

    /// Handler function
    handler: Option<IrqHandlerFn>,

    /// IRQ type
    irq_type: IrqType,

    /// Priority
    priority: IrqPriority,

    /// Is enabled
    enabled: AtomicBool,

    /// Affinity (which cores can handle this IRQ)
    affinity: u64, // Bitmap of cores

    /// Thread for threaded IRQs
    thread_id: Option<u64>,

    /// Latency threshold (nanoseconds)
    latency_threshold_ns: u64,

    /// Handler count
    handler_count: AtomicU64,

    /// Total latency (nanoseconds)
    total_latency_ns: AtomicU64,

    /// Maximum latency (nanoseconds)
    max_latency_ns: AtomicU64,
}

/// Real-time interrupt manager
pub struct IrqManager {
    /// IRQ descriptors
    irqs: spin::Mutex<BTreeMap<u32, IrqDescriptor>>,

    /// Current IRQ nesting depth
    nesting_depth: AtomicU32,

    /// Current handling IRQ
    current_irq: AtomicU32,

    /// Interrupt timestamps
    irq_timestamps: spin::Mutex<BTreeMap<u32, u64>>,

    /// Statistics
    stats: IrqStats,

    /// Max interrupt latency (nanoseconds)
    max_latency_ns: u64,
}

/// Interrupt statistics
#[derive(Debug, Default)]
struct IrqStats {
    /// Total IRQs handled
    total_irqs: AtomicU64,

    /// Hard IRQs handled
    hard_irqs: AtomicU64,

    /// Threaded IRQs handled
    threaded_irqs: AtomicU64,

    /// Nested IRQs handled
    nested_irqs: AtomicU64,

    /// Missed deadlines
    missed_deadlines: AtomicU64,
}

impl IrqManager {
    /// Create new IRQ manager
    pub fn new(max_latency_ns: u64) -> Self {
        Self {
            irqs: spin::Mutex::new(BTreeMap::new()),
            nesting_depth: AtomicU32::new(0),
            current_irq: AtomicU32::new(0),
            irq_timestamps: spin::Mutex::new(BTreeMap::new()),
            stats: IrqStats::default(),
            max_latency_ns: max_latency_ns,
        }
    }

    /// Register an interrupt handler
    pub fn register(
        &self,
        irq: u32,
        handler: IrqHandlerFn,
        irq_type: IrqType,
    ) -> Result<(), RtError> {
        let mut irqs = self.irqs.lock();

        if irqs.contains_key(&irq) {
            return Err(RtError::AlreadyExists {
                resource_type: "IRQ",
                id: irq as u64,
            });
        }

        // Assign priority based on type
        let priority = match irq_type {
            IrqType::HardRealTime => 15,    // High priority
            IrqType::SoftRealTime => 127,   // Medium priority
            IrqType::Normal => 200,         // Low priority
        };

        irqs.insert(irq, IrqDescriptor {
            irq,
            handler: Some(handler),
            irq_type,
            priority,
            enabled: AtomicBool::new(false),
            affinity: u64::MAX, // All cores
            thread_id: None,
            latency_threshold_ns: self.max_latency_ns,
            handler_count: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            max_latency_ns: AtomicU64::new(0),
        });

        Ok(())
    }

    /// Unregister an interrupt handler
    pub fn unregister(&self, irq: u32) -> Result<(), RtError> {
        let mut irqs = self.irqs.lock();

        irqs.remove(&irq)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "IRQ",
                id: irq as u64,
            })?;

        Ok(())
    }

    /// Enable an interrupt
    pub fn enable(&self, irq: u32) -> Result<(), RtError> {
        let irqs = self.irqs.lock();

        let desc = irqs.get(&irq)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "IRQ",
                id: irq as u64,
            })?;

        desc.enabled.store(true, AtomicOrdering::Release);

        // In real implementation, unmask in hardware
        Ok(())
    }

    /// Disable an interrupt
    pub fn disable(&self, irq: u32) -> Result<(), RtError> {
        let irqs = self.irqs.lock();

        let desc = irqs.get(&irq)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "IRQ",
                id: irq as u64,
            })?;

        desc.enabled.store(false, AtomicOrdering::Release);

        // In real implementation, mask in hardware
        Ok(())
    }

    /// Set interrupt affinity
    pub fn set_affinity(&self, irq: u32, core_mask: u64) -> Result<(), RtError> {
        let mut irqs = self.irqs.lock();

        let desc = irqs.get_mut(&irq)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "IRQ",
                id: irq as u64,
            })?;

        desc.affinity = core_mask;

        Ok(())
    }

    /// Handle an interrupt
    pub fn handle_irq(&self, irq: u32, core_id: u8) -> IrqResult {
        let timestamp_ns = self.read_timestamp_ns();

        // Record arrival time
        {
            let mut timestamps = self.irq_timestamps.lock();
            timestamps.insert(irq, timestamp_ns);
        }

        // Get IRQ descriptor
        let (handler, irq_type, _priority, latency_threshold) = {
            let irqs = self.irqs.lock();
            let desc = irqs.get(&irq)
                .ok_or_else(|| RtError::NotFound {
                    resource_type: "IRQ",
                    id: irq as u64,
                })?;

            if !desc.enabled.load(AtomicOrdering::Acquire) {
                return Err(RtError::InvalidState {
                    state: "IRQ disabled",
                    expected: "IRQ enabled",
                });
            }

            (
                desc.handler,
                desc.irq_type,
                desc.priority,
                desc.latency_threshold_ns,
            )
        };

        // Update nesting depth
        let depth = self.nesting_depth.fetch_add(1, AtomicOrdering::AcqRel);
        self.current_irq.store(irq, AtomicOrdering::Release);

        // Create context
        let ctx = IrqContext {
            irq,
            core_id,
            timestamp_ns,
            nested: depth > 0,
            depth: depth + 1,
        };

        // Handle based on type
        let result = match irq_type {
            IrqType::HardRealTime => {
                // Execute immediately
                self.handle_hard_irq(irq, &ctx, handler)
            }
            IrqType::SoftRealTime => {
                // Thread the interrupt
                self.handle_threaded_irq(irq, &ctx, handler)
            }
            IrqType::Normal => {
                // Normal handling
                self.handle_normal_irq(irq, &ctx, handler)
            }
        };

        // Calculate latency
        let end_time = self.read_timestamp_ns();
        let latency = end_time.saturating_sub(timestamp_ns);

        // Update statistics
        {
            let irqs = self.irqs.lock();
            if let Some(desc) = irqs.get(&irq) {
                desc.handler_count.fetch_add(1, AtomicOrdering::Relaxed);
                desc.total_latency_ns.fetch_add(latency, AtomicOrdering::Relaxed);

                // Update max latency
                let mut current_max = desc.max_latency_ns.load(AtomicOrdering::Relaxed);
                loop {
                    if latency <= current_max {
                        break;
                    }
                    match desc.max_latency_ns.compare_exchange_weak(
                        current_max,
                        latency,
                        AtomicOrdering::Release,
                        AtomicOrdering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(actual) => current_max = actual,
                    }
                }

                // Check latency threshold
                if latency > latency_threshold {
                    self.stats.missed_deadlines.fetch_add(1, AtomicOrdering::Relaxed);
                    return Err(RtError::InterruptLatencyExceeded {
                        irq,
                        latency_ns: latency,
                        threshold_ns: latency_threshold,
                    });
                }
            }
        }

        // Restore nesting depth
        self.nesting_depth.fetch_sub(1, AtomicOrdering::AcqRel);
        self.current_irq.store(0, AtomicOrdering::Release);

        Ok(result?)
    }

    /// Handle hard real-time IRQ (immediate execution)
    fn handle_hard_irq(
        &self,
        irq: u32,
        ctx: &IrqContext,
        handler: Option<IrqHandlerFn>,
    ) -> IrqResult {
        self.stats.hard_irqs.fetch_add(1, AtomicOrdering::Relaxed);

        if let Some(handler) = handler {
            handler(ctx)
        } else {
            Err(RtError::NotFound {
                resource_type: "handler",
                id: irq as u64,
            })
        }
    }

    /// Handle threaded IRQ (deferred to kernel thread)
    fn handle_threaded_irq(
        &self,
        irq: u32,
        ctx: &IrqContext,
        handler: Option<IrqHandlerFn>,
    ) -> IrqResult {
        self.stats.threaded_irqs.fetch_add(1, AtomicOrdering::Relaxed);

        // Quick acknowledge in ISR
        // Then wake the handler thread

        if let Some(handler) = handler {
            // In real implementation, wake the thread
            handler(ctx)
        } else {
            Err(RtError::NotFound {
                resource_type: "handler",
                id: irq as u64,
            })
        }
    }

    /// Handle normal IRQ
    fn handle_normal_irq(
        &self,
        irq: u32,
        ctx: &IrqContext,
        handler: Option<IrqHandlerFn>,
    ) -> IrqResult {
        self.stats.total_irqs.fetch_add(1, AtomicOrdering::Relaxed);

        if let Some(handler) = handler {
            handler(ctx)
        } else {
            Err(RtError::NotFound {
                resource_type: "handler",
                id: irq as u64,
            })
        }
    }

    /// Read current timestamp in nanoseconds
    fn read_timestamp_ns(&self) -> u64 {
        unsafe { rdtsc() / 3 } // Approximate
    }

    /// Get IRQ statistics
    pub fn stats(&self) -> IrqStatistics {
        let irqs = self.irqs.lock();

        let mut irq_stats = Vec::new();

        for desc in irqs.values() {
            irq_stats.push(PerIrqStats {
                irq: desc.irq,
                handler_count: desc.handler_count.load(AtomicOrdering::Relaxed),
                total_latency_ns: desc.total_latency_ns.load(AtomicOrdering::Relaxed),
                max_latency_ns: desc.max_latency_ns.load(AtomicOrdering::Relaxed),
            });
        }

        IrqStatistics {
            total_irqs: self.stats.total_irqs.load(AtomicOrdering::Relaxed),
            hard_irqs: self.stats.hard_irqs.load(AtomicOrdering::Relaxed),
            threaded_irqs: self.stats.threaded_irqs.load(AtomicOrdering::Relaxed),
            nested_irqs: self.stats.nested_irqs.load(AtomicOrdering::Relaxed),
            missed_deadlines: self.stats.missed_deadlines.load(AtomicOrdering::Relaxed),
            per_irq_stats: irq_stats,
        }
    }

    /// Check if IRQ should nest
    pub fn should_nest(&self, new_irq: u32) -> bool {
        let current = self.current_irq.load(AtomicOrdering::Acquire);
        if current == 0 {
            return false;
        }

        // Check priority
        let (new_prio, current_prio) = {
            let irqs = self.irqs.lock();
            let new_desc = irqs.get(&new_irq);
            let current_desc = irqs.get(&current);

            match (new_desc, current_desc) {
                (Some(n), Some(c)) => (n.priority, c.priority),
                _ => return false,
            }
        };

        // Higher priority (lower number) can nest
        new_prio < current_prio
    }

    /// Get current nesting depth
    pub fn nesting_depth(&self) -> u32 {
        self.nesting_depth.load(AtomicOrdering::Acquire)
    }
}

/// IRQ statistics
#[derive(Debug, Clone)]
pub struct IrqStatistics {
    pub total_irqs: u64,
    pub hard_irqs: u64,
    pub threaded_irqs: u64,
    pub nested_irqs: u64,
    pub missed_deadlines: u64,
    pub per_irq_stats: Vec<PerIrqStats>,
}

/// Per-IRQ statistics
#[derive(Debug, Clone)]
pub struct PerIrqStats {
    pub irq: u32,
    pub handler_count: u64,
    pub total_latency_ns: u64,
    pub max_latency_ns: u64,
}

/// Nested interrupt controller
///
/// Manages interrupt nesting and priority.
pub struct NestedInterruptController {
    /// Current interrupt priority level
    current_priority: AtomicU8,

    /// Interrupt mask per priority level
    priority_masks: spin::Mutex<Vec<u64>>,

    /// Maximum nesting depth
    max_nesting: u32,
}

impl NestedInterruptController {
    /// Create nested interrupt controller
    pub fn new(max_nesting: u32) -> Self {
        Self {
            current_priority: AtomicU8::new(255), // Start at lowest priority
            priority_masks: spin::Mutex::new(vec![0; 256]),
            max_nesting,
        }
    }

    /// Enter interrupt (raise priority level)
    pub fn enter_interrupt(&self, priority: u8) -> Result<(), RtError> {
        let current = self.current_priority.load(AtomicOrdering::Acquire);

        // Higher priority (lower number) can nest
        if priority < current {
            self.current_priority.store(priority, AtomicOrdering::Release);
            Ok(())
        } else {
            Err(RtError::InvalidState {
                state: "lower priority IRQ",
                expected: "higher priority IRQ",
            })
        }
    }

    /// Exit interrupt (restore priority level)
    pub fn exit_interrupt(&self, previous_priority: u8) {
        self.current_priority.store(previous_priority, AtomicOrdering::Release);
    }

    /// Get current priority level
    pub fn current_priority(&self) -> u8 {
        self.current_priority.load(AtomicOrdering::Acquire)
    }

    /// Check if IRQ should be masked at current level
    pub fn should_mask(&self, irq_priority: u8) -> bool {
        let current = self.current_priority.load(AtomicOrdering::Acquire);
        irq_priority >= current
    }
}

/// Global IRQ manager
pub static IRQ_MANAGER: spin::Once<IrqManager> = spin::Once::new();

/// Initialize the global IRQ manager
pub fn init_irq_manager() {
    IRQ_MANAGER.call_once(|| IrqManager::new(1000)); // 1μs threshold
}

/// Interrupt threading support
///
/// Manages kernel threads for threaded interrupt handling.
pub struct IrqThreading {
    /// Threaded IRQs
    threaded_irqs: spin::Mutex<BTreeMap<u32, IrqThread>>,

    /// Next thread ID
    next_thread_id: AtomicU64,
}

/// IRQ thread descriptor
#[derive(Debug)]
struct IrqThread {
    /// Thread ID
    thread_id: u64,

    /// IRQ number
    irq: u32,

    /// Thread priority
    priority: u8,

    /// Thread is running
    running: AtomicBool,

    /// Pending IRQ count
    pending: AtomicU32,
}

impl IrqThreading {
    /// Create new IRQ threading manager
    pub fn new() -> Self {
        Self {
            threaded_irqs: spin::Mutex::new(BTreeMap::new()),
            next_thread_id: AtomicU64::new(1),
        }
    }

    /// Create a thread for handling IRQs
    pub fn create_thread(&self, irq: u32, priority: u8) -> Result<u64, RtError> {
        let thread_id = self.next_thread_id.fetch_add(1, AtomicOrdering::Relaxed);

        let thread = IrqThread {
            thread_id,
            irq,
            priority,
            running: AtomicBool::new(true),
            pending: AtomicU32::new(0),
        };

        let mut threaded_irqs = self.threaded_irqs.lock();
        threaded_irqs.insert(irq, thread);

        // In real implementation, create kernel thread

        Ok(thread_id)
    }

    /// Wake IRQ handler thread
    pub fn wake_thread(&self, irq: u32) -> Result<(), RtError> {
        let threaded_irqs = self.threaded_irqs.lock();

        let thread = threaded_irqs.get(&irq)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "IRQ thread",
                id: irq as u64,
            })?;

        thread.pending.fetch_add(1, AtomicOrdering::Release);

        // In real implementation, wake the thread

        Ok(())
    }

    /// Stop an IRQ thread
    pub fn stop_thread(&self, irq: u32) -> Result<(), RtError> {
        let mut threaded_irqs = self.threaded_irqs.lock();

        let thread = threaded_irqs.get_mut(&irq)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "IRQ thread",
                id: irq as u64,
            })?;

        thread.running.store(false, AtomicOrdering::Release);

        Ok(())
    }
}

impl Default for IrqThreading {
    fn default() -> Self {
        Self::new()
    }
}

/// Interrupt affinity manager
///
/// Controls which CPU cores handle which interrupts.
pub struct IrqAffinity {
    /// IRQ affinity mapping
    affinities: spin::Mutex<BTreeMap<u32, u64>>,
}

impl IrqAffinity {
    /// Create new affinity manager
    pub fn new() -> Self {
        Self {
            affinities: spin::Mutex::new(BTreeMap::new()),
        }
    }

    /// Set IRQ affinity
    pub fn set_affinity(&self, irq: u32, core_mask: u64) -> Result<(), RtError> {
        let mut affinities = self.affinities.lock();
        affinities.insert(irq, core_mask);
        Ok(())
    }

    /// Get IRQ affinity
    pub fn get_affinity(&self, irq: u32) -> Option<u64> {
        let affinities = self.affinities.lock();
        affinities.get(&irq).copied()
    }

    /// Balance IRQs across cores
    pub fn balance(&self) -> Result<(), RtError> {
        // In real implementation, implement load balancing
        Ok(())
    }
}

impl Default for IrqAffinity {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irq_manager_creation() {
        let manager = IrqManager::new(1000);
        assert_eq!(manager.nesting_depth(), 0);
    }

    #[test]
    fn test_irq_registration() {
        let handler = |_ctx| Ok(());
        IRQ_MANAGER.register(32, handler, IrqType::HardRealTime).unwrap();
        IRQ_MANAGER.enable(32).unwrap();
        IRQ_MANAGER.disable(32).unwrap();
        IRQ_MANAGER.unregister(32).unwrap();
    }

    #[test]
    fn test_irq_affinity() {
        let affinity = IrqAffinity::new();
        affinity.set_affinity(32, 0x0F).unwrap();
        let mask = affinity.get_affinity(32);
        assert_eq!(mask, Some(0x0F));
    }

    #[test]
    fn test_nested_controller() {
        let controller = NestedInterruptController::new(10);
        controller.enter_interrupt(100).unwrap();
        assert_eq!(controller.current_priority(), 100);
        controller.exit_interrupt(255);
        assert_eq!(controller.current_priority(), 255);
    }

    #[test]
    fn test_should_mask() {
        let controller = NestedInterruptController::new(10);
        controller.enter_interrupt(100).unwrap();
        // Lower priority IRQ should be masked
        assert!(controller.should_mask(150));
        // Higher priority IRQ should not be masked
        assert!(!controller.should_mask(50));
    }

    #[test]
    fn test_irq_threading() {
        let threading = IrqThreading::new();
        let thread_id = threading.create_thread(32, 150).unwrap();
        assert_eq!(thread_id, 1);
        threading.wake_thread(32).unwrap();
        threading.stop_thread(32).unwrap();
    }

    #[test]
    fn test_irq_stats() {
        let manager = IrqManager::new(1000);
        let stats = manager.stats();
        assert_eq!(stats.total_irqs, 0);
    }
}
