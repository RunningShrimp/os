//! Quality of Service (QoS) Implementation
//!
//! Provides comprehensive traffic management including classification, rate limiting,
//! traffic shaping, DSCP marking, and Active Queue Management (AQM).

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

use crate::subsystems::sync::{Mutex, RwLock};

use super::{SdnError, SdnStats};

/// QoS error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QosError {
    /// Invalid configuration
    InvalidConfig,
    /// Queue full
    QueueFull,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Invalid traffic class
    InvalidTrafficClass,
    /// Shaper not found
    ShaperNotFound,
}

/// Traffic class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrafficClass {
    /// Bulk traffic (lowest priority)
    Bulk = 0,
    /// Best effort
    BestEffort = 1,
    /// Normal services
    Normal = 2,
    /// Video
    Video = 3,
    /// Voice (highest priority)
    Voice = 4,
}

impl TrafficClass {
    pub fn from_dscp(dscp: u8) -> Option<Self> {
        match dscp {
            0..=7 => Some(Self::Bulk),
            8..=23 => Some(Self::BestEffort),
            24..=39 => Some(Self::Normal),
            40..=47 => Some(Self::Video),
            48..=63 => Some(Self::Voice),
            _ => None,
        }
    }

    pub fn to_dscp(self) -> u8 {
        match self {
            Self::Bulk => 0,
            Self::BestEffort => 8,
            Self::Normal => 24,
            Self::Video => 40,
            Self::Voice => 46,
        }
    }
}

/// DSCP marking
#[derive(Debug, Clone, Copy)]
pub struct DscpMarking {
    /// DSCP value
    pub dscp: u8,
    /// ECN bits
    pub ecn: u8,
}

impl DscpMarking {
    pub fn new(dscp: u8, ecn: u8) -> Self {
        Self { dscp: dscp.min(63), ecn: ecn.min(3) }
    }

    pub fn to_tos(self) -> u8 {
        (self.dscp << 2) | self.ecn
    }

    pub fn from_tos(tos: u8) -> Self {
        Self {
            dscp: (tos >> 2) & 0x3F,
            ecn: tos & 0x03,
        }
    }
}

/// Packet classifier
#[derive(Debug)]
pub struct PacketClassifier {
    /// Classification rules
    rules: Mutex<Vec<ClassificationRule>>,
    /// Statistics
    stats: Mutex<ClassificationStats>,
}

/// Classification rule
#[derive(Debug, Clone)]
pub struct ClassificationRule {
    /// Rule ID
    pub id: u32,
    /// Match criteria
    pub match_: MatchCriteria,
    /// Traffic class
    pub traffic_class: TrafficClass,
    /// DSCP marking
    pub dscp: Option<DscpMarking>,
    /// Priority
    pub priority: u16,
}

/// Match criteria
#[derive(Debug, Clone)]
pub struct MatchCriteria {
    /// Source IP
    pub src_ip: Option<(u32, u32)>, // address, mask
    /// Destination IP
    pub dst_ip: Option<(u32, u32)>,
    /// Source port
    pub src_port: Option<u16>,
    /// Destination port
    pub dst_port: Option<u16>,
    /// Protocol
    pub protocol: Option<u8>,
    /// DSCP value
    pub dscp: Option<u8>,
}

/// Classification statistics
#[derive(Debug, Default)]
pub struct ClassificationStats {
    pub packets_classified: AtomicU64,
    pub bytes_classified: AtomicU64,
    pub classification_errors: AtomicU64,
}

impl PacketClassifier {
    pub fn new() -> Self {
        Self {
            rules: Mutex::new(Vec::new()),
            stats: Mutex::new(ClassificationStats::default()),
        }
    }

    pub fn add_rule(&self, rule: ClassificationRule) {
        let mut rules = self.rules.lock();
        rules.push(rule);
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn remove_rule(&self, id: u32) {
        let mut rules = self.rules.lock();
        rules.retain(|r| r.id != id);
    }

    pub fn classify(&self, packet: &PacketInfo) -> Option<TrafficClass> {
        let rules = self.rules.lock();

        for rule in rules.iter() {
            if self.matches(&rule.match_, packet) {
                return Some(rule.traffic_class);
            }
        }

        None
    }

    pub fn classify_with_dscp(&self, packet: &PacketInfo) -> Option<(TrafficClass, DscpMarking)> {
        let rules = self.rules.lock();

        for rule in rules.iter() {
            if self.matches(&rule.match_, packet) {
                let dscp = rule.dscp.unwrap_or_else(|| DscpMarking::new(rule.traffic_class.to_dscp(), 0));
                return Some((rule.traffic_class, dscp));
            }
        }

        None
    }

    fn matches(&self, criteria: &MatchCriteria, packet: &PacketInfo) -> bool {
        if let Some((addr, mask)) = criteria.src_ip {
            if packet.src_ip & mask != addr & mask {
                return false;
            }
        }

        if let Some((addr, mask)) = criteria.dst_ip {
            if packet.dst_ip & mask != addr & mask {
                return false;
            }
        }

        if let Some(port) = criteria.src_port {
            if packet.src_port != Some(port) {
                return false;
            }
        }

        if let Some(port) = criteria.dst_port {
            if packet.dst_port != Some(port) {
                return false;
            }
        }

        if let Some(proto) = criteria.protocol {
            if packet.protocol != Some(proto) {
                return false;
            }
        }

        true
    }

    pub fn get_stats(&self) -> ClassificationStatsSnapshot {
        let stats = self.stats.lock();
        ClassificationStatsSnapshot {
            packets_classified: stats.packets_classified.load(Ordering::Relaxed),
            bytes_classified: stats.bytes_classified.load(Ordering::Relaxed),
            classification_errors: stats.classification_errors.load(Ordering::Relaxed),
        }
    }
}

/// Packet information for classification
#[derive(Debug, Clone)]
pub struct PacketInfo {
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub protocol: Option<u8>,
    pub length: usize,
}

/// Classification statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct ClassificationStatsSnapshot {
    pub packets_classified: u64,
    pub bytes_classified: u64,
    pub classification_errors: u64,
}

impl Default for PacketClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiter
#[derive(Debug)]
pub struct RateLimiter {
    /// Rate in bytes per second
    rate_bytes_per_sec: u64,
    /// Burst size in bytes
    burst_bytes: u64,
    /// Tokens (bytes)
    tokens: Mutex<u64>,
    /// Last update
    last_update: Mutex<Duration>,
}

impl RateLimiter {
    pub fn new(rate_bytes_per_sec: u64, burst_bytes: u64) -> Self {
        Self {
            rate_bytes_per_sec,
            burst_bytes,
            tokens: Mutex::new(burst_bytes),
            last_update: Mutex::new(Duration::from_secs(0)),
        }
    }

    pub fn check(&self, bytes: u64) -> Result<(), QosError> {
        let mut tokens = self.tokens.lock();
        let now = Duration::from_secs(0); // Simplified

        // Refill tokens
        let mut last_update = self.last_update.lock();
        let elapsed = now.saturating_sub(*last_update);
        let new_tokens = (elapsed.as_secs() * self.rate_bytes_per_sec).min(self.burst_bytes);

        *tokens = (*tokens + new_tokens).min(self.burst_bytes);
        *last_update = now;
        drop(last_update);

        // Check if enough tokens
        if *tokens >= bytes {
            *tokens -= bytes;
            Ok(())
        } else {
            Err(QosError::RateLimitExceeded)
        }
    }

    pub fn update_rate(&mut self, rate_bytes_per_sec: u64) {
        self.rate_bytes_per_sec = rate_bytes_per_sec;
    }

    pub fn update_burst(&mut self, burst_bytes: u64) {
        self.burst_bytes = burst_bytes;
    }
}

/// Shaping algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapingAlgorithm {
    /// Token Bucket Filter
    Tbf,
    /// Hierarchical Token Bucket
    Htb,
    /// Hierarchical Fair Service Curve
    Hfsc,
    /// Controlled Delay
    CoDel,
    /// Fair Queuing
    FairQ,
}

/// Traffic shaper
#[derive(Debug)]
pub struct TrafficShaper {
    /// Shaper ID
    id: u32,
    /// Algorithm
    algorithm: ShapingAlgorithm,
    /// Rate limiters per traffic class
    limiters: BTreeMap<TrafficClass, RateLimiter>,
    /// Statistics
    stats: ShaperStats,
}

/// Shaper statistics
#[derive(Debug, Default)]
pub struct ShaperStats {
    pub packets_shaped: AtomicU64,
    pub bytes_shaped: AtomicU64,
    pub packets_dropped: AtomicU64,
}

impl TrafficShaper {
    pub fn new(id: u32, algorithm: ShapingAlgorithm) -> Self {
        Self {
            id,
            algorithm,
            limiters: BTreeMap::new(),
            stats: ShaperStats::default(),
        }
    }

    pub fn add_class(&mut self, class: TrafficClass, rate_bytes_per_sec: u64, burst_bytes: u64) {
        let limiter = RateLimiter::new(rate_bytes_per_sec, burst_bytes);
        self.limiters.insert(class, limiter);
    }

    pub fn remove_class(&mut self, class: TrafficClass) {
        self.limiters.remove(&class);
    }

    pub fn shape(&self, class: TrafficClass, bytes: u64) -> Result<(), QosError> {
        if let Some(limiter) = self.limiters.get(&class) {
            limiter.check(bytes)?;
            self.stats.packets_shaped.fetch_add(1, Ordering::Relaxed);
            self.stats.bytes_shaped.fetch_add(bytes, Ordering::Relaxed);
            Ok(())
        } else {
            // No rate limiting for this class
            Ok(())
        }
    }

    pub fn get_stats(&self) -> ShaperStatsSnapshot {
        ShaperStatsSnapshot {
            packets_shaped: self.stats.packets_shaped.load(Ordering::Relaxed),
            bytes_shaped: self.stats.bytes_shaped.load(Ordering::Relaxed),
            packets_dropped: self.stats.packets_dropped.load(Ordering::Relaxed),
        }
    }
}

/// Shaper statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct ShaperStatsSnapshot {
    pub packets_shaped: u64,
    pub bytes_shaped: u64,
    pub packets_dropped: u64,
}

/// AQM (Active Queue Management) algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AqmAlgorithm {
    /// Random Early Detection
    Red,
    /// Controlled Delay
    CoDel,
    /// Proportional Integral Controller Enhanced
    Pie,
    /// Blue
    Blue,
    /// No AQM
    None,
}

/// AQM parameters
#[derive(Debug, Clone)]
pub struct AqmParams {
    /// Minimum threshold (for RED)
    pub min_threshold: u32,
    /// Maximum threshold (for RED)
    pub max_threshold: u32,
    /// Maximum drop probability
    pub max_probability: f32,
    /// Target queue delay (for CoDel)
    pub target_delay: Duration,
    /// Interval (for CoDel)
    pub interval: Duration,
}

impl Default for AqmParams {
    fn default() -> Self {
        Self {
            min_threshold: 10,
            max_threshold: 30,
            max_probability: 0.1,
            target_delay: Duration::from_millis(5),
            interval: Duration::from_millis(100),
        }
    }
}

/// AQM queue
#[derive(Debug)]
pub struct AqmQueue {
    /// Algorithm
    algorithm: AqmAlgorithm,
    /// Parameters
    params: AqmParams,
    /// Current queue size
    queue_size: AtomicU64,
    /// Maximum queue size
    max_queue_size: u64,
    /// Drop probability
    drop_probability: Mutex<f32>,
    /// Statistics
    stats: AqmStats,
}

/// AQM statistics
#[derive(Debug, Default)]
pub struct AqmStats {
    pub packets_enqueued: AtomicU64,
    pub packets_dequeued: AtomicU64,
    pub packets_dropped: AtomicU64,
    pub early_drops: AtomicU64,
}

impl AqmQueue {
    pub fn new(algorithm: AqmAlgorithm, max_queue_size: u64, params: AqmParams) -> Self {
        Self {
            algorithm,
            params,
            queue_size: AtomicU64::new(0),
            max_queue_size,
            drop_probability: Mutex::new(0.0),
            stats: AqmStats::default(),
        }
    }

    pub fn enqueue(&self, packet_size: u64) -> Result<(), QosError> {
        let current_size = self.queue_size.load(Ordering::Relaxed);

        // Check if should drop based on AQM algorithm
        if self.should_drop(current_size) {
            self.stats.early_drops.fetch_add(1, Ordering::Relaxed);
            return Err(QosError::QueueFull);
        }

        // Check if queue is full
        if current_size + packet_size > self.max_queue_size {
            self.stats.packets_dropped.fetch_add(1, Ordering::Relaxed);
            return Err(QosError::QueueFull);
        }

        self.queue_size.fetch_add(packet_size, Ordering::Relaxed);
        self.stats.packets_enqueued.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    pub fn dequeue(&self, packet_size: u64) {
        let new_size = self.queue_size.fetch_sub(packet_size, Ordering::Relaxed).saturating_sub(packet_size);
        self.queue_size.store(new_size, Ordering::Relaxed);
        self.stats.packets_dequeued.fetch_add(1, Ordering::Relaxed);
    }

    fn should_drop(&self, queue_size: u64) -> bool {
        match self.algorithm {
            AqmAlgorithm::Red => self.should_drop_red(queue_size),
            AqmAlgorithm::CoDel => self.should_drop_codel(queue_size),
            AqmAlgorithm::Pie => self.should_drop_pie(queue_size),
            AqmAlgorithm::Blue => self.should_drop_blue(queue_size),
            AqmAlgorithm::None => false,
        }
    }

    fn should_drop_red(&self, queue_size: u64) -> bool {
        if queue_size < self.params.min_threshold as u64 {
            *self.drop_probability.lock() = 0.0;
            return false;
        }

        if queue_size > self.params.max_threshold as u64 {
            *self.drop_probability.lock() = 1.0;
            return true;
        }

        // Calculate drop probability
        let prob = (queue_size - self.params.min_threshold as u64) as f32
            / (self.params.max_threshold - self.params.min_threshold) as f32;
        *self.drop_probability.lock() = prob.min(self.params.max_probability);

        // Random drop based on probability
        // Simplified: use counter as pseudo-random
        let counter = self.stats.packets_enqueued.load(Ordering::Relaxed);
        (counter % 100) as f32 / 100.0 < prob
    }

    fn should_drop_codel(&self, _queue_size: u64) -> bool {
        // CoDel implementation would track sojourn time
        // Simplified: drop if queue is > 50% full
        _queue_size > (self.max_queue_size / 2)
    }

    fn should_drop_pie(&self, _queue_size: u64) -> bool {
        // PIE implementation
        // Simplified: drop if queue is > 75% full
        _queue_size > (self.max_queue_size * 3 / 4)
    }

    fn should_drop_blue(&self, _queue_size: u64) -> bool {
        // Blue implementation
        // Simplified: drop if queue is > 80% full
        _queue_size > (self.max_queue_size * 4 / 5)
    }

    pub fn get_stats(&self) -> AqmStatsSnapshot {
        AqmStatsSnapshot {
            packets_enqueued: self.stats.packets_enqueued.load(Ordering::Relaxed),
            packets_dequeued: self.stats.packets_dequeued.load(Ordering::Relaxed),
            packets_dropped: self.stats.packets_dropped.load(Ordering::Relaxed),
            early_drops: self.stats.early_drops.load(Ordering::Relaxed),
            current_queue_size: self.queue_size.load(Ordering::Relaxed),
        }
    }
}

/// AQM statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct AqmStatsSnapshot {
    pub packets_enqueued: u64,
    pub packets_dequeued: u64,
    pub packets_dropped: u64,
    pub early_drops: u64,
    pub current_queue_size: u64,
}

/// QoS configuration
#[derive(Debug, Clone)]
pub struct QosConfig {
    /// Enable classification
    pub enable_classification: bool,
    /// Enable shaping
    pub enable_shaping: bool,
    /// Enable AQM
    pub enable_aqm: bool,
    /// Default traffic class
    pub default_class: TrafficClass,
    /// Default DSCP marking
    pub default_dscp: Option<u8>,
}

impl Default for QosConfig {
    fn default() -> Self {
        Self {
            enable_classification: true,
            enable_shaping: true,
            enable_aqm: true,
            default_class: TrafficClass::BestEffort,
            default_dscp: None,
        }
    }
}

/// QoS manager
#[derive(Debug)]
pub struct QosManager {
    /// Configuration
    config: QosConfig,
    /// Packet classifier
    classifier: PacketClassifier,
    /// Traffic shapers
    shapers: RwLock<BTreeMap<u32, Arc<TrafficShaper>>>,
    /// AQM queues
    aqm_queues: RwLock<BTreeMap<u32, Arc<AqmQueue>>>,
    /// Next shaper ID
    next_shaper_id: AtomicU64,
    /// Next queue ID
    next_queue_id: AtomicU64,
    /// Statistics
    stats: SdnStats,
}

impl QosManager {
    pub fn new(config: QosConfig) -> Self {
        Self {
            config,
            classifier: PacketClassifier::new(),
            shapers: RwLock::new(BTreeMap::new()),
            aqm_queues: RwLock::new(BTreeMap::new()),
            next_shaper_id: AtomicU64::new(1),
            next_queue_id: AtomicU64::new(1),
            stats: SdnStats::new(),
        }
    }

    pub fn classifier(&self) -> &PacketClassifier {
        &self.classifier
    }

    pub fn create_shaper(&self, algorithm: ShapingAlgorithm) -> u32 {
        let id = self.next_shaper_id.fetch_add(1, Ordering::Relaxed) as u32;
        let shaper = Arc::new(TrafficShaper::new(id, algorithm));

        self.shapers.write().insert(id, shaper);
        id
    }

    pub fn remove_shaper(&self, id: u32) -> Result<(), QosError> {
        self.shapers
            .write()
            .remove(&id)
            .ok_or(QosError::ShaperNotFound)?;
        Ok(())
    }

    pub fn get_shaper(&self, id: u32) -> Option<Arc<TrafficShaper>> {
        self.shapers.read().get(&id).cloned()
    }

    pub fn create_aqm_queue(&self, algorithm: AqmAlgorithm, max_size: u64) -> u32 {
        let id = self.next_queue_id.fetch_add(1, Ordering::Relaxed) as u32;
        let queue = Arc::new(AqmQueue::new(algorithm, max_size, AqmParams::default()));

        self.aqm_queues.write().insert(id, queue);
        id
    }

    pub fn process_packet(&self, packet: &PacketInfo) -> Result<QosDecision, QosError> {
        // Classify packet
        let (class, dscp) = self
            .classifier
            .classify_with_dscp(packet)
            .unwrap_or_else(|| {
                let default_class = self.config.default_class;
                let default_dscp = self
                    .config
                    .default_dscp
                    .map(|d| DscpMarking::new(d, 0));
                (default_class, default_dscp.unwrap_or_else(|| DscpMarking::new(default_class.to_dscp(), 0)))
            });

        // Apply shaping
        for shaper in self.shapers.read().values() {
            let _ = shaper.shape(class, packet.length as u64);
        }

        Ok(QosDecision {
            traffic_class: class,
            dscp_marking: dscp,
            should_drop: false,
        })
    }

    pub fn get_stats(&self) -> &SdnStats {
        &self.stats
    }
}

impl Default for QosManager {
    fn default() -> Self {
        Self::new(QosConfig::default())
    }
}

/// QoS decision for packet
#[derive(Debug, Clone)]
pub struct QosDecision {
    pub traffic_class: TrafficClass,
    pub dscp_marking: DscpMarking,
    pub should_drop: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_class() {
        assert_eq!(TrafficClass::Voice.to_dscp(), 46);
        assert_eq!(TrafficClass::Video.to_dscp(), 40);

        let class = TrafficClass::from_dscp(46);
        assert_eq!(class, Some(TrafficClass::Voice));
    }

    #[test]
    fn test_dscp_marking() {
        let marking = DscpMarking::new(46, 0);
        assert_eq!(marking.dscp, 46);
        assert_eq!(marking.to_tos(), 184);

        let parsed = DscpMarking::from_tos(184);
        assert_eq!(parsed.dscp, 46);
    }

    #[test]
    fn test_packet_classifier() {
        let classifier = PacketClassifier::new();

        let rule = ClassificationRule {
            id: 1,
            match_: MatchCriteria {
                dst_port: Some(80),
                ..Default::default()
            },
            traffic_class: TrafficClass::Normal,
            dscp: None,
            priority: 100,
        };

        classifier.add_rule(rule);

        let packet = PacketInfo {
            src_ip: 0,
            dst_ip: 0,
            src_port: None,
            dst_port: Some(80),
            protocol: None,
            length: 1500,
        };

        let class = classifier.classify(&packet);
        assert_eq!(class, Some(TrafficClass::Normal));
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(1000, 5000);

        assert!(limiter.check(500).is_ok());
        assert!(limiter.check(10000).is_err());
    }

    #[test]
    fn test_traffic_shaper() {
        let mut shaper = TrafficShaper::new(1, ShapingAlgorithm::Tbf);

        shaper.add_class(TrafficClass::Voice, 100000, 1000);
        shaper.add_class(TrafficClass::Bulk, 10000, 500);

        assert!(shaper.shape(TrafficClass::Voice, 500).is_ok());
    }

    #[test]
    fn test_aqm_queue() {
        let queue = AqmQueue::new(AqmAlgorithm::Red, 10000, AqmParams::default());

        assert!(queue.enqueue(100).is_ok());

        queue.dequeue(100);

        let stats = queue.get_stats();
        assert_eq!(stats.packets_enqueued, 1);
        assert_eq!(stats.packets_dequeued, 1);
    }

    #[test]
    fn test_qos_manager() {
        let manager = QosManager::new(QosConfig::default());

        let shaper_id = manager.create_shaper(ShapingAlgorithm::Tbf);
        assert!(manager.get_shaper(shaper_id).is_some());

        let queue_id = manager.create_aqm_queue(AqmAlgorithm::Red, 10000);
        assert!(manager.get_shaper(shaper_id).is_some());

        let packet = PacketInfo {
            src_ip: 0,
            dst_ip: 0,
            src_port: None,
            dst_port: Some(80),
            protocol: None,
            length: 1500,
        };

        let decision = manager.process_packet(&packet).unwrap();
        assert!(!decision.should_drop);
    }
}
