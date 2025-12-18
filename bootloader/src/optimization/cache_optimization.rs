//! Cache Optimization - Boot Time Cache Management
//!
//! Optimizes cache usage during boot:
//! - Cache statistics tracking
//! - Prefetch strategies
//! - Cache warming
//! - Performance metrics

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLevel {
    L1I,
    L1D,
    L2,
    L3,
}

impl fmt::Display for CacheLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheLevel::L1I => write!(f, "L1-I"),
            CacheLevel::L1D => write!(f, "L1-D"),
            CacheLevel::L2 => write!(f, "L2"),
            CacheLevel::L3 => write!(f, "L3"),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub level: CacheLevel,
    pub size: u32,
    pub line_size: u32,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheStats {
    /// Create new cache stats
    pub fn new(level: CacheLevel, size: u32) -> Self {
        CacheStats {
            level,
            size,
            line_size: 64, // Default cache line size
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Record hit
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    /// Record miss
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// Record eviction
    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }

    /// Get hit rate percentage
    pub fn get_hit_rate(&self) -> u32 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0;
        }
        ((self.hits as u64 * 100) / total) as u32
    }

    /// Get miss rate percentage
    pub fn get_miss_rate(&self) -> u32 {
        100u32.saturating_sub(self.get_hit_rate())
    }

    /// Get total accesses
    pub fn get_total_accesses(&self) -> u64 {
        self.hits + self.misses
    }
}

impl fmt::Display for CacheStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}MB (Hit: {}%, Miss: {}%, Evict: {})",
            self.level,
            self.size / 1048576,
            self.get_hit_rate(),
            self.get_miss_rate(),
            self.evictions
        )
    }
}

/// Prefetch strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefetchStrategy {
    Sequential,
    Spatial,
    Temporal,
    Adaptive,
}

impl fmt::Display for PrefetchStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrefetchStrategy::Sequential => write!(f, "Sequential"),
            PrefetchStrategy::Spatial => write!(f, "Spatial"),
            PrefetchStrategy::Temporal => write!(f, "Temporal"),
            PrefetchStrategy::Adaptive => write!(f, "Adaptive"),
        }
    }
}

/// Cache warm-up entry
#[derive(Debug, Clone)]
pub struct CacheWarmupEntry {
    pub address: u64,
    pub size: u32,
    pub priority: u8,
    pub is_warmed: bool,
}

impl CacheWarmupEntry {
    /// Create new entry
    pub fn new(address: u64, size: u32) -> Self {
        CacheWarmupEntry {
            address,
            size,
            priority: 0,
            is_warmed: false,
        }
    }

    /// Set priority
    pub fn set_priority(&mut self, priority: u8) {
        self.priority = priority;
    }

    /// Mark as warmed
    pub fn mark_warmed(&mut self) {
        self.is_warmed = true;
    }
}

impl fmt::Display for CacheWarmupEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Entry {{ addr: 0x{:x}, size: {}KB, warmed: {} }}",
            self.address, self.size / 1024, self.is_warmed
        )
    }
}

/// Cache Optimizer
pub struct CacheOptimizer {
    cache_levels: Vec<CacheStats>,
    prefetch_strategy: PrefetchStrategy,
    warmup_entries: Vec<CacheWarmupEntry>,
    warmup_enabled: bool,
    total_boot_time: u64,
}

impl CacheOptimizer {
    /// Create new cache optimizer
    pub fn new() -> Self {
        let mut levels = Vec::new();
        levels.push(CacheStats::new(CacheLevel::L1I, 32 * 1024));
        levels.push(CacheStats::new(CacheLevel::L1D, 32 * 1024));
        levels.push(CacheStats::new(CacheLevel::L2, 256 * 1024));
        levels.push(CacheStats::new(CacheLevel::L3, 8 * 1024 * 1024));

        CacheOptimizer {
            cache_levels: levels,
            prefetch_strategy: PrefetchStrategy::Adaptive,
            warmup_entries: Vec::new(),
            warmup_enabled: false,
            total_boot_time: 0,
        }
    }

    /// Record cache hit
    pub fn record_hit(&mut self, level: CacheLevel) {
        for cache in &mut self.cache_levels {
            if cache.level == level {
                cache.record_hit();
            }
        }
    }

    /// Record cache miss
    pub fn record_miss(&mut self, level: CacheLevel) {
        for cache in &mut self.cache_levels {
            if cache.level == level {
                cache.record_miss();
            }
        }
    }

    /// Set prefetch strategy
    pub fn set_prefetch_strategy(&mut self, strategy: PrefetchStrategy) {
        self.prefetch_strategy = strategy;
    }

    /// Add warmup entry
    pub fn add_warmup_entry(&mut self, entry: CacheWarmupEntry) -> bool {
        self.warmup_entries.push(entry);
        true
    }

    /// Enable warmup
    pub fn enable_warmup(&mut self) -> bool {
        self.warmup_enabled = true;
        true
    }

    /// Execute warmup
    pub fn execute_warmup(&mut self) -> u32 {
        let mut warmed = 0;
        for entry in &mut self.warmup_entries {
            entry.mark_warmed();
            warmed += 1;
        }
        warmed
    }

    /// 设置总启动时间
    pub fn set_total_boot_time(&mut self, time_ms: u64) {
        self.total_boot_time = time_ms;
        log::trace!("Set total boot time: {} ms", time_ms);
    }

    /// 获取总启动时间
    pub fn get_total_boot_time(&self) -> u64 {
        self.total_boot_time
    }

    /// Get overall hit rate
    pub fn get_overall_hit_rate(&self) -> u32 {
        let total_hits: u64 = self.cache_levels.iter().map(|c| c.hits).sum();
        let total_misses: u64 = self.cache_levels.iter().map(|c| c.misses).sum();
        let total = total_hits + total_misses;

        if total == 0 {
            return 0;
        }
        ((total_hits as u64 * 100) / total) as u32
    }

    /// Get average memory latency estimate
    pub fn estimate_avg_latency(&self) -> u32 {
        let l1_hit_cycles = 4;
        let l2_hit_cycles = 12;
        let l3_hit_cycles = 42;
        let _memory_cycles = 200;
        log::trace!("Estimating average memory latency");

        let total_accesses: u64 = self.cache_levels.iter()
            .map(|c| c.hits + c.misses)
            .sum::<u64>()
            .max(1);

        let mut total_cycles = 0u64;
        for cache in &self.cache_levels {
            match cache.level {
                CacheLevel::L1I | CacheLevel::L1D => {
                    total_cycles += cache.hits * (l1_hit_cycles as u64);
                }
                CacheLevel::L2 => {
                    total_cycles += cache.hits * (l2_hit_cycles as u64);
                }
                CacheLevel::L3 => {
                    total_cycles += cache.hits * (l3_hit_cycles as u64);
                }
            }
        }

        ((total_cycles / total_accesses) as u32).max(1)
    }

    /// Get cache optimization report
    pub fn cache_report(&self) -> String {
        let mut report = String::from("=== Cache Optimization Report ===\n");

        report.push_str(&format!("Prefetch Strategy: {}\n", self.prefetch_strategy));
        report.push_str(&format!("Warmup Enabled: {}\n", self.warmup_enabled));
        report.push_str(&format!("Total Boot Time: {} ms\n", self.get_total_boot_time()));
        report.push_str(&format!("Overall Hit Rate: {}%\n\n", self.get_overall_hit_rate()));

        report.push_str("--- Cache Levels ---\n");
        for cache in &self.cache_levels {
            report.push_str(&format!("{}\n", cache));
        }

        report.push_str(&format!("\nEstimated Avg Latency: {} cycles\n", 
            self.estimate_avg_latency()));
        report.push_str(&format!("Warmup Entries: {}\n", self.warmup_entries.len()));

        report
    }
}

impl fmt::Display for CacheOptimizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CacheOptimizer {{ strategy: {}, hit_rate: {}%, warmup: {} }}",
            self.prefetch_strategy,
            self.get_overall_hit_rate(),
            self.warmup_enabled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats() {
        let stats = CacheStats::new(CacheLevel::L1D, 32 * 1024);
        assert_eq!(stats.get_hit_rate(), 0);
    }

    #[test]
    fn test_cache_stats_hit() {
        let mut stats = CacheStats::new(CacheLevel::L1D, 32 * 1024);
        stats.record_hit();
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn test_cache_stats_miss() {
        let mut stats = CacheStats::new(CacheLevel::L1D, 32 * 1024);
        stats.record_miss();
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::new(CacheLevel::L1D, 32 * 1024);
        for _ in 0..75 {
            stats.record_hit();
        }
        for _ in 0..25 {
            stats.record_miss();
        }
        assert_eq!(stats.get_hit_rate(), 75);
    }

    #[test]
    fn test_cache_warmup_entry() {
        let entry = CacheWarmupEntry::new(0x1000, 4096);
        assert!(!entry.is_warmed);
    }

    #[test]
    fn test_cache_warmup_entry_mark() {
        let mut entry = CacheWarmupEntry::new(0x1000, 4096);
        entry.mark_warmed();
        assert!(entry.is_warmed);
    }

    #[test]
    fn test_cache_optimizer_creation() {
        let opt = CacheOptimizer::new();
        assert_eq!(opt.cache_levels.len(), 4);
    }

    #[test]
    fn test_cache_optimizer_record_hit() {
        let mut opt = CacheOptimizer::new();
        opt.record_hit(CacheLevel::L1D);
        assert!(opt.cache_levels[1].hits > 0);
    }

    #[test]
    fn test_cache_optimizer_prefetch() {
        let mut opt = CacheOptimizer::new();
        opt.set_prefetch_strategy(PrefetchStrategy::Spatial);
        assert_eq!(opt.prefetch_strategy, PrefetchStrategy::Spatial);
    }

    #[test]
    fn test_cache_optimizer_warmup() {
        let mut opt = CacheOptimizer::new();
        let entry = CacheWarmupEntry::new(0x1000, 4096);
        opt.add_warmup_entry(entry);
        opt.enable_warmup();
        assert_eq!(opt.execute_warmup(), 1);
    }

    #[test]
    fn test_cache_optimizer_overall_hit_rate() {
        let mut opt = CacheOptimizer::new();
        for _ in 0..80 {
            opt.record_hit(CacheLevel::L1D);
        }
        for _ in 0..20 {
            opt.record_miss(CacheLevel::L1D);
        }
        assert_eq!(opt.get_overall_hit_rate(), 80);
    }

    #[test]
    fn test_cache_optimizer_latency() {
        let opt = CacheOptimizer::new();
        let latency = opt.estimate_avg_latency();
        assert!(latency >= 1);
    }

    #[test]
    fn test_cache_optimizer_report() {
        let opt = CacheOptimizer::new();
        let report = opt.cache_report();
        assert!(report.contains("Cache Optimization Report"));
    }
}
