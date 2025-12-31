//! Offline Computation Support
//!
//! Provides queue-and-forward mechanism, local decision making,
//! and graceful degradation for intermittent connectivity.

use alloc::{
    collections::{BTreeMap, VecDeque},
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use crate::prelude::*;
use crate::edge::sync::{OfflineQueue, QueuedOp, SyncOpType, SyncPriority};
use crate::subsystems::sync::Mutex;

/// Operation identifier
pub type OpId = u64;

/// Disconnection detection threshold in seconds
const DISCONNECTION_THRESHOLD_SECS: u64 = 30;

/// Offline queue item
#[derive(Debug, Clone)]
pub struct OfflineQueueItem {
    /// Operation ID
    pub op_id: OpId,
    /// Operation type
    pub op_type: OfflineOpType,
    /// Operation data
    pub data: Vec<u8>,
    /// Timestamp
    pub timestamp: u64,
    /// Priority
    pub priority: OpPriority,
    /// Retry count
    pub retry_count: usize,
    /// Max retries
    pub max_retries: usize,
    /// Dependencies (operations that must complete first)
    pub dependencies: Vec<OpId>,
    /// Estimated processing time in milliseconds
    pub estimated_time_ms: u64,
}

/// Offline operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineOpType {
    /// Compute operation
    Compute,
    /// Data sync operation
    DataSync,
    /// Function invocation
    FunctionInvoke,
    /// State update
    StateUpdate,
    /// Log write
    LogWrite,
    /// Metric report
    MetricReport,
}

/// Operation priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpPriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

/// Disconnection detector
pub struct DisconnectionDetector {
    /// Connected flag
    connected: AtomicBool,
    /// Last successful connection timestamp
    last_connection: AtomicU64,
    /// Detection interval
    detection_interval: Duration,
    /// Heartbeat timeout
    heartbeat_timeout: Duration,
    /// Connection history
    connection_history: Mutex<VecDeque<ConnectionEvent>>,
    /// Max history size
    max_history_size: usize,
}

/// Connection event
#[derive(Debug, Clone)]
pub struct ConnectionEvent {
    /// Event type
    pub event_type: ConnectionEventType,
    /// Timestamp
    pub timestamp: u64,
    /// Connection quality
    pub quality: ConnectionQuality,
}

/// Connection event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionEventType {
    /// Connection established
    Connected,
    /// Connection lost
    Disconnected,
    /// Connection degraded
    Degraded,
    /// Connection recovered
    Recovered,
}

/// Connection quality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionQuality {
    /// Excellent (low latency, high bandwidth)
    Excellent,
    /// Good (acceptable latency and bandwidth)
    Good,
    /// Fair (degraded but usable)
    Fair,
    /// Poor (barely functional)
    Poor,
    /// Unusable (effectively disconnected)
    Unusable,
}

impl DisconnectionDetector {
    /// Create a new disconnection detector
    pub fn new(detection_interval: Duration, heartbeat_timeout: Duration) -> Self {
        Self {
            connected: AtomicBool::new(true),
            last_connection: AtomicU64::new(nos_api::event::get_time_ns()),
            detection_interval,
            heartbeat_timeout,
            connection_history: Mutex::new(VecDeque::new()),
            max_history_size: 100,
        }
    }

    /// Check connection status
    pub fn check_connection(&self) -> ConnectionStatus {
        let last_conn = self.last_connection.load(Ordering::Relaxed);
        let current_time = nos_api::event::get_time_ns();
        let elapsed_ns = current_time.saturating_sub(last_conn);
        let elapsed_secs = elapsed_ns / 1_000_000_000;

        if elapsed_secs > DISCONNECTION_THRESHOLD_SECS {
            ConnectionStatus::Disconnected {
                duration_secs: elapsed_secs,
            }
        } else {
            ConnectionStatus::Connected {
                latency_ms: 0, // Would be measured in real implementation
                quality: ConnectionQuality::Good,
            }
        }
    }

    /// Update connection status
    pub fn update_connection(&self, connected: bool) {
        let was_connected = self.connected.swap(connected, Ordering::SeqCst);

        if connected && !was_connected {
            // Transition from disconnected to connected
            self.record_event(ConnectionEventType::Connected, ConnectionQuality::Good);
        } else if !connected && was_connected {
            // Transition from connected to disconnected
            self.record_event(ConnectionEventType::Disconnected, ConnectionQuality::Unusable);
        }

        if connected {
            self.last_connection.store(nos_api::event::get_time_ns(), Ordering::Relaxed);
        }
    }

    /// Record connection event
    fn record_event(&self, event_type: ConnectionEventType, quality: ConnectionQuality) {
        let mut history = self.connection_history.lock();
        let event = ConnectionEvent {
            event_type,
            timestamp: nos_api::event::get_time_ns(),
            quality,
        };

        history.push_back(event);

        // Trim history if needed
        while history.len() > self.max_history_size {
            history.pop_front();
        }
    }

    /// Get connection history
    pub fn get_history(&self) -> Vec<ConnectionEvent> {
        let history = self.connection_history.lock();
        history.iter().cloned().collect()
    }

    /// Predict next disconnection (simple heuristic)
    pub fn predict_disconnection(&self) -> Option<Duration> {
        let history = self.connection_history.lock();

        if history.len() < 3 {
            return None;
        }

        // Simple heuristic: if we've had recent disconnections,
        // predict another one soon
        let recent_events: Vec<_> = history.iter()
            .rev()
            .take(10)
            .filter(|e| e.event_type == ConnectionEventType::Disconnected)
            .collect();

        if recent_events.len() >= 3 {
            Some(Duration::from_secs(60)) // Predict disconnection within 1 minute
        } else {
            None
        }
    }
}

/// Connection status
#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    /// Connected
    Connected {
        /// Latency in milliseconds
        latency_ms: u64,
        /// Connection quality
        quality: ConnectionQuality,
    },
    /// Disconnected
    Disconnected {
        /// Duration in seconds
        duration_secs: u64,
    },
}

/// Predictive cache
pub struct PredictiveCache {
    /// Cache ID
    cache_id: u64,
    /// Cached items
    cached_items: Mutex<BTreeMap<String, CachedItem>>,
    /// Cache size limit in bytes
    size_limit_bytes: u64,
    /// Current size in bytes
    current_size_bytes: AtomicU64,
    /// Prediction model
    prediction_model: PredictionModel,
}

/// Cached item
#[derive(Debug, Clone)]
pub struct CachedItem {
    /// Item key
    pub key: String,
    /// Item data
    pub data: Vec<u8>,
    /// Item size in bytes
    pub size_bytes: u64,
    /// Access timestamp
    pub last_access: u64,
    /// Access count
    pub access_count: AtomicU64,
    /// Priority
    pub priority: CachePriority,
    /// Expiration timestamp (0 = no expiration)
    pub expires_at: u64,
}

/// Cache priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CachePriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

/// Prediction model
#[derive(Debug, Clone)]
pub enum PredictionModel {
    /// No prediction (cache everything)
    None,
    /// Simple frequency-based prediction
    FrequencyBased,
    /// LRU (least recently used)
    LRU,
    /// LFU (least frequently used)
    LFU,
    /// Custom model
    Custom,
}

impl PredictiveCache {
    /// Create a new predictive cache
    pub fn new(cache_id: u64, size_limit_bytes: u64, model: PredictionModel) -> Self {
        Self {
            cache_id,
            cached_items: Mutex::new(BTreeMap::new()),
            size_limit_bytes,
            current_size_bytes: AtomicU64::new(0),
            prediction_model: model,
        }
    }

    /// Get item from cache
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut items = self.cached_items.lock();

        if let Some(item) = items.get_mut(key) {
            // Check expiration
            if item.expires_at != 0 && item.expires_at < nos_api::event::get_time_ns() {
                items.remove(key);
                let size = item.size_bytes;
                self.current_size_bytes.fetch_sub(size, Ordering::Relaxed);
                return None;
            }

            // Update access info
            item.last_access = nos_api::event::get_time_ns();
            item.access_count.fetch_add(1, Ordering::Relaxed);

            Some(item.data.clone())
        } else {
            None
        }
    }

    /// Put item in cache
    pub fn put(&self, key: String, data: Vec<u8>, priority: CachePriority) -> Result<()> {
        let size_bytes = data.len() as u64;

        // Check if we need to evict items
        while self.current_size_bytes.load(Ordering::Relaxed) + size_bytes > self.size_limit_bytes {
            self.evict_item()?;
        }

        let item = CachedItem {
            key: key.clone(),
            data,
            size_bytes,
            last_access: nos_api::event::get_time_ns(),
            access_count: AtomicU64::new(1),
            priority,
            expires_at: 0, // No expiration by default
        };

        let mut items = self.cached_items.lock();
        items.insert(key, item);

        self.current_size_bytes.fetch_add(size_bytes, Ordering::Relaxed);

        Ok(())
    }

    /// Evict item based on policy
    fn evict_item(&self) -> Result<()> {
        let mut items = self.cached_items.lock();

        if items.is_empty() {
            return Err(nos_api::Error::NotFound);
        }

        // Find item to evict based on model
        let evict_key = match self.prediction_model {
            PredictionModel::LRU => {
                // Evict least recently used
                items.iter()
                    .min_by_key(|(_, item)| item.last_access)
                    .map(|(k, _)| k.clone())
            }
            PredictionModel::LFU => {
                // Evict least frequently used
                items.iter()
                    .min_by_key(|(_, item)| item.access_count.load(Ordering::Relaxed))
                    .map(|(k, _)| k.clone())
            }
            _ => {
                // Evict lowest priority, then oldest
                items.iter()
                    .min_by_key(|(_, item)| (item.priority, item.last_access))
                    .map(|(k, _)| k.clone())
            }
        };

        if let Some(key) = evict_key {
            if let Some(item) = items.remove(&key) {
                self.current_size_bytes.fetch_sub(item.size_bytes, Ordering::Relaxed);
                crate::println!("[offline-cache] Evicted item: {}", key);
            }
        }

        Ok(())
    }

    /// Predict items to pre-fetch
    pub fn predict_prefetch(&self) -> Vec<String> {
        match self.prediction_model {
            PredictionModel::FrequencyBased => {
                // Return most frequently accessed items
                let items = self.cached_items.lock();
                let mut item_vec: Vec<_> = items.iter().collect();
                item_vec.sort_by(|a, b| {
                    b.1.access_count.load(Ordering::Relaxed)
                        .cmp(&a.1.access_count.load(Ordering::Relaxed))
                });

                item_vec.iter()
                    .take(5)
                    .map(|(k, _)| k.clone())
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let items = self.cached_items.lock();

        let mut total_access_count = 0;
        for item in items.values() {
            total_access_count += item.access_count.load(Ordering::Relaxed);
        }

        CacheStats {
            cache_id: self.cache_id,
            item_count: items.len(),
            total_size_bytes: self.current_size_bytes.load(Ordering::Relaxed),
            size_limit_bytes: self.size_limit_bytes,
            total_access_count,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Cache ID
    pub cache_id: u64,
    /// Number of items
    pub item_count: usize,
    /// Total size in bytes
    pub total_size_bytes: u64,
    /// Size limit in bytes
    pub size_limit_bytes: u64,
    /// Total access count
    pub total_access_count: u64,
}

/// Reconnection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectionStrategy {
    /// Immediate reconnection
    Immediate,
    /// Exponential backoff
    ExponentialBackoff,
    /// Linear backoff
    LinearBackoff,
    /// Fixed interval
    FixedInterval,
    /// Adaptive (based on history)
    Adaptive,
}

/// Offline manager
pub struct OfflineManager {
    /// Manager ID
    manager_id: u64,
    /// Offline operation queue
    operation_queue: Mutex<VecDeque<OfflineQueueItem>>,
    /// Disconnection detector
    disconnection_detector: DisconnectionDetector,
    /// Predictive cache
    cache: PredictiveCache,
    /// Reconnection strategy
    reconnection_strategy: ReconnectionStrategy,
    /// Queue limits
    max_queue_size: usize,
    max_queue_bytes: u64,
    /// Current queue bytes
    current_queue_bytes: AtomicU64,
    /// Graceful degradation enabled
    graceful_degradation: bool,
}

impl OfflineManager {
    /// Create a new offline manager
    pub fn new(
        cache_size_bytes: u64,
        reconnection_strategy: ReconnectionStrategy,
    ) -> Self {
        Self {
            manager_id: nos_api::event::get_time_ns(),
            operation_queue: Mutex::new(VecDeque::new()),
            disconnection_detector: DisconnectionDetector::new(
                Duration::from_secs(5),
                Duration::from_secs(30),
            ),
            cache: PredictiveCache::new(
                1,
                cache_size_bytes,
                PredictionModel::LFU,
            ),
            reconnection_strategy,
            max_queue_size: 10_000,
            max_queue_bytes: 100 * 1024 * 1024, // 100MB
            current_queue_bytes: AtomicU64::new(0),
            graceful_degradation: true,
        }
    }

    /// Queue offline operation
    pub fn queue_operation(&self, item: OfflineQueueItem) -> Result<()> {
        // Check queue limits
        let queue = self.operation_queue.lock();

        if queue.len() >= self.max_queue_size {
            return Err(nos_api::Error::NoMemory);
        }

        let item_size = item.data.len() as u64;
        if self.current_queue_bytes.load(Ordering::Relaxed) + item_size > self.max_queue_bytes {
            return Err(nos_api::Error::NoMemory);
        }

        drop(queue); // Release lock before enqueue

        // Enqueue operation
        {
            let mut queue = self.operation_queue.lock();
            queue.push_back(item.clone());
        }

        self.current_queue_bytes.fetch_add(item_size, Ordering::Relaxed);

        crate::println!("[offline] Queued operation {} ({} bytes)", item.op_id, item_size);

        Ok(())
    }

    /// Process queued operations
    pub fn process_queue(&mut self) -> Result<usize> {
        let mut processed = 0;
        let mut queue = self.operation_queue.lock();

        // Process operations while connected
        while let Some(item) = queue.pop_front() {
            // Check dependencies
            if !item.dependencies.is_empty() {
                // Skip if dependencies not met (simplified)
                continue;
            }

            // Process operation (placeholder)
            crate::println!("[offline] Processing operation {}", item.op_id);

            let item_size = item.data.len() as u64;
            self.current_queue_bytes.fetch_sub(item_size, Ordering::Relaxed);

            processed += 1;
        }

        crate::println!("[offline] Processed {} operations", processed);

        Ok(processed)
    }

    /// Get connection status
    pub fn get_connection_status(&self) -> ConnectionStatus {
        self.disconnection_detector.check_connection()
    }

    /// Handle disconnection
    pub fn handle_disconnection(&self) {
        crate::println!("[offline] Handling disconnection");

        self.disconnection_detector.update_connection(false);

        // Pre-fetch predicted items
        let predicted = self.cache.predict_prefetch();
        for key in predicted {
            crate::println!("[offline] Pre-fetching predicted item: {}", key);
        }
    }

    /// Handle reconnection
    pub fn handle_reconnection(&self) {
        crate::println!("[offline] Handling reconnection");

        self.disconnection_detector.update_connection(true);
    }

    /// Get cache
    pub fn get_cache(&self) -> &PredictiveCache {
        &self.cache
    }

    /// Get offline queue statistics
    pub fn get_queue_stats(&self) -> OfflineQueueStats {
        let queue = self.operation_queue.lock();

        let total_bytes = self.current_queue_bytes.load(Ordering::Relaxed);

        let mut priority_counts = [0usize; 4];
        for item in queue.iter() {
            priority_counts[item.priority as usize] += 1;
        }

        OfflineQueueStats {
            queue_length: queue.len(),
            max_queue_size: self.max_queue_size,
            total_bytes,
            max_queue_bytes: self.max_queue_bytes,
            critical_count: priority_counts[3],
            high_count: priority_counts[2],
            normal_count: priority_counts[1],
            low_count: priority_counts[0],
        }
    }
}

/// Offline queue statistics
#[derive(Debug, Clone)]
pub struct OfflineQueueStats {
    /// Current queue length
    pub queue_length: usize,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Total bytes in queue
    pub total_bytes: u64,
    /// Maximum queue bytes
    pub max_queue_bytes: u64,
    /// Critical priority count
    pub critical_count: usize,
    /// High priority count
    pub high_count: usize,
    /// Normal priority count
    pub normal_count: usize,
    /// Low priority count
    pub low_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disconnection_detector() {
        let detector = DisconnectionDetector::new(
            Duration::from_secs(5),
            Duration::from_secs(30),
        );

        let status = detector.check_connection();
        match status {
            ConnectionStatus::Connected { .. } => {}
            _ => panic!("Should be connected initially"),
        }
    }

    #[test]
    fn test_predictive_cache() {
        let cache = PredictiveCache::new(
            1,
            1024 * 1024, // 1MB
            PredictionModel::LRU,
        );

        let data = vec![1, 2, 3, 4];
        cache.put("key1".to_string(), data.clone(), CachePriority::Normal).unwrap();

        let retrieved = cache.get("key1");
        assert_eq!(retrieved, Some(data));
    }

    #[test]
    fn test_cache_stats() {
        let cache = PredictiveCache::new(
            1,
            1024 * 1024,
            PredictionModel::None,
        );

        let stats = cache.get_stats();
        assert_eq!(stats.cache_id, 1);
        assert_eq!(stats.item_count, 0);
    }

    #[test]
    fn test_offline_manager() {
        let manager = OfflineManager::new(
            10 * 1024 * 1024, // 10MB cache
            ReconnectionStrategy::ExponentialBackoff,
        );

        let status = manager.get_connection_status();
        match status {
            ConnectionStatus::Connected { .. } => {}
            _ => panic!("Should be connected initially"),
        }
    }
}
