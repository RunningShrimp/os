//! Hardware Performance Counters
//!
//! This module provides hardware performance counters for x86_64 architecture.
//! It uses CPU performance monitoring units (PMU) to collect low-level metrics.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::cpu::NCPU;

/// Read Time-Stamp Counter (x86_64 specific)
#[inline]
#[cfg(target_arch = "x86_64")]
unsafe fn rdtsc() -> u64 {
    let (high, low): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("edx") high,
            out("eax") low,
            options(nostack, nomem, pure)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Read Time-Stamp Counter (fallback for non-x86_64)
#[inline]
#[cfg(not(target_arch = "x86_64"))]
unsafe fn rdtsc() -> u64 {
    // Fallback implementation using current time
    nos_api::event::get_time_ns()
}

/// Maximum number of hardware counters per category
const MAX_COUNTERS: usize = 16;

/// Hardware counter types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardwareCounterType {
    // CPU Counters
    InstructionsRetired,
    CpuCyclesUnhalted,
    ReferenceCycles,

    // Cache Counters (L1)
    L1CacheReferences,
    L1CacheMisses,
    L1InstructionCacheMisses,
    L1DataCacheMisses,

    // Cache Counters (L2)
    L2CacheReferences,
    L2CacheMisses,

    // Cache Counters (L3)
    L3CacheReferences,
    L3CacheMisses,

    // Branch Prediction
    BranchInstructions,
    BranchMisses,

    // TLB Counters
    InstructionTlbHits,
    InstructionTlbMisses,
    DataTlbHits,
    DataTlbMisses,

    // Memory Access
    MemoryAccesses,
    MemoryCycles,

    // Pipeline
    StalledCycles,
    StalledCyclesFrontend,
    StalledCyclesBackend,
}

impl HardwareCounterType {
    /// Get all hardware counter types
    pub fn all() -> alloc::vec::Vec<Self> {
        alloc::vec![
            Self::InstructionsRetired,
            Self::CpuCyclesUnhalted,
            Self::ReferenceCycles,
            Self::L1CacheReferences,
            Self::L1CacheMisses,
            Self::L1InstructionCacheMisses,
            Self::L1DataCacheMisses,
            Self::L2CacheReferences,
            Self::L2CacheMisses,
            Self::L3CacheReferences,
            Self::L3CacheMisses,
            Self::BranchInstructions,
            Self::BranchMisses,
            Self::InstructionTlbHits,
            Self::InstructionTlbMisses,
            Self::DataTlbHits,
            Self::DataTlbMisses,
            Self::MemoryAccesses,
            Self::MemoryCycles,
            Self::StalledCycles,
            Self::StalledCyclesFrontend,
            Self::StalledCyclesBackend,
        ]
    }

    /// Get counter name
    pub fn name(&self) -> &str {
        match self {
            Self::InstructionsRetired => "instructions_retired",
            Self::CpuCyclesUnhalted => "cpu_cycles_unhalted",
            Self::ReferenceCycles => "reference_cycles",
            Self::L1CacheReferences => "l1_cache_references",
            Self::L1CacheMisses => "l1_cache_misses",
            Self::L1InstructionCacheMisses => "l1_instruction_cache_misses",
            Self::L1DataCacheMisses => "l1_data_cache_misses",
            Self::L2CacheReferences => "l2_cache_references",
            Self::L2CacheMisses => "l2_cache_misses",
            Self::L3CacheReferences => "l3_cache_references",
            Self::L3CacheMisses => "l3_cache_misses",
            Self::BranchInstructions => "branch_instructions",
            Self::BranchMisses => "branch_misses",
            Self::InstructionTlbHits => "instruction_tlb_hits",
            Self::InstructionTlbMisses => "instruction_tlb_misses",
            Self::DataTlbHits => "data_tlb_hits",
            Self::DataTlbMisses => "data_tlb_misses",
            Self::MemoryAccesses => "memory_accesses",
            Self::MemoryCycles => "memory_cycles",
            Self::StalledCycles => "stalled_cycles",
            Self::StalledCyclesFrontend => "stalled_cycles_frontend",
            Self::StalledCyclesBackend => "stalled_cycles_backend",
        }
    }

    /// Get counter description
    pub fn description(&self) -> &str {
        match self {
            Self::InstructionsRetired => "Number of instructions retired",
            Self::CpuCyclesUnhalted => "CPU cycles while not halted",
            Self::ReferenceCycles => "Reference clock cycles",
            Self::L1CacheReferences => "L1 cache references",
            Self::L1CacheMisses => "L1 cache misses",
            Self::L1InstructionCacheMisses => "L1 instruction cache misses",
            Self::L1DataCacheMisses => "L1 data cache misses",
            Self::L2CacheReferences => "L2 cache references",
            Self::L2CacheMisses => "L2 cache misses",
            Self::L3CacheReferences => "L3 cache references",
            Self::L3CacheMisses => "L3 cache misses",
            Self::BranchInstructions => "Branch instructions retired",
            Self::BranchMisses => "Branch mispredictions",
            Self::InstructionTlbHits => "Instruction TLB hits",
            Self::InstructionTlbMisses => "Instruction TLB misses",
            Self::DataTlbHits => "Data TLB hits",
            Self::DataTlbMisses => "Data TLB misses",
            Self::MemoryAccesses => "Memory access operations",
            Self::MemoryCycles => "Cycles waiting for memory",
            Self::StalledCycles => "Stalled cycles",
            Self::StalledCyclesFrontend => "Frontend stalled cycles",
            Self::StalledCyclesBackend => "Backend stalled cycles",
        }
    }

    /// Get counter unit
    pub fn unit(&self) -> &str {
        match self {
            Self::InstructionsRetired => "instructions",
            Self::CpuCyclesUnhalted => "cycles",
            Self::ReferenceCycles => "cycles",
            _ => "events",
        }
    }
}

/// Hardware counter value
#[derive(Debug, Clone)]
pub struct HardwareCounterValue {
    /// Counter type
    pub counter_type: HardwareCounterType,
    /// Current value
    pub value: u64,
    /// Previous value (for delta calculation)
    pub prev_value: u64,
    /// Time of last update
    pub timestamp: u64,
}

impl HardwareCounterValue {
    /// Create new counter value
    pub fn new(counter_type: HardwareCounterType) -> Self {
        Self {
            counter_type,
            value: 0,
            prev_value: 0,
            timestamp: 0,
        }
    }

    /// Update counter value
    pub fn update(&mut self, value: u64, timestamp: u64) {
        self.prev_value = self.value;
        self.value = value;
        self.timestamp = timestamp;
    }

    /// Get delta since last update
    pub fn delta(&self) -> u64 {
        self.value.saturating_sub(self.prev_value)
    }

    /// Get rate per second
    pub fn rate(&self) -> f64 {
        let delta = self.delta();
        let time_delta = if self.timestamp > self.prev_value {
            self.timestamp - self.prev_value
        } else {
            1
        };

        if time_delta == 0 {
            0.0
        } else {
            (delta as f64 * 1_000_000_000.0) / (time_delta as f64)
        }
    }
}

/// Per-CPU hardware counters
#[repr(C)]
pub struct PerCpuHardwareCounters {
    /// Instructions retired
    pub instructions_retired: AtomicU64,
    /// CPU cycles unhalted
    pub cpu_cycles_unhalted: AtomicU64,
    /// Reference cycles
    pub reference_cycles: AtomicU64,
    /// L1 cache references
    pub l1_cache_references: AtomicU64,
    /// L1 cache misses
    pub l1_cache_misses: AtomicU64,
    /// L1 instruction cache misses
    pub l1_instruction_cache_misses: AtomicU64,
    /// L1 data cache misses
    pub l1_data_cache_misses: AtomicU64,
    /// L2 cache references
    pub l2_cache_references: AtomicU64,
    /// L2 cache misses
    pub l2_cache_misses: AtomicU64,
    /// L3 cache references
    pub l3_cache_references: AtomicU64,
    /// L3 cache misses
    pub l3_cache_misses: AtomicU64,
    /// Branch instructions
    pub branch_instructions: AtomicU64,
    /// Branch misses
    pub branch_misses: AtomicU64,
    /// Instruction TLB hits
    pub instruction_tlb_hits: AtomicU64,
    /// Instruction TLB misses
    pub instruction_tlb_misses: AtomicU64,
    /// Data TLB hits
    pub data_tlb_hits: AtomicU64,
    /// Data TLB misses
    pub data_tlb_misses: AtomicU64,
    /// Memory accesses
    pub memory_accesses: AtomicU64,
    /// Memory cycles
    pub memory_cycles: AtomicU64,
    /// Stalled cycles
    pub stalled_cycles: AtomicU64,
    /// Stalled cycles frontend
    pub stalled_cycles_frontend: AtomicU64,
    /// Stalled cycles backend
    pub stalled_cycles_backend: AtomicU64,
}

impl PerCpuHardwareCounters {
    /// Create new per-CPU counters
    pub const fn new() -> Self {
        Self {
            instructions_retired: AtomicU64::new(0),
            cpu_cycles_unhalted: AtomicU64::new(0),
            reference_cycles: AtomicU64::new(0),
            l1_cache_references: AtomicU64::new(0),
            l1_cache_misses: AtomicU64::new(0),
            l1_instruction_cache_misses: AtomicU64::new(0),
            l1_data_cache_misses: AtomicU64::new(0),
            l2_cache_references: AtomicU64::new(0),
            l2_cache_misses: AtomicU64::new(0),
            l3_cache_references: AtomicU64::new(0),
            l3_cache_misses: AtomicU64::new(0),
            branch_instructions: AtomicU64::new(0),
            branch_misses: AtomicU64::new(0),
            instruction_tlb_hits: AtomicU64::new(0),
            instruction_tlb_misses: AtomicU64::new(0),
            data_tlb_hits: AtomicU64::new(0),
            data_tlb_misses: AtomicU64::new(0),
            memory_accesses: AtomicU64::new(0),
            memory_cycles: AtomicU64::new(0),
            stalled_cycles: AtomicU64::new(0),
            stalled_cycles_frontend: AtomicU64::new(0),
            stalled_cycles_backend: AtomicU64::new(0),
        }
    }

    /// Increment counter
    #[inline]
    pub fn increment(&self, counter_type: HardwareCounterType, delta: u64) {
        match counter_type {
            HardwareCounterType::InstructionsRetired => {
                self.instructions_retired.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::CpuCyclesUnhalted => {
                self.cpu_cycles_unhalted.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::ReferenceCycles => {
                self.reference_cycles.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::L1CacheReferences => {
                self.l1_cache_references.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::L1CacheMisses => {
                self.l1_cache_misses.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::L1InstructionCacheMisses => {
                self.l1_instruction_cache_misses.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::L1DataCacheMisses => {
                self.l1_data_cache_misses.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::L2CacheReferences => {
                self.l2_cache_references.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::L2CacheMisses => {
                self.l2_cache_misses.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::L3CacheReferences => {
                self.l3_cache_references.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::L3CacheMisses => {
                self.l3_cache_misses.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::BranchInstructions => {
                self.branch_instructions.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::BranchMisses => {
                self.branch_misses.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::InstructionTlbHits => {
                self.instruction_tlb_hits.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::InstructionTlbMisses => {
                self.instruction_tlb_misses.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::DataTlbHits => {
                self.data_tlb_hits.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::DataTlbMisses => {
                self.data_tlb_misses.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::MemoryAccesses => {
                self.memory_accesses.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::MemoryCycles => {
                self.memory_cycles.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::StalledCycles => {
                self.stalled_cycles.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::StalledCyclesFrontend => {
                self.stalled_cycles_frontend.fetch_add(delta, Ordering::Relaxed);
            }
            HardwareCounterType::StalledCyclesBackend => {
                self.stalled_cycles_backend.fetch_add(delta, Ordering::Relaxed);
            }
        }
    }

    /// Get counter value
    #[inline]
    pub fn get(&self, counter_type: HardwareCounterType) -> u64 {
        match counter_type {
            HardwareCounterType::InstructionsRetired => self.instructions_retired.load(Ordering::Relaxed),
            HardwareCounterType::CpuCyclesUnhalted => self.cpu_cycles_unhalted.load(Ordering::Relaxed),
            HardwareCounterType::ReferenceCycles => self.reference_cycles.load(Ordering::Relaxed),
            HardwareCounterType::L1CacheReferences => self.l1_cache_references.load(Ordering::Relaxed),
            HardwareCounterType::L1CacheMisses => self.l1_cache_misses.load(Ordering::Relaxed),
            HardwareCounterType::L1InstructionCacheMisses => {
                self.l1_instruction_cache_misses.load(Ordering::Relaxed)
            }
            HardwareCounterType::L1DataCacheMisses => self.l1_data_cache_misses.load(Ordering::Relaxed),
            HardwareCounterType::L2CacheReferences => self.l2_cache_references.load(Ordering::Relaxed),
            HardwareCounterType::L2CacheMisses => self.l2_cache_misses.load(Ordering::Relaxed),
            HardwareCounterType::L3CacheReferences => self.l3_cache_references.load(Ordering::Relaxed),
            HardwareCounterType::L3CacheMisses => self.l3_cache_misses.load(Ordering::Relaxed),
            HardwareCounterType::BranchInstructions => self.branch_instructions.load(Ordering::Relaxed),
            HardwareCounterType::BranchMisses => self.branch_misses.load(Ordering::Relaxed),
            HardwareCounterType::InstructionTlbHits => self.instruction_tlb_hits.load(Ordering::Relaxed),
            HardwareCounterType::InstructionTlbMisses => self.instruction_tlb_misses.load(Ordering::Relaxed),
            HardwareCounterType::DataTlbHits => self.data_tlb_hits.load(Ordering::Relaxed),
            HardwareCounterType::DataTlbMisses => self.data_tlb_misses.load(Ordering::Relaxed),
            HardwareCounterType::MemoryAccesses => self.memory_accesses.load(Ordering::Relaxed),
            HardwareCounterType::MemoryCycles => self.memory_cycles.load(Ordering::Relaxed),
            HardwareCounterType::StalledCycles => self.stalled_cycles.load(Ordering::Relaxed),
            HardwareCounterType::StalledCyclesFrontend => self.stalled_cycles_frontend.load(Ordering::Relaxed),
            HardwareCounterType::StalledCyclesBackend => self.stalled_cycles_backend.load(Ordering::Relaxed),
        }
    }

    /// Reset all counters
    pub fn reset(&self) {
        self.instructions_retired.store(0, Ordering::Relaxed);
        self.cpu_cycles_unhalted.store(0, Ordering::Relaxed);
        self.reference_cycles.store(0, Ordering::Relaxed);
        self.l1_cache_references.store(0, Ordering::Relaxed);
        self.l1_cache_misses.store(0, Ordering::Relaxed);
        self.l1_instruction_cache_misses.store(0, Ordering::Relaxed);
        self.l1_data_cache_misses.store(0, Ordering::Relaxed);
        self.l2_cache_references.store(0, Ordering::Relaxed);
        self.l2_cache_misses.store(0, Ordering::Relaxed);
        self.l3_cache_references.store(0, Ordering::Relaxed);
        self.l3_cache_misses.store(0, Ordering::Relaxed);
        self.branch_instructions.store(0, Ordering::Relaxed);
        self.branch_misses.store(0, Ordering::Relaxed);
        self.instruction_tlb_hits.store(0, Ordering::Relaxed);
        self.instruction_tlb_misses.store(0, Ordering::Relaxed);
        self.data_tlb_hits.store(0, Ordering::Relaxed);
        self.data_tlb_misses.store(0, Ordering::Relaxed);
        self.memory_accesses.store(0, Ordering::Relaxed);
        self.memory_cycles.store(0, Ordering::Relaxed);
        self.stalled_cycles.store(0, Ordering::Relaxed);
        self.stalled_cycles_frontend.store(0, Ordering::Relaxed);
        self.stalled_cycles_backend.store(0, Ordering::Relaxed);
    }

    /// Get all counters as a map
    pub fn as_map(&self) -> BTreeMap<String, u64> {
        let mut map = BTreeMap::new();
        map.insert("instructions_retired".to_string(), self.instructions_retired.load(Ordering::Relaxed));
        map.insert("cpu_cycles_unhalted".to_string(), self.cpu_cycles_unhalted.load(Ordering::Relaxed));
        map.insert("reference_cycles".to_string(), self.reference_cycles.load(Ordering::Relaxed));
        map.insert("l1_cache_references".to_string(), self.l1_cache_references.load(Ordering::Relaxed));
        map.insert("l1_cache_misses".to_string(), self.l1_cache_misses.load(Ordering::Relaxed));
        map.insert(
            "l1_instruction_cache_misses".to_string(),
            self.l1_instruction_cache_misses.load(Ordering::Relaxed),
        );
        map.insert(
            "l1_data_cache_misses".to_string(),
            self.l1_data_cache_misses.load(Ordering::Relaxed),
        );
        map.insert("l2_cache_references".to_string(), self.l2_cache_references.load(Ordering::Relaxed));
        map.insert("l2_cache_misses".to_string(), self.l2_cache_misses.load(Ordering::Relaxed));
        map.insert("l3_cache_references".to_string(), self.l3_cache_references.load(Ordering::Relaxed));
        map.insert("l3_cache_misses".to_string(), self.l3_cache_misses.load(Ordering::Relaxed));
        map.insert(
            "branch_instructions".to_string(),
            self.branch_instructions.load(Ordering::Relaxed),
        );
        map.insert("branch_misses".to_string(), self.branch_misses.load(Ordering::Relaxed));
        map.insert(
            "instruction_tlb_hits".to_string(),
            self.instruction_tlb_hits.load(Ordering::Relaxed),
        );
        map.insert(
            "instruction_tlb_misses".to_string(),
            self.instruction_tlb_misses.load(Ordering::Relaxed),
        );
        map.insert("data_tlb_hits".to_string(), self.data_tlb_hits.load(Ordering::Relaxed));
        map.insert("data_tlb_misses".to_string(), self.data_tlb_misses.load(Ordering::Relaxed));
        map.insert("memory_accesses".to_string(), self.memory_accesses.load(Ordering::Relaxed));
        map.insert("memory_cycles".to_string(), self.memory_cycles.load(Ordering::Relaxed));
        map.insert("stalled_cycles".to_string(), self.stalled_cycles.load(Ordering::Relaxed));
        map.insert(
            "stalled_cycles_frontend".to_string(),
            self.stalled_cycles_frontend.load(Ordering::Relaxed),
        );
        map.insert(
            "stalled_cycles_backend".to_string(),
            self.stalled_cycles_backend.load(Ordering::Relaxed),
        );
        map
    }
}

/// Hardware counter manager
pub struct HardwareCounterManager {
    /// Per-CPU counters
    per_cpu_counters: [PerCpuHardwareCounters; NCPU],
    /// Counter values
    counter_values: spin::Mutex<BTreeMap<String, HardwareCounterValue>>,
    /// Is PMU available
    pmu_available: bool,
}

impl HardwareCounterManager {
    /// Create new hardware counter manager
    pub fn new() -> Self {
        Self {
            per_cpu_counters: [(); NCPU].map(|_| PerCpuHardwareCounters::new()),
            counter_values: spin::Mutex::new(BTreeMap::new()),
            pmu_available: Self::detect_pmu(),
        }
    }

    /// Detect if PMU is available
    fn detect_pmu() -> bool {
        // Check CPUID for performance monitoring support
        // For now, assume available on x86_64
        cfg!(target_arch = "x86_64")
    }

    /// Increment hardware counter
    #[inline]
    pub fn increment(&self, cpu_id: usize, counter_type: HardwareCounterType, delta: u64) {
        if cpu_id < NCPU {
            self.per_cpu_counters[cpu_id].increment(counter_type, delta);
        }
    }

    /// Get counter value for specific CPU
    #[inline]
    pub fn get_cpu_counter(&self, cpu_id: usize, counter_type: HardwareCounterType) -> u64 {
        if cpu_id < NCPU {
            self.per_cpu_counters[cpu_id].get(counter_type)
        } else {
            0
        }
    }

    /// Get aggregated counter value across all CPUs
    pub fn get_aggregated_counter(&self, counter_type: HardwareCounterType) -> u64 {
        let mut total = 0u64;
        for cpu in 0..NCPU {
            total += self.per_cpu_counters[cpu].get(counter_type);
        }
        total
    }

    /// Get all counters for specific CPU
    pub fn get_cpu_counters(&self, cpu_id: usize) -> BTreeMap<String, u64> {
        if cpu_id < NCPU {
            self.per_cpu_counters[cpu_id].as_map()
        } else {
            BTreeMap::new()
        }
    }

    /// Get all counters aggregated across all CPUs
    pub fn get_all_counters(&self) -> BTreeMap<String, u64> {
        let mut aggregated = BTreeMap::new();
        for cpu in 0..NCPU {
            let cpu_counters = self.per_cpu_counters[cpu].as_map();
            for (key, value) in cpu_counters {
                *aggregated.entry(key).or_insert(0) += value;
            }
        }
        aggregated
    }

    /// Reset counters for specific CPU
    pub fn reset_cpu_counters(&self, cpu_id: usize) {
        if cpu_id < NCPU {
            self.per_cpu_counters[cpu_id].reset();
        }
    }

    /// Reset all counters
    pub fn reset_all_counters(&self) {
        for cpu in 0..NCPU {
            self.per_cpu_counters[cpu].reset();
        }
    }

    /// Read TSC (Time Stamp Counter)
    #[inline]
    pub fn read_tsc(&self) -> u64 {
        unsafe { rdtsc() }
    }

    /// Calculate IPC (Instructions Per Cycle)
    pub fn calculate_ipc(&self, cpu_id: usize) -> f64 {
        let instructions = self.get_cpu_counter(cpu_id, HardwareCounterType::InstructionsRetired);
        let cycles = self.get_cpu_counter(cpu_id, HardwareCounterType::CpuCyclesUnhalted);

        if cycles == 0 {
            0.0
        } else {
            instructions as f64 / cycles as f64
        }
    }

    /// Calculate cache miss rate
    pub fn calculate_cache_miss_rate(&self, cpu_id: usize, level: u8) -> f64 {
        let (references, misses) = match level {
            1 => (
                self.get_cpu_counter(cpu_id, HardwareCounterType::L1CacheReferences),
                self.get_cpu_counter(cpu_id, HardwareCounterType::L1CacheMisses),
            ),
            2 => (
                self.get_cpu_counter(cpu_id, HardwareCounterType::L2CacheReferences),
                self.get_cpu_counter(cpu_id, HardwareCounterType::L2CacheMisses),
            ),
            3 => (
                self.get_cpu_counter(cpu_id, HardwareCounterType::L3CacheReferences),
                self.get_cpu_counter(cpu_id, HardwareCounterType::L3CacheMisses),
            ),
            _ => return 0.0,
        };

        if references == 0 {
            0.0
        } else {
            (misses as f64 / references as f64) * 100.0
        }
    }

    /// Calculate branch miss rate
    pub fn calculate_branch_miss_rate(&self, cpu_id: usize) -> f64 {
        let instructions = self.get_cpu_counter(cpu_id, HardwareCounterType::BranchInstructions);
        let misses = self.get_cpu_counter(cpu_id, HardwareCounterType::BranchMisses);

        if instructions == 0 {
            0.0
        } else {
            (misses as f64 / instructions as f64) * 100.0
        }
    }

    /// Calculate TLB miss rate
    pub fn calculate_tlb_miss_rate(&self, cpu_id: usize) -> f64 {
        let hits = self.get_cpu_counter(cpu_id, HardwareCounterType::DataTlbHits)
            + self.get_cpu_counter(cpu_id, HardwareCounterType::InstructionTlbHits);
        let misses = self.get_cpu_counter(cpu_id, HardwareCounterType::DataTlbMisses)
            + self.get_cpu_counter(cpu_id, HardwareCounterType::InstructionTlbMisses);

        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            (misses as f64 / total as f64) * 100.0
        }
    }

    /// Get PMU availability
    pub fn is_pmu_available(&self) -> bool {
        self.pmu_available
    }

    /// Get performance metrics summary
    pub fn get_metrics_summary(&self, cpu_id: usize) -> BTreeMap<String, f64> {
        let mut summary = BTreeMap::new();

        summary.insert("ipc".to_string(), self.calculate_ipc(cpu_id));
        summary.insert("l1_miss_rate".to_string(), self.calculate_cache_miss_rate(cpu_id, 1));
        summary.insert("l2_miss_rate".to_string(), self.calculate_cache_miss_rate(cpu_id, 2));
        summary.insert("l3_miss_rate".to_string(), self.calculate_cache_miss_rate(cpu_id, 3));
        summary.insert("branch_miss_rate".to_string(), self.calculate_branch_miss_rate(cpu_id));
        summary.insert("tlb_miss_rate".to_string(), self.calculate_tlb_miss_rate(cpu_id));

        summary
    }
}

impl Default for HardwareCounterManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global hardware counter manager
static mut GLOBAL_HW_COUNTER_MANAGER: Option<HardwareCounterManager> = None;
static HW_COUNTER_INIT: spin::Mutex<bool> = spin::Mutex::new(false);

/// Initialize hardware counter manager
pub fn init_hardware_counters() {
    let mut is_init = HW_COUNTER_INIT.lock();
    if *is_init {
        return;
    }

    unsafe {
        GLOBAL_HW_COUNTER_MANAGER = Some(HardwareCounterManager::new());
    }

    *is_init = true;
    log::info!("Hardware performance counters initialized");
}

/// Get hardware counter manager
pub fn get_hw_counter_manager() -> &'static HardwareCounterManager {
    unsafe {
        if GLOBAL_HW_COUNTER_MANAGER.is_none() {
            init_hardware_counters();
        }
        GLOBAL_HW_COUNTER_MANAGER.as_ref().unwrap()
    }
}

/// Convenience function to increment hardware counter
#[inline]
pub fn increment_hw_counter(counter_type: HardwareCounterType, delta: u64) {
    let cpu_id = crate::cpu::cpuid();
    get_hw_counter_manager().increment(cpu_id, counter_type, delta);
}

/// Convenience function to read TSC
#[inline]
pub fn read_tsc() -> u64 {
    unsafe { rdtsc() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_creation() {
        let counter = HardwareCounterValue::new(HardwareCounterType::InstructionsRetired);
        assert_eq!(counter.value, 0);
        assert_eq!(counter.delta(), 0);
    }

    #[test]
    fn test_counter_update() {
        let mut counter = HardwareCounterValue::new(HardwareCounterType::CpuCyclesUnhalted);
        counter.update(1000, 100);
        assert_eq!(counter.value, 1000);
        assert_eq!(counter.delta(), 1000);

        counter.update(2000, 200);
        assert_eq!(counter.value, 2000);
        assert_eq!(counter.delta(), 1000);
    }

    #[test]
    fn test_per_cpu_counters() {
        let counters = PerCpuHardwareCounters::new();
        counters.increment(HardwareCounterType::InstructionsRetired, 100);
        assert_eq!(counters.get(HardwareCounterType::InstructionsRetired), 100);

        counters.increment(HardwareCounterType::InstructionsRetired, 50);
        assert_eq!(counters.get(HardwareCounterType::InstructionsRetired), 150);
    }

    #[test]
    fn test_manager_creation() {
        let manager = HardwareCounterManager::new();
        assert_eq!(manager.is_pmu_available(), cfg!(target_arch = "x86_64"));
    }
}
