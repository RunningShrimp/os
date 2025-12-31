//! Software-Defined Networking (SDN) and Advanced Networking
//!
//! This module provides comprehensive SDN capabilities and advanced networking
//! features for the NOS kernel, including:
//!
//! - **SDN Controller**: OpenFlow protocol implementation for centralized network control
//! - **VXLAN**: Virtual Extensible LAN for network virtualization
//! - **Load Balancing**: L4/L7 load balancing with multiple algorithms
//! - **Proxy**: Reverse/forward proxy with HTTP/HTTPS/WebSocket support
//! - **Tunneling**: WireGuard, IPsec, GRE, IPIP tunnel protocols
//! - **QoS**: Traffic classification, shaping, and quality of service
//!
//! ## Architecture
//!
//! The SDN subsystem is designed with the following layers:
//!
//! 1. **Control Plane**: SDN controller and network orchestration
//! 2. **Data Plane**: High-speed packet processing and forwarding
//! 3. **Management Plane**: Configuration, monitoring, and telemetry
//! 4. **Services**: Load balancing, proxying, tunneling, QoS
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::sdn::{SdnController, FlowRule, Action};
//!
//! // Create SDN controller
//! let controller = SdnController::new();
//!
//! // Add flow rule
//! let rule = FlowRule::new()
//!     .match_src_ip("192.168.1.0/24")
//!     .match_dst_port(80)
//!     .add_action(Action::ForwardTo(10));
//!
//! controller.install_flow_rule(rule)?;
//! ```

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

pub mod load_balancer;
pub mod proxy;
pub mod qos;
pub mod sdn;
pub mod tunnel;
pub mod vxlan;

/// Re-export common SDN types
pub use load_balancer::{
    Backend, BackendHealth, BackendStatus, LoadBalancer, LoadBalancerAlgorithm,
    LoadBalancerConfig, LoadBalancerError,
};
pub use proxy::{
    HttpProxy, ProxyBackend, ProxyConfig, ProxyError, ProxyType, ReverseProxy, Route,
    TransparentProxy,
};
pub use qos::{
    AqmAlgorithm, DscpMarking, PacketClassifier, QosConfig, QosError, QosManager,
    RateLimiter, ShapingAlgorithm, TrafficClass, TrafficShaper,
};
pub use sdn::{
    FlowAction, FlowEntry, FlowMatch, FlowModifier, FlowTable, NetworkVirtualization,
    OpenFlowController, OpenFlowError, OpenFlowMessage, OpenFlowVersion, Port, PortConfig,
    PortStatus, SdnController, SdnError, TopologyDiscovery, TopologyNode,
};
pub use tunnel::{
    GreTunnel, IpSecTunnel, IpipTunnel, Tunnel, TunnelConfig, TunnelEndpoint,
    TunnelEncryption, TunnelError, TunnelMonitor, TunnelStats, WireGuardTunnel,
};
pub use vxlan::{
    EvpnIntegration, Vni, VniManager, Vxlan, VxlanConfig, VxlanEncapsulation, VxlanError,
    VxlanNetwork, VxlanPacket,
};

/// SDN statistics tracking
#[derive(Debug, Default)]
pub struct SdnStats {
    /// Total packets processed
    pub packets_processed: AtomicU64,
    /// Total bytes processed
    pub bytes_processed: AtomicU64,
    /// Flow entries installed
    pub flow_entries_installed: AtomicU64,
    /// Flow entries removed
    pub flow_entries_removed: AtomicU64,
    /// Controller uptime in seconds
    pub uptime_seconds: AtomicU64,
}

impl SdnStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment packet counter
    pub fn inc_packets(&self) {
        self.packets_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment byte counter
    pub fn inc_bytes(&self, bytes: u64) {
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment flow entries installed
    pub fn inc_flow_entries_installed(&self) {
        self.flow_entries_installed.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment flow entries removed
    pub fn inc_flow_entries_removed(&self) {
        self.flow_entries_removed.fetch_add(1, Ordering::Relaxed);
    }

    /// Get all statistics
    pub fn get_all(&self) -> SdnStatsSnapshot {
        SdnStatsSnapshot {
            packets_processed: self.packets_processed.load(Ordering::Relaxed),
            bytes_processed: self.bytes_processed.load(Ordering::Relaxed),
            flow_entries_installed: self.flow_entries_installed.load(Ordering::Relaxed),
            flow_entries_removed: self.flow_entries_removed.load(Ordering::Relaxed),
            uptime_seconds: self.uptime_seconds.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of SDN statistics
#[derive(Debug, Clone, Copy)]
pub struct SdnStatsSnapshot {
    pub packets_processed: u64,
    pub bytes_processed: u64,
    pub flow_entries_installed: u64,
    pub flow_entries_removed: u64,
    pub uptime_seconds: u64,
}

/// SDN configuration
#[derive(Debug, Clone)]
pub struct SdnConfig {
    /// Maximum number of flow entries
    pub max_flow_entries: usize,
    /// Flow entry timeout in seconds
    pub flow_timeout: Duration,
    /// Enable statistics collection
    pub enable_stats: bool,
    /// Controller heartbeat interval
    pub heartbeat_interval: Duration,
    /// Maximum number of tunnels
    pub max_tunnels: usize,
    /// Maximum number of VXLAN networks
    pub max_vxlan_networks: usize,
    /// Maximum number of load balancers
    pub max_load_balancers: usize,
    /// Maximum number of proxy routes
    pub max_proxy_routes: usize,
    /// QoS update interval
    pub qos_update_interval: Duration,
}

impl Default for SdnConfig {
    fn default() -> Self {
        Self {
            max_flow_entries: 10000,
            flow_timeout: Duration::from_secs(60),
            enable_stats: true,
            heartbeat_interval: Duration::from_secs(5),
            max_tunnels: 1000,
            max_vxlan_networks: 256,
            max_load_balancers: 100,
            max_proxy_routes: 1000,
            qos_update_interval: Duration::from_secs(1),
        }
    }
}

/// Generic SDN error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdnError {
    /// Flow table full
    FlowTableFull,
    /// Invalid flow rule
    InvalidFlowRule,
    /// Controller not connected
    ControllerNotConnected,
    /// Timeout
    Timeout,
    /// Buffer overflow
    BufferOverflow,
    /// Invalid configuration
    InvalidConfig,
    /// Resource exhausted
    ResourceExhausted,
    /// Operation not supported
    NotSupported,
    /// Tunnel error
    TunnelError(String),
    /// VXLAN error
    VxlanError(String),
    /// Load balancer error
    LoadBalancerError(String),
    /// Proxy error
    ProxyError(String),
    /// QoS error
    QosError(String),
}

impl core::fmt::Display for SdnError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FlowTableFull => write!(f, "Flow table full"),
            Self::InvalidFlowRule => write!(f, "Invalid flow rule"),
            Self::ControllerNotConnected => write!(f, "Controller not connected"),
            Self::Timeout => write!(f, "Operation timeout"),
            Self::BufferOverflow => write!(f, "Buffer overflow"),
            Self::InvalidConfig => write!(f, "Invalid configuration"),
            Self::ResourceExhausted => write!(f, "Resource exhausted"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::TunnelError(msg) => write!(f, "Tunnel error: {}", msg),
            Self::VxlanError(msg) => write!(f, "VXLAN error: {}", msg),
            Self::LoadBalancerError(msg) => write!(f, "Load balancer error: {}", msg),
            Self::ProxyError(msg) => write!(f, "Proxy error: {}", msg),
            Self::QosError(msg) => write!(f, "QoS error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdn_stats() {
        let stats = SdnStats::new();

        stats.inc_packets();
        stats.inc_bytes(1500);
        stats.inc_flow_entries_installed();

        let snapshot = stats.get_all();
        assert_eq!(snapshot.packets_processed, 1);
        assert_eq!(snapshot.bytes_processed, 1500);
        assert_eq!(snapshot.flow_entries_installed, 1);
    }

    #[test]
    fn test_sdn_config_default() {
        let config = SdnConfig::default();
        assert_eq!(config.max_flow_entries, 10000);
        assert_eq!(config.max_tunnels, 1000);
    }

    #[test]
    fn test_sdn_error_display() {
        let err = SdnError::FlowTableFull;
        assert_eq!(format!("{}", err), "Flow table full");

        let err = SdnError::TunnelError("test".to_string());
        assert_eq!(format!("{}", err), "Tunnel error: test");
    }
}
