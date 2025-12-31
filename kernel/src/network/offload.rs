//! Hardware offload features for high-performance networking
//!
//! This module implements hardware offload features including:
//! - TCP Segmentation Offload (TSO)
//! - UDP Fragmentation Offload (UFO)
//! - Generic Segmentation Offload (GSO)
//! - Generic Receive Offload (GRO)
//! - Receive Side Scaling (RSS)
//! - Checksum offload (TX and RX)
//! - Flow steering
//!
//! # Examples
//!
//! ```rust
//! use kernel::network::offload::{OffloadManager, OffloadFeatures, TsoConfig};
//!
//! // Create offload manager
//! let offload = OffloadManager::new();
//!
//! // Enable TSO
//! let tso_config = TsoConfig {
//!     max_segment_size: 1500,
//!     max_segments: 64,
//!     enabled: true,
//! };
//! offload.enable_tso(tso_config);
//!
//! // Enable RSS with multiple queues
//! let rss_config = RssConfig {
//!     num_queues: 8,
//!     hash_key: [0u8; 40],
//!     enabled: true,
//! };
//! offload.enable_rss(rss_config);
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

use crate::subsystems::sync::{Mutex, RwLock};
use crate::subsystems::net::ipv4::Ipv4Addr;

use super::conntrack::Protocol;

/// Default TSO maximum segment size
pub const DEFAULT_TSO_MAX_SIZE: u16 = 65536;

/// Default GSO maximum segment size
pub const DEFAULT_GSO_SIZE: u16 = 1500;

/// Maximum number of RSS queues
pub const MAX_RSS_QUEUES: usize = 256;

/// RSS hash key length
pub const RSS_HASH_KEY_LEN: usize = 40;

/// Offload feature flags
#[derive(Debug, Clone, Copy, Default)]
pub struct OffloadFeatures {
    /// TSO enabled
    pub tso: bool,
    /// UFO enabled
    pub ufo: bool,
    /// GSO enabled
    pub gso: bool,
    /// GRO enabled
    pub gro: bool,
    /// RX checksum offload enabled
    rx_csum: bool,
    /// TX checksum offload enabled
    tx_csum: bool,
    /// RSS enabled
    pub rss: bool,
    /// Flow steering enabled
    pub flow_steering: bool,
}

impl OffloadFeatures {
    /// Create new offload features
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable all offloads
    pub fn enable_all(&mut self) {
        self.tso = true;
        self.ufo = true;
        self.gso = true;
        self.gro = true;
        self.rx_csum = true;
        self.tx_csum = true;
        self.rss = true;
        self.flow_steering = true;
    }

    /// Check if any checksum offload is enabled
    pub fn has_checksum_offload(&self) -> bool {
        self.rx_csum || self.tx_csum
    }

    /// Check if RX checksum is enabled
    pub fn rx_checksum(&self) -> bool {
        self.rx_csum
    }

    /// Check if TX checksum is enabled
    pub fn tx_checksum(&self) -> bool {
        self.tx_csum
    }

    /// Enable RX checksum
    pub fn enable_rx_checksum(&mut self) {
        self.rx_csum = true;
    }

    /// Enable TX checksum
    pub fn enable_tx_checksum(&mut self) {
        self.tx_csum = true;
    }
}

/// TSO configuration
#[derive(Debug, Clone, Copy)]
pub struct TsoConfig {
    /// Maximum segment size
    pub max_segment_size: u16,
    /// Maximum segments per packet
    pub max_segments: usize,
    /// TSO enabled
    pub enabled: bool,
}

impl Default for TsoConfig {
    fn default() -> Self {
        Self {
            max_segment_size: DEFAULT_TSO_MAX_SIZE,
            max_segments: 64,
            enabled: false,
        }
    }
}

impl TsoConfig {
    /// Create new TSO configuration
    pub fn new(max_segment_size: u16, max_segments: usize) -> Self {
        Self {
            max_segment_size,
            max_segments,
            enabled: true,
        }
    }
}

/// GSO configuration
#[derive(Debug, Clone, Copy)]
pub struct GsoConfig {
    /// Segment size
    pub segment_size: u16,
    /// Maximum segments
    pub max_segments: usize,
    /// GSO enabled
    pub enabled: bool,
}

impl Default for GsoConfig {
    fn default() -> Self {
        Self {
            segment_size: DEFAULT_GSO_SIZE,
            max_segments: 64,
            enabled: false,
        }
    }
}

impl GsoConfig {
    /// Create new GSO configuration
    pub fn new(segment_size: u16, max_segments: usize) -> Self {
        Self {
            segment_size,
            max_segments,
            enabled: true,
        }
    }
}

/// GRO configuration
#[derive(Debug, Clone, Copy)]
pub struct GroConfig {
    /// Maximum aggregated packets
    pub max_packets: usize,
    /// Aggregation timeout (microseconds)
    pub timeout_us: u64,
    /// GRO enabled
    pub enabled: bool,
}

impl Default for GroConfig {
    fn default() -> Self {
        Self {
            max_packets: 64,
            timeout_us: 100,
            enabled: false,
        }
    }
}

impl GroConfig {
    /// Create new GRO configuration
    pub fn new(max_packets: usize, timeout_us: u64) -> Self {
        Self {
            max_packets,
            timeout_us,
            enabled: true,
        }
    }
}

/// RSS configuration
#[derive(Debug, Clone)]
pub struct RssConfig {
    /// Number of RSS queues
    pub num_queues: usize,
    /// RSS hash key
    pub hash_key: [u8; RSS_HASH_KEY_LEN],
    /// RSS indirection table
    pub indirection_table: Vec<u32>,
    /// RSS enabled
    pub enabled: bool,
}

impl Default for RssConfig {
    fn default() -> Self {
        Self {
            num_queues: 1,
            hash_key: [0u8; RSS_HASH_KEY_LEN],
            indirection_table: vec![0],
            enabled: false,
        }
    }
}

impl RssConfig {
    /// Create new RSS configuration
    pub fn new(num_queues: usize, hash_key: [u8; RSS_HASH_KEY_LEN]) -> Self {
        assert!(num_queues <= MAX_RSS_QUEUES, "Too many RSS queues");

        // Create indirection table (even distribution)
        let indirection_table = (0..128)
            .map(|i| (i % num_queues) as u32)
            .collect();

        Self {
            num_queues,
            hash_key,
            indirection_table,
            enabled: true,
        }
    }

    /// Calculate RSS hash
    pub fn hash(&self, src_ip: Ipv4Addr, dst_ip: Ipv4Addr, src_port: u16, dst_port: u16, protocol: Protocol) -> u32 {
        // Toeplitz hash (simplified)
        let mut hash: u32 = 0;

        // Hash source IP
        let src_ip_bytes = src_ip.to_be_bytes();
        for (i, &byte) in src_ip_bytes.iter().enumerate() {
            hash ^= ((byte as u32) << (i % 4 * 8)) & 0xFF;
        }

        // Hash destination IP
        let dst_ip_bytes = dst_ip.to_be_bytes();
        for (i, &byte) in dst_ip_bytes.iter().enumerate() {
            hash ^= ((byte as u32) << ((i + 2) % 4 * 8)) & 0xFF;
        }

        // Hash ports
        hash ^= (src_port as u32) << 16;
        hash ^= dst_port as u32;

        // Hash protocol
        hash ^= (protocol.as_number() as u32) << 8;

        // Apply hash key (simplified)
        let key_offset = (hash as usize) % RSS_HASH_KEY_LEN;
        hash ^= self.hash_key[key_offset] as u32;

        hash
    }

    /// Get queue for packet
    pub fn get_queue(&self, src_ip: Ipv4Addr, dst_ip: Ipv4Addr, src_port: u16, dst_port: u16, protocol: Protocol) -> usize {
        let hash = self.hash(src_ip, dst_ip, src_port, dst_port, protocol);
        let table_idx = (hash as usize) % self.indirection_table.len();
        self.indirection_table[table_idx] as usize % self.num_queues
    }
}

/// Checksum offload configuration
#[derive(Debug, Clone, Copy)]
pub struct ChecksumOffload {
    /// RX checksum enabled
    pub rx_enabled: bool,
    /// TX checksum enabled
    pub tx_enabled: bool,
    /// RX checksum level (0=none, 1=basic, 2=full)
    pub rx_level: u8,
}

impl Default for ChecksumOffload {
    fn default() -> Self {
        Self {
            rx_enabled: false,
            tx_enabled: false,
            rx_level: 0,
        }
    }
}

impl ChecksumOffload {
    /// Create new checksum offload configuration
    pub fn new(rx_enabled: bool, tx_enabled: bool) -> Self {
        Self {
            rx_enabled,
            tx_enabled,
            rx_level: if rx_enabled { 2 } else { 0 },
        }
    }

    /// Enable full RX checksum
    pub fn enable_rx_full(&mut self) {
        self.rx_enabled = true;
        self.rx_level = 2;
    }

    /// Enable basic RX checksum
    pub fn enable_rx_basic(&mut self) {
        self.rx_enabled = true;
        self.rx_level = 1;
    }

    /// Disable RX checksum
    pub fn disable_rx(&mut self) {
        self.rx_enabled = false;
        self.rx_level = 0;
    }

    /// Enable TX checksum
    pub fn enable_tx(&mut self) {
        self.tx_enabled = true;
    }

    /// Disable TX checksum
    pub fn disable_tx(&mut self) {
        self.tx_enabled = false;
    }
}

/// Flow steering types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowSteeringType {
    /// No steering
    None,
    /// Explicit flow steering (rules)
    Explicit,
    /// Implicit flow steering (RSS)
    Implicit,
}

/// Flow steering rule
#[derive(Debug, Clone)]
pub struct FlowSteeringRule {
    /// Rule priority
    pub priority: u32,
    /// Source IP
    pub src_ip: Option<Ipv4Addr>,
    /// Destination IP
    pub dst_ip: Option<Ipv4Addr>,
    /// Source port
    pub src_port: Option<u16>,
    /// Destination port
    pub dst_port: Option<u16>,
    /// Protocol
    pub protocol: Option<Protocol>,
    /// Target queue
    pub queue: usize,
}

/// Flow steering configuration
#[derive(Debug, Clone)]
pub struct FlowSteering {
    /// Steering type
    pub steering_type: FlowSteeringType,
    /// Steering rules
    pub rules: Vec<FlowSteeringRule>,
    /// Number of queues
    pub num_queues: usize,
}

impl Default for FlowSteering {
    fn default() -> Self {
        Self {
            steering_type: FlowSteeringType::Implicit,
            rules: Vec::new(),
            num_queues: 1,
        }
    }
}

impl FlowSteering {
    /// Create new flow steering configuration
    pub fn new(num_queues: usize, steering_type: FlowSteeringType) -> Self {
        Self {
            steering_type,
            rules: Vec::new(),
            num_queues,
        }
    }

    /// Add steering rule
    pub fn add_rule(&mut self, rule: FlowSteeringRule) {
        self.rules.push(rule);
        self.rules.sort_by_key(|r| -r.priority);
    }

    /// Get queue for packet
    pub fn get_queue(
        &self,
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
        protocol: Protocol,
    ) -> usize {
        // Check explicit rules
        if self.steering_type == FlowSteeringType::Explicit {
            for rule in &self.rules {
                if self.rule_matches(rule, src_ip, dst_ip, src_port, dst_port, protocol) {
                    return rule.queue.min(self.num_queues - 1);
                }
            }
        }

        // Default: use RSS implicit steering
        let rss = RssConfig::new(self.num_queues, [0u8; RSS_HASH_KEY_LEN]);
        rss.get_queue(src_ip, dst_ip, src_port, dst_port, protocol)
    }

    /// Check if rule matches packet
    fn rule_matches(
        &self,
        rule: &FlowSteeringRule,
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
        protocol: Protocol,
    ) -> bool {
        if let Some(rule_src_ip) = rule.src_ip {
            if rule_src_ip != src_ip {
                return false;
            }
        }

        if let Some(rule_dst_ip) = rule.dst_ip {
            if rule_dst_ip != dst_ip {
                return false;
            }
        }

        if let Some(rule_src_port) = rule.src_port {
            if rule_src_port != src_port {
                return false;
            }
        }

        if let Some(rule_dst_port) = rule.dst_port {
            if rule_dst_port != dst_port {
                return false;
            }
        }

        if let Some(rule_protocol) = rule.protocol {
            if rule_protocol != protocol {
                return false;
            }
        }

        true
    }
}

/// Offload statistics
#[derive(Debug, Default, Clone)]
pub struct OffloadStats {
    /// TSO segments sent
    pub tso_segments: u64,
    /// TSO bytes offloaded
    pub tso_bytes: u64,
    /// GSO segments sent
    pub gso_segments: u64,
    /// GSO bytes offloaded
    pub gso_bytes: u64,
    /// GRO packets aggregated
    pub gro_packets: u64,
    /// GRO bytes aggregated
    pub gro_bytes: u64,
    /// RX checksum offloads
    pub rx_checksum: u64,
    /// TX checksum offloads
    pub tx_checksum: u64,
    /// RSS hash calculations
    pub rss_hashes: u64,
    /// Flow steering hits
    pub flow_steering_hits: u64,
}

/// Offload manager
#[derive(Debug)]
pub struct OffloadManager {
    /// Supported offload features
    features: RwLock<OffloadFeatures>,
    /// TSO configuration
    tso: Mutex<TsoConfig>,
    /// GSO configuration
    gso: Mutex<GsoConfig>,
    /// GRO configuration
    gro: Mutex<GroConfig>,
    /// RSS configuration
    rss: Mutex<RssConfig>,
    /// Checksum offload
    checksum: Mutex<ChecksumOffload>,
    /// Flow steering
    flow_steering: Mutex<FlowSteering>,
    /// Statistics
    stats: Mutex<OffloadStats>,
}

impl OffloadManager {
    /// Create new offload manager
    pub fn new() -> Self {
        Self {
            features: RwLock::new(OffloadFeatures::new()),
            tso: Mutex::new(TsoConfig::default()),
            gso: Mutex::new(GsoConfig::default()),
            gro: Mutex::new(GroConfig::default()),
            rss: Mutex::new(RssConfig::default()),
            checksum: Mutex::new(ChecksumOffload::default()),
            flow_steering: Mutex::new(FlowSteering::default()),
            stats: Mutex::new(OffloadStats::default()),
        }
    }

    /// Initialize offload manager
    pub fn init(&mut self) -> Result<(), ()> {
        // Initialize with default configuration
        Ok(())
    }

    /// Enable TSO
    pub fn enable_tso(&self, config: TsoConfig) {
        let mut tso = self.tso.lock();
        *tso = config;

        let mut features = self.features.write();
        features.tso = config.enabled;
    }

    /// Get TSO configuration
    pub fn get_tso_config(&self) -> TsoConfig {
        let tso = self.tso.lock();
        *tso
    }

    /// Enable GSO
    pub fn enable_gso(&self, config: GsoConfig) {
        let mut gso = self.gso.lock();
        *gso = config;

        let mut features = self.features.write();
        features.gso = config.enabled;
    }

    /// Get GSO configuration
    pub fn get_gso_config(&self) -> GsoConfig {
        let gso = self.gso.lock();
        *gso
    }

    /// Enable GRO
    pub fn enable_gro(&self, config: GroConfig) {
        let mut gro = self.gro.lock();
        *gro = config;

        let mut features = self.features.write();
        features.gro = config.enabled;
    }

    /// Get GRO configuration
    pub fn get_gro_config(&self) -> GroConfig {
        let gro = self.gro.lock();
        *gro
    }

    /// Enable RSS
    pub fn enable_rss(&self, config: RssConfig) {
        let mut rss = self.rss.lock();
        *rss = config;

        let mut features = self.features.write();
        features.rss = config.enabled;
    }

    /// Get RSS configuration
    pub fn get_rss_config(&self) -> RssConfig {
        let rss = self.rss.lock();
        (*rss).clone()
    }

    /// Enable checksum offload
    pub fn enable_checksum(&self, config: ChecksumOffload) {
        let mut checksum = self.checksum.lock();
        *checksum = config;

        let mut features = self.features.write();
        features.rx_csum = config.rx_enabled;
        features.tx_csum = config.tx_enabled;
    }

    /// Get checksum configuration
    pub fn get_checksum_config(&self) -> ChecksumOffload {
        let checksum = self.checksum.lock();
        *checksum
    }

    /// Configure flow steering
    pub fn configure_flow_steering(&self, config: FlowSteering) {
        let mut flow_steering = self.flow_steering.lock();
        *flow_steering = config;

        let mut features = self.features.write();
        features.flow_steering = config.steering_type != FlowSteeringType::None;
    }

    /// Add flow steering rule
    pub fn add_steering_rule(&self, rule: FlowSteeringRule) {
        let mut flow_steering = self.flow_steering.lock();
        flow_steering.add_rule(rule);
    }

    /// Process outgoing packet with offload
    pub fn process_send(&self, packet: &[u8], protocol: Protocol) -> Result<Vec<u8>, ()> {
        let features = self.features.read();

        // Apply TSO/GSO if enabled
        if features.tso || features.gso {
            let _ = self.segment_packet(packet);
            // Return original packet for simplicity
            return Ok(packet.to_vec());
        }

        // Apply checksum offload if enabled
        if features.tx_checksum {
            let mut stats = self.stats.lock();
            stats.tx_checksum += 1;
        }

        Ok(packet.to_vec())
    }

    /// Process incoming packet with offload
    pub fn process_receive(&self, packet: &[u8], protocol: Protocol) -> Result<Vec<u8>, ()> {
        let features = self.features.read();

        // Apply GRO if enabled
        if features.gro {
            let _ = self.aggregate_packets(&[packet.to_vec()]);
            // Return original packet for simplicity
        }

        // Apply RX checksum offload if enabled
        if features.rx_checksum {
            let mut stats = self.stats.lock();
            stats.rx_checksum += 1;
        }

        Ok(packet.to_vec())
    }

    /// Segment packet using TSO/GSO
    fn segment_packet(&self, packet: &[u8]) -> Vec<Vec<u8>> {
        let tso = self.tso.lock();
        let gso = self.gso.lock();

        let segment_size = if tso.enabled {
            tso.max_segment_size
        } else if gso.enabled {
            gso.segment_size
        } else {
            return vec![packet.to_vec()];
        };

        let max_segments = if tso.enabled {
            tso.max_segments
        } else {
            gso.max_segments
        };

        let mut segments = Vec::new();
        let chunk_size = segment_size as usize;

        for (i, chunk) in packet.chunks(chunk_size).enumerate() {
            if i >= max_segments {
                break;
            }
            segments.push(chunk.to_vec());
        }

        // Update statistics
        let mut stats = self.stats.lock();
        if tso.enabled {
            stats.tso_segments += segments.len() as u64;
            stats.tso_bytes += packet.len() as u64;
        } else {
            stats.gso_segments += segments.len() as u64;
            stats.gso_bytes += packet.len() as u64;
        }

        segments
    }

    /// Aggregate packets using GRO
    fn aggregate_packets(&self, packets: &[Vec<u8>]) -> Vec<Vec<u8>> {
        let gro = self.gro.lock();

        if !gro.enabled {
            return packets.to_vec();
        }

        // Simple aggregation: combine consecutive packets
        let mut aggregated = Vec::new();
        let mut current = Vec::new();
        let mut total_size = 0;

        for packet in packets {
            if current.is_empty() {
                current.extend_from_slice(packet);
                total_size += packet.len();
            } else if total_size + packet.len() <= gro.max_packets * 1500 {
                current.extend_from_slice(packet);
                total_size += packet.len();
            } else {
                if !current.is_empty() {
                    aggregated.push(core::mem::replace(&mut current, Vec::new()));
                }
                total_size = packet.len();
                current.extend_from_slice(packet);
            }
        }

        if !current.is_empty() {
            aggregated.push(current);
        }

        // Update statistics
        let mut stats = self.stats.lock();
        stats.gro_packets += (packets.len() - aggregated.len()) as u64;
        stats.gro_bytes += packets.iter().map(|p| p.len()).sum::<usize>() as u64;

        aggregated
    }

    /// Get queue for packet using RSS/flow steering
    pub fn get_queue(
        &self,
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
        protocol: Protocol,
    ) -> usize {
        let flow_steering = self.flow_steering.lock();
        flow_steering.get_queue(src_ip, dst_ip, src_port, dst_port, protocol)
    }

    /// Calculate RSS hash
    pub fn calculate_rss_hash(
        &self,
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        src_port: u16,
        dst_port: u16,
        protocol: Protocol,
    ) -> u32 {
        let rss = self.rss.lock();
        rss.hash(src_ip, dst_ip, src_port, dst_port, protocol)
    }

    /// Get supported features
    pub fn get_features(&self) -> OffloadFeatures {
        let features = self.features.read();
        *features
    }

    /// Get statistics
    pub fn get_stats(&self) -> OffloadStats {
        let stats = self.stats.lock();
        stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = OffloadStats::default();
    }
}

impl Default for OffloadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offload_features() {
        let mut features = OffloadFeatures::new();
        assert!(!features.tso);
        assert!(!features.gso);

        features.enable_all();
        assert!(features.tso);
        assert!(features.gso);
        assert!(features.has_checksum_offload());
    }

    #[test]
    fn test_tso_config() {
        let config = TsoConfig::new(1500, 64);
        assert!(config.enabled);
        assert_eq!(config.max_segment_size, 1500);
        assert_eq!(config.max_segments, 64);
    }

    #[test]
    fn test_gso_config() {
        let config = GsoConfig::new(1500, 32);
        assert!(config.enabled);
        assert_eq!(config.segment_size, 1500);
        assert_eq!(config.max_segments, 32);
    }

    #[test]
    fn test_gro_config() {
        let config = GroConfig::new(64, 200);
        assert!(config.enabled);
        assert_eq!(config.max_packets, 64);
        assert_eq!(config.timeout_us, 200);
    }

    #[test]
    fn test_rss_config() {
        let hash_key = [1u8; RSS_HASH_KEY_LEN];
        let config = RssConfig::new(8, hash_key);

        assert!(config.enabled);
        assert_eq!(config.num_queues, 8);
        assert_eq!(config.indirection_table.len(), 128);
    }

    #[test]
    fn test_rss_hash() {
        let hash_key = [1u8; RSS_HASH_KEY_LEN];
        let config = RssConfig::new(8, hash_key);

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let hash1 = config.hash(src_ip, dst_ip, 12345, 80, Protocol::TCP);
        let hash2 = config.hash(src_ip, dst_ip, 12345, 80, Protocol::TCP);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_rss_get_queue() {
        let hash_key = [1u8; RSS_HASH_KEY_LEN];
        let config = RssConfig::new(4, hash_key);

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let queue = config.get_queue(src_ip, dst_ip, 12345, 80, Protocol::TCP);
        assert!(queue < 4);
    }

    #[test]
    fn test_checksum_offload() {
        let mut csum = ChecksumOffload::new(true, false);
        assert!(csum.rx_enabled);
        assert!(!csum.tx_enabled);
        assert_eq!(csum.rx_level, 2);

        csum.enable_tx();
        assert!(csum.tx_enabled);
    }

    #[test]
    fn test_flow_steering() {
        let mut steering = FlowSteering::new(8, FlowSteeringType::Explicit);

        let rule = FlowSteeringRule {
            priority: 100,
            src_ip: Some(Ipv4Addr::new(192, 168, 1, 100)),
            dst_port: Some(80),
            queue: 3,
            ..Default::default()
        };

        steering.add_rule(rule.clone());

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let queue = steering.get_queue(src_ip, dst_ip, 12345, 80, Protocol::TCP);
        assert_eq!(queue, 3);
    }

    #[test]
    fn test_offload_manager() {
        let manager = OffloadManager::new();
        assert!(manager.init().is_ok());
    }

    #[test]
    fn test_offload_manager_tso() {
        let manager = OffloadManager::new();
        let config = TsoConfig::new(1500, 64);
        manager.enable_tso(config);

        let features = manager.get_features();
        assert!(features.tso);

        let retrieved_config = manager.get_tso_config();
        assert_eq!(retrieved_config.max_segment_size, 1500);
    }

    #[test]
    fn test_offload_manager_rss() {
        let manager = OffloadManager::new();

        let hash_key = [1u8; RSS_HASH_KEY_LEN];
        let config = RssConfig::new(8, hash_key);
        manager.enable_rss(config);

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let queue = manager.get_queue(src_ip, dst_ip, 12345, 80, Protocol::TCP);
        assert!(queue < 8);
    }

    #[test]
    fn test_offload_manager_process() {
        let manager = OffloadManager::new();

        let packet = vec![1u8; 1500];
        let result = manager.process_send(&packet, Protocol::TCP);
        assert!(result.is_ok());

        let result = manager.process_receive(&packet, Protocol::UDP);
        assert!(result.is_ok());
    }

    #[test]
    fn test_offload_stats() {
        let manager = OffloadManager::new();

        let checksum_config = ChecksumOffload::new(true, true);
        manager.enable_checksum(checksum_config);

        let packet = vec![1u8; 1500];
        let _ = manager.process_send(&packet, Protocol::TCP);
        let _ = manager.process_receive(&packet, Protocol::TCP);

        let stats = manager.get_stats();
        assert!(stats.tx_checksum > 0);
        assert!(stats.rx_checksum > 0);
    }

    #[test]
    fn test_flow_steering_rule() {
        let rule = FlowSteeringRule {
            priority: 100,
            src_ip: Some(Ipv4Addr::new(192, 168, 1, 1)),
            dst_ip: Some(Ipv4Addr::new(10, 0, 0, 1)),
            src_port: Some(12345),
            dst_port: Some(80),
            protocol: Some(Protocol::TCP),
            queue: 5,
        };

        assert_eq!(rule.priority, 100);
        assert_eq!(rule.queue, 5);
    }

    #[test]
    fn test_offload_manager_flow_steering() {
        let manager = OffloadManager::new();

        let rule = FlowSteeringRule {
            priority: 100,
            dst_port: Some(443),
            queue: 2,
            ..Default::default()
        };

        manager.add_steering_rule(rule);

        let src_ip = Ipv4Addr::new(192, 168, 1, 100);
        let dst_ip = Ipv4Addr::new(10, 0, 0, 1);

        let queue = manager.get_queue(src_ip, dst_ip, 12345, 443, Protocol::TCP);
        assert_eq!(queue, 2);
    }
}
