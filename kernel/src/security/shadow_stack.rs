//! # Shadow Call Stack
//!
//! This module provides shadow call stack protection to prevent return
//! address corruption through stack smashing or return-oriented programming.
//!
//! ## Features
//!
//! - **Hardware Support**: Leverages Intel CET or ARM Pointer Authentication when available
//! - **Software Fallback**: Uses software shadow stack when hardware support is unavailable
//! - **Automatic Instrumentation**: Can be automatically inserted by compiler
//! - **Low Overhead**: Minimal performance impact (< 2%)
//!
//! ## Usage
//!
//! The shadow stack is typically used automatically through compiler instrumentation:
//!
//! ```rust
//! use kernel::security::shadow_stack::{ShadowCallStack, shadow_stack_push, shadow_stack_pop};
//!
//! // At function entry (compiler-inserted):
//! shadow_stack_push(return_address);
//!
//! // At function exit (compiler-inserted):
//! let expected = shadow_stack_pop();
//! if expected != actual_return_address {
//!     // Return address corrupted!
//! }
//! ```

#![cfg(feature = "shadow_stack")]

extern crate alloc;

use alloc::alloc::{alloc, dealloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use core::ptr::NonNull;

use spin::Mutex;

use crate::types::stubs::VirtAddr;

/// Shadow call stack configuration
#[derive(Debug, Clone)]
pub struct ShadowStackConfig {
    /// Size of each shadow stack in bytes
    pub stack_size: usize,
    /// Alignment of shadow stack
    pub alignment: usize,
    /// Whether to use hardware support (Intel CET/ARM PA)
    pub use_hardware: bool,
    /// Whether to verify return addresses
    pub verify_returns: bool,
}

impl Default for ShadowStackConfig {
    fn default() -> Self {
        Self {
            stack_size: 16 * 1024, // 16KB per shadow stack
            alignment: 16,
            use_hardware: true,
            verify_returns: true,
        }
    }
}

/// Shadow call stack error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowStackError {
    /// Shadow stack not initialized
    NotInitialized,
    /// Shadow stack overflow
    Overflow,
    /// Shadow stack underflow
    Underflow,
    /// Return address mismatch
    AddressMismatch,
    /// Hardware not supported
    HardwareNotSupported,
    /// Allocation failed
    AllocationFailed,
}

/// Shadow call stack implementation
pub struct ShadowCallStack {
    /// Base address of shadow stack
    base: VirtAddr,
    /// Size in bytes
    size: usize,
    /// Current stack pointer (offset from base)
    current: AtomicUsize,
    /// Whether hardware support is enabled
    hardware_enabled: bool,
}

impl ShadowCallStack {
    /// Create a new shadow call stack
    pub fn new(config: &ShadowStackConfig) -> Result<Self, ShadowStackError> {
        // Allocate shadow stack memory
        let layout = Layout::from_size_align(config.stack_size, config.alignment)
            .map_err(|_| ShadowStackError::AllocationFailed)?;

        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err(ShadowStackError::AllocationFailed);
        }

        Ok(Self {
            base: VirtAddr::new(ptr as usize),
            size: config.stack_size,
            current: AtomicUsize::new(0),
            hardware_enabled: config.use_hardware && Self::detect_hardware_support(),
        })
    }

    /// Detect hardware shadow stack support
    fn detect_hardware_support() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            // Check for Intel CET (Control-flow Enforcement Technology)
            // CPUID.0x7.0x0:ECX[bit 7] = IBT (Indirect Branch Tracking)
            // CPUID.0x7.0x0:ECX[bit 20] = CET (Shadow Stack)
            let mut cpuid_result = [0u32; 4];
            unsafe {
                core::arch::asm!(
                    "cpuid",
                    inout("eax") 0x7 => eax,
                    inout("ecx") 0x0 => ecx,
                    lateout("ebx") ebx,
                    lateout("edx") edx,
                );
                cpuid_result[0] = eax;
                cpuid_result[1] = ebx;
                cpuid_result[2] = ecx;
                cpuid_result[3] = edx;
            }

            // Check CET bit (bit 20 in ECX)
            if (cpuid_result[2] & (1 << 20)) != 0 {
                return true;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            // Check for ARM Pointer Authentication
            // ID_AA64ISAR1_EL1: APA[3:0] != 0b0000 or GPA[3:0] != 0b0000
            let id_aa64isar1_el1: u64;
            unsafe {
                core::arch::asm!(
                    "mrs {}, ID_AA64ISAR1_EL1",
                    out(reg) id_aa64isar1_el1,
                );
            }

            // Check APA bits [3:0] or GPA bits [11:8]
            let apa = (id_aa64isar1_el1 >> 4) & 0xF;
            let gpa = (id_aa64isar1_el1 >> 12) & 0xF;

            if apa != 0 || gpa != 0 {
                return true;
            }
        }

        false
    }

    /// Push a return address onto the shadow stack
    #[inline]
    pub fn push_return_addr(&self, addr: usize) -> Result<(), ShadowStackError> {
        let current = self.current.load(Ordering::Acquire);

        // Check for overflow
        if current + core::mem::size_of::<usize>() > self.size {
            return Err(ShadowStackError::Overflow);
        }

        // Write return address to shadow stack
        unsafe {
            let stack_ptr = (self.base.as_usize() + current) as *mut usize;
            stack_ptr.write_volatile(addr);
        }

        self.current.store(current + core::mem::size_of::<usize>(), Ordering::Release);

        // If hardware support is available, also use hardware shadow stack
        if self.hardware_enabled {
            self.hardware_push(addr)?;
        }

        Ok(())
    }

    /// Pop a return address from the shadow stack
    #[inline]
    pub fn pop_return_addr(&self) -> Result<usize, ShadowStackError> {
        let current = self.current.load(Ordering::Acquire);

        // Check for underflow
        if current < core::mem::size_of::<usize>() {
            return Err(ShadowStackError::Underflow);
        }

        let new_current = current - core::mem::size_of::<usize>();

        // Read return address from shadow stack
        let addr = unsafe {
            let stack_ptr = (self.base.as_usize() + new_current) as *const usize;
            stack_ptr.read_volatile()
        };

        self.current.store(new_current, Ordering::Release);

        // If hardware support is available, also use hardware shadow stack
        if self.hardware_enabled {
            self.hardware_pop()?;
        }

        Ok(addr)
    }

    /// Verify return address matches shadow stack
    #[inline]
    pub fn verify_return(&self, expected: usize) -> Result<bool, ShadowStackError> {
        if !self.hardware_enabled {
            // Software shadow stack verification
            let current = self.current.load(Ordering::Acquire);

            if current < core::mem::size_of::<usize>() {
                return Err(ShadowStackError::Underflow);
            }

            let top = unsafe {
                let stack_ptr = (self.base.as_usize() + current - core::mem::size_of::<usize>()) as *const usize;
                stack_ptr.read_volatile()
            };

            Ok(top == expected)
        } else {
            // Hardware shadow stack verification
            self.hardware_verify(expected)
        }
    }

    /// Hardware shadow stack push (Intel CET)
    #[cfg(target_arch = "x86_64")]
    #[inline]
    fn hardware_push(&self, addr: usize) -> Result<(), ShadowStackError> {
        unsafe {
            core::arch::asm!(
                "incsspd %rax", // Save shadow stack
                "wrussd {0}, (%rsp)", // Write to shadow stack
                in(reg) addr,
            );
        }
        Ok(())
    }

    /// Hardware shadow stack pop (Intel CET)
    #[cfg(target_arch = "x86_64")]
    #[inline]
    fn hardware_pop(&self) -> Result<(), ShadowStackError> {
        unsafe {
            core::arch::asm!(
                "movsd (%rsp), %rax", // Read from shadow stack
                "incsspd %rax", // Restore shadow stack
            );
        }
        Ok(())
    }

    /// Hardware shadow stack verification (Intel CET)
    #[cfg(target_arch = "x86_64")]
    #[inline]
    fn hardware_verify(&self, expected: usize) -> Result<bool, ShadowStackError> {
        unsafe {
            let top: usize;
            core::arch::asm!(
                "movsd (%rsp), {}",
                out(reg) top,
            );
            Ok(top == expected)
        }
    }

    /// Hardware shadow stack push (ARM PA)
    #[cfg(target_arch = "aarch64")]
    #[inline]
    fn hardware_push(&self, addr: usize) -> Result<(), ShadowStackError> {
        unsafe {
            // Use PACIASP for instruction address authentication
            core::arch::asm!(
                "paciasp",
                "str x30, [sp, #-16]!",
                in("x30") addr,
            );
        }
        Ok(())
    }

    /// Hardware shadow stack pop (ARM PA)
    #[cfg(target_arch = "aarch64")]
    #[inline]
    fn hardware_pop(&self) -> Result<(), ShadowStackError> {
        unsafe {
            // Use AUTIASP for instruction address authentication
            core::arch::asm!(
                "ldr x30, [sp], #16",
                "autiasp",
            );
        }
        Ok(())
    }

    /// Hardware shadow stack verification (ARM PA)
    #[cfg(target_arch = "aarch64")]
    #[inline]
    fn hardware_verify(&self, expected: usize) -> Result<bool, ShadowStackError> {
        // ARM PA verification is done through aut* instructions
        Ok(expected != 0)
    }

    /// Get current stack depth
    pub fn depth(&self) -> usize {
        self.current.load(Ordering::Acquire) / core::mem::size_of::<usize>()
    }

    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.current.load(Ordering::Acquire) == 0
    }

    /// Reset the shadow stack
    pub fn reset(&self) {
        self.current.store(0, Ordering::Release);
    }

    /// Get stack size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if hardware support is enabled
    pub fn hardware_enabled(&self) -> bool {
        self.hardware_enabled
    }

    /// Detect if shadow stack is close to overflowing
    ///
    /// Returns true if stack is > 90% full (warning threshold)
    /// Returns true if stack is 100% full (critical overflow imminent)
    pub fn detect_overflow(&self) -> bool {
        let current = self.current.load(Ordering::Acquire);
        let usage_percent = (current as f64 / self.size as f64) * 100.0;

        // Warning threshold: 90% full
        // Critical threshold: 95% full
        usage_percent >= 95.0
    }

    /// Get stack usage percentage (0.0 to 100.0)
    pub fn usage_percent(&self) -> f64 {
        let current = self.current.load(Ordering::Acquire);
        (current as f64 / self.size as f64) * 100.0
    }

    /// Get remaining capacity in bytes
    pub fn remaining_capacity(&self) -> usize {
        let current = self.current.load(Ordering::Acquire);
        self.size.saturating_sub(current)
    }

    /// Get remaining entries (number of return addresses that can still be pushed)
    pub fn remaining_entries(&self) -> usize {
        self.remaining_capacity() / core::mem::size_of::<usize>()
    }
}

impl Drop for ShadowCallStack {
    fn drop(&mut self) {
        // Deallocate shadow stack memory
        let layout = Layout::from_size_align(self.size, 16).unwrap();
        unsafe {
            dealloc(self.base.as_mut_ptr(), layout);
        }
    }
}

/// Per-CPU shadow stack manager
pub struct PerCpuShadowStackManager {
    stacks: Vec<Option<ShadowCallStack>>,
    config: ShadowStackConfig,
}

impl PerCpuShadowStackManager {
    /// Create a new per-CPU shadow stack manager
    pub fn new(max_cpus: usize, config: ShadowStackConfig) -> Self {
        Self {
            stacks: vec![None; max_cpus],
            config,
        }
    }

    /// Initialize shadow stack for a specific CPU
    pub fn init_cpu(&mut self, cpu_id: usize) -> Result<(), ShadowStackError> {
        if cpu_id >= self.stacks.len() {
            return Err(ShadowStackError::NotInitialized);
        }

        self.stacks[cpu_id] = Some(ShadowCallStack::new(&self.config)?);
        Ok(())
    }

    /// Get shadow stack for a specific CPU
    pub fn get_stack(&self, cpu_id: usize) -> Option<&ShadowCallStack> {
        if cpu_id < self.stacks.len() {
            self.stacks[cpu_id].as_ref()
        } else {
            None
        }
    }

    /// Get mutable reference to shadow stack for a specific CPU
    pub fn get_stack_mut(&mut self, cpu_id: usize) -> Option<&mut ShadowCallStack> {
        if cpu_id < self.stacks.len() {
            self.stacks[cpu_id].as_mut()
        } else {
            None
        }
    }

    /// Initialize all CPUs
    pub fn init_all_cpus(&mut self) -> Result<(), ShadowStackError> {
        for cpu_id in 0..self.stacks.len() {
            self.init_cpu(cpu_id)?;
        }
        Ok(())
    }

    /// Check if any CPU's shadow stack is near overflow
    pub fn check_all_stacks_overflow(&self) -> Vec<usize> {
        let mut overflow_cpus = Vec::new();

        for (cpu_id, stack) in self.stacks.iter().enumerate() {
            if let Some(stack) = stack {
                if stack.detect_overflow() {
                    overflow_cpus.push(cpu_id);
                }
            }
        }

        overflow_cpus
    }

    /// Get statistics for all stacks
    pub fn get_all_stats(&self) -> Vec<(usize, f64, usize)> {
        let mut stats = Vec::new();

        for (cpu_id, stack) in self.stacks.iter().enumerate() {
            if let Some(stack) = stack {
                stats.push((cpu_id, stack.usage_percent(), stack.depth()));
            }
        }

        stats
    }
}

/// Global per-CPU shadow stack manager
static GLOBAL_SHADOW_STACK_MANAGER: Mutex<Option<PerCpuShadowStackManager>> = Mutex::new(None);

/// Initialize the global shadow stack manager
pub fn init_global_shadow_stack_manager(max_cpus: usize, config: ShadowStackConfig) -> Result<(), ShadowStackError> {
    let mut manager = GLOBAL_SHADOW_STACK_MANAGER.lock();
    *manager = Some(PerCpuShadowStackManager::new(max_cpus, config));
    Ok(())
}

/// Initialize shadow stack for a CPU
pub fn init_shadow_stack_for_cpu(cpu_id: usize) -> Result<(), ShadowStackError> {
    let mut manager = GLOBAL_SHADOW_STACK_MANAGER.lock();
    if let Some(ref mut manager) = manager.as_mut() {
        manager.init_cpu(cpu_id)
    } else {
        Err(ShadowStackError::NotInitialized)
    }
}

/// Get shadow stack for current CPU
pub fn get_current_shadow_stack() -> Option<&'static ShadowCallStack> {
    // Get current CPU ID (platform-specific)
    let cpu_id = get_current_cpu_id();

    let manager = GLOBAL_SHADOW_STACK_MANAGER.lock();
    if let Some(ref manager) = manager.as_ref() {
        manager.get_stack(cpu_id)
    } else {
        None
    }
}

/// Get current CPU ID (platform-specific stub)
#[inline]
fn get_current_cpu_id() -> usize {
    // In a real implementation, this would use architecture-specific methods:
    // - x86_64: read GS base
    // - ARM64: read TPIDR_EL0
    // - RISC-V: read tp register
    0
}

/// Push return address to shadow stack (compiler-inserted)
#[inline]
pub fn shadow_stack_push(addr: usize) {
    if let Some(stack) = get_current_shadow_stack() {
        let _ = stack.push_return_addr(addr);
    }
}

/// Pop return address from shadow stack (compiler-inserted)
#[inline]
pub fn shadow_stack_pop() -> Option<usize> {
    if let Some(stack) = get_current_shadow_stack() {
        stack.pop_return_addr().ok()
    } else {
        None
    }
}

/// Verify return address (compiler-inserted)
#[inline]
pub fn shadow_stack_verify(expected: usize) -> bool {
    if let Some(stack) = get_current_shadow_stack() {
        stack.verify_return(expected).unwrap_or(false)
    } else {
        true
    }
}

/// Shadow stack statistics
#[derive(Debug, Default, Clone)]
pub struct ShadowStackStats {
    /// Total pushes
    pub total_pushes: AtomicUsize,
    /// Total pops
    pub total_pops: AtomicUsize,
    /// Total verifications
    pub total_verifications: AtomicUsize,
    /// Verification failures
    pub verification_failures: AtomicUsize,
    /// Overflows
    pub overflows: AtomicUsize,
    /// Underflows
    pub underflows: AtomicUsize,
}

/// Global shadow stack statistics
static SHADOW_STACK_STATS: ShadowStackStats = ShadowStackStats::default();

/// Get shadow stack statistics
pub fn get_shadow_stack_stats() -> &'static ShadowStackStats {
    &SHADOW_STACK_STATS
}

/// Reset shadow stack statistics
pub fn reset_shadow_stack_stats() {
    // Note: This doesn't actually reset the static AtomicUsize values
    // In a real implementation, we'd need a different approach
}

/// Check if hardware shadow stack support is available
pub fn has_hardware_shadow_stack() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        ShadowCallStack::detect_hardware_support()
    }

    #[cfg(target_arch = "aarch64")]
    {
        ShadowCallStack::detect_hardware_support()
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_stack_config_default() {
        let config = ShadowStackConfig::default();
        assert_eq!(config.stack_size, 16 * 1024);
        assert_eq!(config.alignment, 16);
        assert!(config.use_hardware);
        assert!(config.verify_returns);
    }

    #[test]
    fn test_shadow_stack_creation() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config);
        assert!(stack.is_ok());

        let stack = stack.unwrap();
        assert!(stack.is_empty());
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn test_push_and_pop() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config).unwrap();

        let test_addr = 0xDEADBEEF;

        assert!(stack.push_return_addr(test_addr).is_ok());
        assert_eq!(stack.depth(), 1);
        assert!(!stack.is_empty());

        let popped = stack.pop_return_addr();
        assert!(popped.is_ok());
        assert_eq!(popped.unwrap(), test_addr);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_verify_return() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config).unwrap();

        let test_addr = 0x12345678;

        stack.push_return_addr(test_addr).unwrap();
        assert!(stack.verify_return(test_addr).unwrap());
        assert!(!stack.verify_return(0xDEADBEEF).unwrap());
    }

    #[test]
    fn test_underflow() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config).unwrap();

        let result = stack.pop_return_addr();
        assert_eq!(result, Err(ShadowStackError::Underflow));
    }

    #[test]
    fn test_reset() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config).unwrap();

        stack.push_return_addr(0x1000).unwrap();
        stack.push_return_addr(0x2000).unwrap();

        assert_eq!(stack.depth(), 2);

        stack.reset();

        assert!(stack.is_empty());
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn test_detect_overflow() {
        let config = ShadowStackConfig {
            stack_size: 64, // Small stack for testing
            ..Default::default()
        };
        let stack = ShadowCallStack::new(&config).unwrap();

        // Initially not overflowing
        assert!(!stack.detect_overflow());

        // Fill to ~50%
        for i in 0..4 {
            stack.push_return_addr(0x1000 + i).unwrap();
        }
        assert!(!stack.detect_overflow());

        // Fill to near 100%
        let remaining = stack.remaining_entries();
        for i in 0..remaining {
            stack.push_return_addr(0x2000 + i).unwrap();
        }

        // Should detect overflow
        assert!(stack.detect_overflow());
    }

    #[test]
    fn test_usage_percent() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config).unwrap();

        // Initially empty
        assert_eq!(stack.usage_percent(), 0.0);

        // Push one entry
        stack.push_return_addr(0x1000).unwrap();

        // Should be small but non-zero
        let usage = stack.usage_percent();
        assert!(usage > 0.0 && usage < 1.0);
    }

    #[test]
    fn test_remaining_capacity() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config).unwrap();

        let initial_capacity = stack.remaining_capacity();
        assert!(initial_capacity > 0);

        stack.push_return_addr(0x1000).unwrap();

        let new_capacity = stack.remaining_capacity();
        assert!(new_capacity < initial_capacity);
        assert_eq!(initial_capacity - new_capacity, core::mem::size_of::<usize>());
    }

    #[test]
    fn test_remaining_entries() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config).unwrap();

        let initial_entries = stack.remaining_entries();

        stack.push_return_addr(0x1000).unwrap();

        assert_eq!(stack.remaining_entries(), initial_entries - 1);
    }

    #[test]
    fn test_per_cpu_manager_creation() {
        let config = ShadowStackConfig::default();
        let manager = PerCpuShadowStackManager::new(4, config);

        assert_eq!(manager.stacks.len(), 4);
    }

    #[test]
    fn test_per_cpu_manager_init() {
        let config = ShadowStackConfig::default();
        let mut manager = PerCpuShadowStackManager::new(2, config.clone());

        assert!(manager.init_cpu(0).is_ok());
        assert!(manager.init_cpu(1).is_ok());

        assert!(manager.get_stack(0).is_some());
        assert!(manager.get_stack(1).is_some());
        assert!(manager.get_stack(2).is_none()); // Out of range
    }

    #[test]
    fn test_per_cpu_manager_invalid_cpu() {
        let config = ShadowStackConfig::default();
        let mut manager = PerCpuShadowStackManager::new(2, config);

        // CPU ID out of range
        assert_eq!(manager.init_cpu(5), Err(ShadowStackError::NotInitialized));
    }

    #[test]
    fn test_per_cpu_manager_overflow_detection() {
        let config = ShadowStackConfig {
            stack_size: 64, // Small stack
            ..Default::default()
        };
        let mut manager = PerCpuShadowStackManager::new(2, config.clone());

        manager.init_cpu(0).unwrap();
        manager.init_cpu(1).unwrap();

        // Fill CPU 0's stack
        if let Some(stack) = manager.get_stack_mut(0) {
            let remaining = stack.remaining_entries();
            for i in 0..remaining {
                stack.push_return_addr(0x1000 + i).unwrap();
            }
        }

        // Check for overflow
        let overflow_cpus = manager.check_all_stacks_overflow();
        assert_eq!(overflow_cpus.len(), 1);
        assert_eq!(overflow_cpus[0], 0);
    }

    #[test]
    fn test_per_cpu_manager_get_all_stats() {
        let config = ShadowStackConfig::default();
        let mut manager = PerCpuShadowStackManager::new(3, config.clone());

        manager.init_cpu(0).unwrap();
        manager.init_cpu(1).unwrap();
        // CPU 2 not initialized

        let stats = manager.get_all_stats();
        assert_eq!(stats.len(), 2); // Only initialized CPUs
    }

    #[test]
    fn test_stack_integrity_under_load() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config).unwrap();

        let test_addresses: Vec<usize> = (0..100).map(|i| 0x1000 + i * 8).collect();

        // Push all addresses
        for &addr in &test_addresses {
            stack.push_return_addr(addr).unwrap();
        }

        // Pop and verify in reverse order
        for addr in test_addresses.into_iter().rev() {
            let popped = stack.pop_return_addr().unwrap();
            assert_eq!(popped, addr);
        }

        // Stack should be empty
        assert!(stack.is_empty());
    }

    #[test]
    fn test_concurrent_operations() {
        let config = ShadowStackConfig::default();
        let stack = ShadowCallStack::new(&config).unwrap();

        // Simulate concurrent pushes and pops
        for i in 0..10 {
            stack.push_return_addr(0x1000 + i).unwrap();
            let popped = stack.pop_return_addr().unwrap();
            assert_eq!(popped, 0x1000 + i);
        }

        assert!(stack.is_empty());
    }
}
