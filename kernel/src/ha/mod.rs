//! # High Availability and Fault Tolerance
//!
//! This module provides enterprise-grade high availability (HA) and fault tolerance
//! features for the NOS kernel, enabling mission-critical deployments with minimal
//! downtime and data loss.
//!
//! ## Architecture Overview
//!
//! The HA system is built around several key components:
//!
//! - **Cluster Management** ([`cluster`]): Raft-based consensus, leader election,
//!   and cluster membership
//! - **Data Replication** ([`replication`]): Synchronous/asynchronous replication
//!   with multi-master support
//! - **Failover Management** ([`failover`]): Active-passive and active-active
//!   failover with health checking
//! - **Backup & Recovery** ([`backup`]): Incremental/differential backups with
//!   point-in-time recovery
//! - **Disaster Recovery** ([`disaster`]): Multi-site replication and geo-redundancy
//! - **Resilience Patterns** ([`resilience`]): Circuit breakers, retry with backoff,
//!   and chaos engineering support
//!
//! ## Design Goals
//!
//! ### High Availability (HA)
//!
//! - **99.999% uptime target** (5 minutes downtime per year)
//! - **Zero single points of failure**
//! - **Automatic failover** in < 1 second
//! - **Graceful degradation** under load
//!
//! ### Fault Tolerance
//!
//! - **Byzantine fault tolerance** for critical operations
//! - **Crash recovery** with consistent state
//! - **Network partition handling** (CAP theorem aware)
//! - **Data corruption detection** and repair
//!
//! ### Disaster Recovery
//!
//! - **RTO < 15 minutes** (Recovery Time Objective)
//! - **RPO < 1 minute** (Recovery Point Objective)
//! - **Multi-site redundancy** across geographic regions
//! - **Automated backup verification**
//!
//! ## Usage Examples
//!
//! ### Initialize a Cluster
//!
//! ```no_run
//! use kernel::ha::cluster::{Cluster, ClusterConfig, NodeId};
//! use kernel::ha::replication::{ReplicationManager, ReplicationMode};
//!
//! // Create a 3-node cluster
//! let config = ClusterConfig::three_node_replicated();
//! let cluster = Cluster::new(NodeId::from(1), config).await?;
//!
//! // Start the cluster
//! cluster.start().await?;
//! # Ok::<(), kernel::ha::HaError>(())
//! ```
//!
//! ### Configure Replication
//!
//! ```no_run
//! use kernel::ha::replication::{ReplicationManager, ReplicationConfig, ReplicationMode};
//!
//! let config = ReplicationConfig {
//!     mode: ReplicationMode::Synchronous,
//!     consistency_level: kernel::ha::replication::ConsistencyLevel::Quorum,
//!     ..Default::default()
//! };
//!
//! let replication = ReplicationManager::new(config);
//! replication.start().await?;
//! # Ok::<(), kernel::ha::HaError>(())
//! ```
//!
//! ### Perform Failover
//!
//! ```no_run
//! use kernel::ha::failover::{FailoverManager, FailoverConfig};
//!
//! let config = FailoverConfig::active_passive();
//! let manager = FailoverManager::new(config);
//!
//! // Automatic failover on health check failure
//! manager.monitor_and_failover().await?;
//! # Ok::<(), kernel::ha::HaError>(())
//! ```
//!
//! ### Create Backup
//!
//! ```no_run
//! use kernel::ha::backup::{BackupManager, BackupType};
//!
//! let manager = BackupManager::new();
//!
//! // Create incremental backup
//! let backup_id = manager.create_backup(BackupType::Incremental).await?;
//!
//! // Verify backup integrity
//! let verified = manager.verify_backup(&backup_id).await?;
//! # Ok::<(), kernel::ha::HaError>(())
//! ```
//!
//! ### Disaster Recovery Setup
//!
//! ```no_run
//! use kernel::ha::disaster::{DisasterRecoveryManager, DrSiteConfig};
//!
//! let config = DrSiteConfig {
//!     primary_site: "us-east-1".to_string(),
//!     dr_site: "us-west-2".to_string(),
//!     replication_lag_secs: 30,
//!     ..Default::default()
//! };
//!
//! let dr_manager = DisasterRecoveryManager::new(config);
//! dr_manager.setup_replication().await?;
//! # Ok::<(), kernel::ha::HaError>(())
//! ```
//!
//! ## Consensus and Replication
//!
//! The cluster uses the Raft consensus algorithm for strong consistency:
//!
//! - **Leader election**: Nodes vote for a leader based on log completeness
//! - **Log replication**: Leader replicates entries to followers
//! - **Safety**: Only committed entries are applied to state machine
//! - **Liveness**: System makes progress as long as majority is available
//!
//! ## Consistency Models
//!
//! The system supports multiple consistency levels:
//!
//! - **Strong**: Linearizable consistency (synchronous replication)
//! - **Eventual**: Updates propagate asynchronously
//! - **Quorum**: Majority ack before commit (tunable)
//! - **Causal**: Preserves cause-effect relationships
//!
//! ## Failure Scenarios
//!
//! ### Network Partition
//!
//! - **Primary partition**: Continues serving requests with majority
//! - **Minority partition**: Steps down, awaits reconnection
//! - **Merge resolution**: Automatic conflict resolution using Raft
//!
//! ### Node Failure
//!
//! - **Detection**: Heartbeat timeout (default 5 seconds)
//! - **Reconfiguration**: Cluster membership updated
//! - **Recovery**: Node catches up via log replication
//! - **Restoration**: Automatic when node rejoins
//!
//! ### Data Corruption
//!
//! - **Detection**: Checksums and Merkle trees
//! - **Isolation**: Corrupted replicas quarantined
//! - **Repair**: Automatic recovery from healthy replicas
//! - **Verification**: Post-repair integrity checks
//!
//! ## Performance Characteristics
//!
//! - **Replication latency**: < 10ms (synchronous, local)
//! - **Failover time**: < 1 second
//! - **Backup throughput**: > 1 GB/s (incremental)
//! - **Recovery time**: < 15 minutes (from backup)
//!
//! ## Monitoring and Observability
//!
//! The HA system provides comprehensive metrics:
//!
//! - Cluster health and membership
//! Replication lag and throughput
//! - Backup completion and verification
//! - Failover events and reasons
//! - RTO/RPO compliance
//!
//! ## Testing and Chaos Engineering
//!
//! The resilience module supports chaos experiments:
//!
//! - Network latency injection
//! - Random node failures
//! - Partition simulation
//! - Stress testing
//!
//! ## Related Modules
//!
//! - [`subsystems::distributed`]: Distributed systems primitives
//! - [`subsystems::sync`]: Synchronization and locking
//! - [`error`]: Error handling and recovery
//! - [`monitoring`]: Health monitoring and alerting
//!
//! ## References
//!
//! - [Raft Consensus Algorithm](https://raft.github.io/)
//! - [CAP Theorem](https://en.wikipedia.org/wiki/CAP_theorem)
//! - [Disaster Recovery Best Practices](https://www.druva.com/blog/what-is-disaster-recovery/)

pub mod cluster;
pub mod replication;
pub mod failover;
pub mod backup;
pub mod disaster;
pub mod resilience;

// Re-export commonly used types
pub use cluster::{
    Cluster, ClusterConfig, NodeId, ClusterState, NodeRole,
    ClusterMembership, LeaderElection,
};
pub use replication::{
    ReplicationManager, ReplicationConfig, ReplicationMode,
    ConsistencyLevel, ReplicationStatus, ConflictResolution,
};
pub use failover::{
    FailoverManager, FailoverConfig, FailoverState,
    FailoverPolicy, FailoverEvent, HealthStatus,
};
pub use backup::{
    BackupManager, BackupConfig, BackupType, BackupStatus,
    BackupVerification, PointInTimeRecovery,
};
pub use disaster::{
    DisasterRecoveryManager, DrSiteConfig, ReplicationSite,
    FailoverPlan, RtoRpoMetrics, GeoRedundancy,
};
pub use resilience::{
    ResilienceManager, CircuitBreaker, RetryPolicy, BackoffStrategy,
    ChaosConfig, ResilienceMetrics,
};

use core::fmt;

/// Unified error type for HA operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HaError {
    /// Cluster-related errors
    ClusterError(ClusterError),

    /// Replication errors
    ReplicationError(ReplicationError),

    /// Failover errors
    FailoverError(FailoverError),

    /// Backup errors
    BackupError(BackupError),

    /// Disaster recovery errors
    DisasterError(DisasterError),

    /// Resilience errors
    ResilienceError(ResilienceError),
}

impl fmt::Display for HaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HaError::ClusterError(e) => write!(f, "Cluster error: {}", e),
            HaError::ReplicationError(e) => write!(f, "Replication error: {}", e),
            HaError::FailoverError(e) => write!(f, "Failover error: {}", e),
            HaError::BackupError(e) => write!(f, "Backup error: {}", e),
            HaError::DisasterError(e) => write!(f, "Disaster recovery error: {}", e),
            HaError::ResilienceError(e) => write!(f, "Resilience error: {}", e),
        }
    }
}

#[cfg(feature = "kernel_tests")]
impl core::error::Error for HaError {}

/// Cluster-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterError {
    /// Node not found in cluster
    NodeNotFound,
    /// Cluster has no leader
    NoLeader,
    /// Election timeout
    ElectionTimeout,
    /// Log inconsistency
    LogInconsistent,
    /// RPC failure
    RpcFailed,
    /// Network partition detected
    NetworkPartition,
    /// Cluster shut down
    ClusterShutdown,
}

impl fmt::Display for ClusterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClusterError::NodeNotFound => write!(f, "Node not found in cluster"),
            ClusterError::NoLeader => write!(f, "Cluster has no leader"),
            ClusterError::ElectionTimeout => write!(f, "Election timeout"),
            ClusterError::LogInconsistent => write!(f, "Log inconsistency detected"),
            ClusterError::RpcFailed => write!(f, "RPC call failed"),
            ClusterError::NetworkPartition => write!(f, "Network partition detected"),
            ClusterError::ClusterShutdown => write!(f, "Cluster is shut down"),
        }
    }
}

/// Replication-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicationError {
    /// Replication lag too high
    LagTooHigh,
    /// Quorum not achieved
    QuorumNotReached,
    /// Conflict detected
    ConflictDetected,
    /// Replication channel full
    BufferFull,
    /// Replica not responding
    ReplicaTimeout,
    /// Data corrupted
    DataCorrupted,
}

impl fmt::Display for ReplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplicationError::LagTooHigh => write!(f, "Replication lag exceeds threshold"),
            ReplicationError::QuorumNotReached => write!(f, "Quorum could not be reached"),
            ReplicationError::ConflictDetected => write!(f, "Replication conflict detected"),
            ReplicationError::BufferFull => write!(f, "Replication buffer is full"),
            ReplicationError::ReplicaTimeout => write!(f, "Replica timed out"),
            ReplicationError::DataCorrupted => write!(f, "Replicated data is corrupted"),
        }
    }
}

/// Failover-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailoverError {
    /// No healthy nodes available
    NoHealthyNodes,
    /// Health check failed
    HealthCheckFailed,
    /// Failover in progress
    FailoverInProgress,
    /// Standby not ready
    StandbyNotReady,
    /// Split-brain detected
    SplitBrainDetected,
    /// Graceful shutdown failed
    ShutdownFailed,
}

impl fmt::Display for FailoverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FailoverError::NoHealthyNodes => write!(f, "No healthy nodes available"),
            FailoverError::HealthCheckFailed => write!(f, "Health check failed"),
            FailoverError::FailoverInProgress => write!(f, "Failover already in progress"),
            FailoverError::StandbyNotReady => write!(f, "Standby node not ready"),
            FailoverError::SplitBrainDetected => write!(f, "Split-brain condition detected"),
            FailoverError::ShutdownFailed => write!(f, "Graceful shutdown failed"),
        }
    }
}

/// Backup-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupError {
    /// Backup not found
    BackupNotFound,
    /// Backup corrupted
    BackupCorrupted,
    /// Verification failed
    VerificationFailed,
    /// Storage insufficient
    InsufficientStorage,
    /// Restore failed
    RestoreFailed,
    /// Backup in progress
    BackupInProgress,
}

impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackupError::BackupNotFound => write!(f, "Backup not found"),
            BackupError::BackupCorrupted => write!(f, "Backup data is corrupted"),
            BackupError::VerificationFailed => write!(f, "Backup verification failed"),
            BackupError::InsufficientStorage => write!(f, "Insufficient storage for backup"),
            BackupError::RestoreFailed => write!(f, "Restore operation failed"),
            BackupError::BackupInProgress => write!(f, "Backup operation already in progress"),
        }
    }
}

/// Disaster recovery-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisasterError {
    /// DR site not reachable
    SiteUnreachable,
    /// RTO/RPO SLA violated
    SlaViolation,
    /// Failover plan invalid
    InvalidPlan,
    /// Replication broken
    ReplicationBroken,
    /// Data loss detected
    DataLossDetected,
    /// Site not ready
    SiteNotReady,
}

impl fmt::Display for DisasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisasterError::SiteUnreachable => write!(f, "DR site unreachable"),
            DisasterError::SlaViolation => write!(f, "RTO/RPO SLA violated"),
            DisasterError::InvalidPlan => write!(f, "Invalid failover plan"),
            DisasterError::ReplicationBroken => write!(f, "Replication to DR site broken"),
            DisasterError::DataLossDetected => write!(f, "Data loss detected"),
            DisasterError::SiteNotReady => write!(f, "DR site not ready"),
        }
    }
}

/// Resilience-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResilienceError {
    /// Circuit breaker open
    CircuitBreakerOpen,
    /// Retry attempts exhausted
    RetriesExhausted,
    /// Chaos test failed
    ChaosTestFailed,
    /// Timeout
    Timeout,
    /// Backoff limit reached
    BackoffLimitReached,
    /// Invalid configuration
    InvalidConfig,
}

impl fmt::Display for ResilienceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResilienceError::CircuitBreakerOpen => write!(f, "Circuit breaker is open"),
            ResilienceError::RetriesExhausted => write!(f, "Retry attempts exhausted"),
            ResilienceError::ChaosTestFailed => write!(f, "Chaos test failed"),
            ResilienceError::Timeout => write!(f, "Operation timed out"),
            ResilienceError::BackoffLimitReached => write!(f, "Backoff limit reached"),
            ResilienceError::InvalidConfig => write!(f, "Invalid configuration"),
        }
    }
}

/// Result type for HA operations
pub type HaResult<T> = core::result::Result<T, HaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = HaError::ClusterError(ClusterError::NoLeader);
        assert!(err.to_string().contains("no leader"));

        let err = HaError::ReplicationError(ReplicationError::LagTooHigh);
        assert!(err.to_string().contains("lag"));
    }

    #[test]
    fn test_error_conversion() {
        // Test that errors can be created and converted
        let cluster_err = ClusterError::NetworkPartition;
        let ha_err = HaError::ClusterError(cluster_err);
        assert!(matches!(ha_err, HaError::ClusterError(_)));
    }
}
