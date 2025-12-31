//! # Edge Computing Module
//!
//! This module provides comprehensive edge computing support for the NOS kernel,
//! enabling efficient computation at the network edge with cloud synchronization.
//!
//! ## Architecture
//!
//! The edge computing framework consists of several coordinated components:
//!
//! - **manager**: Edge node management and orchestration
//! - **microvm**: Firecracker-style MicroVM implementation
//! - **faas**: Function-as-a-Service framework
//! - **sync**: Edge-cloud synchronization protocols
//! - **offline**: Offline computation support
//! - **adaptive**: Resource-aware adaptive computing
//!
//! ## Key Features
//!
//! - **Hierarchical Edge Topology**: Multi-tier edge computing with cloud-edge-device hierarchy
//! - **MicroVM Isolation**: Fast boot (<100ms) microVMs for secure function execution
//! - **FaaS Platform**: Event-driven serverless functions with cold start optimization
//! - **CRDT Sync**: Conflict-free replicated data types for edge-cloud synchronization
//! - **Offline Operation**: Queue-and-forward for intermittent connectivity
//! - **Adaptive Computing**: Resource-aware scheduling with energy efficiency
//!
//! ## Integration
//!
//! - Container Runtime: `kernel/src/container/`
//! - Virtualization: `kernel/src/vmm/`
//! - Performance Monitoring: `kernel/src/perf/`
//! - Real-time Scheduling: `kernel/src/sched/rt_sched.rs`
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::edge::{EdgeManager, FunctionConfig};
//!
//! // Initialize edge manager
//! let mut manager = EdgeManager::new();
//! manager.initialize()?;
//!
//! // Deploy serverless function
//! let config = FunctionConfig::new("process_data");
//! let func_id = manager.deploy_function(config)?;
//! ```

pub mod manager;
pub mod microvm;
pub mod faas;
pub mod sync;
pub mod offline;
pub mod adaptive;

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::prelude::*;

// Re-export main types
pub use self::manager::{
    EdgeManager, EdgeNode, EdgeNodeInfo, EdgeTopology, NodeHealth, NodeState,
    EdgeOrchestration, CloudSyncStatus, FederationConfig,
};

pub use self::microvm::{
    MicroVM, MicroVMConfig, MicroVMTemplate, MicroVMSnapshot,
    MicroVMState, UnikernelConfig, ContainerMicroVMConverter,
};

pub use self::faas::{
    FunctionService, FunctionConfig, FunctionInstance, FunctionState,
    FunctionInvocation, FunctionChain, AutoscalingPolicy, ResourceQuota,
};

pub use self::sync::{
    EdgeSyncProtocol, CRDTReplica, DeltaSyncState, SyncPriority,
    OfflineQueue, ConflictResolver, BandwidthAwareTransfer,
};

pub use self::offline::{
    OfflineManager, OfflineQueueItem, DisconnectionDetector,
    PredictiveCache, ReconnectionStrategy,
};

pub use self::adaptive::{
    AdaptiveScheduler, QoSPolicy, EnergyPolicy, ThermalState,
    DVFSController, PowerState, ResourceAllocation,
};

/// Edge computing system information
#[derive(Debug, Clone)]
pub struct EdgeSystemInfo {
    /// Number of edge nodes
    pub node_count: usize,
    /// Number of active MicroVMs
    pub microvm_count: usize,
    /// Number of deployed functions
    pub function_count: usize,
    /// Synchronization status
    pub sync_status: CloudSyncStatus,
    /// Offline mode status
    pub offline_mode: bool,
    /// Adaptive policy active
    pub adaptive_policy_active: bool,
    /// Total resource usage
    pub total_resources: ResourceUsage,
}

/// Edge resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// CPU cores used
    pub cpu_cores: usize,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Storage usage in bytes
    pub storage_bytes: u64,
    /// Network bandwidth in bytes/sec
    pub network_bandwidth: u64,
    /// Energy consumption in milliwatts
    pub energy_mw: u64,
}

/// Edge computing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeMode {
    /// Online mode (connected to cloud)
    Online,
    /// Offline mode (disconnected from cloud)
    Offline,
    /// Hybrid mode (partial connectivity)
    Hybrid,
}

/// Edge capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeCapabilities {
    /// MicroVM support
    pub microvm: bool,
    /// FaaS support
    pub faas: bool,
    /// Offline computation
    pub offline: bool,
    /// Adaptive computing
    pub adaptive: bool,
    /// CRDT synchronization
    pub crdt_sync: bool,
    /// Container-to-MicroVM conversion
    pub container_to_microvm: bool,
    /// Unikernel support
    pub unikernel: bool,
    /// DVFS support
    pub dvfs: bool,
}

impl EdgeCapabilities {
    /// Get all capabilities
    pub const fn all() -> Self {
        Self {
            microvm: true,
            faas: true,
            offline: true,
            adaptive: true,
            crdt_sync: true,
            container_to_microvm: true,
            unikernel: true,
            dvfs: true,
        }
    }

    /// Get minimal capabilities
    pub const fn minimal() -> Self {
        Self {
            microvm: true,
            faas: true,
            offline: false,
            adaptive: false,
            crdt_sync: true,
            container_to_microvm: false,
            unikernel: false,
            dvfs: false,
        }
    }
}

/// Global edge system state
static EDGE_MODE: Mutex<EdgeMode> = Mutex::new(EdgeMode::Online);
static CAPABILITIES: Mutex<EdgeCapabilities> = Mutex::new(EdgeCapabilities::all());

/// Set edge computing mode
pub fn set_edge_mode(mode: EdgeMode) {
    let mut current = EDGE_MODE.lock();
    *current = mode;
    crate::println!("[edge] Edge mode set to {:?}", mode);
}

/// Get current edge computing mode
pub fn get_edge_mode() -> EdgeMode {
    let current = EDGE_MODE.lock();
    *current
}

/// Set edge capabilities
pub fn set_capabilities(capabilities: EdgeCapabilities) {
    let mut current = CAPABILITIES.lock();
    *current = capabilities;
}

/// Get edge capabilities
pub fn get_capabilities() -> EdgeCapabilities {
    let current = CAPABILITIES.lock();
    *current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_mode() {
        set_edge_mode(EdgeMode::Offline);
        assert_eq!(get_edge_mode(), EdgeMode::Offline);

        set_edge_mode(EdgeMode::Online);
        assert_eq!(get_edge_mode(), EdgeMode::Online);
    }

    #[test]
    fn test_capabilities() {
        let caps = EdgeCapabilities::all();
        assert!(caps.microvm);
        assert!(caps.faas);

        let minimal = EdgeCapabilities::minimal();
        assert!(minimal.microvm);
        assert!(!minimal.offline);
    }
}
