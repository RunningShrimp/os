//! Failure Detection and Automatic Failover
//!
//! This module implements failure detection using heartbeat and phi accrual algorithms,
//! automatic failover mechanisms, state synchronization, and fencing (STONITH).

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::sync::{Mutex, RwLock};

/// Unique identifier for a node
pub type NodeId = u64;

/// Health status of a node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeHealth {
    /// Node is healthy
    Healthy,
    /// Node is suspected to be down
    Suspected,
    /// Node is confirmed down
    Down,
    /// Node is recovering
    Recovering,
    /// Node is leaving the cluster
    Leaving,
}

/// Failure detection method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureDetector {
    /// Simple heartbeat-based detection
    Heartbeat,
    /// Phi accrual failure detector
    PhiAccrual,
    /// Adaptive failure detection
    Adaptive,
}

/// Phi accrual failure detector state
#[derive(Debug, Clone)]
struct PhiAccrualState {
    /// Last heartbeat timestamp
    last_heartbeat: u64,
    /// Interval samples
    intervals: VecDeque<Duration>,
    /// Mean interval
    mean_interval: Duration,
    /// Standard deviation
    std_dev: Duration,
    /// Phi threshold
    phi_threshold: f64,
}

/// Node health information
#[derive(Debug, Clone)]
pub struct NodeHealthInfo {
    /// Node ID
    pub node_id: NodeId,
    /// Current health status
    pub health: NodeHealth,
    /// Last heartbeat received
    pub last_heartbeat_ms: u64,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Phi value (if using phi accrual)
    pub phi: Option<f64>,
    /// Last state update time
    pub last_update_ms: u64,
}

/// Failover action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailoverAction {
    /// No action needed
    None,
    /// Trigger failover for a node
    TriggerFailover(NodeId),
    /// Fence a node
    FenceNode(NodeId),
    /// Start recovery process
    StartRecovery(NodeId),
    /// Remove node from cluster
    RemoveNode(NodeId),
}

/// Fencing method (STONITH - Shoot The Other Node In The Head)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FencingMethod {
    /// Power off the node
    PowerOff,
    /// Reset the node
    Reset,
    /// Network isolation
    NetworkIsolation,
    /// Storage quorum revoke
    StorageRevoke,
    /// Custom fencing
    Custom,
}

/// Fencing result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FencingResult {
    /// Fencing successful
    Success,
    /// Fencing failed
    Failed,
    /// Fencing in progress
    InProgress,
    /// Node not reachable
    Unreachable,
}

/// Failover state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverState {
    /// No failover in progress
    Idle,
    /// Detecting failure
    Detecting,
    /// Coordinating failover
    Coordinating,
    /// Performing failover
    FailingOver,
    /// Synchronizing state
    Synchronizing,
    /// Recovering
    Recovering,
}

/// Statistics for failover module
#[derive(Debug, Default)]
pub struct FailoverStatistics {
    /// Total failures detected
    pub failures_detected: AtomicU64,
    /// Failovers triggered
    pub failovers_triggered: AtomicU64,
    /// Failovers completed
    pub failovers_completed: AtomicU64,
    /// Nodes fenced
    pub nodes_fenced: AtomicU64,
    /// False positives (healthy node marked as failed)
    pub false_positives: AtomicU64,
    /// Average failure detection time (ms)
    pub avg_detection_time_ms: AtomicU64,
    /// Average failover time (ms)
    pub avg_failover_time_ms: AtomicU64,
    /// Recovery attempts
    pub recovery_attempts: AtomicU64,
    /// Successful recoveries
    pub successful_recoveries: AtomicU64,
}

/// Errors that can occur during failover
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailoverError {
    /// Node not found
    NodeNotFound(NodeId),
    /// Failover already in progress
    FailoverInProgress,
    /// Quorum not reachable
    QuorumNotReachable,
    /// Fencing failed
    FencingFailed {
        node_id: NodeId,
        method: FencingMethod,
    },
    /// State synchronization failed
    SyncFailed(String),
    /// No suitable replacement node
    NoReplacementNode,
    /// Cluster is partitioned
    ClusterPartitioned,
    /// Resource allocation failed
    ResourceAllocationFailed,
    /// Timeout during failover
    Timeout,
    /// Permission denied
    PermissionDenied,
    /// Internal error
    InternalError(String),
}

impl core::fmt::Display for FailoverError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FailoverError::NodeNotFound(node_id) => {
                write!(f, "Node {} not found", node_id)
            }
            FailoverError::FailoverInProgress => {
                write!(f, "Failover already in progress")
            }
            FailoverError::QuorumNotReachable => {
                write!(f, "Quorum not reachable")
            }
            FailoverError::FencingFailed { node_id, method } => {
                write!(f, "Fencing failed for node {} using {:?}", node_id, method)
            }
            FailoverError::SyncFailed(msg) => {
                write!(f, "State synchronization failed: {}", msg)
            }
            FailoverError::NoReplacementNode => {
                write!(f, "No suitable replacement node available")
            }
            FailoverError::ClusterPartitioned => {
                write!(f, "Cluster is partitioned")
            }
            FailoverError::ResourceAllocationFailed => {
                write!(f, "Failed to allocate resources for failover")
            }
            FailoverError::Timeout => {
                write!(f, "Timeout during failover operation")
            }
            FailoverError::PermissionDenied => {
                write!(f, "Permission denied for failover operation")
            }
            FailoverError::InternalError(msg) => {
                write!(f, "Internal error: {}", msg)
            }
        }
    }
}

/// Failure detector configuration
#[derive(Debug, Clone)]
pub struct FailureDetectorConfig {
    /// Detection method
    pub method: FailureDetector,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Failure timeout (for heartbeat method)
    pub failure_timeout: Duration,
    /// Phi threshold (for phi accrual method)
    pub phi_threshold: f64,
    /// Maximum samples for phi accrual
    pub max_samples: usize,
    /// Minimum samples before calculating phi
    pub min_samples: usize,
}

impl Default for FailureDetectorConfig {
    fn default() -> Self {
        Self {
            method: FailureDetector::PhiAccrual,
            heartbeat_interval: Duration::from_millis(100),
            failure_timeout: Duration::from_secs(5),
            phi_threshold: 8.0,
            max_samples: 1000,
            min_samples: 50,
        }
    }
}

/// Failure Detector
pub struct FailureDetector {
    /// Local node ID
    local_node: NodeId,
    /// Configuration
    config: FailureDetectorConfig,
    /// Health state of all nodes
    node_health: RwLock<BTreeMap<NodeId, NodeHealthInfo>>,
    /// Phi accrual states
    phi_states: RwLock<BTreeMap<NodeId, PhiAccrualState>>,
    /// Monitored nodes
    monitored_nodes: RwLock<BTreeSet<NodeId>>,
    /// Current time (milliseconds)
    current_time_ms: AtomicU64,
    /// Enabled flag
    enabled: AtomicBool,
}

impl FailureDetector {
    /// Create a new failure detector
    pub fn new(local_node: NodeId, config: FailureDetectorConfig) -> Self {
        Self {
            local_node,
            config,
            node_health: RwLock::new(BTreeMap::new()),
            phi_states: RwLock::new(BTreeMap::new()),
            monitored_nodes: RwLock::new(BTreeSet::new()),
            current_time_ms: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Start monitoring a node
    pub fn monitor_node(&self, node_id: NodeId) -> Result<(), FailoverError> {
        let mut monitored = self.monitored_nodes.write();
        if monitored.contains(&node_id) {
            return Ok(());
        }

        monitored.insert(node_id);

        // Initialize health info
        let health_info = NodeHealthInfo {
            node_id,
            health: NodeHealth::Healthy,
            last_heartbeat_ms: self.current_time(),
            consecutive_failures: 0,
            phi: None,
            last_update_ms: self.current_time(),
        };

        let mut health = self.node_health.write();
        health.insert(node_id, health_info);

        // Initialize phi accrual state if needed
        if self.config.method == FailureDetector::PhiAccrual {
            let phi_state = PhiAccrualState {
                last_heartbeat: self.current_time(),
                intervals: VecDeque::with_capacity(self.config.max_samples),
                mean_interval: self.config.heartbeat_interval,
                std_dev: Duration::from_millis(0),
                phi_threshold: self.config.phi_threshold,
            };
            let mut phi_states = self.phi_states.write();
            phi_states.insert(node_id, phi_state);
        }

        Ok(())
    }

    /// Stop monitoring a node
    pub fn stop_monitoring(&self, node_id: NodeId) -> Result<(), FailoverError> {
        let mut monitored = self.monitored_nodes.write();
        if !monitored.remove(&node_id) {
            return Err(FailoverError::NodeNotFound(node_id));
        }

        let mut health = self.node_health.write();
        health.remove(&node_id);

        let mut phi_states = self.phi_states.write();
        phi_states.remove(&node_id);

        Ok(())
    }

    /// Process a heartbeat from a node
    pub fn process_heartbeat(&self, node_id: NodeId) -> Result<NodeHealth, FailoverError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(NodeHealth::Healthy);
        }

        let current_time = self.current_time();

        // Update phi accrual state
        if self.config.method == FailureDetector::PhiAccrual {
            self.update_phi_state(node_id, current_time)?;
        }

        // Update health info
        let mut health = self.node_health.write();
        if let Some(info) = health.get_mut(&node_id) {
            info.last_heartbeat_ms = current_time;
            info.consecutive_failures = 0;
            info.health = NodeHealth::Healthy;
            info.phi = None;
            info.last_update_ms = current_time;
            return Ok(info.health);
        }

        Err(FailoverError::NodeNotFound(node_id))
    }

    /// Update phi accrual state for a node
    fn update_phi_state(
        &self,
        node_id: NodeId,
        current_time: u64,
    ) -> Result<(), FailoverError> {
        let mut phi_states = self.phi_states.write();
        let state = phi_states
            .get_mut(&node_id)
            .ok_or(FailoverError::NodeNotFound(node_id))?;

        // Calculate interval since last heartbeat
        let interval = Duration::from_millis(current_time.saturating_sub(state.last_heartbeat));
        state.last_heartbeat = current_time;

        // Add to samples
        state.intervals.push_back(interval);
        if state.intervals.len() > self.config.max_samples {
            state.intervals.pop_front();
        }

        // Recalculate statistics if we have enough samples
        if state.intervals.len() >= self.config.min_samples {
            let sum: Duration = state.intervals.iter().sum();
            let count = state.intervals.len() as u64;
            state.mean_interval = sum / count;

            // Calculate standard deviation
            let variance_sum: u64 = state
                .intervals
                .iter()
                .map(|i| {
                    let diff = i.as_millis().saturating_sub(state.mean_interval.as_millis() as u64);
                    diff * diff
                })
                .sum();
            let variance = variance_sum / count;
            state.std_dev = Duration::from_millis((variance as f64).sqrt() as u64);
        }

        Ok(())
    }

    /// Calculate phi value for a node
    fn calculate_phi(&self, node_id: NodeId) -> Option<f64> {
        let phi_states = self.phi_states.read();
        let state = phi_states.get(&node_id)?;

        if state.intervals.len() < self.config.min_samples {
            return None;
        }

        let current_time = self.current_time();
        let time_since_last = current_time.saturating_sub(state.last_heartbeat);

        // Phi = log10(1 - CDF(t)) where t is time since last heartbeat
        // Using normal approximation
        if state.std_dev.as_millis() == 0 {
            return None;
        }

        let z = time_since_last as f64 / state.std_dev.as_millis() as f64;
        // Approximate phi
        let phi = if z > 0.0 {
            z * z / 2.0 + 0.5
        } else {
            0.0
        };

        Some(phi)
    }

    /// Check if a node is suspected to have failed
    pub fn check_node_failure(&self, node_id: NodeId) -> Result<bool, FailoverError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(false);
        }

        let current_time = self.current_time();
        let health = self.node_health.read();
        let info = health
            .get(&node_id)
            .ok_or(FailoverError::NodeNotFound(node_id))?;

        match self.config.method {
            FailureDetector::Heartbeat => {
                let elapsed = current_time.saturating_sub(info.last_heartbeat_ms);
                Ok(elapsed > self.config.failure_timeout.as_millis() as u64)
            }
            FailureDetector::PhiAccrual => {
                if let Some(phi) = self.calculate_phi(node_id) {
                    Ok(phi > self.config.phi_threshold)
                } else {
                    Ok(false)
                }
            }
            FailureDetector::Adaptive => {
                // Combination of methods
                let elapsed = current_time.saturating_sub(info.last_heartbeat_ms);
                let timeout_check = elapsed > self.config.failure_timeout.as_millis() as u64;

                let phi_check = if let Some(phi) = self.calculate_phi(node_id) {
                    phi > self.config.phi_threshold
                } else {
                    false
                };

                Ok(timeout_check || phi_check)
            }
        }
    }

    /// Get health information for all monitored nodes
    pub fn get_all_health(&self) -> Vec<NodeHealthInfo> {
        let health = self.node_health.read();
        health.values().cloned().collect()
    }

    /// Get health information for a specific node
    pub fn get_health(&self, node_id: NodeId) -> Option<NodeHealthInfo> {
        let health = self.node_health.read();
        health.get(&node_id).cloned()
    }

    /// Update health status of a node
    pub fn set_health(&self, node_id: NodeId, health: NodeHealth) -> Result<(), FailoverError> {
        let mut health_map = self.node_health.write();
        let info = health_map
            .get_mut(&node_id)
            .ok_or(FailoverError::NodeNotFound(node_id))?;
        info.health = health;
        info.last_update_ms = self.current_time();
        Ok(())
    }

    /// Get current time in milliseconds
    fn current_time(&self) -> u64 {
        self.current_time_ms.load(Ordering::Relaxed)
    }

    /// Set current time (for testing)
    pub fn set_time(&self, time: u64) {
        self.current_time_ms.store(time, Ordering::Relaxed);
    }

    /// Enable or disable the failure detector
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

/// Failover Manager
pub struct FailoverManager {
    /// Local node ID
    local_node: NodeId,
    /// Failure detector
    detector: Arc<FailureDetector>,
    /// Current failover state
    failover_state: Mutex<FailoverState>,
    /// Nodes currently being failed over
    failing_over: Mutex<BTreeSet<NodeId>>,
    /// Statistics
    stats: Arc<FailoverStatistics>,
    /// Fencing timeout
    fencing_timeout: Duration,
    /// State sync timeout
    sync_timeout: Duration,
    /// Enable automatic failover
    auto_failover_enabled: AtomicBool,
}

impl FailoverManager {
    /// Create a new failover manager
    pub fn new(
        local_node: NodeId,
        detector_config: FailureDetectorConfig,
        fencing_timeout: Duration,
        sync_timeout: Duration,
    ) -> Self {
        Self {
            local_node,
            detector: Arc::new(FailureDetector::new(local_node, detector_config)),
            failover_state: Mutex::new(FailoverState::Idle),
            failing_over: Mutex::new(BTreeSet::new()),
            stats: Arc::new(FailoverStatistics::default()),
            fencing_timeout,
            sync_timeout,
            auto_failover_enabled: AtomicBool::new(true),
        }
    }

    /// Get the failure detector
    pub fn detector(&self) -> Arc<FailureDetector> {
        Arc::clone(&self.detector)
    }

    /// Monitor a node for failures
    pub fn monitor_node(&self, node_id: NodeId) -> Result<(), FailoverError> {
        self.detector.monitor_node(node_id)
    }

    /// Stop monitoring a node
    pub fn stop_monitoring(&self, node_id: NodeId) -> Result<(), FailoverError> {
        self.detector.stop_monitoring(node_id)
    }

    /// Process heartbeat from a node
    pub fn process_heartbeat(&self, node_id: NodeId) -> Result<NodeHealth, FailoverError> {
        self.detector.process_heartbeat(node_id)
    }

    /// Check for failures and trigger failover if needed
    pub fn check_and_failover(&self) -> Result<Vec<FailoverAction>, FailoverError> {
        let mut actions = Vec::new();
        let health_info = self.detector.get_all_health();

        for info in health_info {
            if self.detector.check_node_failure(info.node_id)? {
                // Node is suspected to have failed
                self.stats.failures_detected.fetch_add(1, Ordering::Relaxed);

                let action = if self.auto_failover_enabled.load(Ordering::Relaxed) {
                    FailoverAction::TriggerFailover(info.node_id)
                } else {
                    FailoverAction::None
                };

                actions.push(action);
            }
        }

        Ok(actions)
    }

    /// Trigger failover for a failed node
    pub fn trigger_failover(&self, node_id: NodeId) -> Result<(), FailoverError> {
        let mut state = self.failover_state.lock();
        if *state != FailoverState::Idle {
            return Err(FailoverError::FailoverInProgress);
        }

        *state = FailoverState::Detecting;

        // Mark node as failing over
        let mut failing = self.failing_over.lock();
        if failing.contains(&node_id) {
            *state = FailoverState::Idle;
            return Err(FailoverError::FailoverInProgress);
        }
        failing.insert(node_id);

        // Update health to down
        self.detector.set_health(node_id, NodeHealth::Down)?;

        self.stats.failovers_triggered.fetch_add(1, Ordering::Relaxed);

        // In a real implementation, this would coordinate with cluster
        // to select a replacement node and migrate resources
        *state = FailoverState::Coordinating;

        Ok(())
    }

    /// Fence a node using STONITH
    pub fn fence_node(
        &self,
        node_id: NodeId,
        method: FencingMethod,
    ) -> Result<FencingResult, FailoverError> {
        // Update health to down
        self.detector.set_health(node_id, NodeHealth::Down)?;

        // In a real implementation, this would execute fencing
        // For now, return success
        self.stats.nodes_fenced.fetch_add(1, Ordering::Relaxed);
        Ok(FencingResult::Success)
    }

    /// Complete failover process
    pub fn complete_failover(&self, node_id: NodeId) -> Result<(), FailoverError> {
        // Remove from failing over set
        let mut failing = self.failing_over.lock();
        failing.remove(&node_id);

        // Reset state
        *self.failover_state.lock() = FailoverState::Idle;

        self.stats.failovers_completed.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Start recovery process for a node
    pub fn start_recovery(&self, node_id: NodeId) -> Result<(), FailoverError> {
        // Update health to recovering
        self.detector.set_health(node_id, NodeHealth::Recovering)?;

        self.stats.recovery_attempts.fetch_add(1, Ordering::Relaxed);

        // In a real implementation, this would:
        // 1. Verify node is back online
        // 2. Synchronize state
        // 3. Restore resources

        Ok(())
    }

    /// Complete recovery process
    pub fn complete_recovery(&self, node_id: NodeId) -> Result<(), FailoverError> {
        // Update health to healthy
        self.detector.set_health(node_id, NodeHealth::Healthy)?;

        self.stats.successful_recoveries.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Check if failover is in progress
    pub fn is_failover_in_progress(&self) -> bool {
        *self.failover_state.lock() != FailoverState::Idle
    }

    /// Get current failover state
    pub fn get_failover_state(&self) -> FailoverState {
        *self.failover_state.lock()
    }

    /// Enable or disable automatic failover
    pub fn set_auto_failover(&self, enabled: bool) {
        self.auto_failover_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Get statistics
    pub fn get_statistics(&self) -> &FailoverStatistics {
        &self.stats
    }

    /// Get nodes currently being failed over
    pub fn get_failing_over_nodes(&self) -> Vec<NodeId> {
        let failing = self.failing_over.lock();
        failing.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_detection() {
        let config = FailureDetectorConfig {
            method: FailureDetector::Heartbeat,
            heartbeat_interval: Duration::from_millis(100),
            failure_timeout: Duration::from_secs(1),
            ..Default::default()
        };

        let detector = FailureDetector::new(1, config);

        detector.monitor_node(2).unwrap();
        detector.process_heartbeat(2).unwrap();

        assert!(!detector.check_node_failure(2).unwrap());
    }

    #[test]
    fn test_failover_trigger() {
        let config = FailureDetectorConfig::default();
        let manager = FailoverManager::new(
            1,
            config,
            Duration::from_secs(5),
            Duration::from_secs(10),
        );

        manager.monitor_node(2).unwrap();
        manager.trigger_failover(2).unwrap();

        assert!(manager.is_failover_in_progress());
    }
}
