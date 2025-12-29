//! Application Processor (AP) startup synchronization
//!
//! This module provides synchronization primitives for coordinating
//! the startup of multiple CPU cores, ensuring all APs are properly
//! initialized before the system continues.

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

/// Maximum number of CPUs supported
const MAX_CPUS: usize = 256;

/// AP startup timeout in nanoseconds (10 seconds)
const AP_STARTUP_TIMEOUT_NS: u64 = 10_000_000_000;

/// CPU startup state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum CpuStartupState {
    /// CPU has not started
    NotStarted = 0,
    /// CPU is initializing
    Initializing = 1,
    /// CPU is ready
    Ready = 2,
    /// CPU failed to start
    Failed = 3,
}

/// AP startup barrier for synchronizing multiple CPUs
pub struct ApStartupBarrier {
    /// Number of CPUs expected to start
    expected_cpus: AtomicU32,
    /// Number of CPUs that have successfully started
    ready_cpus: AtomicU32,
    /// Number of CPUs that failed to start
    failed_cpus: AtomicU32,
    /// Whether the barrier has been initialized
    initialized: AtomicBool,
    /// Per-CPU startup state
    cpu_states: [AtomicU32; MAX_CPUS],
    /// Timestamp when barrier was initialized (for timeout)
    start_timestamp: AtomicU64,
}

impl ApStartupBarrier {
    /// Create a new AP startup barrier
    pub const fn new() -> Self {
        // Create array of AtomicU32 with default value
        const INIT_ATOMIC: AtomicU32 = AtomicU32::new(0);
        let cpu_states = [INIT_ATOMIC; MAX_CPUS];

        Self {
            expected_cpus: AtomicU32::new(0),
            ready_cpus: AtomicU32::new(0),
            failed_cpus: AtomicU32::new(0),
            initialized: AtomicBool::new(false),
            cpu_states,
            start_timestamp: AtomicU64::new(0),
        }
    }

    /// Initialize the barrier for a given number of CPUs
    ///
    /// # Arguments
    /// * `cpu_count` - Total number of CPUs including BSP
    ///
    /// # Returns
    /// `true` if initialization succeeded, `false` if already initialized
    pub fn initialize(&self, cpu_count: u32) -> bool {
        // Use CAS to ensure only one initialization
        if self
            .initialized
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            return false;
        }

        self.expected_cpus.store(cpu_count, Ordering::SeqCst);
        self.ready_cpus.store(0, Ordering::SeqCst);
        self.failed_cpus.store(0, Ordering::SeqCst);

        // Reset all CPU states
        for state in &self.cpu_states {
            state.store(CpuStartupState::NotStarted as u32, Ordering::Relaxed);
        }

        // Record start timestamp
        self.start_timestamp.store(Self::get_timestamp(), Ordering::SeqCst);

        true
    }

    /// Mark a CPU as starting initialization
    ///
    /// # Arguments
    /// * `cpu_id` - CPU ID (0 = BSP, 1+ = APs)
    ///
    /// # Returns
    /// `true` if the state was set, `false` if the CPU ID is invalid
    pub fn cpu_starting(&self, cpu_id: usize) -> bool {
        if cpu_id >= MAX_CPUS {
            return false;
        }

        self.cpu_states[cpu_id].store(
            CpuStartupState::Initializing as u32,
            Ordering::SeqCst,
        );
        true
    }

    /// Mark a CPU as ready (completed initialization)
    ///
    /// # Arguments
    /// * `cpu_id` - CPU ID
    ///
    /// # Returns
    /// `true` if the state was set, `false` if the CPU ID is invalid
    pub fn cpu_ready(&self, cpu_id: usize) -> bool {
        if cpu_id >= MAX_CPUS {
            return false;
        }

        self.cpu_states[cpu_id].store(CpuStartupState::Ready as u32, Ordering::SeqCst);
        let count = self.ready_cpus.fetch_add(1, Ordering::SeqCst);
        crate::println!("[ap_sync] CPU {} is ready ({} / {} ready)", cpu_id, count + 1, self.expected_cpus.load(Ordering::Relaxed));

        true
    }

    /// Mark a CPU as failed
    ///
    /// # Arguments
    /// * `cpu_id` - CPU ID
    ///
    /// # Returns
    /// `true` if the state was set, `false` if the CPU ID is invalid
    pub fn cpu_failed(&self, cpu_id: usize) -> bool {
        if cpu_id >= MAX_CPUS {
            return false;
        }

        self.cpu_states[cpu_id].store(CpuStartupState::Failed as u32, Ordering::SeqCst);
        let count = self.failed_cpus.fetch_add(1, Ordering::SeqCst);
        crate::println!("[ap_sync] CPU {} failed to start ({} / {} failed)", cpu_id, count + 1, self.expected_cpus.load(Ordering::Relaxed));

        true
    }

    /// Wait for all CPUs to be ready or timeout
    ///
    /// # Returns
    /// `ApStartupResult` indicating success, timeout, or failure
    pub fn wait_for_all(&self) -> ApStartupResult {
        let expected = self.expected_cpus.load(Ordering::Acquire) as usize;
        let start = self.get_timestamp();

        loop {
            let ready = self.ready_cpus.load(Ordering::Acquire) as usize;
            let failed = self.failed_cpus.load(Ordering::Acquire) as usize;
            let total = ready + failed;

            // Check if all CPUs have reported in
            if total >= expected {
                if failed > 0 {
                    return ApStartupResult::PartialFailure {
                        ready_cpus: ready,
                        failed_cpus: failed,
                    };
                }
                return ApStartupResult::AllReady { cpu_count: ready };
            }

            // Check for timeout
            let elapsed = self.get_timestamp().saturating_sub(start);
            if elapsed > AP_STARTUP_TIMEOUT_NS {
                return ApStartupResult::Timeout {
                    ready_cpus: ready,
                    failed_cpus: failed,
                    pending_cpus: expected.saturating_sub(total),
                };
            }

            // Small delay to prevent busy-waiting
            self.cpu_relax();
        }
    }

    /// Check if all CPUs are ready without waiting
    pub fn all_ready(&self) -> bool {
        let ready = self.ready_cpus.load(Ordering::Acquire);
        let expected = self.expected_cpus.load(Ordering::Acquire);
        ready == expected
    }

    /// Get the current number of ready CPUs
    pub fn ready_count(&self) -> usize {
        self.ready_cpus.load(Ordering::Acquire) as usize
    }

    /// Get the current number of failed CPUs
    pub fn failed_count(&self) -> usize {
        self.failed_cpus.load(Ordering::Acquire) as usize
    }

    /// Get the expected number of CPUs
    pub fn expected_count(&self) -> usize {
        self.expected_cpus.load(Ordering::Acquire) as usize
    }

    /// Get the state of a specific CPU
    pub fn cpu_state(&self, cpu_id: usize) -> Option<CpuStartupState> {
        if cpu_id >= MAX_CPUS {
            return None;
        }

        match self.cpu_states[cpu_id].load(Ordering::Acquire) {
            0 => Some(CpuStartupState::NotStarted),
            1 => Some(CpuStartupState::Initializing),
            2 => Some(CpuStartupState::Ready),
            3 => Some(CpuStartupState::Failed),
            _ => None,
        }
    }

    /// Get list of failed CPU IDs
    pub fn failed_cpu_ids(&self) -> alloc::vec::Vec<usize> {
        let mut failed = alloc::vec::Vec::new();
        let expected = self.expected_cpus.load(Ordering::Acquire) as usize;

        for cpu_id in 0..core::cmp::min(expected, MAX_CPUS) {
            if self.cpu_state(cpu_id) == Some(CpuStartupState::Failed) {
                failed.push(cpu_id);
            }
        }

        failed
    }

    /// Small CPU pause to reduce power consumption during busy-wait
    #[inline]
    fn cpu_relax(&self) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("pause");
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("yield");
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            // RISC-V doesn't have a pause instruction, use a hint
            core::arch::asm!("addi x0, x0, 0");
        }
    }

    /// Get current timestamp in nanoseconds
    ///
    /// Note: This is a simplified implementation. In a real system,
    /// this would read from a proper timer/counter.
    fn get_timestamp() -> u64 {
        // TODO: Use actual timer (TSC, generic timer, etc.)
        // For now, return 0 as a placeholder
        0
    }

    /// Reset the barrier for reuse
    pub fn reset(&self) {
        self.initialized.store(false, Ordering::SeqCst);
        self.expected_cpus.store(0, Ordering::SeqCst);
        self.ready_cpus.store(0, Ordering::SeqCst);
        self.failed_cpus.store(0, Ordering::SeqCst);
        self.start_timestamp.store(0, Ordering::SeqCst);

        for state in &self.cpu_states {
            state.store(CpuStartupState::NotStarted as u32, Ordering::Relaxed);
        }
    }
}

/// Result of AP startup operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApStartupResult {
    /// All CPUs started successfully
    AllReady { cpu_count: usize },
    /// Some CPUs failed to start
    PartialFailure {
        ready_cpus: usize,
        failed_cpus: usize,
    },
    /// Startup timed out
    Timeout {
        ready_cpus: usize,
        failed_cpus: usize,
        pending_cpus: usize,
    },
}

impl ApStartupResult {
    /// Check if startup was successful
    pub fn is_success(&self) -> bool {
        matches!(self, ApStartupResult::AllReady { .. })
    }

    /// Get the total number of CPUs that reported in
    pub fn total_reported(&self) -> usize {
        match self {
            ApStartupResult::AllReady { cpu_count } => *cpu_count,
            ApStartupResult::PartialFailure {
                ready_cpus,
                failed_cpus,
            } => ready_cpus + failed_cpus,
            ApStartupResult::Timeout {
                ready_cpus,
                failed_cpus,
                pending_cpus,
            } => ready_cpus + failed_cpus + pending_cpus,
        }
    }

    /// Get description of the result
    pub fn description(&self) -> alloc::string::String {
        match self {
            ApStartupResult::AllReady { cpu_count } => {
                alloc::format!("All {} CPUs started successfully", cpu_count)
            }
            ApStartupResult::PartialFailure {
                ready_cpus,
                failed_cpus,
            } => {
                alloc::format!(
                    "Partial failure: {} CPUs ready, {} CPUs failed",
                    ready_cpus, failed_cpus
                )
            }
            ApStartupResult::Timeout {
                ready_cpus,
                failed_cpus,
                pending_cpus,
            } => {
                alloc::format!(
                    "Timeout: {} CPUs ready, {} CPUs failed, {} CPUs still pending",
                    ready_cpus, failed_cpus, pending_cpus
                )
            }
        }
    }
}

/// Global AP startup barrier instance
static AP_BARRIER: ApStartupBarrier = ApStartupBarrier::new();

/// Get the global AP startup barrier
pub fn get_ap_barrier() -> &'static ApStartupBarrier {
    &AP_BARRIER
}

/// Initialize AP synchronization for a given number of CPUs
pub fn init_ap_sync(cpu_count: u32) -> bool {
    get_ap_barrier().initialize(cpu_count)
}

/// Mark the calling CPU as ready
pub fn mark_cpu_ready(cpu_id: usize) -> bool {
    get_ap_barrier().cpu_ready(cpu_id)
}

/// Mark the calling CPU as failed
pub fn mark_cpu_failed(cpu_id: usize) -> bool {
    get_ap_barrier().cpu_failed(cpu_id)
}

/// Wait for all CPUs to be ready
pub fn wait_all_cpus() -> ApStartupResult {
    get_ap_barrier().wait_for_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barrier_initialization() {
        let barrier = ApStartupBarrier::new();
        assert!(barrier.initialize(4));
        assert_eq!(barrier.expected_count(), 4);
        assert_eq!(barrier.ready_count(), 0);
        assert!(barrier.all_ready() == false);
    }

    #[test]
    fn test_cpu_states() {
        let barrier = ApStartupBarrier::new();
        barrier.initialize(2);

        assert!(barrier.cpu_starting(0));
        assert!(barrier.cpu_ready(0));
        assert_eq!(barrier.cpu_state(0), Some(CpuStartupState::Ready));
        assert_eq!(barrier.ready_count(), 1);

        assert!(barrier.cpu_failed(1));
        assert_eq!(barrier.cpu_state(1), Some(CpuStartupState::Failed));
        assert_eq!(barrier.failed_count(), 1);
    }

    #[test]
    fn test_invalid_cpu_id() {
        let barrier = ApStartupBarrier::new();
        barrier.initialize(4);

        assert!(!barrier.cpu_starting(MAX_CPUS));
        assert!(!barrier.cpu_ready(MAX_CPUS));
        assert!(!barrier.cpu_failed(MAX_CPUS));
    }

    #[test]
    fn test_result_description() {
        let result = ApStartupResult::AllReady { cpu_count: 4 };
        assert!(result.description().contains("4"));
        assert!(result.is_success());

        let result = ApStartupResult::PartialFailure {
            ready_cpus: 2,
            failed_cpus: 1,
        };
        assert!(!result.is_success());
        assert!(result.description().contains("2"));
        assert!(result.description().contains("1"));
    }
}
