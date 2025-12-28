//! POSIX Real-time Extensions
//!
//! This module implements POSIX real-time scheduling and CPU affinity features:
//! - sched_setscheduler() / sched_getscheduler() - Scheduling policy management
//! - sched_setparam() / sched_getparam() - Scheduling parameter management
//! - sched_get_priority_max() / sched_get_priority_min() - Priority range queries
//! - sched_rr_get_interval() - Round-robin time slice
//! - sched_setaffinity() / sched_getaffinity() - CPU affinity management

use crate::posix::Pid;
use crate::subsystems::sync::Mutex;
use alloc::collections::BTreeMap;


/// Scheduling policies
pub const SCHED_NORMAL: i32 = 0;    // Normal (non-real-time) scheduling
pub const SCHED_FIFO: i32 = 1;      // First-in, first-out real-time scheduling
pub const SCHED_RR: i32 = 2;        // Round-robin real-time scheduling
pub const SCHED_BATCH: i32 = 3;      // Batch scheduling (Linux-specific)
pub const SCHED_IDLE: i32 = 5;       // Idle scheduling (Linux-specific)
pub const SCHED_DEADLINE: i32 = 6;   // Deadline scheduling (Linux-specific)

/// Scheduling parameters structure
#[derive(Debug, Clone, Copy)]
pub struct SchedParam {
    /// Scheduling priority (0-99 for real-time policies)
    pub sched_priority: i32,
}

impl Default for SchedParam {
    fn default() -> Self {
        Self { sched_priority: 0 }
    }
}

impl SchedParam {
    /// Create new scheduling parameters
    pub fn new(priority: i32) -> Self {
        Self { sched_priority: priority }
    }

    /// Validate priority for given policy
    pub fn is_valid_for_policy(&self, policy: i32) -> bool {
        match policy {
            SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE => self.sched_priority == 0,
            SCHED_FIFO | SCHED_RR => {
                self.sched_priority >= 1 && self.sched_priority <= 99
            }
            SCHED_DEADLINE => {
                // Deadline scheduling uses different parameters
                false
            }
            _ => false,
        }
    }
}

/// CPU set structure for affinity management
#[derive(Debug, Clone)]
pub struct CpuSet {
    /// CPU bitmap (up to 1024 CPUs)
    bits: [u64; 16], // 16 * 64 = 1024 bits
}

impl CpuSet {
    /// Create an empty CPU set
    pub fn new() -> Self {
        Self { bits: [0; 16] }
    }

    /// Create a CPU set with all CPUs set
    pub fn all() -> Self {
        Self { bits: [0xFFFFFFFFFFFFFFFF; 16] }
    }

    /// Set a CPU in the set
    pub fn set(&mut self, cpu: usize) {
        if cpu < 1024 {
            let word = cpu / 64;
            let bit = cpu % 64;
            self.bits[word] |= 1u64 << bit;
        }
    }

    /// Clear a CPU from the set
    pub fn clear(&mut self, cpu: usize) {
        if cpu < 1024 {
            let word = cpu / 64;
            let bit = cpu % 64;
            self.bits[word] &= !(1u64 << bit);
        }
    }

    /// Check if a CPU is set
    pub fn is_set(&self, cpu: usize) -> bool {
        if cpu < 1024 {
            let word = cpu / 64;
            let bit = cpu % 64;
            (self.bits[word] & (1u64 << bit)) != 0
        } else {
            false
        }
    }

    /// Get the number of CPUs set
    pub fn count(&self) -> usize {
        self.bits.iter().map(|word| word.count_ones() as usize).sum()
    }

    /// Get the first CPU set in the set
    pub fn first(&self) -> Option<usize> {
        for (word_idx, &word) in self.bits.iter().enumerate() {
            if word != 0 {
                let bit = word.trailing_zeros() as usize;
                return Some(word_idx * 64 + bit);
            }
        }
        None
    }

    /// Clear all CPUs
    pub fn clear_all(&mut self) {
        self.bits = [0; 16];
    }

    /// Set all CPUs
    pub fn set_all(&mut self) {
        self.bits = [0xFFFFFFFFFFFFFFFF; 16];
    }

    /// Convert to byte array for system calls
    pub fn to_bytes(&self) -> [u8; 128] {
        let mut bytes = [0u8; 128];
        for (i, &word) in self.bits.iter().enumerate() {
            let start = i * 8;
            bytes[start..start + 8].copy_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    /// Convert from byte array
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut bits = [0u64; 16];
        for i in 0..16 {
            let start = i * 8;
            if start + 8 <= bytes.len() {
                bits[i] = u64::from_le_bytes([
                    bytes[start], bytes[start + 1], bytes[start + 2], bytes[start + 3],
                    bytes[start + 4], bytes[start + 5], bytes[start + 6], bytes[start + 7],
                ]);
            }
        }
        Self { bits }
    }
}

impl Default for CpuSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Process scheduling information
#[derive(Debug, Clone)]
pub struct ProcessSchedInfo {
    /// Scheduling policy
    pub policy: i32,
    /// Scheduling parameters
    pub param: SchedParam,
    /// CPU affinity
    pub affinity: CpuSet,
    /// Round-robin time slice (in nanoseconds)
    pub rr_timeslice: u64,
    /// Last scheduled time
    pub last_scheduled: u64,
    /// CPU time used (in nanoseconds)
    pub cpu_time: u64,
}

impl ProcessSchedInfo {
    /// Create new scheduling info with default values
    pub fn new() -> Self {
        Self {
            policy: SCHED_NORMAL,
            param: SchedParam::default(),
            affinity: CpuSet::all(),
            rr_timeslice: 10_000_000, // 10ms default
            last_scheduled: crate::subsystems::time::get_timestamp(),
            cpu_time: 0,
        }
    }

    /// Create new scheduling info with specified policy and priority
    pub fn with_policy(policy: i32, priority: i32) -> Result<Self, SchedError> {
        let param = SchedParam::new(priority);
        if !param.is_valid_for_policy(policy) {
            return Err(SchedError::InvalidPriority);
        }

        Ok(Self {
            policy,
            param,
            affinity: CpuSet::all(),
            rr_timeslice: 10_000_000,
            last_scheduled: crate::subsystems::time::get_timestamp(),
            cpu_time: 0,
        })
    }

    /// Update scheduling policy and parameters
    pub fn set_policy(&mut self, policy: i32, param: SchedParam) -> Result<(), SchedError> {
        if !param.is_valid_for_policy(policy) {
            return Err(SchedError::InvalidPriority);
        }

        self.policy = policy;
        self.param = param;
        self.last_scheduled = crate::subsystems::time::get_timestamp();
        Ok(())
    }

    /// Update scheduling parameters
    pub fn set_param(&mut self, param: SchedParam) -> Result<(), SchedError> {
        if !param.is_valid_for_policy(self.policy) {
            return Err(SchedError::InvalidPriority);
        }

        self.param = param;
        self.last_scheduled = crate::subsystems::time::get_timestamp();
        Ok(())
    }

    /// Set CPU affinity
    pub fn set_affinity(&mut self, affinity: CpuSet) -> Result<(), SchedError> {
        if affinity.count() == 0 {
            return Err(SchedError::InvalidAffinity);
        }

        self.affinity = affinity;
        Ok(())
    }

    /// Get priority range for the current policy
    pub fn get_priority_range(&self) -> (i32, i32) {
        match self.policy {
            SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE => (0, 0),
            SCHED_FIFO | SCHED_RR => (1, 99),
            SCHED_DEADLINE => (0, 0), // Deadline scheduling uses different parameters
            _ => (0, 0),
        }
    }

    /// Check if this is a real-time scheduling policy
    pub fn is_realtime(&self) -> bool {
        matches!(self.policy, SCHED_FIFO | SCHED_RR | SCHED_DEADLINE)
    }

    /// Update CPU usage statistics
    pub fn update_cpu_time(&mut self, delta_ns: u64) {
        self.cpu_time += delta_ns;
    }

    /// Get CPU usage in milliseconds
    pub fn get_cpu_time_ms(&self) -> u64 {
        self.cpu_time / 1_000_000
    }
}

/// Scheduling errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedError {
    /// Invalid scheduling policy
    InvalidPolicy,
    /// Invalid priority for policy
    InvalidPriority,
    /// Invalid CPU affinity
    InvalidAffinity,
    /// Process not found
    ProcessNotFound,
    /// Permission denied
    PermissionDenied,
    /// Operation not supported
    NotSupported,
}

/// Global scheduling registry
pub static SCHED_REGISTRY: Mutex<SchedRegistry> = Mutex::new(SchedRegistry::new());

/// Scheduling registry for managing process scheduling information
#[derive(Debug)]
pub struct SchedRegistry {
    /// Map from process ID to scheduling info
    processes: BTreeMap<Pid, ProcessSchedInfo>,
    /// Number of available CPUs
    cpu_count: usize,
    /// Default round-robin time slice
    default_rr_timeslice: u64,
}

impl SchedRegistry {
    /// Create new scheduling registry
    pub const fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            cpu_count: 1, // Will be updated during init
            default_rr_timeslice: 10_000_000, // 10ms
        }
    }

    /// Initialize the registry
    pub fn init(&mut self, cpu_count: usize) {
        self.cpu_count = cpu_count;
        crate::println!("[sched] Initializing scheduling registry with {} CPUs", cpu_count);
    }

    /// Get or create scheduling info for a process
    pub fn get_or_create(&mut self, pid: Pid) -> &mut ProcessSchedInfo {
        if !self.processes.contains_key(&pid) {
            self.processes.insert(pid, ProcessSchedInfo::new());
        }
        self.processes.get_mut(&pid).unwrap()
    }

    /// Get scheduling info for a process
    pub fn get(&self, pid: Pid) -> Option<&ProcessSchedInfo> {
        self.processes.get(&pid)
    }

    /// Remove scheduling info for a process
    pub fn remove(&mut self, pid: Pid) -> Option<ProcessSchedInfo> {
        self.processes.remove(&pid)
    }

    /// Get all processes with real-time scheduling
    pub fn get_realtime_processes(&self) -> impl Iterator<Item = (&Pid, &ProcessSchedInfo)> {
        self.processes.iter().filter(|(_, info)| info.is_realtime())
    }

    /// Get scheduling statistics
    pub fn get_stats(&self) -> SchedStats {
        let mut policy_counts = [0u32; 7]; // Count for each policy type
        let mut realtime_count = 0;
        let mut total_cpu_time = 0u64;

        for info in self.processes.values() {
            let policy_idx = match info.policy {
                SCHED_NORMAL => 0,
                SCHED_FIFO => 1,
                SCHED_RR => 2,
                SCHED_BATCH => 3,
                SCHED_IDLE => 4,
                SCHED_DEADLINE => 5,
                _ => 6,
            };
            if policy_idx < 7 {
                policy_counts[policy_idx] += 1;
            }

            if info.is_realtime() {
                realtime_count += 1;
            }

            total_cpu_time += info.cpu_time;
        }

        SchedStats {
            total_processes: self.processes.len(),
            realtime_processes: realtime_count,
            policy_counts,
            total_cpu_time_ms: total_cpu_time / 1_000_000,
            cpu_count: self.cpu_count,
            default_rr_timeslice_ms: self.default_rr_timeslice / 1_000_000,
        }
    }
}

/// Scheduling statistics
#[derive(Debug, Clone)]
pub struct SchedStats {
    /// Total number of processes
    pub total_processes: usize,
    /// Number of real-time processes
    pub realtime_processes: usize,
    /// Count of processes by policy
    pub policy_counts: [u32; 7],
    /// Total CPU time used by all processes (ms)
    pub total_cpu_time_ms: u64,
    /// Number of CPUs in the system
    pub cpu_count: usize,
    /// Default round-robin time slice (ms)
    pub default_rr_timeslice_ms: u64,
}

/// Set scheduling policy and parameters for a process
pub fn sched_setscheduler(pid: Pid, policy: i32, param: SchedParam) -> Result<(), SchedError> {
    // Validate policy
    match policy {
        SCHED_NORMAL | SCHED_FIFO | SCHED_RR | SCHED_BATCH | SCHED_IDLE | SCHED_DEADLINE => {},
        _ => return Err(SchedError::InvalidPolicy),
    }

    // Check permissions (simplified - in real implementation would check capabilities)
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SchedError::ProcessNotFound),
    };

    if pid != current_pid && current_pid != 0 {
        return Err(SchedError::PermissionDenied);
    }

    // Update scheduling info
    let mut registry = SCHED_REGISTRY.lock();
    let sched_info = registry.get_or_create(pid);
    sched_info.set_policy(policy, param)
}

/// Get scheduling policy for a process
pub fn sched_getscheduler(pid: Pid) -> Result<i32, SchedError> {
    let registry = SCHED_REGISTRY.lock();
    match registry.get(pid) {
        Some(info) => Ok(info.policy),
        None => Err(SchedError::ProcessNotFound),
    }
}

/// Set scheduling parameters for a process
pub fn sched_setparam(pid: Pid, param: SchedParam) -> Result<(), SchedError> {
    // Check permissions
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SchedError::ProcessNotFound),
    };

    if pid != current_pid && current_pid != 0 {
        return Err(SchedError::PermissionDenied);
    }

    // Update scheduling parameters
    let mut registry = SCHED_REGISTRY.lock();
    let sched_info = registry.get_or_create(pid);
    sched_info.set_param(param)
}

/// Get scheduling parameters for a process
pub fn sched_getparam(pid: Pid) -> Result<SchedParam, SchedError> {
    let registry = SCHED_REGISTRY.lock();
    match registry.get(pid) {
        Some(info) => Ok(info.param),
        None => Err(SchedError::ProcessNotFound),
    }
}

/// Get maximum priority for a scheduling policy
pub fn sched_get_priority_max(policy: i32) -> Result<i32, SchedError> {
    match policy {
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE | SCHED_DEADLINE => Ok(0),
        SCHED_FIFO | SCHED_RR => Ok(99),
        _ => Err(SchedError::InvalidPolicy),
    }
}

/// Get minimum priority for a scheduling policy
pub fn sched_get_priority_min(policy: i32) -> Result<i32, SchedError> {
    match policy {
        SCHED_NORMAL | SCHED_BATCH | SCHED_IDLE | SCHED_DEADLINE => Ok(0),
        SCHED_FIFO | SCHED_RR => Ok(1),
        _ => Err(SchedError::InvalidPolicy),
    }
}

/// Get round-robin time slice for a process
pub fn sched_rr_get_interval(pid: Pid) -> Result<u64, SchedError> {
    let registry = SCHED_REGISTRY.lock();
    match registry.get(pid) {
        Some(info) => {
            if info.policy == SCHED_RR {
                Ok(info.rr_timeslice)
            } else {
                Err(SchedError::InvalidPolicy)
            }
        }
        None => Err(SchedError::ProcessNotFound),
    }
}

/// Set CPU affinity for a process
pub fn sched_setaffinity(pid: Pid, cpusetsize: usize, affinity: &CpuSet) -> Result<(), SchedError> {
    // Check permissions
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SchedError::ProcessNotFound),
    };

    if pid != current_pid && current_pid != 0 {
        return Err(SchedError::PermissionDenied);
    }

    // Validate CPU set size
    let cpu_count = {
        let registry = SCHED_REGISTRY.lock();
        registry.cpu_count
    };

    if cpusetsize == 0 || affinity.count() == 0 {
        return Err(SchedError::InvalidAffinity);
    }

    // Update affinity
    let mut registry = SCHED_REGISTRY.lock();
    let sched_info = registry.get_or_create(pid);
    sched_info.set_affinity(affinity.clone())
}

/// Get CPU affinity for a process
pub fn sched_getaffinity(pid: Pid, cpusetsize: usize) -> Result<CpuSet, SchedError> {
    let registry = SCHED_REGISTRY.lock();
    match registry.get(pid) {
        Some(info) => {
            let mut affinity = info.affinity.clone();
            
            // Mask out CPUs beyond the requested size
            for cpu in cpusetsize..1024 {
                affinity.clear(cpu);
            }
            
            Ok(affinity)
        }
        None => Err(SchedError::ProcessNotFound),
    }
}

/// Initialize real-time scheduling subsystem
pub fn init_realtime() {
    crate::println!("[sched] Initializing POSIX real-time scheduling subsystem");
    
    // Detect CPU count (simplified)
    let cpu_count = 4; // In real implementation, would detect actual CPU count
    
    let mut registry = SCHED_REGISTRY.lock();
    registry.init(cpu_count);
    
    crate::println!("[sched] Real-time scheduling initialized");
    crate::println!("[sched] Available scheduling policies:");
    crate::println!("[sched]   SCHED_NORMAL ({})", SCHED_NORMAL);
    crate::println!("[sched]   SCHED_FIFO ({})", SCHED_FIFO);
    crate::println!("[sched]   SCHED_RR ({})", SCHED_RR);
    crate::println!("[sched]   SCHED_BATCH ({})", SCHED_BATCH);
    crate::println!("[sched]   SCHED_IDLE ({})", SCHED_IDLE);
    crate::println!("[sched]   SCHED_DEADLINE ({})", SCHED_DEADLINE);
    crate::println!("[sched] Real-time priority range: 1-99");
    crate::println!("[sched] Default RR timeslice: {}ms", registry.default_rr_timeslice / 1_000_000);
}

/// Cleanup real-time scheduling subsystem
pub fn cleanup_realtime() {
    crate::println!("[sched] Cleaning up POSIX real-time scheduling subsystem");
    
    let registry = SCHED_REGISTRY.lock();
    let stats = registry.get_stats();
    
    crate::println!("[sched] Cleanup stats:");
    crate::println!("[sched]   Total processes: {}", stats.total_processes);
    crate::println!("[sched]   Real-time processes: {}", stats.realtime_processes);
    crate::println!("[sched]   Total CPU time: {}ms", stats.total_cpu_time_ms);
    crate::println!("[sched]   CPU count: {}", stats.cpu_count);
    
    // Note: We don't clear the registry here as it might be needed for cleanup
}