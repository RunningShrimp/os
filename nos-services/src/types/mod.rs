//! Service types
//!
//! This module provides common types for services.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Service type constants
pub mod service_type {
    pub const UNKNOWN: u32 = 0;
    pub const FILE_SYSTEM: u32 = 1;
    pub const PROCESS: u32 = 2;
    pub const NETWORK: u32 = 3;
    pub const IPC: u32 = 4;
    pub const MEMORY: u32 = 5;
    pub const TIME: u32 = 6;
    pub const SECURITY: u32 = 7;
    pub const DEVICE: u32 = 8;
    pub const GRAPHICS: u32 = 9;
    pub const AUDIO: u32 = 10;
    pub const INPUT: u32 = 11;
}

/// Service priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServicePriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

impl Default for ServicePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Service dependency
#[derive(Debug, Clone)]
pub struct ServiceDependency {
    /// Service name
    pub name: String,
    /// Minimum version
    pub min_version: String,
    /// Maximum version
    pub max_version: Option<String>,
    /// Required flag
    pub required: bool,
}

/// Service endpoint
#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    /// Endpoint type
    pub endpoint_type: EndpointType,
    /// Endpoint address
    pub address: String,
    /// Endpoint port
    pub port: Option<u16>,
    /// Endpoint protocol
    pub protocol: Option<String>,
    /// Endpoint parameters
    pub parameters: BTreeMap<String, String>,
}

/// Endpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    /// Local endpoint
    Local,
    /// Network endpoint
    Network,
    /// IPC endpoint
    Ipc,
    /// Memory endpoint
    Memory,
}

/// Service event
#[derive(Debug, Clone)]
pub struct ServiceEvent {
    /// Event ID
    pub id: u64,
    /// Event type
    pub event_type: ServiceEventType,
    /// Service ID
    pub service_id: u32,
    /// Service name
    pub service_name: String,
    /// Event timestamp
    pub timestamp: u64,
    /// Event data
    pub data: Vec<u8>,
    /// Event metadata
    pub metadata: BTreeMap<String, String>,
}

/// Service event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceEventType {
    /// Service registered
    Registered,
    /// Service started
    Started,
    /// Service stopped
    Stopped,
    /// Service error
    Error,
    /// Service restarted
    Restarted,
    /// Service updated
    Updated,
    /// Service unregistered
    Unregistered,
}

/// Service health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceHealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded
    Degraded,
    /// Service is unhealthy
    Unhealthy,
    /// Service status unknown
    Unknown,
}

impl Default for ServiceHealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Service metrics
#[derive(Debug, Clone)]
pub struct ServiceMetrics {
    /// Service ID
    pub service_id: u32,
    /// Service name
    pub service_name: String,
    /// CPU usage (percentage)
    pub cpu_usage: f64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Number of requests
    pub request_count: u64,
    /// Number of errors
    pub error_count: u64,
    /// Average response time (microseconds)
    pub avg_response_time: u64,
    /// Uptime (seconds)
    pub uptime: u64,
    /// Last update time
    pub last_update: u64,
}

impl Default for ServiceMetrics {
    fn default() -> Self {
        Self {
            service_id: 0,
            service_name: String::new(),
            cpu_usage: 0.0,
            memory_usage: 0,
            request_count: 0,
            error_count: 0,
            avg_response_time: 0,
            uptime: 0,
            last_update: 0,
        }
    }
}

impl Default for ServiceDependency {
    fn default() -> Self {
        Self {
            name: String::new(),
            min_version: String::new(),
            max_version: None,
            required: false,
        }
    }
}

impl Default for ServiceEndpoint {
    fn default() -> Self {
        Self {
            endpoint_type: EndpointType::Local,
            address: String::new(),
            port: None,
            protocol: None,
            parameters: BTreeMap::new(),
        }
    }
}

impl Default for ServiceEvent {
    fn default() -> Self {
        Self {
            id: 0,
            event_type: ServiceEventType::Started,
            service_id: 0,
            service_name: String::new(),
            timestamp: 0,
            data: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_priority() {
        assert!(ServicePriority::Low < ServicePriority::Normal);
        assert!(ServicePriority::Normal < ServicePriority::High);
        assert!(ServicePriority::High < ServicePriority::Critical);
        
        assert_eq!(ServicePriority::default(), ServicePriority::Normal);
    }

    #[test]
    fn test_service_dependency() {
        let dependency = ServiceDependency {
            name: "test_service".to_string(),
            min_version: "1.0.0".to_string(),
            max_version: Some("2.0.0".to_string()),
            required: true,
        };
        
        assert_eq!(dependency.name, "test_service");
        assert_eq!(dependency.min_version, "1.0.0");
        assert_eq!(dependency.max_version, Some("2.0.0".to_string()));
        assert!(dependency.required);
    }

    #[test]
    fn test_service_metrics() {
        let metrics = ServiceMetrics::default();
        assert_eq!(metrics.service_id, 0);
        assert_eq!(metrics.service_name, "");
        assert_eq!(metrics.cpu_usage, 0.0);
        assert_eq!(metrics.memory_usage, 0);
        assert_eq!(metrics.request_count, 0);
        assert_eq!(metrics.error_count, 0);
        assert_eq!(metrics.avg_response_time, 0);
        assert_eq!(metrics.uptime, 0);
        assert_eq!(metrics.last_update, 0);
    }
}