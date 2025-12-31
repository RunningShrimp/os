//! Cache Optimization Module
//!
//! This module provides cache-aware data structures and optimization strategies:
//! - Cache line optimization and padding
//! - Prefetching strategies for data access patterns
//! - Cache-friendly data structures
//! - NUMA-aware data placement
//! - Cache simulation and analysis
//!
//! # Architecture
//!
//! Modern CPUs have multi-level cache hierarchies:
//! - L1: ~32 KB per core, ~4 cycle latency
//! - L2: ~256 KB per core, ~12 cycle latency
//! - L3: ~8 MB shared, ~40 cycle latency
//! - RAM: Gigabytes, ~200 cycle latency
//!
//! # Performance
//!
//! Cache optimization can provide 2-10x speedup for memory-intensive workloads.

#![allow(missing_docs)]

use crate::prelude::*;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// Cache line size (typical for x86_64)
pub const CACHE_LINE_SIZE: usize = 64;

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheLevel {
    /// L1 cache
    L1 = 1,
    /// L2 cache
    L2 = 2,
    /// L3 cache
    L3 = 3,
}

/// Cache information
#[derive(Debug, Clone)]
pub struct CacheInfo {
    /// Cache level
    pub level: CacheLevel,
    /// Size in bytes
    pub size: usize,
    /// Line size in bytes
    pub line_size: usize,
    /// Associativity (ways)
    pub associativity: usize,
    /// Number of sets
    pub num_sets: usize,
}

impl CacheInfo {
    /// Create new cache info
    pub fn new(level: CacheLevel, size: usize, line_size: usize, associativity: usize) -> Self {
        let num_sets = size / (line_size * associativity);

        Self {
            level,
            size,
            line_size,
            associativity,
            num_sets,
        }
    }

    /// Get typical L1 cache info
    pub fn l1() -> Self {
        Self::new(CacheLevel::L1, 32 * 1024, 64, 8)
    }

    /// Get typical L2 cache info
    pub fn l2() -> Self {
        Self::new(CacheLevel::L2, 256 * 1024, 64, 8)
    }

    /// Get typical L3 cache info
    pub fn l3() -> Self {
        Self::new(CacheLevel::L3, 8 * 1024 * 1024, 64, 16)
    }

    /// Calculate cache index for address
    pub fn index(&self, addr: usize) -> usize {
        (addr / self.line_size) % self.num_sets
    }

    /// Calculate cache tag for address
    pub fn tag(&self, addr: usize) -> usize {
        addr / (self.line_size * self.num_sets)
    }
}

/// Cache set for simulation
#[derive(Debug, Clone)]
struct CacheSet {
    /// Cache lines
    lines: Vec<CacheLine>,
}

/// Cache line
#[derive(Debug, Clone)]
struct CacheLine {
    /// Tag
    tag: usize,
    /// Valid bit
    valid: bool,
    /// Dirty bit
    dirty: bool,
    /// Access count (for LRU)
    access_count: u64,
}

impl CacheLine {
    /// Create new invalid cache line
    fn new() -> Self {
        Self {
            tag: 0,
            valid: false,
            dirty: false,
            access_count: 0,
        }
    }
}

impl CacheSet {
    /// Create new cache set
    fn new(associativity: usize) -> Self {
        Self {
            lines: vec![CacheLine::new(); associativity],
        }
    }

    /// Access cache line
    fn access(&mut self, tag: usize) -> CacheAccess {
        // First try to find matching tag
        for (i, line) in self.lines.iter_mut().enumerate() {
            if line.valid && line.tag == tag {
                line.access_count += 1;
                return CacheAccess::Hit(i);
            }
        }

        // Not found, need to allocate
        // Find invalid line or LRU line
        let lru_index = self.lines.iter()
            .enumerate()
            .min_by_key(|(_, line)| line.access_count)
            .map(|(i, _)| i)
            .unwrap();

        let line = &mut self.lines[lru_index];
        let was_valid = line.valid;
        let old_tag = line.tag;

        line.tag = tag;
        line.valid = true;
        line.access_count += 1;

        if was_valid {
            CacheAccess::MissReplace(lru_index, old_tag)
        } else {
            CacheAccess::MissAllocate(lru_index)
        }
    }

    /// Invalidate cache line
    fn invalidate(&mut self, tag: usize) {
        for line in &mut self.lines {
            if line.valid && line.tag == tag {
                line.valid = false;
                line.dirty = false;
            }
        }
    }
}

/// Cache access result
#[derive(Debug, Clone, PartialEq)]
enum CacheAccess {
    /// Cache hit
    Hit(usize),
    /// Cache miss, allocated invalid line
    MissAllocate(usize),
    /// Cache miss, replaced existing line
    MissReplace(usize, usize),
}

/// Cache simulator for performance analysis
pub struct CacheSimulator {
    /// Cache info
    info: CacheInfo,
    /// Cache sets
    sets: Vec<CacheSet>,
    /// Total accesses
    total_accesses: AtomicU64,
    /// Total hits
    total_hits: AtomicU64,
    /// Total misses
    total_misses: AtomicU64,
}

impl CacheSimulator {
    /// Create new cache simulator
    pub fn new(info: CacheInfo) -> Self {
        let sets = (0..info.num_sets)
            .map(|_| CacheSet::new(info.associativity))
            .collect();

        Self {
            info,
            sets,
            total_accesses: AtomicU64::new(0),
            total_hits: AtomicU64::new(0),
            total_misses: AtomicU64::new(0),
        }
    }

    /// Simulate memory access
    pub fn access(&self, addr: usize) -> CacheAccessResult {
        self.total_accesses.fetch_add(1, Ordering::Relaxed);

        let index = self.info.index(addr);
        let tag = self.info.tag(addr);

        // We need interior mutability for simulation
        // In real implementation, use Mutex or atomic operations
        let sets = unsafe { &mut *(self.sets.as_ptr() as *mut Vec<CacheSet>) };
        let result = sets[index].access(tag);

        match result {
            CacheAccess::Hit(_) => {
                self.total_hits.fetch_add(1, Ordering::Relaxed);
                CacheAccessResult::Hit
            }
            _ => {
                self.total_misses.fetch_add(1, Ordering::Relaxed);
                CacheAccessResult::Miss
            }
        }
    }

    /// Get hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_accesses.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            self.total_hits.load(Ordering::Relaxed) as f64 / total as f64
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            total_accesses: self.total_accesses.load(Ordering::Relaxed),
            total_hits: self.total_hits.load(Ordering::Relaxed),
            total_misses: self.total_misses.load(Ordering::Relaxed),
            hit_rate: self.hit_rate(),
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.total_accesses.store(0, Ordering::Release);
        self.total_hits.store(0, Ordering::Release);
        self.total_misses.store(0, Ordering::Release);
    }
}

/// Cache access result
#[derive(Debug, Clone, PartialEq)]
pub enum CacheAccessResult {
    /// Cache hit
    Hit,
    /// Cache miss
    Miss,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total accesses
    pub total_accesses: u64,
    /// Total hits
    pub total_hits: u64,
    /// Total misses
    pub total_misses: u64,
    /// Hit rate
    pub hit_rate: f64,
}

/// Cache-friendly aligned buffer
#[repr(C, align(64))]
pub struct AlignedBuffer<T, const N: usize> {
    /// Data
    pub data: [T; N],
}

impl<T: Clone + Default + Copy, const N: usize> AlignedBuffer<T, N> {
    /// Create new aligned buffer
    pub fn new(value: T) -> Self {
        Self {
            data: [value; N],
        }
    }

    /// Create zeroed buffer
    pub fn zeroed() -> Self
    where
        T: Default + Clone,
    {
        Self::new(T::default())
    }

    /// Get buffer as slice
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Get buffer as mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }
}

impl<T: Clone + Default, const N: usize> AlignedBuffer<T, N> {
    /// Create zeroed buffer using Default trait
    pub fn from_default() -> Self {
        let mut data = Vec::with_capacity(N);
        for _ in 0..N {
            data.push(T::default());
        }
        // Convert Vec to array - this is safe because we initialized N elements
        let mut iter = data.into_iter();
        Self {
            data: [(); N].map(|_| iter.next().unwrap()),
        }
    }
}

/// Padded struct to prevent false sharing
#[repr(C, align(64))]
pub struct CachePadded<T> {
    /// Value
    pub value: T,
}

impl<T> CachePadded<T> {
    /// Create new padded value
    pub fn new(value: T) -> Self {
        Self { value }
    }

    /// Get inner value
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Get inner value mutably
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Into inner value
    pub fn into_inner(self) -> T {
        self.value
    }
}

/// Prefetch strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefetchStrategy {
    /// No prefetching
    None,
    /// Sequential prefetch (forward)
    SequentialForward,
    /// Sequential prefetch (backward)
    SequentialBackward,
    /// Strided prefetch with fixed stride
    Strided { stride: usize },
    /// Adaptive prefetch based on pattern detection
    Adaptive,
}

/// Prefetcher for memory access optimization
pub struct Prefetcher {
    /// Strategy
    strategy: PrefetchStrategy,
    /// History of recent accesses
    access_history: Mutex<Vec<usize>>,
    /// Detected stride
    detected_stride: Mutex<Option<usize>>,
}

impl Prefetcher {
    /// Create new prefetcher
    pub fn new(strategy: PrefetchStrategy) -> Self {
        Self {
            strategy,
            access_history: Mutex::new(Vec::with_capacity(16)),
            detected_stride: Mutex::new(None),
        }
    }

    /// Record memory access
    pub fn record_access(&self, addr: usize) {
        let mut history = self.access_history.lock();
        history.push(addr);

        if history.len() > 16 {
            history.remove(0);
        }

        // Detect stride if adaptive
        if self.strategy == PrefetchStrategy::Adaptive && history.len() >= 3 {
            let stride = history[history.len() - 1] - history[history.len() - 2];

            // Check if stride is consistent
            let consistent = if history.len() >= 3 {
                history.windows(2)
                    .all(|w| w[1] - w[0] == stride)
            } else {
                false
            };

            if consistent {
                let mut detected = self.detected_stride.lock();
                *detected = Some(stride);
            }
        }
    }

    /// Get prefetch address
    pub fn get_prefetch_addr(&self, current_addr: usize) -> Option<usize> {
        match self.strategy {
            PrefetchStrategy::None => None,
            PrefetchStrategy::SequentialForward => Some(current_addr + 64),
            PrefetchStrategy::SequentialBackward => {
                current_addr.checked_sub(64)
            }
            PrefetchStrategy::Strided { stride } => Some(current_addr + stride),
            PrefetchStrategy::Adaptive => {
                let detected = self.detected_stride.lock();
                detected.map(|stride| current_addr + stride)
            }
        }
    }

    /// Update strategy
    pub fn set_strategy(&mut self, strategy: PrefetchStrategy) {
        self.strategy = strategy;
    }

    /// Get detected stride
    pub fn detected_stride(&self) -> Option<usize> {
        *self.detected_stride.lock()
    }
}

/// Cache-friendly hash table using open addressing
pub struct CacheHashTable<K, V>
where
    K: Clone + PartialEq + core::hash::Hash,
    V: Clone,
{
    /// Table buckets
    buckets: Vec<Option<(K, V)>>,
    /// Number of elements
    len: usize,
    /// Load factor threshold
    load_factor_threshold: f64,
}

impl<K: Clone + PartialEq + core::hash::Hash, V: Clone> CacheHashTable<K, V> {
    /// Create new hash table
    pub fn new(capacity: usize) -> Self {
        Self {
            buckets: vec![None; capacity.next_power_of_two()],
            len: 0,
            load_factor_threshold: 0.7,
        }
    }

    /// Insert key-value pair
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.len as f64 / self.buckets.len() as f64 > self.load_factor_threshold {
            self.resize();
        }

        let index = self.hash(&key) % self.buckets.len();

        // Linear probe
        for i in 0..self.buckets.len() {
            let idx = (index + i) % self.buckets.len();
            match &mut self.buckets[idx] {
                Some((k, v)) if k == &key => {
                    let old = core::mem::replace(v, value.clone());
                    return Some(old);
                }
                None => {
                    self.buckets[idx] = Some((key, value));
                    self.len += 1;
                    return None;
                }
                _ => {}
            }
        }

        None
    }

    /// Get value by key
    pub fn get(&self, key: &K) -> Option<&V> {
        let index = self.hash(key) % self.buckets.len();

        for i in 0..self.buckets.len() {
            let idx = (index + i) % self.buckets.len();
            match &self.buckets[idx] {
                Some((k, v)) if k == key => return Some(v),
                None => return None,
                _ => {}
            }
        }

        None
    }

    /// Remove key
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let index = self.hash(key) % self.buckets.len();

        for i in 0..self.buckets.len() {
            let idx = (index + i) % self.buckets.len();
            if let Some((k, _)) = &self.buckets[idx] {
                if k == key {
                    let entry = self.buckets[idx].take();
                    self.len -= 1;
                    return entry.map(|(_, v)| v);
                }
            }
        }

        None
    }

    /// Hash function
    fn hash(&self, key: &K) -> usize {
        use core::hash::{Hash, Hasher};
        struct SimpleHasher(u64);
        impl Hasher for SimpleHasher {
            fn finish(&self) -> u64 {
                self.0
            }
            fn write(&mut self, bytes: &[u8]) {
                for byte in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(*byte as u64);
                }
            }
        }
        let mut hasher = SimpleHasher(0);
        key.hash(&mut hasher);
        hasher.finish() as usize
    }

    /// Resize table
    fn resize(&mut self) {
        let new_capacity = self.buckets.len() * 2;
        let old_buckets = core::mem::replace(&mut self.buckets, vec![None; new_capacity]);

        for entry in old_buckets.into_iter().flatten() {
            let index = self.hash(&entry.0) % self.buckets.len();

            for i in 0..self.buckets.len() {
                let idx = (index + i) % self.buckets.len();
                if self.buckets[idx].is_none() {
                    self.buckets[idx] = Some(entry);
                    break;
                }
            }
        }
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Cache-optimized array layout (structure of arrays)
pub struct StructureOfArrays<T, const N: usize> {
    /// Data arrays
    arrays: Vec<AlignedBuffer<T, N>>,
}

impl<T: Clone + Default + Copy, const N: usize> StructureOfArrays<T, N> {
    /// Create new SOA layout
    pub fn new(num_arrays: usize) -> Self {
        Self {
            arrays: (0..num_arrays)
                .map(|_| AlignedBuffer::zeroed())
                .collect(),
        }
    }

    /// Get element
    pub fn get(&self, array_index: usize, element_index: usize) -> Option<&T> {
        self.arrays
            .get(array_index)
            .and_then(|arr| arr.data.get(element_index))
    }

    /// Set element
    pub fn set(&mut self, array_index: usize, element_index: usize, value: T) -> bool {
        if let Some(arr) = self.arrays.get_mut(array_index) {
            if element_index < N {
                arr.data[element_index] = value;
                return true;
            }
        }
        false
    }

    /// Get array
    pub fn get_array(&self, array_index: usize) -> Option<&[T]> {
        self.arrays
            .get(array_index)
            .map(|arr| arr.as_slice())
    }

    /// Get array mutably
    pub fn get_array_mut(&mut self, array_index: usize) -> Option<&mut [T]> {
        self.arrays
            .get_mut(array_index)
            .map(|arr| arr.as_mut_slice())
    }
}

/// Memory access pattern analyzer
pub struct AccessPatternAnalyzer {
    /// Access history
    history: Mutex<Vec<AccessRecord>>,
    /// Detected pattern
    pattern: Mutex<Option<AccessPattern>>,
}

/// Access record
#[derive(Debug, Clone)]
struct AccessRecord {
    /// Address
    addr: usize,
    /// Timestamp
    timestamp: u64,
    /// Access size
    size: usize,
}

/// Detected access pattern
#[derive(Debug, Clone, PartialEq)]
pub enum AccessPattern {
    /// Sequential access
    Sequential { stride: usize, direction: i8 },
    /// Random access
    Random,
    /// Strided access
    Strided { stride: usize },
    /// Irregular access
    Irregular,
}

impl AccessPatternAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            history: Mutex::new(Vec::with_capacity(100)),
            pattern: Mutex::new(None),
        }
    }

    /// Record access
    pub fn record_access(&self, addr: usize, size: usize) {
        let mut history = self.history.lock();
        history.push(AccessRecord {
            addr,
            timestamp: crate::subsystems::time::hrtime_nanos(),
            size,
        });

        if history.len() > 100 {
            history.remove(0);
        }

        // Analyze pattern every 10 accesses
        if history.len() >= 10 && history.len() % 10 == 0 {
            let detected = self.analyze_pattern(&history);
            let mut pattern = self.pattern.lock();
            *pattern = Some(detected);
        }
    }

    /// Analyze access pattern
    fn analyze_pattern(&self, history: &[AccessRecord]) -> AccessPattern {
        if history.len() < 3 {
            return AccessPattern::Random;
        }

        // Check for sequential pattern
        let mut all_sequential = true;
        let _forward = true;
        let stride = history[1].addr as i64 - history[0].addr as i64;

        for window in history.windows(2) {
            let diff = window[1].addr as i64 - window[0].addr as i64;
            if diff != stride {
                all_sequential = false;
                break;
            }
        }

        if all_sequential && stride != 0 {
            return AccessPattern::Sequential {
                stride: stride.abs() as usize,
                direction: if stride > 0 { 1 } else { -1 },
            };
        }

        // Check for strided pattern
        let strides: Vec<i64> = history.windows(2)
            .map(|w| w[1].addr as i64 - w[0].addr as i64)
            .collect();

        if strides.iter().all(|&s| s == strides[0]) {
            return AccessPattern::Strided {
                stride: strides[0].abs() as usize,
            };
        }

        AccessPattern::Random
    }

    /// Get detected pattern
    pub fn get_pattern(&self) -> Option<AccessPattern> {
        self.pattern.lock().clone()
    }
}

/// Cache optimization advisor
pub struct CacheOptimizationAdvisor {
    /// L1 cache simulator
    l1_sim: Option<CacheSimulator>,
    /// Access pattern analyzer
    pattern_analyzer: AccessPatternAnalyzer,
}

impl CacheOptimizationAdvisor {
    /// Create new advisor
    pub fn new() -> Self {
        Self {
            l1_sim: Some(CacheSimulator::new(CacheInfo::l1())),
            pattern_analyzer: AccessPatternAnalyzer::new(),
        }
    }

    /// Record memory access
    pub fn record_access(&self, addr: usize, size: usize) {
        if let Some(ref sim) = self.l1_sim {
            sim.access(addr);
        }

        self.pattern_analyzer.record_access(addr, size);
    }

    /// Get optimization recommendations
    pub fn get_recommendations(&self) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // Check cache hit rate
        if let Some(ref sim) = self.l1_sim {
            let stats = sim.get_stats();
            if stats.hit_rate < 0.8 {
                recommendations.push(OptimizationRecommendation::LowCacheHitRate {
                    current_rate: stats.hit_rate,
                    suggested_improvement: "Improve data locality or use prefetching".to_string(),
                });
            }
        }

        // Check access pattern
        if let Some(pattern) = self.pattern_analyzer.get_pattern() {
            match pattern {
                AccessPattern::Sequential { stride, .. } => {
                    if stride > 64 {
                        recommendations.push(OptimizationRecommendation::EnablePrefetching {
                            strategy: PrefetchStrategy::SequentialForward,
                        });
                    }
                }
                AccessPattern::Strided { stride } => {
                    recommendations.push(OptimizationRecommendation::EnablePrefetching {
                        strategy: PrefetchStrategy::Strided { stride },
                    });
                }
                _ => {}
            }
        }

        recommendations
    }

    /// Get L1 cache statistics
    pub fn get_l1_stats(&self) -> Option<CacheStats> {
        self.l1_sim.as_ref().map(|sim| sim.get_stats())
    }
}

/// Optimization recommendation
#[derive(Debug, Clone)]
pub enum OptimizationRecommendation {
    /// Low cache hit rate detected
    LowCacheHitRate {
        /// Current hit rate
        current_rate: f64,
        /// Suggested improvement
        suggested_improvement: String,
    },
    /// Enable prefetching
    EnablePrefetching {
        /// Recommended strategy
        strategy: PrefetchStrategy,
    },
    /// Use cache-aligned data structures
    UseCacheAlignment,
    /// Restructure data for better locality
    ImproveDataLocality,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_info() {
        let l1 = CacheInfo::l1();
        assert_eq!(l1.level, CacheLevel::L1);
        assert_eq!(l1.size, 32 * 1024);
        assert_eq!(l1.line_size, 64);
    }

    #[test]
    fn test_cache_index_tag() {
        let l1 = CacheInfo::l1();

        let addr = 0x1000;
        let index = l1.index(addr);
        let tag = l1.tag(addr);

        assert!(index < l1.num_sets);
        assert!(tag > 0);
    }

    #[test]
    fn test_cache_simulator() {
        let sim = CacheSimulator::new(CacheInfo::l1());

        // Access same address twice (should hit)
        sim.access(0x1000);
        let result2 = sim.access(0x1000);

        assert_eq!(result2, CacheAccessResult::Hit);

        let stats = sim.get_stats();
        assert_eq!(stats.total_accesses, 2);
        assert_eq!(stats.total_hits, 1);
    }

    #[test]
    fn test_cache_hit_rate() {
        let sim = CacheSimulator::new(CacheInfo::l1());

        // Same address multiple times
        for _ in 0..10 {
            sim.access(0x2000);
        }

        let rate = sim.hit_rate();
        assert!(rate > 0.5); // Should have many hits
    }

    #[test]
    fn test_aligned_buffer() {
        let buf: AlignedBuffer<u64, 16> = AlignedBuffer::new(42);

        assert_eq!(buf.data[0], 42);
        assert_eq!(buf.data.len(), 16);
    }

    #[test]
    fn test_cache_padded() {
        let padded: CachePadded<u64> = CachePadded::new(123);

        assert_eq!(*padded.get(), 123);
        assert_eq!(padded.into_inner(), 123);
    }

    #[test]
    fn test_prefetcher_sequential() {
        let prefetcher = Prefetcher::new(PrefetchStrategy::SequentialForward);

        let addr = prefetcher.get_prefetch_addr(0x1000);
        assert_eq!(addr, Some(0x1000 + 64));
    }

    #[test]
    fn test_prefetcher_strided() {
        let prefetcher = Prefetcher::new(PrefetchStrategy::Strided { stride: 256 });

        let addr = prefetcher.get_prefetch_addr(0x1000);
        assert_eq!(addr, Some(0x1000 + 256));
    }

    #[test]
    fn test_prefetcher_adaptive() {
        let mut prefetcher = Prefetcher::new(PrefetchStrategy::Adaptive);

        // Feed sequential pattern
        for addr in (0..10).map(|i| i * 128) {
            prefetcher.record_access(addr);
        }

        let stride = prefetcher.detected_stride();
        assert_eq!(stride, Some(128));
    }

    #[test]
    fn test_cache_hash_table() {
        let mut table = CacheHashTable::new(16);

        assert!(table.insert(1, "one").is_none());
        assert!(table.insert(1, "ONE").is_some());

        assert_eq!(table.get(&1), Some(&"ONE"));
        assert_eq!(table.get(&2), None);

        assert_eq!(table.remove(&1), Some("ONE"));
        assert_eq!(table.remove(&1), None);
    }

    #[test]
    fn test_structure_of_arrays() {
        let mut soa: StructureOfArrays<u64, 8> = StructureOfArrays::new(4);

        soa.set(0, 0, 10);
        soa.set(1, 0, 20);

        assert_eq!(soa.get(0, 0), Some(&10));
        assert_eq!(soa.get(1, 0), Some(&20));
    }

    #[test]
    fn test_access_pattern_analyzer() {
        let analyzer = AccessPatternAnalyzer::new();

        // Feed sequential pattern
        for addr in (0..20).map(|i| i * 64) {
            analyzer.record_access(addr, 8);
        }

        let pattern = analyzer.get_pattern();
        assert!(matches!(pattern, Some(AccessPattern::Sequential { .. })));
    }

    #[test]
    fn test_cache_optimization_advisor() {
        let advisor = CacheOptimizationAdvisor::new();

        for addr in (0..100).map(|i| i * 128) {
            advisor.record_access(addr, 8);
        }

        let recommendations = advisor.get_recommendations();
        assert!(!recommendations.is_empty());

        let l1_stats = advisor.get_l1_stats();
        assert!(l1_stats.is_some());
    }
}
