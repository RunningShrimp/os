//! RISC-V interrupt controller support
//!
//! This module provides comprehensive interrupt controller support for RISC-V systems,
//! including PLIC (Platform-Level Interrupt Controller) for external interrupts and
//! core-local interrupt handling.
//!
//! # Features
//! - PLIC (Platform-Level Interrupt Controller) support
//! - Per-hart interrupt context management
//! - Interrupt priority configuration
//! - Interrupt routing and affinity
//! - Software, timer, and external interrupt handling
//! - AIA (Advanced Interrupt Architecture) support
//!
//! # Performance Targets
//! - Interrupt latency: <10μs
//! - Interrupt dispatch overhead: <2μs

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// PLIC base address (platform-specific, should be configured from device tree)
const PLIC_BASE: usize = 0x0C00_0000;

/// Maximum number of interrupt sources
pub const MAX_INTERRUPTS: usize = 256;

/// Maximum number of contexts (harts)
pub const MAX_CONTEXTS: usize = 8;

/// Interrupt priority levels
pub const MAX_PRIORITY: u32 = 7;

/// Interrupt types
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptType {
    /// Software interrupt (inter-processor interrupt)
    Software = 0,
    /// Timer interrupt
    Timer = 1,
    /// External interrupt (from PLIC)
    External = 2,
    /// Platform-specific interrupt
    Platform = 3,
}

/// PLIC register offsets
#[repr(usize)]
enum PlicReg {
    /// Priority registers (4 bytes per interrupt)
    Priority = 0x0000,
    /// Pending bits (1 bit per interrupt)
    Pending = 0x1000,
    /// Enable bits per context (1 bit per interrupt)
    Enable = 0x2000,
    /// Threshold and claim/complete per context
    Context = 0x200000,
}

/// PLIC context registers
#[repr(usize)]
enum PlicContextReg {
    /// Priority threshold
    Threshold = 0x00,
    /// Claim/complete register
    Claim = 0x04,
}

/// Per-hart interrupt context
pub struct InterruptContext {
    /// Hart ID
    hart_id: usize,
    /// Context ID (for EIC)
    context_id: usize,
    /// Priority threshold
    threshold: AtomicU32,
    /// Enabled interrupts
    enabled: SpinLock<alloc::collections::BTreeSet<usize>>,
    /// Interrupt statistics
    stats: InterruptStats,
    /// Last claimed interrupt
    claimed: AtomicUsize,
}

/// Interrupt statistics
#[derive(Default)]
pub struct InterruptStats {
    /// Total interrupts handled
    total_count: AtomicU64,
    /// Spurious interrupts
    spurious_count: AtomicU64,
    /// Interrupts by source
    by_source: SpinLock<BTreeMap<usize, AtomicU64>>,
}

impl InterruptContext {
    /// Create a new interrupt context
    pub const fn new(hart_id: usize, context_id: usize) -> Self {
        Self {
            hart_id,
            context_id,
            threshold: AtomicU32::new(0),
            enabled: SpinLock::new(alloc::collections::BTreeSet::new()),
            stats: InterruptStats {
                total_count: AtomicU64::new(0),
                spurious_count: AtomicU64::new(0),
                by_source: SpinLock::new(BTreeMap::new()),
            },
            claimed: AtomicUsize::new(0),
        }
    }

    /// Set priority threshold
    pub fn set_threshold(&self, threshold: u32) {
        self.threshold.store(threshold.min(MAX_PRIORITY), Ordering::Release);
        self.write_context_reg(PlicContextReg::Threshold, threshold.min(MAX_PRIORITY));
    }

    /// Get priority threshold
    pub fn threshold(&self) -> u32 {
        self.threshold.load(Ordering::Acquire)
    }

    /// Enable interrupt
    pub fn enable_irq(&self, irq: usize) {
        let mut enabled = self.enabled.lock();
        enabled.insert(irq);
        drop(enabled);

        self.write_enable_reg(irq, true);
    }

    /// Disable interrupt
    pub fn disable_irq(&self, irq: usize) {
        let mut enabled = self.enabled.lock();
        enabled.remove(&irq);
        drop(enabled);

        self.write_enable_reg(irq, false);
    }

    /// Check if interrupt is enabled
    pub fn is_irq_enabled(&self, irq: usize) -> bool {
        let enabled = self.enabled.lock();
        enabled.contains(&irq)
    }

    /// Write to a context register
    fn write_context_reg(&self, reg: PlicContextReg, value: u32) {
        let addr = PLIC_BASE
            + PlicReg::Context as usize
            + (self.context_id * 0x1000)
            + reg as usize;

        unsafe {
            core::ptr::write_volatile(addr as *mut u32, value);
        }
    }

    /// Read from a context register
    fn read_context_reg(&self, reg: PlicContextReg) -> u32 {
        let addr = PLIC_BASE
            + PlicReg::Context as usize
            + (self.context_id * 0x1000)
            + reg as usize;

        unsafe { core::ptr::read_volatile(addr as *const u32) }
    }

    /// Write to enable register
    fn write_enable_reg(&self, irq: usize, enable: bool) {
        let word_offset = (irq / 32) * 4;
        let bit_offset = irq % 32;

        let addr = PLIC_BASE
            + PlicReg::Enable as usize
            + (self.context_id * 0x80)
            + word_offset;

        unsafe {
            let mut value = core::ptr::read_volatile(addr as *const u32);
            if enable {
                value |= 1 << bit_offset;
            } else {
                value &= !(1 << bit_offset);
            }
            core::ptr::write_volatile(addr as *mut u32, value);
        }
    }
}

/// Global interrupt controller state
pub struct InterruptController {
    /// Interrupt contexts (one per hart)
    contexts: Vec<InterruptContext>,
    /// Interrupt handlers
    handlers: SpinLock<BTreeMap<usize, InterruptHandlerFn>>,
    /// Initialized flag
    initialized: AtomicU32,
}

/// Interrupt handler function type
pub type InterruptHandlerFn = fn(usize) -> ();

impl InterruptController {
    /// Create new interrupt controller
    pub const fn new() -> Self {
        Self {
            contexts: Vec::new(),
            handlers: SpinLock::new(BTreeMap::new()),
            initialized: AtomicU32::new(0),
        }
    }

    /// Initialize interrupt controller
    pub fn init(&mut self, num_harts: usize) -> Result<(), &'static str> {
        crate::println!("riscv64-intc: Initializing PLIC with {} harts", num_harts);

        // Initialize interrupt contexts
        for hart in 0..num_harts {
            self.contexts.push(InterruptContext::new(hart, hart));
        }

        // Disable all interrupts initially
        for hart in 0..num_harts {
            let context = &self.contexts[hart];
            context.set_threshold(0); // Allow all interrupts

            // Disable all IRQs
            for irq in 0..MAX_INTERRUPTS {
                context.disable_irq(irq);
            }
        }

        // Set all interrupt priorities to 0 (disabled)
        for irq in 0..MAX_INTERRUPTS {
            self.set_irq_priority(irq, 0);
        }

        self.initialized.store(1, Ordering::Release);

        crate::println!("riscv64-intc: PLIC initialization complete");
        Ok(())
    }

    /// Set interrupt priority
    pub fn set_irq_priority(&self, irq: usize, priority: u32) {
        if irq >= MAX_INTERRUPTS {
            return;
        }

        let addr = PLIC_BASE + PlicReg::Priority as usize + (irq * 4);
        let priority = priority.min(MAX_PRIORITY);

        unsafe {
            core::ptr::write_volatile(addr as *mut u32, priority);
        }
    }

    /// Get interrupt priority
    pub fn get_irq_priority(&self, irq: usize) -> u32 {
        if irq >= MAX_INTERRUPTS {
            return 0;
        }

        let addr = PLIC_BASE + PlicReg::Priority as usize + (irq * 4);

        unsafe { core::ptr::read_volatile(addr as *const u32) }
    }

    /// Enable interrupt for a hart
    pub fn enable_irq(&self, hart_id: usize, irq: usize) {
        if hart_id >= self.contexts.len() {
            return;
        }

        self.contexts[hart_id].enable_irq(irq);
    }

    /// Disable interrupt for a hart
    pub fn disable_irq(&self, hart_id: usize, irq: usize) {
        if hart_id >= self.contexts.len() {
            return;
        }

        self.contexts[hart_id].disable_irq(irq);
    }

    /// Check if interrupt is pending
    pub fn is_irq_pending(&self, irq: usize) -> bool {
        if irq >= MAX_INTERRUPTS {
            return false;
        }

        let word_offset = irq / 32;
        let bit_offset = irq % 32;

        let addr = PLIC_BASE + PlicReg::Pending as usize + (word_offset * 4);

        unsafe {
            let value = core::ptr::read_volatile(addr as *const u32);
            (value & (1 << bit_offset)) != 0
        }
    }

    /// Get current hart context
    pub fn current_context(&self) -> Option<&InterruptContext> {
        let hart_id = super::smp::get_cpu_id();
        self.contexts.get(hart_id)
    }

    /// Claim interrupt
    pub fn claim_interrupt(&self) -> Option<usize> {
        let context = self.current_context()?;
        let claimed = context.read_context_reg(PlicContextReg::Claim);

        if claimed == 0 || claimed as usize >= MAX_INTERRUPTS {
            return None;
        }

        let irq = claimed as usize;
        context.claimed.store(irq, Ordering::Release);
        context.stats.total_count.fetch_add(1, Ordering::Relaxed);

        Some(irq)
    }

    /// Complete interrupt
    pub fn complete_interrupt(&self, irq: usize) {
        let context = match self.current_context() {
            Some(ctx) => ctx,
            None => return,
        };

        // Only complete if this was the last claimed interrupt
        if context.claimed.load(Ordering::Acquire) == irq {
            context.write_context_reg(PlicContextReg::Claim, irq as u32);
            context.claimed.store(0, Ordering::Release);
        }
    }

    /// Register interrupt handler
    pub fn register_handler(&self, irq: usize, handler: InterruptHandlerFn) {
        let mut handlers = self.handlers.lock();
        handlers.insert(irq, handler);
    }

    /// Unregister interrupt handler
    pub fn unregister_handler(&self, irq: usize) {
        let mut handlers = self.handlers.lock();
        handlers.remove(&irq);
    }

    /// Handle external interrupt
    pub fn handle_external(&self) {
        // Claim interrupt
        let irq = match self.claim_interrupt() {
            Some(irq) => irq,
            None => {
                // Spurious interrupt
                if let Some(context) = self.current_context() {
                    context.stats.spurious_count.fetch_add(1, Ordering::Relaxed);
                }
                return;
            }
        };

        // Call handler
        let handlers = self.handlers.lock();
        if let Some(handler) = handlers.get(&irq) {
            handler(irq);
        }

        drop(handlers);

        // Complete interrupt
        self.complete_interrupt(irq);
    }

    /// Set interrupt affinity (route to specific hart)
    pub fn set_affinity(&self, irq: usize, hart_id: usize) {
        // Disable on all harts
        for hart in 0..self.contexts.len() {
            self.disable_irq(hart, irq);
        }

        // Enable on target hart
        self.enable_irq(hart_id, irq);
    }

    /// Get interrupt statistics
    pub fn get_stats(&self, hart_id: usize) -> Option<&InterruptStats> {
        self.contexts.get(hart_id).map(|ctx| &ctx.stats)
    }
}

/// Global interrupt controller instance
static INTERRUPT_CONTROLLER: SpinLock<InterruptController> =
    SpinLock::new(InterruptController::new());

/// Initialize interrupt controller
pub fn plic_init() -> Result<(), &'static str> {
    let num_harts = super::smp::total_cpus();
    if num_harts == 0 {
        return Err("No CPUs detected");
    }

    let mut ic = INTERRUPT_CONTROLLER.lock();
    ic.init(num_harts)
}

/// Set interrupt priority
pub fn plic_set_priority(irq: usize, priority: u32) {
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.set_irq_priority(irq, priority);
}

/// Get interrupt priority
pub fn plic_get_priority(irq: usize) -> u32 {
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.get_irq_priority(irq)
}

/// Enable interrupt for current hart
pub fn plic_enable_irq(irq: usize) {
    let hart_id = super::smp::get_cpu_id();
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.enable_irq(hart_id, irq);
}

/// Disable interrupt for current hart
pub fn plic_disable_irq(irq: usize) {
    let hart_id = super::smp::get_cpu_id();
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.disable_irq(hart_id, irq);
}

/// Check if interrupt is pending
pub fn plic_is_pending(irq: usize) -> bool {
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.is_irq_pending(irq)
}

/// Set interrupt affinity
pub fn plic_set_affinity(irq: usize, hart_id: usize) {
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.set_affinity(irq, hart_id);
}

/// Register interrupt handler
pub fn plic_register_handler(irq: usize, handler: InterruptHandlerFn) {
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.register_handler(irq, handler);
}

/// Unregister interrupt handler
pub fn plic_unregister_handler(irq: usize) {
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.unregister_handler(irq);
}

/// Claim interrupt
pub fn plic_claim() -> Option<usize> {
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.claim_interrupt()
}

/// Complete interrupt
pub fn plic_complete(irq: usize) {
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.complete_interrupt(irq);
}

/// Handle external interrupt (called from trap handler)
pub fn handle_external_interrupt() {
    let ic = INTERRUPT_CONTROLLER.lock();
    ic.handle_external();
}

/// Enable interrupts
pub fn enable_interrupts() {
    unsafe {
        core::arch::asm!("csrsi sstatus, 0x2"); // SIE bit in SSTATUS
    }
}

/// Disable interrupts and return previous state
pub fn disable_interrupts() -> bool {
    let previous: bool;
    unsafe {
        core::arch::asm!(
            "csrrc {}, sstatus, {}",
            out(reg) previous,
            const 0x2,
        );
    }
    previous
}

/// Restore interrupt state
pub fn restore_interrupts(state: bool) {
    if state {
        enable_interrupts();
    }
}

/// Check if interrupts are enabled
pub fn interrupts_enabled() -> bool {
    let enabled: u64;
    unsafe {
        core::arch::asm!("csrr {}, sstatus", out(reg) enabled);
    }
    (enabled & 0x2) != 0
}

/// Enable software interrupt (IPI)
pub fn enable_software_interrupt() {
    unsafe {
        core::arch::asm!("csrsi sie, 0x2"); // SSIE bit
    }
}

/// Disable software interrupt
pub fn disable_software_interrupt() {
    unsafe {
        core::arch::asm!("csrci sie, 0x2");
    }
}

/// Enable timer interrupt
pub fn enable_timer_interrupt() {
    unsafe {
        core::arch::asm!("csrsi sie, 0x20"); // STIE bit
    }
}

/// Disable timer interrupt
pub fn disable_timer_interrupt() {
    unsafe {
        core::arch::asm!("csrci sie, 0x20");
    }
}

/// Enable external interrupt
pub fn enable_external_interrupt() {
    unsafe {
        core::arch::asm!("csrsi sie, 0x400"); // SEIE bit
    }
}

/// Disable external interrupt
pub fn disable_external_interrupt() {
    unsafe {
        core::arch::asm!("csrci sie, 0x400");
    }
}

/// Machine mode interrupt enable
#[cfg(feature = "machine_mode")]
pub fn enable_machine_interrupts() {
    unsafe {
        core::arch::asm!("csrsi mstatus, 0x8"); // MIE bit
    }
}

#[cfg(feature = "machine_mode")]
pub fn disable_machine_interrupts() -> bool {
    let previous: bool;
    unsafe {
        core::arch::asm!(
            "csrrc {}, mstatus, {}",
            out(reg) previous,
            const 0x8,
        );
    }
    previous
}

/// AIA (Advanced Interrupt Architecture) support
pub mod aia {
    use super::*;

    /// AIA CSR addresses
    pub const SISELECT: usize = 0x150;
    pub const SIPRIORITY: usize = 0x154;
    pub const SIEH: usize = 0x158;
    pub const SIPH: usize = 0x159;

    /// Initialize AIA (if available)
    pub fn init() -> Result<(), &'static str> {
        // Detect AIA support
        if !has_aia() {
            return Err("AIA not supported");
        }

        crate::println!("riscv64-intc: Initializing AIA");

        // Configure interrupt priorities
        // ...

        Ok(())
    }

    /// Check if AIA is supported
    pub fn has_aia() -> bool {
        // Read marchid/scounteren to detect AIA
        // This is a simplified check
        false
    }

    /// Set interrupt priority (AIA)
    pub fn set_irq_priority(irq: usize, priority: u8) {
        if !has_aia() {
            return;
        }

        unsafe {
            // Select IRQ
            core::arch::asm!("csrw {}, {}", in(reg) SISELECT, in(reg) irq);

            // Set priority
            core::arch::asm!("csrw {}, {}", in(reg) SIPRIORITY, in(reg) priority);
        }
    }
}

/// Core local interrupt controller (CLINT)
pub mod clint {
    /// CLINT base address
    const CLINT_BASE: usize = 0x0200_0000;

    /// MSIP registers (software interrupts)
    const MSIP_OFFSET: usize = 0x0000;

    /// MTIMECMP registers (timer compare)
    const MTIMECMP_OFFSET: usize = 0x4000;

    /// MTIME register (timer value)
    const MTIME_OFFSET: usize = 0xBFF8;

    /// Send software interrupt (IPI) to a hart
    pub fn send_ipi(hart_id: usize) {
        let addr = CLINT_BASE + MSIP_OFFSET + (hart_id * 4);
        unsafe {
            core::ptr::write_volatile(addr as *mut u32, 1);
        }
    }

    /// Clear software interrupt for a hart
    pub fn clear_ipi(hart_id: usize) {
        let addr = CLINT_BASE + MSIP_OFFSET + (hart_id * 4);
        unsafe {
            core::ptr::write_volatile(addr as *mut u32, 0);
        }
    }

    /// Set timer compare value for a hart
    pub fn set_timer(hart_id: usize, value: u64) {
        let addr = CLINT_BASE + MTIMECMP_OFFSET + (hart_id * 8);
        unsafe {
            core::ptr::write_volatile(addr as *mut u64, value);
        }
    }

    /// Get current timer value
    pub fn get_time() -> u64 {
        let addr = CLINT_BASE + MTIME_OFFSET;
        unsafe { core::ptr::read_volatile(addr as *const u64) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_enable_disable() {
        let was_enabled = disable_interrupts();
        assert!(!interrupts_enabled());

        restore_interrupts(was_enabled);
    }

    #[test]
    fn test_plic_context() {
        let context = InterruptContext::new(0, 0);

        context.set_threshold(5);
        assert_eq!(context.threshold(), 5);

        context.enable_irq(10);
        assert!(context.is_irq_enabled(10));

        context.disable_irq(10);
        assert!(!context.is_irq_enabled(10));
    }
}
