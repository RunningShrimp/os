//! Per-process resource limits (RLIMIT)
//!
//! This module implements POSIX-style resource limits for processes, following
//! the Linux `getrlimit`/`setrlimit` interface with enhanced tracking and enforcement.
//!
//! # Overview
//!
//! Resource limits provide per-process constraints on system resources to prevent
//! individual processes from consuming excessive resources. Each limit has two values:
//! - **Soft limit**: The current enforced limit, can be raised by the process up to the hard limit
//! - **Hard limit**: The maximum value the soft limit can be raised to, can only be
//!   lowered by unprivileged processes
//!
//! # Supported Limits
//!
//! - `RLIMIT_CPU`: Maximum CPU time in seconds
//! - `RLIMIT_FSIZE`: Maximum file size in bytes
//! - `RLIMIT_DATA`: Maximum data segment size
//! - `RLIMIT_STACK`: Maximum stack size
//! - `RLIMIT_CORE`: Maximum core file size
//! - `RLIMIT_RSS`: Maximum resident set size
//! - `RLIMIT_NPROC`: Maximum number of processes
//! - `RLIMIT_NOFILE`: Maximum number of open file descriptors
//! - `RLIMIT_MEMLOCK`: Maximum locked-in-memory address space
//! - `RLIMIT_AS`: Maximum address space size
//! - `RLIMIT_LOCKS`: Maximum number of file locks
//! - `RLIMIT_SIGPENDING`: Maximum number of pending signals
//! - `RLIMIT_MSGQUEUE`: Maximum bytes in POSIX message queues
//! - `RLIMIT_NICE`: Maximum nice priority
//! - `RLIMIT_RTPRIO`: Maximum real-time priority
//! - `RLIMIT_RTTIME`: Maximum CPU time for real-time tasks
//!
//! # Examples
//!
//! ```no_run
//! use kernel::resource::limit::{ResourceLimit, LimitValue, ResourceType};
//!
//! // Create a new resource limit
//! let limit = ResourceLimit::new(
//!     ResourceType::RlimitNofile,
//!     LimitValue {
//!         soft: 1024,
//!         hard: 4096,
//!     }
//! );
//!
//! // Check if a value is within the limit
//! assert!(limit.check(100));
//! assert!(!limit.check(5000));
//! ```
//!
//! # Architecture
//!
//! ```text
//! ResourceLimit (per limit type)
//!     ├── soft: Current enforced limit
//!     ├── hard: Maximum allowed limit
//!     └── enforcement: Limit checking and action
//!
//! LimitsTable (per process)
//!     ├── Array of ResourceLimit entries
//!     ├── Inheritance from parent
//!     └── Privilege-based modification
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

use alloc::string::ToString;
use crate::error::Error;
use crate::process::ProcessId;
use crate::sync::SpinLock;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// Maximum number of concurrent resource limits
const MAX_LIMITS: usize = 16;

/// Default soft limit value
const DEFAULT_SOFT_LIMIT: u64 = 10 * 1024 * 1024; // 10 MB

/// Default hard limit value
const DEFAULT_HARD_LIMIT: u64 = u64::MAX;

/// Maximum CPU time in seconds (infinity)
const RLIM_INFINITY: u64 = u64::MAX;

/// Maximum file size in bytes
const MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024 * 1024; // 1 TB

/// Maximum number of file descriptors
const MAX_NOFILE: u64 = 1024 * 1024;

/// Maximum number of processes
const MAX_NPROC: u64 = 4096;

/// Maximum stack size in bytes
const MAX_STACK_SIZE: u64 = 8 * 1024 * 1024; // 8 MB

/// Maximum locked memory in bytes
const MAX_MEMLOCK: u64 = 64 * 1024 * 1024; // 64 MB

/// Types of resource limits following POSIX standard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ResourceType {
    /// CPU time limit in seconds
    RlimitCpu = 0,

    /// Maximum file size in bytes
    RlimitFsize = 1,

    /// Maximum size of data segment in bytes
    RlimitData = 2,

    /// Maximum size of stack in bytes
    RlimitStack = 3,

    /// Maximum core file size in bytes
    RlimitCore = 4,

    /// Maximum resident set size in bytes
    RlimitRss = 5,

    /// Maximum number of processes
    RlimitNproc = 6,

    /// Maximum number of open file descriptors
    RlimitNofile = 7,

    /// Maximum locked-in-memory address space
    RlimitMemlock = 8,

    /// Maximum address space size in bytes
    RlimitAs = 9,

    /// Maximum number of file locks
    RlimitLocks = 10,

    /// Maximum number of pending signals
    RlimitSigpending = 11,

    /// Maximum bytes in POSIX message queues
    RlimitMsgqueue = 12,

    /// Maximum nice priority
    RlimitNice = 13,

    /// Maximum real-time priority
    RlimitRtprio = 14,

    /// Maximum CPU time for real-time tasks in microseconds
    RlimitRttime = 15,
}

impl ResourceType {
    /// Get all resource limit types
    pub fn all() -> &'static [ResourceType] {
        &[
            ResourceType::RlimitCpu,
            ResourceType::RlimitFsize,
            ResourceType::RlimitData,
            ResourceType::RlimitStack,
            ResourceType::RlimitCore,
            ResourceType::RlimitRss,
            ResourceType::RlimitNproc,
            ResourceType::RlimitNofile,
            ResourceType::RlimitMemlock,
            ResourceType::RlimitAs,
            ResourceType::RlimitLocks,
            ResourceType::RlimitSigpending,
            ResourceType::RlimitMsgqueue,
            ResourceType::RlimitNice,
            ResourceType::RlimitRtprio,
            ResourceType::RlimitRttime,
        ]
    }

    /// Get the default soft limit for this resource type
    pub fn default_soft_limit(&self) -> u64 {
        match self {
            ResourceType::RlimitCpu => RLIM_INFINITY,
            ResourceType::RlimitFsize => MAX_FILE_SIZE,
            ResourceType::RlimitData => RLIM_INFINITY,
            ResourceType::RlimitStack => MAX_STACK_SIZE,
            ResourceType::RlimitCore => 0, // No core dumps by default
            ResourceType::RlimitRss => RLIM_INFINITY,
            ResourceType::RlimitNproc => MAX_NPROC,
            ResourceType::RlimitNofile => 1024,
            ResourceType::RlimitMemlock => MAX_MEMLOCK,
            ResourceType::RlimitAs => RLIM_INFINITY,
            ResourceType::RlimitLocks => RLIM_INFINITY,
            ResourceType::RlimitSigpending => 1024,
            ResourceType::RlimitMsgqueue => 8 * 1024 * 1024, // 8 MB
            ResourceType::RlimitNice => 0,
            ResourceType::RlimitRtprio => 0,
            ResourceType::RlimitRttime => RLIM_INFINITY,
        }
    }

    /// Get the default hard limit for this resource type
    pub fn default_hard_limit(&self) -> u64 {
        self.default_soft_limit().max(DEFAULT_HARD_LIMIT)
    }

    /// Get the name of this resource type
    pub fn name(&self) -> &'static str {
        match self {
            ResourceType::RlimitCpu => "CPU",
            ResourceType::RlimitFsize => "FSIZE",
            ResourceType::RlimitData => "DATA",
            ResourceType::RlimitStack => "STACK",
            ResourceType::RlimitCore => "CORE",
            ResourceType::RlimitRss => "RSS",
            ResourceType::RlimitNproc => "NPROC",
            ResourceType::RlimitNofile => "NOFILE",
            ResourceType::RlimitMemlock => "MEMLOCK",
            ResourceType::RlimitAs => "AS",
            ResourceType::RlimitLocks => "LOCKS",
            ResourceType::RlimitSigpending => "SIGPENDING",
            ResourceType::RlimitMsgqueue => "MSGQUEUE",
            ResourceType::RlimitNice => "NICE",
            ResourceType::RlimitRtprio => "RTPRIO",
            ResourceType::RlimitRttime => "RTTIME",
        }
    }

    /// Get the unit for this resource type
    pub fn unit(&self) -> &'static str {
        match self {
            ResourceType::RlimitCpu => "seconds",
            ResourceType::RlimitFsize => "bytes",
            ResourceType::RlimitData => "bytes",
            ResourceType::RlimitStack => "bytes",
            ResourceType::RlimitCore => "bytes",
            ResourceType::RlimitRss => "bytes",
            ResourceType::RlimitNproc => "processes",
            ResourceType::RlimitNofile => "fds",
            ResourceType::RlimitMemlock => "bytes",
            ResourceType::RlimitAs => "bytes",
            ResourceType::RlimitLocks => "locks",
            ResourceType::RlimitSigpending => "signals",
            ResourceType::RlimitMsgqueue => "bytes",
            ResourceType::RlimitNice => "priority",
            ResourceType::RlimitRtprio => "priority",
            ResourceType::RlimitRttime => "microseconds",
        }
    }
}

/// Resource limit value with soft and hard limits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitValue {
    /// Soft limit: Current enforced limit
    pub soft: u64,

    /// Hard limit: Maximum allowed limit
    pub hard: u64,
}

impl LimitValue {
    /// Create a new limit value
    pub fn new(soft: u64, hard: u64) -> Self {
        debug_assert!(hard >= soft, "Hard limit must be >= soft limit");
        Self { soft, hard }
    }

    /// Create a limit value with infinity
    pub fn infinity() -> Self {
        Self {
            soft: RLIM_INFINITY,
            hard: RLIM_INFINITY,
        }
    }

    /// Check if the soft limit is infinity
    pub fn is_soft_infinity(&self) -> bool {
        self.soft == RLIM_INFINITY
    }

    /// Check if the hard limit is infinity
    pub fn is_hard_infinity(&self) -> bool {
        self.hard == RLIM_INFINITY
    }

    /// Check if both limits are infinity
    pub fn is_infinity(&self) -> bool {
        self.is_soft_infinity() && self.is_hard_infinity()
    }

    /// Clamp a value to the soft limit
    pub fn clamp_soft(&self, value: u64) -> u64 {
        if self.is_soft_infinity() {
            value
        } else {
            value.min(self.soft)
        }
    }

    /// Clamp a value to the hard limit
    pub fn clamp_hard(&self, value: u64) -> u64 {
        if self.is_hard_infinity() {
            value
        } else {
            value.min(self.hard)
        }
    }
}

/// Single resource limit with enforcement
#[derive(Debug, Clone)]
pub struct ResourceLimit {
    /// Type of resource limit
    resource_type: ResourceType,

    /// Soft and hard limits
    value: LimitValue,

    /// Current usage
    usage: Arc<AtomicU64>,

    /// Whether this limit is enforced
    enforced: bool,
}

impl ResourceLimit {
    /// Create a new resource limit
    pub fn new(resource_type: ResourceType, value: LimitValue) -> Self {
        Self {
            resource_type,
            value,
            usage: Arc::new(AtomicU64::new(0)),
            enforced: true,
        }
    }

    /// Create a default resource limit
    pub fn default_for(resource_type: ResourceType) -> Self {
        Self::new(
            resource_type,
            LimitValue::new(
                resource_type.default_soft_limit(),
                resource_type.default_hard_limit(),
            ),
        )
    }

    /// Get the resource type
    pub fn resource_type(&self) -> ResourceType {
        self.resource_type
    }

    /// Get the limit value
    pub fn value(&self) -> LimitValue {
        self.value
    }

    /// Set the limit value
    ///
    /// Returns error if the new hard limit is less than the current soft limit,
    /// or if the new soft limit exceeds the hard limit.
    pub fn set_value(&mut self, new_value: LimitValue) -> Result<(), Error> {
        if new_value.hard < new_value.soft {
            return Err(Error::Other(
                "Hard limit must be >= soft limit".to_string(),
            ));
        }
        self.value = new_value;
        Ok(())
    }

    /// Get the current usage
    pub fn usage(&self) -> u64 {
        self.usage.load(Ordering::Relaxed)
    }

    /// Set the current usage
    pub fn set_usage(&self, usage: u64) {
        self.usage.store(usage, Ordering::Relaxed);
    }

    /// Add to the current usage
    pub fn add_usage(&self, amount: u64) -> Result<(), Error> {
        let current = self.usage.load(Ordering::Relaxed);
        let new_usage = current.saturating_add(amount);

        if self.enforced && !self.value.is_soft_infinity() && new_usage > self.value.soft {
            return Err(Error::ResourceLimitExceeded {
                resource: self.resource_type.name().into(),
                usage: new_usage,
                limit: self.value.soft,
            });
        }

        self.usage.store(new_usage, Ordering::Relaxed);
        Ok(())
    }

    /// Subtract from the current usage
    pub fn sub_usage(&self, amount: u64) {
        let current = self.usage.load(Ordering::Relaxed);
        let new_usage = current.saturating_sub(amount);
        self.usage.store(new_usage, Ordering::Relaxed);
    }

    /// Check if a value is within the limit
    pub fn check(&self, value: u64) -> bool {
        if !self.enforced {
            return true;
        }
        if self.value.is_soft_infinity() {
            return true;
        }
        value <= self.value.soft
    }

    /// Check if a value is within the hard limit
    pub fn check_hard(&self, value: u64) -> bool {
        if self.value.is_hard_infinity() {
            return true;
        }
        value <= self.value.hard
    }

    /// Get the percentage of limit used
    pub fn usage_percent(&self) -> f64 {
        if self.value.is_soft_infinity() {
            0.0
        } else {
            (self.usage() as f64 / self.value.soft as f64) * 100.0
        }
    }

    /// Check if the limit is exceeded
    pub fn is_exceeded(&self) -> bool {
        !self.check(self.usage())
    }

    /// Enable or disable enforcement
    pub fn set_enforced(&mut self, enforced: bool) {
        self.enforced = enforced;
    }

    /// Check if enforcement is enabled
    pub fn is_enforced(&self) -> bool {
        self.enforced
    }

    /// Reset usage to zero
    pub fn reset_usage(&self) {
        self.usage.store(0, Ordering::Relaxed);
    }

    /// Get statistics for this limit
    pub fn stats(&self) -> LimitStats {
        LimitStats {
            resource_type: self.resource_type,
            soft_limit: self.value.soft,
            hard_limit: self.value.hard,
            current_usage: self.usage(),
            usage_percent: self.usage_percent(),
            enforced: self.enforced,
            is_infinity: self.value.is_infinity(),
            is_exceeded: self.is_exceeded(),
        }
    }
}

/// Statistics for a resource limit
#[derive(Debug, Clone, Copy)]
pub struct LimitStats {
    /// Type of resource
    pub resource_type: ResourceType,

    /// Soft limit
    pub soft_limit: u64,

    /// Hard limit
    pub hard_limit: u64,

    /// Current usage
    pub current_usage: u64,

    /// Percentage of limit used
    pub usage_percent: f64,

    /// Whether enforcement is enabled
    pub enforced: bool,

    /// Whether limit is infinity
    pub is_infinity: bool,

    /// Whether limit is exceeded
    pub is_exceeded: bool,
}

/// Table of resource limits for a process
#[derive(Clone)]
pub struct LimitsTable {
    /// Map of resource type to limit
    limits: BTreeMap<ResourceType, Arc<SpinLock<ResourceLimit>>>,

    /// Process ID for these limits
    process_id: ProcessId,
}

impl core::fmt::Debug for LimitsTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LimitsTable")
            .field("process_id", &self.process_id)
            .field("limits_count", &self.limits.len())
            .finish()
    }
}

impl LimitsTable {
    /// Create a new limits table with default values
    pub fn new(process_id: ProcessId) -> Self {
        let mut limits = BTreeMap::new();

        for &resource_type in ResourceType::all() {
            limits.insert(
                resource_type,
                Arc::new(SpinLock::new(ResourceLimit::default_for(resource_type))),
            );
        }

        Self { limits, process_id }
    }

    /// Create a limits table inherited from parent
    pub fn inherit_from(parent: &LimitsTable, child_process_id: ProcessId) -> Self {
        let mut limits = BTreeMap::new();

        for (&resource_type, parent_limit) in &parent.limits {
            let parent_value = parent_limit.lock().value();
            limits.insert(
                resource_type,
                Arc::new(SpinLock::new(ResourceLimit::new(resource_type, parent_value))),
            );
        }

        Self {
            limits,
            process_id: child_process_id,
        }
    }

    /// Get the process ID
    pub fn process_id(&self) -> ProcessId {
        self.process_id
    }

    /// Get a specific resource limit
    pub fn get(&self, resource_type: ResourceType) -> Option<Arc<SpinLock<ResourceLimit>>> {
        self.limits.get(&resource_type).cloned()
    }

    /// Get the value of a specific resource limit
    pub fn get_value(&self, resource_type: ResourceType) -> Option<LimitValue> {
        self.limits
            .get(&resource_type)
            .map(|limit| limit.lock().value())
    }

    /// Set a resource limit
    ///
    /// # Arguments
    ///
    /// * `resource_type` - Type of resource to set
    /// * `new_value` - New limit value
    /// * `privileged` - Whether the caller has CAP_SYS_RESOURCE capability
    ///
    /// # Returns
    ///
    /// Returns error if:
    /// - New soft limit exceeds hard limit
    /// - Unprivileged process tries to raise hard limit
    /// - Resource type is invalid
    pub fn set(
        &self,
        resource_type: ResourceType,
        new_value: LimitValue,
        privileged: bool,
    ) -> Result<(), Error> {
        let limit = self
            .limits
            .get(&resource_type)
            .ok_or_else(|| Error::Other("Unknown resource type".to_string()))?;

        let mut limit_guard = limit.lock();
        let current_value = limit_guard.value();

        // Check hard limit changes for unprivileged processes
        if !privileged && new_value.hard != current_value.hard {
            return Err(Error::PermissionDenied);
        }

        // Validate the new limits
        if new_value.soft > new_value.hard {
            return Err(Error::Other(
                "Soft limit cannot exceed hard limit".to_string(),
            ));
        }

        limit_guard.set_value(new_value)?;
        Ok(())
    }

    /// Check if a resource value is within the limit
    pub fn check(&self, resource_type: ResourceType, value: u64) -> bool {
        self.limits
            .get(&resource_type)
            .map(|limit| limit.lock().check(value))
            .unwrap_or(true)
    }

    /// Check if a resource value is within the hard limit
    pub fn check_hard(&self, resource_type: ResourceType, value: u64) -> bool {
        self.limits
            .get(&resource_type)
            .map(|limit| limit.lock().check_hard(value))
            .unwrap_or(true)
    }

    /// Add usage to a resource limit
    pub fn add_usage(&self, resource_type: ResourceType, amount: u64) -> Result<(), Error> {
        self.limits
            .get(&resource_type)
            .ok_or_else(|| Error::Other("Unknown resource type".to_string()))?
            .lock()
            .add_usage(amount)
    }

    /// Subtract usage from a resource limit
    pub fn sub_usage(&self, resource_type: ResourceType, amount: u64) {
        if let Some(limit) = self.limits.get(&resource_type) {
            limit.lock().sub_usage(amount);
        }
    }

    /// Get the current usage of a resource
    pub fn get_usage(&self, resource_type: ResourceType) -> u64 {
        self.limits
            .get(&resource_type)
            .map(|limit| limit.lock().usage())
            .unwrap_or(0)
    }

    /// Get all limits
    pub fn get_all(&self) -> BTreeMap<ResourceType, LimitValue> {
        self.limits
            .iter()
            .map(|(&rtype, limit)| (rtype, limit.lock().value()))
            .collect()
    }

    /// Get statistics for all limits
    pub fn get_all_stats(&self) -> Vec<LimitStats> {
        self.limits
            .iter()
            .map(|(_, limit)| limit.lock().stats())
            .collect()
    }

    /// Get statistics for a specific limit
    pub fn get_stats(&self, resource_type: ResourceType) -> Option<LimitStats> {
        self.limits
            .get(&resource_type)
            .map(|limit| limit.lock().stats())
    }

    /// Reset all usage counters
    pub fn reset_all_usage(&self) {
        for limit in self.limits.values() {
            limit.lock().reset_usage();
        }
    }

    /// Check if any limit is exceeded
    pub fn any_exceeded(&self) -> bool {
        self.limits.values().any(|limit| limit.lock().is_exceeded())
    }

    /// Get all exceeded limits
    pub fn get_exceeded(&self) -> Vec<ResourceType> {
        self.limits
            .iter()
            .filter(|(_, limit)| limit.lock().is_exceeded())
            .map(|(&rtype, _)| rtype)
            .collect()
    }

    /// Clone limits for a new process
    pub fn clone_for_process(&self, new_process_id: ProcessId) -> Self {
        Self::inherit_from(self, new_process_id)
    }
}

impl Default for LimitsTable {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Global resource limit manager
pub struct LimitManager {
    /// Per-process limits tables
    process_limits: SpinLock<BTreeMap<ProcessId, Arc<LimitsTable>>>,

    /// Next process ID to assign
    next_pid: AtomicU64,
}

impl LimitManager {
    /// Create a new limit manager
    pub fn new() -> Self {
        Self {
            process_limits: SpinLock::new(BTreeMap::new()),
            next_pid: AtomicU64::new(1),
        }
    }

    /// Create limits for a new process
    pub fn create_limits(&self, process_id: ProcessId) -> Arc<LimitsTable> {
        let limits = Arc::new(LimitsTable::new(process_id));
        self.process_limits.lock().insert(process_id, limits.clone());
        limits
    }

    /// Create limits inherited from parent process
    pub fn create_limits_inherited(
        &self,
        child_pid: ProcessId,
        parent_pid: ProcessId,
    ) -> Result<Arc<LimitsTable>, Error> {
        let parent_limits = self
            .process_limits
            .lock()
            .get(&parent_pid)
            .cloned()
            .ok_or_else(|| Error::Other("Parent limits not found".to_string()))?;

        let child_limits = Arc::new(LimitsTable::inherit_from(&parent_limits, child_pid));
        self.process_limits.lock().insert(child_pid, child_limits.clone());
        Ok(child_limits)
    }

    /// Get limits for a process
    pub fn get_limits(&self, process_id: ProcessId) -> Option<Arc<LimitsTable>> {
        self.process_limits.lock().get(&process_id).cloned()
    }

    /// Remove limits for a process
    pub fn remove_limits(&self, process_id: ProcessId) {
        self.process_limits.lock().remove(&process_id);
    }

    /// Get all process IDs with limits
    pub fn get_all_process_ids(&self) -> Vec<ProcessId> {
        self.process_limits.lock().keys().copied().collect()
    }

    /// Get statistics for all processes
    pub fn get_all_stats(&self) -> Vec<(ProcessId, Vec<LimitStats>)> {
        self.process_limits
            .lock()
            .iter()
            .map(|(&pid, limits)| (pid, limits.get_all_stats()))
            .collect()
    }

    /// Get all exceeded limits across all processes
    pub fn get_all_exceeded(&self) -> Vec<(ProcessId, Vec<ResourceType>)> {
        self.process_limits
            .lock()
            .iter()
            .filter_map(|(&pid, limits)| {
                let exceeded = limits.get_exceeded();
                if !exceeded.is_empty() {
                    Some((pid, exceeded))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get total usage of a specific resource type across all processes
    pub fn get_total_usage(&self, resource_type: ResourceType) -> u64 {
        self.process_limits
            .lock()
            .values()
            .map(|limits| limits.get_usage(resource_type))
            .sum()
    }

    /// Find processes exceeding their limits
    pub fn find_exceeding_processes(&self) -> Vec<(ProcessId, ResourceType, u64, u64)> {
        let mut results = Vec::new();

        for (&pid, limits) in self.process_limits.lock().iter() {
            for &rtype in ResourceType::all() {
                let usage = limits.get_usage(rtype);
                if let Some(limit) = limits.get_value(rtype) {
                    if !limit.is_soft_infinity() && usage > limit.soft {
                        results.push((pid, rtype, usage, limit.soft));
                    }
                }
            }
        }

        results
    }
}

impl Default for LimitManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to format resource limits for display
pub fn format_limit_value(value: u64, resource_type: ResourceType) -> alloc::string::String {
    use alloc::format;

    if value == RLIM_INFINITY {
        return "unlimited".into();
    }

    match resource_type {
        ResourceType::RlimitCpu => {
            if value >= 3600 {
                format!("{}h {}m", value / 3600, (value % 3600) / 60)
            } else {
                format!("{}s", value)
            }
        }
        ResourceType::RlimitFsize
        | ResourceType::RlimitData
        | ResourceType::RlimitStack
        | ResourceType::RlimitCore
        | ResourceType::RlimitRss
        | ResourceType::RlimitMemlock
        | ResourceType::RlimitAs
        | ResourceType::RlimitMsgqueue => {
            if value >= 1024 * 1024 * 1024 {
                format!("{} GB", value / (1024 * 1024 * 1024))
            } else if value >= 1024 * 1024 {
                format!("{} MB", value / (1024 * 1024))
            } else if value >= 1024 {
                format!("{} KB", value / 1024)
            } else {
                format!("{} bytes", value)
            }
        }
        _ => format!("{}", value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_value() {
        let value = LimitValue::new(1000, 2000);
        assert_eq!(value.soft, 1000);
        assert_eq!(value.hard, 2000);
        assert!(!value.is_soft_infinity());
        assert!(!value.is_hard_infinity());

        let infinity = LimitValue::infinity();
        assert!(infinity.is_infinity());

        assert_eq!(value.clamp_soft(1500), 1000);
        assert_eq!(value.clamp_hard(1500), 1500);
    }

    #[test]
    fn test_resource_limit() {
        let limit = ResourceLimit::new(ResourceType::RlimitNofile, LimitValue::new(100, 200));

        assert!(limit.check(50));
        assert!(limit.check(100));
        assert!(!limit.check(150));

        assert!(limit.add_usage(50).is_ok());
        assert_eq!(limit.usage(), 50);

        assert!(limit.add_usage(60).is_err());
        assert_eq!(limit.usage(), 50); // Usage unchanged on error

        limit.sub_usage(30);
        assert_eq!(limit.usage(), 20);

        assert!(limit.add_usage(100).is_ok());
    }

    #[test]
    fn test_limits_table() {
        let table = LimitsTable::new(ProcessId::new(1));

        let nofile = table.get_value(ResourceType::RlimitNofile).unwrap();
        assert_eq!(nofile.soft, 1024);

        let result = table.set(
            ResourceType::RlimitNofile,
            LimitValue::new(2048, 4096),
            true,
        );
        assert!(result.is_ok());

        let new_nofile = table.get_value(ResourceType::RlimitNofile).unwrap();
        assert_eq!(new_nofile.soft, 2048);
        assert_eq!(new_nofile.hard, 4096);

        // Test unprivileged hard limit change
        let result = table.set(
            ResourceType::RlimitNofile,
            LimitValue::new(2048, 8192),
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_limit_inheritance() {
        let parent_table = LimitsTable::new(ProcessId::new(1));
        parent_table
            .set(
                ResourceType::RlimitNofile,
                LimitValue::new(500, 1000),
                true,
            )
            .unwrap();

        let child_table = LimitsTable::inherit_from(&parent_table, ProcessId::new(2));

        let child_nofile = child_table.get_value(ResourceType::RlimitNofile).unwrap();
        assert_eq!(child_nofile.soft, 500);
        assert_eq!(child_nofile.hard, 1000);

        // Child and parent are independent
        child_table
            .set(
                ResourceType::RlimitNofile,
                LimitValue::new(600, 1000),
                true,
            )
            .unwrap();

        let parent_nofile = parent_table.get_value(ResourceType::RlimitNofile).unwrap();
        assert_eq!(parent_nofile.soft, 500);
    }
}
