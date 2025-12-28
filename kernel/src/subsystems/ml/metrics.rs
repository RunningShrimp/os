//! ML Metrics Collection
//!
//! This module implements metrics collection for ML optimization:
//! - Performance metrics
//! - Resource utilization metrics
//! - Latency and throughput metrics
//! - Model evaluation metrics
//!
//! Features:
//! - Time series metrics
//! - Histogram metrics
//! - Statistical aggregations
//! - Real-time metrics updates

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// ML Metrics Constants
// ============================================================================

/// Metrics collection interval (milliseconds)
pub const METRICS_COLLECTION_INTERVAL_MS: u64 = 100;

/// Histogram bucket count
pub const HISTOGRAM_BUCKETS: usize = 64;

/// Rolling average window size
pub const ROLLING_AVERAGE_WINDOW: usize = 128;

// ============================================================================
// Metric Types
// ============================================================================

/// Metric category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricCategory {
    /// CPU metrics
    Cpu,
    
    /// Memory metrics
    Memory,
    
    /// I/O metrics
    Io,
    
    /// Network metrics
    Network,
    
    /// Scheduling metrics
    Scheduling,
    
    /// ML model metrics
    MlModel,
}

/// Metric aggregation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricAggregation {
    /// Average (arithmetic mean)
    Average,
    
    /// Minimum value
    Min,
    
    /// Maximum value
    Max,
    
    /// Percentile (P50)
    P50,
    
    /// Percentile (P95)
    P95,
    
    /// Percentile (P99)
    P99,
    
    /// Standard deviation
    StdDev,
    
    /// Rate (per second)
    Rate,
}

// ============================================================================
// Time Series Metric
// ============================================================================

/// Time series metric
#[derive(Debug, Clone)]
pub struct TimeSeriesMetric {
    /// Metric name
    pub name: String,
    
    /// Category
    pub category: MetricCategory,
    
    /// Values (time-ordered)
    pub values: Vec<f64>,
    
    /// Timestamps (time-ordered)
    pub timestamps: Vec<u64>,
    
    /// Current value
    pub current_value: AtomicU64,
    
    /// Min value
    pub min_value: AtomicU64,
    
    /// Max value
    pub max_value: AtomicU64,
    
    /// Aggregated statistics
    pub stats: Mutex<MetricStats>,
}

/// Metric statistics
#[derive(Debug, Clone, Copy)]
pub struct MetricStats {
    /// Count
    pub count: usize,
    
    /// Average
    pub average: f64,
    
    /// Minimum
    pub min: f64,
    
    /// Maximum
    pub max: f64,
    
    /// Standard deviation
    pub std_dev: f64,
    
    /// P50 percentile
    pub p50: f64,
    
    /// P95 percentile
    pub p95: f64,
    
    /// P99 percentile
    pub p99: f64,
    
    /// Rate (per second)
    pub rate: f64,
}

impl Default for MetricStats {
    fn default() -> Self {
        Self {
            count: 0,
            average: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            std_dev: 0.0,
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
            rate: 0.0,
        }
    }
}

impl TimeSeriesMetric {
    /// Create new time series metric
    pub fn new(name: String, category: MetricCategory) -> Self {
        Self {
            name,
            category,
            values: Vec::new(),
            timestamps: Vec::new(),
            current_value: AtomicU64::new(0),
            min_value: AtomicU64::new(u64::MAX),
            max_value: AtomicU64::new(0),
            stats: Mutex::new(MetricStats::default()),
        }
    }
    
    /// Record value
    pub fn record(&self, value: f64, timestamp: u64) {
        let old_count = self.values.len();
        
        // Add to history
        self.values.push(value);
        self.timestamps.push(timestamp);
        self.current_value.store(value.to_bits() as u64, Ordering::Release);
        
        // Update min/max
        let current_min = self.min_value.load(Ordering::Relaxed) as f64;
        let current_max = self.max_value.load(Ordering::Relaxed) as f64;
        
        if value < current_min {
            self.min_value.store(value.to_bits() as u64, Ordering::Release);
        }
        
        if value > current_max {
            self.max_value.store(value.to_bits() as u64, Ordering::Release);
        }
        
        // Update statistics
        self.update_stats();
        
        // Maintain history window
        if self.values.len() > ROLLING_AVERAGE_WINDOW {
            self.values.remove(0);
            self.timestamps.remove(0);
        }
    }
    
    /// Update statistics
    fn update_stats(&self) {
        let mut stats = self.stats.lock();
        
        if self.values.is_empty() {
            return;
        }
        
        let count = self.values.len();
        stats.count = count;
        
        // Calculate average
        let sum: f64 = self.values.iter().sum();
        stats.average = sum / count as f64;
        
        // Calculate min/max
        let min = *self.values.iter().min().unwrap_or(&0.0);
        let max = *self.values.iter().max().unwrap_or(&0.0);
        stats.min = min;
        stats.max = max;
        
        // Calculate standard deviation
        let variance: f64 = self.values.iter()
            .map(|&x| {
                let diff = x - stats.average;
                diff * diff
            })
            .sum::<f64>() / count as f64;
        
        stats.std_dev = variance.sqrt();
        
        // Calculate percentiles (simplified)
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        
        let p50_idx = sorted.len() * 50 / 100;
        let p95_idx = sorted.len() * 95 / 100;
        let p99_idx = sorted.len() * 99 / 100;
        
        stats.p50 = *sorted.get(p50_idx).unwrap_or(&0.0);
        stats.p95 = *sorted.get(p95_idx).unwrap_or(&0.0);
        stats.p99 = *sorted.get(p99_idx).unwrap_or(&0.0);
        
        // Calculate rate (if timestamps available)
        if self.timestamps.len() > 1 {
            let time_diff = self.timestamps[self.timestamps.len() - 1] - self.timestamps[0];
            if time_diff > 0 {
                stats.rate = (sum - self.values[0]) as f64 / time_diff as f64 * 1_000_000_000.0; // Convert to per second
            }
        }
    }
    
    /// Get aggregated value by type
    pub fn get_aggregated(&self, aggregation: MetricAggregation) -> f64 {
        let stats = self.stats.lock();
        
        match aggregation {
            MetricAggregation::Average => stats.average,
            MetricAggregation::Min => stats.min,
            MetricAggregation::Max => stats.max,
            MetricAggregation::P50 => stats.p50,
            MetricAggregation::P95 => stats.p95,
            MetricAggregation::P99 => stats.p99,
            MetricAggregation::StdDev => stats.std_dev,
            MetricAggregation::Rate => stats.rate,
        }
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> MetricStats {
        self.update_stats();
        *self.stats.lock()
    }
}

// ============================================================================
// Metrics Collector
// ============================================================================

/// Metrics collector
pub struct MetricsCollector {
    /// All time series metrics
    pub metrics: Mutex<BTreeMap<String, Arc<TimeSeriesMetric>>>>,
    
    /// Metric categories
    pub categories: Mutex<BTreeMap<MetricCategory, BTreeSet<String>>>>,
    
    /// Collection count
    pub collection_count: AtomicU64,
    
    /// Last collection time
    pub last_collection_time: AtomicU64,
    
    /// Collector statistics
    pub stats: Mutex<CollectorStats>,
}

/// Collector statistics
#[derive(Debug, Clone, Copy)]
pub struct CollectorStats {
    /// Total metrics
    pub total_metrics: usize,
    
    /// Metrics by category
    pub metrics_by_category: [usize; 8], // One per MetricCategory
    
    /// Total collections
    pub total_collections: u64,
    
    /// Average collection time (milliseconds)
    pub avg_collection_time_ms: f64,
}

impl Default for CollectorStats {
    fn default() -> Self {
        Self {
            total_metrics: 0,
            metrics_by_category: [0; 8],
            total_collections: 0,
            avg_collection_time_ms: 0.0,
        }
    }
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        let mut categories = BTreeMap::new();
        
        // Initialize all categories
        for category in [
            MetricCategory::Cpu,
            MetricCategory::Memory,
            MetricCategory::Io,
            MetricCategory::Network,
            MetricCategory::Scheduling,
            MetricCategory::MlModel,
        ] {
            categories.insert(category, BTreeSet::new());
        }
        
        Self {
            metrics: Mutex::new(BTreeMap::new()),
            categories: Mutex::new(categories),
            collection_count: AtomicU64::new(0),
            last_collection_time: AtomicU64::new(0),
            stats: Mutex::new(CollectorStats::default()),
        }
    }
    
    /// Register metric
    pub fn register_metric(&self, name: String, category: MetricCategory) 
        -> Result<(), MetricError> {
        
        let mut metrics = self.metrics.lock();
        
        if metrics.contains_key(&name) {
            return Err(MetricError::MetricAlreadyExists { name });
        }
        
        let metric = Arc::new(TimeSeriesMetric::new(name, category));
        metrics.insert(name, metric);
        
        // Add to category
        let mut categories = self.categories.lock();
        if let Some(category_metrics) = categories.get_mut(&category) {
            category_metrics.insert(name);
        }
        
        let mut stats = self.stats.lock();
        stats.total_metrics = metrics.len();
        stats.metrics_by_category[category as usize] = 
            categories.get(&category).map_or(&BTreeSet::new(), |m| m.len());
        
        crate::println!("[metrics] Registered metric {} (category: {:?})",
                        name, category);
        
        Ok(())
    }
    
    /// Record metric value
    pub fn record_metric(&self, name: String, value: f64, timestamp: u64) 
        -> Result<(), MetricError> {
        
        let metrics = self.metrics.lock();
        
        if let Some(metric) = metrics.get(&name) {
            metric.record(value, timestamp);
        } else {
            return Err(MetricError::MetricNotFound { name });
        }
        
        Ok(())
    }
    
    /// Collect all metrics (should be called periodically)
    pub fn collect_all(&self) {
        let start_time = crate::subsystems::time::timestamp_nanos();
        
        let metrics = self.metrics.lock();
        let mut count = 0usize;
        
        for metric in metrics.values() {
            // In real implementation, would trigger metric collection
            // For now, just count metrics
            count += 1;
        }
        
        let end_time = crate::subsystems::time::timestamp_nanos();
        let collection_time_ms = (end_time - start_time) / 1_000_000; // Convert ns to ms
        
        self.collection_count.fetch_add(1, Ordering::Relaxed);
        self.last_collection_time.store(end_time, Ordering::Relaxed);
        
        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_collections = self.collection_count.load(Ordering::Relaxed);
        
        // Update average collection time
        let total_time = stats.avg_collection_time_ms * (stats.total_collections as f64);
        stats.avg_collection_time_ms = (total_time + collection_time_ms) / (stats.total_collections as f64 + 1.0);
        
        crate::println!("[metrics] Collected {} metrics ({}ms)",
                        count, collection_time_ms);
    }
    
    /// Get metric
    pub fn get_metric(&self, name: String) -> Option<Arc<TimeSeriesMetric>> {
        let metrics = self.metrics.lock();
        metrics.get(&name).cloned()
    }
    
    /// Get metrics by category
    pub fn get_metrics_by_category(&self, category: MetricCategory) 
        -> Vec<Arc<TimeSeriesMetric>> {
        
        let metrics = self.metrics.lock();
        let categories = self.categories.lock();
        
        if let Some(category_metrics) = categories.get(&category) {
            category_metrics.iter()
                .filter_map(|name| metrics.get(name).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get collector statistics
    pub fn get_stats(&self) -> CollectorStats {
        *self.stats.lock()
    }
}

/// Metric error
#[derive(Debug, Clone)]
pub enum MetricError {
    /// Metric already exists
    MetricAlreadyExists {
        name: String,
    },
    
    /// Metric not found
    MetricNotFound {
        name: String,
    },
    
    /// Collection failed
    CollectionFailed {
        reason: String,
    },
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series_metric() {
        let metric = TimeSeriesMetric::new(
            String::from("test_metric"),
            MetricCategory::Cpu
        );
        
        assert_eq!(metric.name, "test_metric");
        assert_eq!(metric.category, MetricCategory::Cpu);
        
        // Record some values
        metric.record(100.0, 1_000);
        metric.record(110.0, 2_000);
        metric.record(90.0, 3_000);
        metric.record(120.0, 4_000);
        
        let stats = metric.get_stats();
        assert_eq!(stats.count, 4);
        assert_eq!(stats.average, 105.0); // (100+110+90+120)/4
        assert_eq!(stats.min, 90.0);
        assert_eq!(stats.max, 120.0);
    }

    #[test]
    fn test_metric_aggregation() {
        let metric = TimeSeriesMetric::new(
            String::from("test_metric"),
            MetricCategory::Cpu
        );
        
        // Record values (100, 110, 90, 120)
        metric.record(100.0, 1_000);
        metric.record(110.0, 2_000);
        metric.record(90.0, 3_000);
        metric.record(120.0, 4_000);
        
        assert_eq!(metric.get_aggregated(MetricAggregation::Average), 105.0);
        assert_eq!(metric.get_aggregated(MetricAggregation::Min), 90.0);
        assert_eq!(metric.get_aggregated(MetricAggregation::Max), 120.0);
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        
        collector.register_metric(
            String::from("cpu_usage"),
            MetricCategory::Cpu
        ).unwrap();
        
        collector.register_metric(
            String::from("memory_usage"),
            MetricCategory::Memory
        ).unwrap();
        
        collector.record_metric(
            String::from("cpu_usage"),
            50.0,
            1_000
        ).unwrap();
        
        let stats = collector.get_stats();
        assert_eq!(stats.total_metrics, 2);
    }
}
