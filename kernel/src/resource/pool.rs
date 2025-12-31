//! Dynamic resource pools for flexible resource allocation
//!
//! This module implements dynamic resource pools that allow flexible allocation
//! and management of system resources. Pools can be created for different purposes
//! (e.g., real-time tasks, background jobs, specific applications).
//!
//! # Overview
//!
//! Resource pools provide:
//! - Dynamic allocation of CPU, memory, and I/O resources
//! - Resource reservation and guarantees
//! - Pool sharing and isolation
//! - Oversubscription management
//! - Dynamic resizing
//!
//! # Architecture
//!
//! ```text
//! ResourcePoolManager
//!     └── ResourcePool (named pools)
//!         ├── CPU resources (cores, bandwidth)
//!         ├── Memory resources (bytes, regions)
//!         ├── I/O resources (bandwidth, ops/sec)
//!         ├── Reserved resources
//          ├── Available resources
//!         └── Allocated resources
//! ```
//!
//! # Pool Types
//!
//! - **Guaranteed**: Resources are reserved and guaranteed
//! - **Best-effort**: Resources are shared based on availability
//! - **Burst**: Base allocation + ability to burst when spare capacity exists
//!
//! # Examples
//!
//! ```no_run
//! use kernel::resource::pool::{ResourcePool, PoolType, PoolConfig};
//!
//! let config = PoolConfig {
//!     name: "realtime".into(),
//!     pool_type: PoolType::Guaranteed,
//!     cpu_cores: Some(2),
//!     memory_bytes: Some(1024 * 1024 * 1024), // 1 GB
//!     ..Default::default()
//! };
//!
//! let pool = ResourcePool::new(config);
//! pool.allocate_cpu_cores(2)?;
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::Error;
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use core::time::Duration;

/// Default pool size for memory (1 GB)
const DEFAULT_MEMORY_POOL: u64 = 1024 * 1024 * 1024;

/// Default pool size for CPU cores
const DEFAULT_CPU_CORES: u64 = 2;

/// Default I/O bandwidth in bytes/sec
const DEFAULT_IO_BANDWIDTH: u64 = 100 * 1024 * 1024; // 100 MB/s

/// Maximum number of resource pools
const MAX_POOLS: usize = 256;

/// Maximum oversubscription ratio (percentage)
const MAX_OVERSUBSCRIPTION: u64 = 200; // 200%

/// Pool allocation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolType {
    /// Guaranteed resources - reserved and not oversubscribed
    Guaranteed,

    /// Best-effort resources - shared based on availability
    BestEffort,

    /// Burst resources - base allocation + burst capability
    Burst {
        /// Base allocation (guaranteed)
        base: u64,

        /// Maximum burst allocation
        max: u64,
    },
}

impl PoolType {
    /// Check if this pool type guarantees resources
    pub fn is_guaranteed(&self) -> bool {
        matches!(self, PoolType::Guaranteed)
    }

    /// Check if this pool type supports bursting
    pub fn supports_burst(&self) -> bool {
        matches!(self, PoolType::Burst { .. })
    }
}

/// CPU resource specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuResource {
    /// Number of CPU cores
    pub cores: u64,

    /// CPU bandwidth in microseconds per second
    pub bandwidth: u64,

    /// CPU set (specific cores)
    pub cpu_set: Option<u64>, // Bitmask for now, should be a set
}

impl Default for CpuResource {
    fn default() -> Self {
        Self {
            cores: 0,
            bandwidth: 0,
            cpu_set: None,
        }
    }
}

impl CpuResource {
    /// Create a new CPU resource
    pub fn new(cores: u64, bandwidth: u64) -> Self {
        Self {
            cores,
            bandwidth,
            cpu_set: None,
        }
    }

    /// Check if resource is zero
    pub fn is_zero(&self) -> bool {
        self.cores == 0 && self.bandwidth == 0
    }

    /// Add resources
    pub fn add(&mut self, other: CpuResource) {
        self.cores = self.cores.saturating_add(other.cores);
        self.bandwidth = self.bandwidth.saturating_add(other.bandwidth);
    }

    /// Subtract resources (saturating)
    pub fn sub(&mut self, other: CpuResource) {
        self.cores = self.cores.saturating_sub(other.cores);
        self.bandwidth = self.bandwidth.saturating_sub(other.bandwidth);
    }

    /// Check if sufficient resources available
    pub fn has_sufficient(&self, other: CpuResource) -> bool {
        self.cores >= other.cores && self.bandwidth >= other.bandwidth
    }
}

/// Memory resource specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryResource {
    /// Memory in bytes
    pub bytes: u64,

    /// Number of memory regions
    pub regions: u64,

    /// Huge pages count
    pub huge_pages: u64,
}

impl Default for MemoryResource {
    fn default() -> Self {
        Self {
            bytes: 0,
            regions: 0,
            huge_pages: 0,
        }
    }
}

impl MemoryResource {
    /// Create a new memory resource
    pub fn new(bytes: u64) -> Self {
        Self {
            bytes,
            regions: 0,
            huge_pages: 0,
        }
    }

    /// Check if resource is zero
    pub fn is_zero(&self) -> bool {
        self.bytes == 0
    }

    /// Add resources
    pub fn add(&mut self, other: MemoryResource) {
        self.bytes = self.bytes.saturating_add(other.bytes);
        self.regions = self.regions.saturating_add(other.regions);
        self.huge_pages = self.huge_pages.saturating_add(other.huge_pages);
    }

    /// Subtract resources (saturating)
    pub fn sub(&mut self, other: MemoryResource) {
        self.bytes = self.bytes.saturating_sub(other.bytes);
        self.regions = self.regions.saturating_sub(other.regions);
        self.huge_pages = self.huge_pages.saturating_sub(other.huge_pages);
    }

    /// Check if sufficient resources available
    pub fn has_sufficient(&self, other: MemoryResource) -> bool {
        self.bytes >= other.bytes
    }
}

/// I/O resource specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoResource {
    /// I/O bandwidth in bytes/sec
    pub bandwidth: u64,

    /// I/O operations per second
    pub iops: u64,

    /// Device-specific limits (device ID -> limit)
    pub device_limits: Option<u64>,
}

impl Default for IoResource {
    fn default() -> Self {
        Self {
            bandwidth: 0,
            iops: 0,
            device_limits: None,
        }
    }
}

impl IoResource {
    /// Create a new I/O resource
    pub fn new(bandwidth: u64, iops: u64) -> Self {
        Self {
            bandwidth,
            iops,
            device_limits: None,
        }
    }

    /// Check if resource is zero
    pub fn is_zero(&self) -> bool {
        self.bandwidth == 0 && self.iops == 0
    }

    /// Add resources
    pub fn add(&mut self, other: IoResource) {
        self.bandwidth = self.bandwidth.saturating_add(other.bandwidth);
        self.iops = self.iops.saturating_add(other.iops);
    }

    /// Subtract resources (saturating)
    pub fn sub(&mut self, other: IoResource) {
        self.bandwidth = self.bandwidth.saturating_sub(other.bandwidth);
        self.iops = self.iops.saturating_sub(other.iops);
    }

    /// Check if sufficient resources available
    pub fn has_sufficient(&self, other: IoResource) -> bool {
        self.bandwidth >= other.bandwidth && self.iops >= other.iops
    }
}

/// Pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Pool name
    pub name: String,

    /// Pool type
    pub pool_type: PoolType,

    /// CPU resources
    pub cpu_cores: Option<u64>,
    pub cpu_bandwidth: Option<u64>,

    /// Memory resources
    pub memory_bytes: Option<u64>,

    /// I/O resources
    pub io_bandwidth: Option<u64>,
    pub io_iops: Option<u64>,

    /// Oversubscription allowed (for best-effort pools)
    pub allow_oversubscription: bool,

    /// Maximum oversubscription ratio
    pub max_oversubscription: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            name: "pool".into(),
            pool_type: PoolType::BestEffort,
            cpu_cores: None,
            cpu_bandwidth: None,
            memory_bytes: None,
            io_bandwidth: None,
            io_iops: None,
            allow_oversubscription: false,
            max_oversubscription: 100,
        }
    }
}

/// Resource allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceAllocation {
    /// Allocated CPU
    pub cpu: CpuResource,

    /// Allocated memory
    pub memory: MemoryResource,

    /// Allocated I/O
    pub io: IoResource,
}

impl Default for ResourceAllocation {
    fn default() -> Self {
        Self {
            cpu: CpuResource::default(),
            memory: MemoryResource::default(),
            io: IoResource::default(),
        }
    }
}

impl ResourceAllocation {
    /// Check if allocation is empty
    pub fn is_empty(&self) -> bool {
        self.cpu.is_zero() && self.memory.is_zero() && self.io.is_zero()
    }
}

/// Pool statistics
#[derive(Debug, Clone, Copy)]
pub struct PoolStats {
    /// Pool name
    pub name: &'static str,

    /// Pool type
    pub pool_type: PoolType,

    /// CPU utilization percentage
    pub cpu_utilization: f64,

    /// Memory utilization percentage
    pub memory_utilization: f64,

    /// I/O utilization percentage
    pub io_utilization: f64,

    /// Number of allocations
    pub num_allocations: usize,

    /// Total allocated CPU
    pub allocated_cpu: CpuResource,

    /// Total allocated memory
    pub allocated_memory: MemoryResource,

    /// Total allocated I/O
    pub allocated_io: IoResource,

    /// Available CPU
    pub available_cpu: CpuResource,

    /// Available memory
    pub available_memory: MemoryResource,

    /// Available I/O
    pub available_io: IoResource,

    /// Oversubscription ratio (percentage)
    pub oversubscription_ratio: f64,
}

/// Resource pool
#[derive(Debug)]
pub struct ResourcePool {
    /// Pool name
    name: String,

    /// Pool type
    pool_type: PoolType,

    /// Total CPU resources
    total_cpu: CpuResource,

    /// Allocated CPU resources
    allocated_cpu: Arc<AtomicU64>, // Cores

    /// CPU bandwidth allocated
    allocated_cpu_bandwidth: Arc<AtomicU64>,

    /// Total memory resources
    total_memory: MemoryResource,

    /// Allocated memory
    allocated_memory: Arc<AtomicU64>,

    /// Total I/O resources
    total_io: IoResource,

    /// Allocated I/O bandwidth
    allocated_io_bandwidth: Arc<AtomicU64>,

    /// Allocated I/O ops
    allocated_io_iops: Arc<AtomicU64>,

    /// Oversubscription settings
    allow_oversubscription: bool,

    /// Maximum oversubscription ratio
    max_oversubscription: u64,

    /// Number of active allocations
    num_allocations: Arc<AtomicU64>,

    /// Pool enabled
    enabled: AtomicBool,

    /// Creation time
    created_at: Duration,
}

impl ResourcePool {
    /// Create a new resource pool
    pub fn new(config: PoolConfig) -> Self {
        let total_cpu = CpuResource::new(
            config.cpu_cores.unwrap_or(0),
            config.cpu_bandwidth.unwrap_or(0),
        );

        let total_memory = MemoryResource::new(config.memory_bytes.unwrap_or(0));

        let total_io = IoResource::new(
            config.io_bandwidth.unwrap_or(0),
            config.io_iops.unwrap_or(0),
        );

        Self {
            name: config.name,
            pool_type: config.pool_type,
            total_cpu,
            allocated_cpu: Arc::new(AtomicU64::new(0)),
            allocated_cpu_bandwidth: Arc::new(AtomicU64::new(0)),
            total_memory,
            allocated_memory: Arc::new(AtomicU64::new(0)),
            total_io,
            allocated_io_bandwidth: Arc::new(AtomicU64::new(0)),
            allocated_io_iops: Arc::new(AtomicU64::new(0)),
            allow_oversubscription: config.allow_oversubscription,
            max_oversubscription: config.max_oversubscription.min(MAX_OVERSUBSCRIPTION),
            num_allocations: Arc::new(AtomicU64::new(0)),
            enabled: AtomicBool::new(true),
            created_at: Duration::from_secs(0),
        }
    }

    /// Get pool name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get pool type
    pub fn pool_type(&self) -> PoolType {
        self.pool_type
    }

    /// Check if pool is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable or disable pool
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Get total CPU resources
    pub fn total_cpu(&self) -> CpuResource {
        self.total_cpu
    }

    /// Get allocated CPU resources
    pub fn allocated_cpu(&self) -> CpuResource {
        CpuResource::new(
            self.allocated_cpu.load(Ordering::Relaxed),
            self.allocated_cpu_bandwidth.load(Ordering::Relaxed),
        )
    }

    /// Get available CPU resources
    pub fn available_cpu(&self) -> CpuResource {
        let allocated = self.allocated_cpu();
        CpuResource::new(
            self.total_cpu.cores.saturating_sub(allocated.cores),
            self.total_cpu.bandwidth.saturating_sub(allocated.bandwidth),
        )
    }

    /// Allocate CPU cores
    pub fn allocate_cpu_cores(&self, cores: u64) -> Result<(), Error> {
        if !self.is_enabled() {
            return Err(Error::ResourceUnavailable);
        }

        let current = self.allocated_cpu.load(Ordering::Relaxed);
        let max_allowed = if self.allow_oversubscription && !self.pool_type.is_guaranteed() {
            (self.total_cpu.cores * self.max_oversubscription) / 100
        } else {
            self.total_cpu.cores
        };

        if current.saturating_add(cores) > max_allowed {
            return Err(Error::InsufficientResources {
                resource: "CPU cores".into(),
                requested: cores,
                available: max_allowed.saturating_sub(current),
            });
        }

        self.allocated_cpu.fetch_add(cores, Ordering::Relaxed);
        self.num_allocations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Release CPU cores
    pub fn release_cpu_cores(&self, cores: u64) {
        self.allocated_cpu.fetch_sub(cores, Ordering::Relaxed);
    }

    /// Get total memory resources
    pub fn total_memory(&self) -> MemoryResource {
        self.total_memory
    }

    /// Get allocated memory
    pub fn allocated_memory(&self) -> MemoryResource {
        MemoryResource::new(self.allocated_memory.load(Ordering::Relaxed))
    }

    /// Get available memory
    pub fn available_memory(&self) -> MemoryResource {
        let allocated = self.allocated_memory();
        MemoryResource::new(self.total_memory.bytes.saturating_sub(allocated.bytes))
    }

    /// Allocate memory
    pub fn allocate_memory(&self, bytes: u64) -> Result<(), Error> {
        if !self.is_enabled() {
            return Err(Error::ResourceUnavailable);
        }

        let current = self.allocated_memory.load(Ordering::Relaxed);
        let max_allowed = if self.allow_oversubscription && !self.pool_type.is_guaranteed() {
            (self.total_memory.bytes * self.max_oversubscription) / 100
        } else {
            self.total_memory.bytes
        };

        if current.saturating_add(bytes) > max_allowed {
            return Err(Error::InsufficientResources {
                resource: "memory".into(),
                requested: bytes,
                available: max_allowed.saturating_sub(current),
            });
        }

        self.allocated_memory.fetch_add(bytes, Ordering::Relaxed);
        self.num_allocations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Release memory
    pub fn release_memory(&self, bytes: u64) {
        self.allocated_memory.fetch_sub(bytes, Ordering::Relaxed);
    }

    /// Get total I/O resources
    pub fn total_io(&self) -> IoResource {
        self.total_io
    }

    /// Get allocated I/O
    pub fn allocated_io(&self) -> IoResource {
        IoResource::new(
            self.allocated_io_bandwidth.load(Ordering::Relaxed),
            self.allocated_io_iops.load(Ordering::Relaxed),
        )
    }

    /// Get available I/O
    pub fn available_io(&self) -> IoResource {
        let allocated = self.allocated_io();
        IoResource::new(
            self.total_io.bandwidth.saturating_sub(allocated.bandwidth),
            self.total_io.iops.saturating_sub(allocated.iops),
        )
    }

    /// Allocate I/O bandwidth
    pub fn allocate_io_bandwidth(&self, bandwidth: u64) -> Result<(), Error> {
        if !self.is_enabled() {
            return Err(Error::ResourceUnavailable);
        }

        let current = self.allocated_io_bandwidth.load(Ordering::Relaxed);
        let max_allowed = if self.allow_oversubscription && !self.pool_type.is_guaranteed() {
            (self.total_io.bandwidth * self.max_oversubscription) / 100
        } else {
            self.total_io.bandwidth
        };

        if current.saturating_add(bandwidth) > max_allowed {
            return Err(Error::InsufficientResources {
                resource: "I/O bandwidth".into(),
                requested: bandwidth,
                available: max_allowed.saturating_sub(current),
            });
        }

        self.allocated_io_bandwidth.fetch_add(bandwidth, Ordering::Relaxed);
        self.num_allocations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Release I/O bandwidth
    pub fn release_io_bandwidth(&self, bandwidth: u64) {
        self.allocated_io_bandwidth.fetch_sub(bandwidth, Ordering::Relaxed);
    }

    /// Allocate multiple resources
    pub fn allocate(&self, allocation: ResourceAllocation) -> Result<(), Error> {
        if !allocation.cpu.is_zero() {
            self.allocate_cpu_cores(allocation.cpu.cores)?;
        }
        if !allocation.memory.is_zero() {
            self.allocate_memory(allocation.memory.bytes)?;
        }
        if !allocation.io.is_zero() {
            self.allocate_io_bandwidth(allocation.io.bandwidth)?;
        }
        Ok(())
    }

    /// Release multiple resources
    pub fn release(&self, allocation: ResourceAllocation) {
        if !allocation.cpu.is_zero() {
            self.release_cpu_cores(allocation.cpu.cores);
        }
        if !allocation.memory.is_zero() {
            self.release_memory(allocation.memory.bytes);
        }
        if !allocation.io.is_zero() {
            self.release_io_bandwidth(allocation.io.bandwidth);
        }
    }

    /// Get number of allocations
    pub fn num_allocations(&self) -> u64 {
        self.num_allocations.load(Ordering::Relaxed)
    }

    /// Resize pool
    pub fn resize(&mut self, cpu_cores: u64, memory_bytes: u64, io_bandwidth: u64) {
        self.total_cpu.cores = cpu_cores;
        self.total_memory.bytes = memory_bytes;
        self.total_io.bandwidth = io_bandwidth;
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let allocated_cpu = self.allocated_cpu();
        let available_cpu = self.available_cpu();

        let allocated_memory = self.allocated_memory();
        let available_memory = self.available_memory();

        let allocated_io = self.allocated_io();
        let available_io = self.available_io();

        let cpu_util = if self.total_cpu.cores > 0 {
            (allocated_cpu.cores as f64 / self.total_cpu.cores as f64) * 100.0
        } else {
            0.0
        };

        let mem_util = if self.total_memory.bytes > 0 {
            (allocated_memory.bytes as f64 / self.total_memory.bytes as f64) * 100.0
        } else {
            0.0
        };

        let io_util = if self.total_io.bandwidth > 0 {
            (allocated_io.bandwidth as f64 / self.total_io.bandwidth as f64) * 100.0
        } else {
            0.0
        };

        PoolStats {
            name: unsafe { core::mem::transmute::<&str, &'static str>(self.name.as_str()) },
            pool_type: self.pool_type,
            cpu_utilization: cpu_util,
            memory_utilization: mem_util,
            io_utilization: io_util,
            num_allocations: self.num_allocations() as usize,
            allocated_cpu,
            allocated_memory,
            allocated_io,
            available_cpu,
            available_memory,
            available_io,
            oversubscription_ratio: self.max_oversubscription as f64,
        }
    }

    /// Check if pool can satisfy allocation
    pub fn can_satisfy(&self, allocation: ResourceAllocation) -> bool {
        let available_cpu = self.available_cpu();
        let available_memory = self.available_memory();
        let available_io = self.available_io();

        available_cpu.has_sufficient(allocation.cpu)
            && available_memory.has_sufficient(allocation.memory)
            && available_io.has_sufficient(allocation.io)
    }
}

/// Resource pool manager
pub struct ResourcePoolManager {
    /// Named pools
    pools: SpinLock<BTreeMap<String, Arc<ResourcePool>>>,

    /// Default pool
    default_pool: Arc<ResourcePool>,
}

impl ResourcePoolManager {
    /// Create a new pool manager
    pub fn new() -> Self {
        let default_config = PoolConfig {
            name: "default".into(),
            pool_type: PoolType::BestEffort,
            cpu_cores: Some(DEFAULT_CPU_CORES),
            memory_bytes: Some(DEFAULT_MEMORY_POOL),
            io_bandwidth: Some(DEFAULT_IO_BANDWIDTH),
            allow_oversubscription: true,
            ..Default::default()
        };

        let default_pool = Arc::new(ResourcePool::new(default_config));

        let mut pools = BTreeMap::new();
        pools.insert("default".into(), default_pool.clone());

        Self {
            pools: SpinLock::new(pools),
            default_pool,
        }
    }

    /// Create a new pool
    pub fn create_pool(&self, config: PoolConfig) -> Result<Arc<ResourcePool>, Error> {
        let mut pools = self.pools.lock();

        if pools.len() >= MAX_POOLS {
            return Err(Error::ResourceLimitExceeded {
                resource: "pools".into(),
                usage: MAX_POOLS as u64,
                limit: MAX_POOLS as u64,
            });
        }

        if pools.contains_key(&config.name) {
            return Err(Error::Other(format!("Pool '{}' already exists", config.name)));
        }

        let name = config.name.clone();
        let pool = Arc::new(ResourcePool::new(config));
        pools.insert(name, pool.clone());
        Ok(pool)
    }

    /// Get a pool by name
    pub fn get_pool(&self, name: &str) -> Option<Arc<ResourcePool>> {
        self.pools.lock().get(name).cloned()
    }

    /// Get the default pool
    pub fn default_pool(&self) -> &Arc<ResourcePool> {
        &self.default_pool
    }

    /// Delete a pool
    pub fn delete_pool(&self, name: &str) -> Result<(), Error> {
        let mut pools = self.pools.lock();

        if name == "default" {
            return Err(Error::PermissionDenied);
        }

        let pool = pools
            .get(name)
            .ok_or_else(|| Error::Other(format!("Pool '{}' not found", name)))?;

        if pool.num_allocations() > 0 {
            return Err(Error::Other("Pool has active allocations".to_string()));
        }

        pools.remove(name);
        Ok(())
    }

    /// List all pools
    pub fn list_pools(&self) -> Vec<String> {
        self.pools.lock().keys().cloned().collect()
    }

    /// Get statistics for all pools
    pub fn get_all_stats(&self) -> Vec<PoolStats> {
        self.pools
            .lock()
            .values()
            .map(|pool| pool.stats())
            .collect()
    }

    /// Find pool with most available resources
    pub fn find_best_pool(&self, allocation: ResourceAllocation) -> Option<Arc<ResourcePool>> {
        self.pools
            .lock()
            .values()
            .filter(|pool| pool.is_enabled() && pool.can_satisfy(allocation))
            .min_by_key(|pool| {
                let available = pool.available_cpu().cores;
                core::cmp::Reverse(available) // Prefer pool with most available
            })
            .cloned()
    }

    /// Get total resources across all pools
    pub fn total_resources(&self) -> (CpuResource, MemoryResource, IoResource) {
        let pools = self.pools.lock();

        let mut total_cpu = CpuResource::default();
        let mut total_memory = MemoryResource::default();
        let mut total_io = IoResource::default();

        for pool in pools.values() {
            total_cpu.add(pool.total_cpu());
            total_memory.add(pool.total_memory());
            total_io.add(pool.total_io());
        }

        (total_cpu, total_memory, total_io)
    }

    /// Get total allocated resources across all pools
    pub fn total_allocated(&self) -> (CpuResource, MemoryResource, IoResource) {
        let pools = self.pools.lock();

        let mut total_cpu = CpuResource::default();
        let mut total_memory = MemoryResource::default();
        let mut total_io = IoResource::default();

        for pool in pools.values() {
            total_cpu.add(pool.allocated_cpu());
            total_memory.add(pool.allocated_memory());
            total_io.add(pool.allocated_io());
        }

        (total_cpu, total_memory, total_io)
    }

    /// Get total available resources across all pools
    pub fn total_available(&self) -> (CpuResource, MemoryResource, IoResource) {
        let pools = self.pools.lock();

        let mut total_cpu = CpuResource::default();
        let mut total_memory = MemoryResource::default();
        let mut total_io = IoResource::default();

        for pool in pools.values() {
            total_cpu.add(pool.available_cpu());
            total_memory.add(pool.available_memory());
            total_io.add(pool.available_io());
        }

        (total_cpu, total_memory, total_io)
    }
}

impl Default for ResourcePoolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let config = PoolConfig {
            name: "test".into(),
            pool_type: PoolType::Guaranteed,
            cpu_cores: Some(4),
            memory_bytes: Some(1024 * 1024 * 1024),
            ..Default::default()
        };

        let pool = ResourcePool::new(config);
        assert_eq!(pool.name(), "test");
        assert_eq!(pool.total_cpu().cores, 4);
        assert_eq!(pool.total_memory().bytes, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_cpu_allocation() {
        let pool = ResourcePool::new(PoolConfig {
            name: "test".into(),
            pool_type: PoolType::Guaranteed,
            cpu_cores: Some(4),
            ..Default::default()
        });

        assert!(pool.allocate_cpu_cores(2).is_ok());
        assert_eq!(pool.allocated_cpu().cores, 2);

        assert!(pool.allocate_cpu_cores(3).is_err());
        assert_eq!(pool.allocated_cpu().cores, 2);

        pool.release_cpu_cores(1);
        assert_eq!(pool.allocated_cpu().cores, 1);
    }

    #[test]
    fn test_memory_allocation() {
        let pool = ResourcePool::new(PoolConfig {
            name: "test".into(),
            pool_type: PoolType::Guaranteed,
            memory_bytes: Some(1024),
            ..Default::default()
        });

        assert!(pool.allocate_memory(512).is_ok());
        assert_eq!(pool.allocated_memory().bytes, 512);

        assert!(pool.allocate_memory(600).is_err());

        pool.release_memory(200);
        assert_eq!(pool.allocated_memory().bytes, 312);
    }

    #[test]
    fn test_oversubscription() {
        let pool = ResourcePool::new(PoolConfig {
            name: "test".into(),
            pool_type: PoolType::BestEffort,
            cpu_cores: Some(4),
            allow_oversubscription: true,
            max_oversubscription: 150,
            ..Default::default()
        });

        // Can allocate up to 150% = 6 cores
        assert!(pool.allocate_cpu_cores(5).is_ok());
        assert!(pool.allocate_cpu_cores(2).is_err());
    }
}
