//! Performance Profiler
//!
//! This module implements performance profiling tools:
//! - Function profiling
//! - System call tracing
//! - Memory profiling
//! - CPU cycle counting
//!
//! Features:
//! - Call graph generation
//! - Hot spot detection
//! - Flame graph output
//! - Statistical analysis

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;

// ============================================================================
// Profiler Constants
// ============================================================================

/// Maximum profiled functions
pub const MAX_PROFILED_FUNCTIONS: usize = 1 << 14;

/// Maximum samples per function
pub const MAX_SAMPLES: usize = 1 << 10;

// ============================================================================
// Profile Types
// ============================================================================

/// Profile type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileType {
    /// CPU time profiling
    CpuTime,
    
    /// Memory allocation profiling
    MemoryAllocation,
    
    /// Cache hit/miss profiling
    CacheProfile,
    
    /// System call profiling
    SyscallProfile,
}

/// Profile entry
#[derive(Debug, Clone)]
pub struct ProfileEntry {
    pub function_name: String,
    pub call_count: AtomicU64,
    pub total_time_ns: AtomicU64,
    pub self_time_ns: AtomicU64,
    pub max_time_ns: AtomicU64,
    pub min_time_ns: AtomicU64,
    pub allocations: AtomicU64,
    pub deallocations: AtomicU64,
    pub peak_memory: AtomicUsize,
    pub call_tree: Mutex<Vec<ProfileCall>>>,
}

/// Profile call (for call tree)
#[derive(Debug, Clone)]
pub struct ProfileCall {
    pub caller: Option<String>,
    pub callee: String,
    pub timestamp: u64,
    pub duration_ns: u64,
}

impl ProfileEntry {
    pub fn new(function_name: String) -> Self {
        Self {
            function_name,
            call_count: AtomicU64::new(0),
            total_time_ns: AtomicU64::new(0),
            self_time_ns: AtomicU64::new(0),
            max_time_ns: AtomicU64::new(0),
            min_time_ns: AtomicU64::new(u64::MAX),
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
            peak_memory: AtomicUsize::new(0),
            call_tree: Mutex::new(Vec::new()),
        }
    }

    pub fn record_call(&self, duration_ns: u64) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        self.total_time_ns.fetch_add(duration_ns, Ordering::Relaxed);
        
        // Update max/min
        let current_max = self.max_time_ns.load(Ordering::Relaxed);
        let mut max_updated = false;
        
        while duration_ns > current_max {
            if self.max_time_ns.compare_exchange(current_max, duration_ns, Ordering::Relaxed) == current_max {
                max_updated = true;
                break;
            }
            max_updated = false;
        }
        
        let current_min = self.min_time_ns.load(Ordering::Relaxed);
        let mut min_updated = false;
        
        while duration_ns < current_min {
            if self.min_time_ns.compare_exchange(current_min, duration_ns, Ordering::Relaxed) == current_min {
                min_updated = true;
                break;
            }
            min_updated = false;
        }
    }

    pub fn record_allocation(&self, size: usize) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        
        // Update peak memory
        let current_peak = self.peak_memory.load(Ordering::Relaxed);
        let mut peak_updated = false;
        
        while size > current_peak {
            if self.peak_memory.compare_exchange(current_peak, size, Ordering::Relaxed) == current_peak {
                peak_updated = true;
                break;
            }
            peak_updated = false;
        }
    }

    pub fn record_deallocation(&self) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_call_count(&self) -> u64 {
        self.call_count.load(Ordering::Relaxed)
    }

    pub fn get_total_time(&self) -> u64 {
        self.total_time_ns.load(Ordering::Relaxed)
    }

    pub fn get_avg_time(&self) -> u64 {
        let count = self.call_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0;
        }
        self.total_time_ns.load(Ordering::Relaxed) / count
    }

    pub fn get_max_time(&self) -> u64 {
        self.max_time_ns.load(Ordering::Relaxed)
    }

    pub fn get_min_time(&self) -> u64 {
        self.min_time_ns.load(Ordering::Relaxed)
    }

    pub fn get_stats(&self) -> ProfileStats {
        ProfileStats {
            call_count: self.get_call_count(),
            total_time_ns: self.get_total_time(),
            avg_time_ns: self.get_avg_time(),
            max_time_ns: self.get_max_time(),
            min_time_ns: self.get_min_time(),
            allocations: self.allocations.load(Ordering::Relaxed),
            deallocations: self.deallocations.load(Ordering::Relaxed),
            peak_memory: self.peak_memory.load(Ordering::Relaxed),
        }
    }
}

/// Profile statistics
#[derive(Debug, Clone, Copy)]
pub struct ProfileStats {
    pub call_count: u64,
    pub total_time_ns: u64,
    pub avg_time_ns: u64,
    pub max_time_ns: u64,
    pub min_time_ns: u64,
    pub allocations: u64,
    pub deallocations: u64,
    pub peak_memory: usize,
}

// ============================================================================
// Performance Profiler
// ============================================================================

/// Performance profiler
pub struct PerformanceProfiler {
    pub profiles: Mutex<BTreeMap<String, Arc<ProfileEntry>>>>,
    pub current_profile: Option<String>,
    pub enabled: AtomicBool,
    pub sample_interval: AtomicU64,
    pub next_sample_time: AtomicU64,
    pub stats: Mutex<ProfilerStats>,
}

/// Profiler statistics
#[derive(Debug, Clone, Copy)]
pub struct ProfilerStats {
    pub total_functions: usize,
    pub active_functions: usize,
    pub total_samples: u64,
    pub profiling_time_ns: u64,
}

impl Default for ProfilerStats {
    fn default() -> Self {
        Self {
            total_functions: 0,
            active_functions: 0,
            total_samples: 0,
            profiling_time_ns: 0,
        }
    }
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            profiles: Mutex::new(BTreeMap::new()),
            current_profile: None,
            enabled: AtomicBool::new(false),
            sample_interval: AtomicU64::new(1_000_000), // 1ms
            next_sample_time: AtomicU64::new(0),
            stats: Mutex::new(ProfilerStats::default()),
        }
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
        crate::println!("[profiler] Performance profiler enabled");
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        crate::println!("[profiler] Performance profiler disabled");
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn start_profile(&self, function_name: String) -> Result<(), String> {
        if !self.is_enabled() {
            return Err("Profiler disabled".to_string());
        }

        let mut profiles = self.profiles.lock();
        
        if !profiles.contains_key(&function_name) {
            if profiles.len() >= MAX_PROFILED_FUNCTIONS {
                return Err("Maximum functions reached".to_string());
            }

            let profile = Arc::new(ProfileEntry::new(function_name.clone()));
            profiles.insert(function_name.clone(), profile);
            crate::println!("[profiler] Started profiling: {}", function_name);
        }

        self.current_profile = Some(function_name);
        Ok(())
    }

    pub fn end_profile(&self, function_name: String, duration_ns: u64) -> Result<(), String> {
        if !self.is_enabled() {
            return Err("Profiler disabled".to_string());
        }

        let profiles = self.profiles.lock();
        let profile = profiles.get(&function_name)
            .ok_or(alloc::string::String::from("Function ") + &function_name.to_string() + alloc::string::String::from(" not profiled"))?;

        profile.record_call(duration_ns);

        if let Some(current) = &self.current_profile {
            if current == &function_name {
                self.current_profile = None;
            }
        }

        Ok(())
    }

    pub fn record_allocation(&self, function_name: String, size: usize) -> Result<(), String> {
        if !self.is_enabled() {
            return Err("Profiler disabled".to_string());
        }

        let profiles = self.profiles.lock();
        let profile = profiles.get(&function_name)
            .ok_or(alloc::string::String::from("Function ") + &function_name.to_string() + alloc::string::String::from(" not profiled"))?;

        profile.record_allocation(size);
        Ok(())
    }

    pub fn record_deallocation(&self, function_name: String) -> Result<(), String> {
        if !self.is_enabled() {
            return Err("Profiler disabled".to_string());
        }

        let profiles = self.profiles.lock();
        let profile = profiles.get(&function_name)
            .ok_or(alloc::string::String::from("Function ") + &function_name.to_string() + alloc::string::String::from(" not profiled"))?;

        profile.record_deallocation();
        Ok(())
    }

    pub fn get_profile(&self, function_name: String) -> Option<Arc<ProfileEntry>> {
        let profiles = self.profiles.lock();
        profiles.get(&function_name).cloned()
    }

    pub fn get_all_profiles(&self) -> Vec<Arc<ProfileEntry>> {
        let profiles = self.profiles.lock();
        profiles.values().cloned().collect()
    }

    pub fn find_hot_spots(&self, threshold_percent: f64) -> Vec<(String, ProfileStats)> {
        let profiles = self.profiles.lock();
        let mut hot_spots = Vec::new();

        let total_time: u64 = profiles.values()
            .map(|p| p.get_total_time())
            .sum();

        for (name, profile) in profiles.iter() {
            let stats = profile.get_stats();
            let percentage = (stats.total_time_ns as f64) / (total_time as f64) * 100.0;

            if percentage >= threshold_percent {
                hot_spots.push((name.clone(), stats));
            }
        }

        // Sort by total time descending
        hot_spots.sort_by(|a, b| b.1.total_time_ns.cmp(&a.1.total_time_ns));

        hot_spots
    }

    pub fn generate_flame_graph(&self) -> String {
        let mut flame = String::new();
        let profiles = self.profiles.lock();

        flame.push_str("flamegraph()\n");

        for (name, profile) in profiles.iter() {
            let stats = profile.get_stats();
            let width = (stats.total_time_ns as f64).log10() as usize;
            
            for _ in 0..width {
                flame.push_str(" ");
            }

            flame.push_str(&name);
            flame.push_str(" ");
            flame.push_str(&(stats.total_time_ns).to_string());
            flame.push_str("\n");
        }

        flame
    }

    pub fn get_stats(&self) -> ProfilerStats {
        let mut stats = self.stats.lock();
        stats.total_functions = self.profiles.lock().len();
        *stats
    }
}
