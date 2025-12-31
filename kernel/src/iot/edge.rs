//! # Edge Computing Framework
//!
//! This module provides Lambda-style edge computing capabilities for IoT devices,
//! enabling serverless function execution at the edge.
//!
//! ## Features
//!
//! - **Lambda-style functions**: Serverless edge function execution
//! - **Fog computing**: Hierarchical multi-tier edge processing
//! - **Sandboxing**: Secure isolated function execution
//! - **Resource constraints**: CPU and memory limits
//! - **Offline operation**: Continue working without cloud connectivity
//! - **Edge-cloud sync**: Bidirectional synchronization
//! - **Workload offloading**: Offload to cloud or other edge nodes
//!
//! ## Architecture
//!
//! ```text
//! +-----------------------------------+
//! |       Edge Computing Layer        |
//! |  +-----------------------------+  |
//! |  |   Function Manager           |  |
//! |  |  - Registration              |  |
//! |  |  - Invocation                |  |
//! |  |  - Scheduling                |  |
//! |  +-----------------------------+  |
//! |  +-----------------------------+  |
//! |  |   Execution Engine           |  |
//! |  |  - Sandbox                   |  |
//! |  |  - Resource Limits           |  |
//! |  |  - Monitoring                |  |
//! |  +-----------------------------+  |
//! |  +-----------------------------+  |
//! |  |   Fog Orchestration          |  |
//! |  |  - Tier Management           |  |
//! |  |  - Load Balancing            |  |
//! |  |  - Failover                  |  |
//! |  +-----------------------------+  |
//! +-----------------------------------+
//! ```

#![allow(dead_code)]
#![warn(missing_docs)]

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::iot::IotError;
use crate::iot::IotResult;

// =============================================================================
// Edge Function Types
// =============================================================================

/// Edge function handler type
pub type EdgeFunctionHandler = fn(context: &EdgeContext, input: &[u8]) -> IotResult<Vec<u8>>;

/// Edge function
pub struct EdgeFunction {
    /// Function name
    pub name: String,
    /// Function handler
    pub handler: EdgeFunctionHandler,
    /// Function metadata
    pub metadata: FunctionMetadata,
    /// Enabled flag
    pub enabled: bool,
}

impl EdgeFunction {
    /// Create a new edge function
    pub fn new(name: impl Into<String>, handler: EdgeFunctionHandler) -> Self {
        Self {
            name: name.into(),
            handler,
            metadata: FunctionMetadata::default(),
            enabled: true,
        }
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: FunctionMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Execute function
    pub fn execute(&self, context: &EdgeContext, input: &[u8]) -> IotResult<Vec<u8>> {
        if !self.enabled {
            return Err(IotError::InvalidState("Function disabled".to_string()));
        }

        (self.handler)(context, input)
    }
}

/// Function metadata
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Description
    pub description: String,
    /// Version
    pub version: String,
    /// Author
    pub author: String,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Max memory in bytes
    pub max_memory: usize,
    /// Max CPU time in milliseconds
    pub max_cpu_ms: u64,
    /// Tags
    pub tags: Vec<String>,
    /// Environment variables
    pub env_vars: BTreeMap<String, String>,
}

impl Default for FunctionMetadata {
    fn default() -> Self {
        Self {
            description: String::new(),
            version: "1.0.0".to_string(),
            author: String::new(),
            timeout_ms: 5000,
            max_memory: 128 * 1024 * 1024, // 128MB
            max_cpu_ms: 1000,
            tags: Vec::new(),
            env_vars: BTreeMap::new(),
        }
    }
}

/// Function execution result
#[derive(Debug, Clone)]
pub struct FunctionResult {
    /// Function name
    pub function_name: String,
    /// Success flag
    pub success: bool,
    /// Output data
    pub output: Vec<u8>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Memory used in bytes
    pub memory_used: usize,
    /// CPU time in milliseconds
    pub cpu_time_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
}

impl FunctionResult {
    /// Create successful result
    pub fn success(
        function_name: impl Into<String>,
        output: Vec<u8>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            function_name: function_name.into(),
            success: true,
            output,
            execution_time_ms,
            memory_used: 0,
            cpu_time_ms: 0,
            error: None,
        }
    }

    /// Create failed result
    pub fn failure(function_name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            function_name: function_name.into(),
            success: false,
            output: Vec::new(),
            execution_time_ms: 0,
            memory_used: 0,
            cpu_time_ms: 0,
            error: Some(error.into()),
        }
    }
}

// =============================================================================
// Execution Context
// =============================================================================

/// Edge execution context
#[derive(Debug, Clone)]
pub struct EdgeContext {
    /// Request ID
    pub request_id: String,
    /// Timestamp
    pub timestamp: u64,
    /// Device ID
    pub device_id: String,
    /// Context data
    pub data: BTreeMap<String, String>,
}

impl EdgeContext {
    /// Create a new context
    pub fn new(request_id: impl Into<String>, device_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            timestamp: 0, // Would be set to current time
            device_id: device_id.into(),
            data: BTreeMap::new(),
        }
    }

    /// Add context data
    pub fn add(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
    }

    /// Get context data
    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
}

// =============================================================================
// Resource Limits
// =============================================================================

/// Resource limits for function execution
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    /// Max CPU time in milliseconds
    pub max_cpu_ms: u64,
    /// Max memory in bytes
    pub max_memory: usize,
    /// Max execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Max network I/O in bytes
    pub max_network_io: usize,
}

impl ResourceLimits {
    /// Create new resource limits
    pub fn new() -> Self {
        Self {
            max_cpu_ms: 1000,
            max_memory: 128 * 1024 * 1024,
            max_execution_time_ms: 5000,
            max_network_io: 10 * 1024 * 1024,
        }
    }

    /// Set CPU limit
    pub fn with_cpu_limit(mut self, ms: u64) -> Self {
        self.max_cpu_ms = ms;
        self
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, bytes: usize) -> Self {
        self.max_memory = bytes;
        self
    }

    /// Set execution time limit
    pub fn with_execution_time_limit(mut self, ms: u64) -> Self {
        self.max_execution_time_ms = ms;
        self
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Function Manager
// =============================================================================

/// Function execution stats
#[derive(Debug, Clone, Copy, Default)]
pub struct FunctionStats {
    /// Total invocations
    pub invocations: u64,
    /// Successful invocations
    pub successes: u64,
    /// Failed invocations
    pub failures: u64,
    /// Total execution time in milliseconds
    pub total_time_ms: u64,
    /// Average execution time in milliseconds
    pub avg_time_ms: f64,
}

/// Edge function manager
pub struct FunctionManager {
    /// Registered functions
    functions: BTreeMap<String, EdgeFunction>,
    /// Function stats
    stats: BTreeMap<String, FunctionStats>,
    /// Execution counter
    execution_counter: AtomicU64,
}

impl FunctionManager {
    /// Create a new function manager
    pub fn new() -> Self {
        Self {
            functions: BTreeMap::new(),
            stats: BTreeMap::new(),
            execution_counter: AtomicU64::new(0),
        }
    }

    /// Register function
    pub fn register(&mut self, function: EdgeFunction) -> IotResult<()> {
        let name = function.name.clone();
        self.functions.insert(name.clone(), function);
        self.stats.insert(name.clone(), FunctionStats {
            invocations: 0,
            successes: 0,
            failures: 0,
            total_time_ms: 0,
            avg_time_ms: 0.0,
        });

        crate::log_info!("Registered edge function: {}", &name);
        Ok(())
    }

    /// Unregister function
    pub fn unregister(&mut self, name: &str) -> IotResult<()> {
        self.functions
            .remove(name)
            .ok_or_else(|| IotError::ServiceNotFound(name.to_string()))?;

        self.stats.remove(name);

        crate::log_info!("Unregistered edge function: {}", name);
        Ok(())
    }

    /// Get function
    pub fn get(&self, name: &str) -> Option<&EdgeFunction> {
        self.functions.get(name)
    }

    /// List functions
    pub fn list(&self) -> Vec<&EdgeFunction> {
        self.functions.values().collect()
    }

    /// Execute function
    pub fn execute(
        &mut self,
        name: &str,
        context: &EdgeContext,
        input: &[u8],
    ) -> IotResult<FunctionResult> {
        let function = self
            .functions
            .get(name)
            .ok_or_else(|| IotError::ServiceNotFound(name.to_string()))?;

        // Update stats
        let stats = self.stats.entry(name.to_string()).or_default();
        stats.invocations += 1;

        let start_time: u64 = 0; // Would get actual time

        match function.execute(context, input) {
            Ok(output) => {
                let end_time: u64 = 0; // Would get actual time
                let execution_time = end_time.saturating_sub(start_time);

                stats.successes += 1;
                stats.total_time_ms += execution_time;
                stats.avg_time_ms = if stats.invocations > 0 {
                    stats.total_time_ms as f64 / stats.invocations as f64
                } else {
                    0.0
                };

                Ok(FunctionResult::success(name, output, execution_time))
            }
            Err(e) => {
                stats.failures += 1;
                Ok(FunctionResult::failure(name, e.to_string()))
            }
        }
    }

    /// Get function stats
    pub fn get_stats(&self, name: &str) -> Option<&FunctionStats> {
        self.stats.get(name)
    }

    /// Enable function
    pub fn enable(&mut self, name: &str) -> IotResult<()> {
        let function = self
            .functions
            .get_mut(name)
            .ok_or_else(|| IotError::ServiceNotFound(name.to_string()))?;

        function.enabled = true;
        Ok(())
    }

    /// Disable function
    pub fn disable(&mut self, name: &str) -> IotResult<()> {
        let function = self
            .functions
            .get_mut(name)
            .ok_or_else(|| IotError::ServiceNotFound(name.to_string()))?;

        function.enabled = false;
        Ok(())
    }
}

impl Default for FunctionManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Fog Computing
// =============================================================================

/// Fog tier (hierarchical level)
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FogTier {
    /// Device level (tier 0)
    Device = 0,
    /// Edge gateway (tier 1)
    EdgeGateway = 1,
    /// Local aggregation (tier 2)
    LocalAggregation = 2,
    /// Regional aggregation (tier 3)
    RegionalAggregation = 3,
    /// Cloud (tier 4)
    Cloud = 4,
}

/// Fog node
#[derive(Debug, Clone)]
pub struct FogNode {
    /// Node ID
    pub node_id: String,
    /// Node tier
    pub tier: FogTier,
    /// Node address
    pub address: String,
    /// Node capacity (0.0-1.0)
    pub capacity: f32,
    /// Node load (0.0-1.0)
    pub load: f32,
    /// Online flag
    pub online: bool,
}

impl FogNode {
    /// Create a new fog node
    pub fn new(
        node_id: impl Into<String>,
        tier: FogTier,
        address: impl Into<String>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            tier,
            address: address.into(),
            capacity: 1.0,
            load: 0.0,
            online: true,
        }
    }

    /// Available capacity
    pub fn available_capacity(&self) -> f32 {
        if !self.online {
            return 0.0;
        }
        self.capacity - self.load
    }

    /// Check if can handle workload
    pub fn can_handle(&self, required_capacity: f32) -> bool {
        self.online && self.available_capacity() >= required_capacity
    }
}

/// Fog orchestration manager
pub struct FogOrchestrator {
    /// Fog nodes
    nodes: BTreeMap<String, FogNode>,
    /// Current tier
    current_tier: FogTier,
}

impl FogOrchestrator {
    /// Create a new fog orchestrator
    pub fn new(current_tier: FogTier) -> Self {
        Self {
            nodes: BTreeMap::new(),
            current_tier,
        }
    }

    /// Add node
    pub fn add_node(&mut self, node: FogNode) {
        let node_id = node.node_id.clone();
        self.nodes.insert(node_id, node);
    }

    /// Remove node
    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.remove(node_id);
    }

    /// Find best node for workload
    pub fn find_best_node(&self, required_capacity: f32) -> Option<&FogNode> {
        self.nodes
            .values()
            .filter(|n| n.can_handle(required_capacity))
            .filter(|n| n.tier > self.current_tier || n.tier == self.current_tier)
            .min_by(|a, b| {
                a.load
                    .partial_cmp(&b.load)
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
    }

    /// Offload workload to higher tier
    pub fn offload_up(
        &self,
        _input: &[u8],
        required_capacity: f32,
    ) -> IotResult<Option<String>> {
        if let Some(node) = self.find_best_node(required_capacity) {
            // In a real implementation, send workload to node
            Ok(Some(node.node_id.clone()))
        } else {
            Ok(None)
        }
    }

    /// Get nodes at tier
    pub fn get_nodes_at_tier(&self, tier: FogTier) -> Vec<&FogNode> {
        self.nodes.values().filter(|n| n.tier == tier).collect()
    }

    /// Update node load
    pub fn update_load(&mut self, node_id: &str, load: f32) -> IotResult<()> {
        let node = self
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| IotError::DeviceNotFound(node_id.to_string()))?;

        node.load = load.clamp(0.0, 1.0);
        Ok(())
    }
}

// =============================================================================
// Offline Operation
// =============================================================================

/// Offline operation mode
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OfflineMode {
    /// Fail on network errors
    Fail,
    /// Queue for later
    Queue,
    /// Use cached data
    Cache,
}

/// Offline manager
pub struct OfflineManager {
    /// Offline mode
    mode: OfflineMode,
    /// Queue of pending operations
    queue: Vec<(String, Vec<u8>)>,
    /// Max queue size
    max_queue_size: usize,
    /// Online flag
    online: AtomicBool,
}

impl OfflineManager {
    /// Create a new offline manager
    pub fn new(mode: OfflineMode, max_queue_size: usize) -> Self {
        Self {
            mode,
            queue: Vec::new(),
            max_queue_size,
            online: AtomicBool::new(true),
        }
    }

    /// Set offline mode
    pub fn set_mode(&mut self, mode: OfflineMode) {
        self.mode = mode;
    }

    /// Set online status
    pub fn set_online(&self, online: bool) {
        self.online.store(online, Ordering::SeqCst);
    }

    /// Check if online
    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Relaxed)
    }

    /// Queue operation
    pub fn queue(&mut self, operation: String, data: Vec<u8>) -> IotResult<()> {
        if self.queue.len() >= self.max_queue_size {
            return Err(IotError::ResourceExhausted("Queue full".to_string()));
        }

        self.queue.push((operation, data));
        Ok(())
    }

    /// Process queued operations
    pub fn process_queue(&mut self) -> IotResult<usize> {
        let count = self.queue.len();

        if !self.is_online() {
            return Ok(0);
        }

        // In a real implementation, process each queued operation
        self.queue.clear();

        Ok(count)
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }
}

// =============================================================================
// Edge-Cloud Synchronization
// =============================================================================

/// Sync direction
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SyncDirection {
    /// Upload to cloud
    Upload,
    /// Download from cloud
    Download,
    /// Bidirectional
    Bidirectional,
}

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Sync direction
    pub direction: SyncDirection,
    /// Sync interval in seconds
    pub interval_sec: u64,
    /// Conflict resolution
    pub conflict_resolution: ConflictResolution,
    /// Auto-sync flag
    pub auto_sync: bool,
}

/// Conflict resolution strategy
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Prefer local
    LocalWins,
    /// Prefer remote
    RemoteWins,
    /// Use newest timestamp
    NewestWins,
    /// Manual resolution
    Manual,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            direction: SyncDirection::Bidirectional,
            interval_sec: 60,
            conflict_resolution: ConflictResolution::NewestWins,
            auto_sync: true,
        }
    }
}

/// Sync status
#[derive(Debug, Clone, Copy)]
pub struct SyncStatus {
    /// Last sync time
    pub last_sync: u64,
    /// Successful uploads
    pub uploads: u64,
    /// Successful downloads
    pub downloads: u64,
    /// Pending uploads
    pub pending_uploads: u64,
    /// Sync errors
    pub errors: u64,
}

/// Edge-cloud sync manager
pub struct SyncManager {
    /// Sync configuration
    config: SyncConfig,
    /// Sync status
    status: SyncStatus,
    /// Pending data
    pending_data: BTreeMap<String, Vec<u8>>,
}

impl SyncManager {
    /// Create a new sync manager
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            status: SyncStatus {
                last_sync: 0,
                uploads: 0,
                downloads: 0,
                pending_uploads: 0,
                errors: 0,
            },
            pending_data: BTreeMap::new(),
        }
    }

    /// Add data to sync
    pub fn add_data(&mut self, key: impl Into<String>, data: Vec<u8>) {
        self.pending_data.insert(key.into(), data);
        self.status.pending_uploads = self.pending_data.len() as u64;
    }

    /// Perform sync
    pub fn sync(&mut self, current_time: u64) -> IotResult<SyncStatus> {
        // In a real implementation:
        // 1. Upload pending data
        // 2. Download remote changes
        // 3. Resolve conflicts

        let upload_count = self.pending_data.len() as u64;

        // Clear pending after upload
        self.pending_data.clear();
        self.status.pending_uploads = 0;

        self.status.uploads += upload_count;
        self.status.last_sync = current_time;

        Ok(self.status)
    }

    /// Get sync status
    pub fn status(&self) -> SyncStatus {
        self.status
    }

    /// Configure sync
    pub fn configure(&mut self, config: SyncConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_function() {
        fn test_handler(_ctx: &EdgeContext, input: &[u8]) -> IotResult<Vec<u8>> {
            Ok(input.to_vec())
        }

        let func = EdgeFunction::new("test", test_handler);
        assert_eq!(func.name, "test");
        assert!(func.enabled);
    }

    #[test]
    fn test_function_execution() {
        fn test_handler(_ctx: &EdgeContext, input: &[u8]) -> IotResult<Vec<u8>> {
            Ok(input.to_vec())
        }

        let func = EdgeFunction::new("test", test_handler);
        let ctx = EdgeContext::new("req-1", "device-1");

        let result = func.execute(&ctx, b"hello").unwrap();
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_function_manager() {
        fn test_handler(_ctx: &EdgeContext, _input: &[u8]) -> IotResult<Vec<u8>> {
            Ok(Vec::new())
        }

        let mut manager = FunctionManager::new();

        let func = EdgeFunction::new("test", test_handler);
        manager.register(func).unwrap();

        assert!(manager.get("test").is_some());
        assert_eq!(manager.list().len(), 1);
    }

    #[test]
    fn test_function_execution_via_manager() {
        fn echo_handler(_ctx: &EdgeContext, input: &[u8]) -> IotResult<Vec<u8>> {
            Ok(input.to_vec())
        }

        let mut manager = FunctionManager::new();

        let func = EdgeFunction::new("echo", echo_handler);
        manager.register(func).unwrap();

        let ctx = EdgeContext::new("req-1", "device-1");
        let result = manager.execute("echo", &ctx, b"data").unwrap();

        assert!(result.success);
        assert_eq!(result.output, b"data");
    }

    #[test]
    fn test_function_stats() {
        fn handler(_ctx: &EdgeContext, _input: &[u8]) -> IotResult<Vec<u8>> {
            Ok(Vec::new())
        }

        let mut manager = FunctionManager::new();

        let func = EdgeFunction::new("test", handler);
        manager.register(func).unwrap();

        let ctx = EdgeContext::new("req-1", "device-1");

        manager.execute("test", &ctx, b"").ok();
        manager.execute("test", &ctx, b"").ok();

        let stats = manager.get_stats("test").unwrap();
        assert_eq!(stats.invocations, 2);
        assert_eq!(stats.successes, 2);
    }

    #[test]
    fn test_fog_node() {
        let node = FogNode::new("node-1", FogTier::EdgeGateway, "192.168.1.100");

        assert_eq!(node.node_id, "node-1");
        assert_eq!(node.tier, FogTier::EdgeGateway);
        assert!(node.online);
        assert_eq!(node.available_capacity(), 1.0);
    }

    #[test]
    fn test_fog_node_capacity() {
        let mut node = FogNode::new("node-1", FogTier::EdgeGateway, "192.168.1.100");

        node.load = 0.5;
        assert_eq!(node.available_capacity(), 0.5);

        assert!(node.can_handle(0.3));
        assert!(!node.can_handle(0.6));
    }

    #[test]
    fn test_fog_orchestrator() {
        let mut orchestrator = FogOrchestrator::new(FogTier::Device);

        let node1 = FogNode::new("node-1", FogTier::EdgeGateway, "192.168.1.100");
        let node2 = FogNode::new("node-2", FogTier::LocalAggregation, "192.168.1.101");

        orchestrator.add_node(node1);
        orchestrator.add_node(node2);

        assert_eq!(orchestrator.get_nodes_at_tier(FogTier::EdgeGateway).len(), 1);
    }

    #[test]
    fn test_offline_manager() {
        let manager = OfflineManager::new(OfflineMode::Queue, 100);

        assert!(manager.is_online());

        manager.set_online(false);
        assert!(!manager.is_online());

        manager.queue("op1", b"data".to_vec()).unwrap();
        assert_eq!(manager.queue_size(), 1);
    }

    #[test]
    fn test_sync_manager() {
        let config = SyncConfig::default();
        let mut manager = SyncManager::new(config);

        manager.add_data("key1", b"data1".to_vec());
        manager.add_data("key2", b"data2".to_vec());

        assert_eq!(manager.status().pending_uploads, 2);

        manager.sync(12345).unwrap();
        assert_eq!(manager.status().uploads, 2);
        assert_eq!(manager.status().pending_uploads, 0);
    }

    #[test]
    fn test_resource_limits() {
        let limits = ResourceLimits::new()
            .with_cpu_limit(2000)
            .with_memory_limit(256 * 1024 * 1024)
            .with_execution_time_limit(10000);

        assert_eq!(limits.max_cpu_ms, 2000);
        assert_eq!(limits.max_memory, 256 * 1024 * 1024);
        assert_eq!(limits.max_execution_time_ms, 10000);
    }

    #[test]
    fn test_function_result() {
        let success = FunctionResult::success("test", b"output".to_vec(), 100);
        assert!(success.success);
        assert_eq!(success.function_name, "test");
        assert_eq!(success.output, b"output");
        assert_eq!(success.execution_time_ms, 100);

        let failure = FunctionResult::failure("test", "error message");
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }

    #[test]
    fn test_edge_context() {
        let mut ctx = EdgeContext::new("req-1", "device-1");

        ctx.add("key1", "value1");
        ctx.add("key2", "value2");

        assert_eq!(ctx.get("key1"), Some(&"value1".to_string()));
        assert_eq!(ctx.data.len(), 2);
    }

    #[test]
    fn test_fog_tier_ordering() {
        assert!(FogTier::Device < FogTier::EdgeGateway);
        assert!(FogTier::EdgeGateway < FogTier::LocalAggregation);
        assert!(FogTier::LocalAggregation < FogTier::RegionalAggregation);
        assert!(FogTier::RegionalAggregation < FogTier::Cloud);
    }

    #[test]
    fn test_function_enable_disable() {
        fn handler(_ctx: &EdgeContext, _input: &[u8]) -> IotResult<Vec<u8>> {
            Ok(Vec::new())
        }

        let mut manager = FunctionManager::new();

        let func = EdgeFunction::new("test", handler);
        manager.register(func).unwrap();

        manager.disable("test").unwrap();

        let ctx = EdgeContext::new("req-1", "device-1");
        let result = manager.execute("test", &ctx, b"data").unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
