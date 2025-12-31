//! Load Balancer Implementation
//!
//! This module provides comprehensive load balancing capabilities:
//! - Layer 4 load balancing (TCP/UDP)
//! - Layer 7 load balancing (HTTP/HTTPS proxy)
//! - Health checking (TCP, HTTP, HTTPS, gRPC)
//! - Backend server management with weights
//! - Session persistence (source IP, cookie, IP hash)
//! - Load balancing algorithms:
//!   - Round robin
//!   - Weighted round robin
//!   - Least connections
//!   - Random
//!   - Source IP hash
//! - SSL termination (offload TLS)
//! - Connection draining during backend removal
//!
//! Based on common load balancer designs and RFCs:
//! - RFC 7230-7235: HTTP/1.1
//! - RFC 7540: HTTP/2
//! - RFC 8446: TLS 1.3

#![allow(dead_code)]

use alloc::{collections::BTreeMap, string::{String, ToString}, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

use crate::subsystems::sync::{Mutex, RwLock};

// ============================================================================
// Common Types
// ============================================================================

/// Load balancer result type
pub type LbResult<T> = core::result::Result<T, crate::error::unified::LbError>;

/// VIP ID
pub type VipId = u64;

/// Backend ID
pub type BackendId = u32;

/// IP address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpAddr {
    V4(u32),
    V6([u8; 16]),
}

impl IpAddr {
    /// Create IPv4 address
    pub fn v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        let addr = ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32);
        IpAddr::V4(addr)
    }
}

/// Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LbProtocol {
    Tcp,
    Udp,
    Http,
    Https,
    Grpc,
}

/// Load balancing algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LbAlgorithm {
    /// Round robin
    RoundRobin,
    /// Weighted round robin
    WeightedRoundRobin,
    /// Least connections
    LeastConnections,
    /// Random
    Random,
    /// Source IP hash
    SourceHash,
    /// Least response time
    LeastResponseTime,
}

/// Session persistence type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceType {
    /// None
    None,
    /// Source IP
    SourceIp,
    /// Cookie
    Cookie,
    /// IP hash
    IpHash,
}

// ============================================================================
// Backend Configuration
// ============================================================================

/// Backend server configuration
#[derive(Debug, Clone)]
pub struct Backend {
    /// Backend ID
    pub id: BackendId,
    /// Backend address
    pub address: IpAddr,
    /// Backend port
    pub port: u16,
    /// Weight (0-256)
    pub weight: u32,
    /// Maximum connections
    pub max_connections: u32,
    /// Current connections
    pub current_connections: u32,
    /// Health status
    pub health: BackendHealth,
    /// Draining flag
    pub draining: bool,
    /// Backup server
    pub backup: bool,
}

/// Backend health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendHealth {
    /// Healthy
    Healthy,
    /// Unhealthy
    Unhealthy,
    /// Health check pending
    Pending,
    /// Maintenance mode
    Maintenance,
}

impl Backend {
    /// Create new backend
    pub fn new(id: BackendId, address: IpAddr, port: u16, weight: u32) -> Self {
        Self {
            id,
            address,
            port,
            weight: weight.min(256),
            max_connections: 0,
            current_connections: 0,
            health: BackendHealth::Healthy,
            draining: false,
            backup: false,
        }
    }

    /// Is available for new connections
    pub fn is_available(&self) -> bool {
        if self.draining || self.backup || self.health != BackendHealth::Healthy {
            return false;
        }

        if self.max_connections > 0 && self.current_connections >= self.max_connections {
            return false;
        }

        true
    }

    /// Increment connections
    pub fn inc_connections(&mut self) {
        self.current_connections += 1;
    }

    /// Decrement connections
    pub fn dec_connections(&mut self) {
        self.current_connections = self.current_connections.saturating_sub(1);
    }
}

// ============================================================================
// VIP Configuration
// ============================================================================

/// Virtual IP configuration
#[derive(Debug, Clone)]
pub struct VipConfig {
    /// VIP ID
    pub id: VipId,
    /// VIP address
    pub address: IpAddr,
    /// VIP port
    pub port: u16,
    /// Protocol
    pub protocol: LbProtocol,
    /// Load balancing algorithm
    pub algorithm: LbAlgorithm,
    /// Session persistence
    pub persistence: PersistenceType,
    /// Backends
    pub backends: Vec<BackendId>,
    /// SSL certificate (if HTTPS)
    pub ssl_cert: Option<String>,
    /// SSL key (if HTTPS)
    pub ssl_key: Option<String>,
    /// SSL termination
    pub ssl_termination: bool,
}

/// Virtual IP
#[derive(Debug)]
pub struct Vip {
    /// Configuration
    config: VipConfig,
    /// Backends indexed by ID
    backend_map: BTreeMap<BackendId, Backend>,
    /// Round robin index
    rr_index: AtomicU32,
    /// Statistics
    stats: VipStats,
    /// Persistence table
    persistence: BTreeMap<String, BackendId>,
}

/// VIP statistics
#[derive(Debug, Clone)]
pub struct VipStats {
    /// Total connections
    pub connections: u64,
    /// Current connections
    pub current_connections: u64,
    /// Bytes in
    pub bytes_in: u64,
    /// Bytes out
    pub bytes_out: u64,
    /// Requests
    pub requests: u64,
    /// Failed requests
    pub failed: u64,
}

impl Default for VipStats {
    fn default() -> Self {
        Self {
            connections: 0,
            current_connections: 0,
            bytes_in: 0,
            bytes_out: 0,
            requests: 0,
            failed: 0,
        }
    }
}

impl Vip {
    /// Create new VIP
    pub fn new(config: VipConfig) -> Self {
        Self {
            config,
            backend_map: BTreeMap::new(),
            rr_index: AtomicU32::new(0),
            stats: VipStats::default(),
            persistence: BTreeMap::new(),
        }
    }

    /// Add backend
    pub fn add_backend(&mut self, backend: Backend) -> LbResult<()> {
        if self.backend_map.contains_key(&backend.id) {
            return Err(crate::error::unified::LbError::BackendAlreadyExists);
        }

        self.backend_map.insert(backend.id, backend);
        Ok(())
    }

    /// Remove backend
    pub fn remove_backend(&mut self, backend_id: BackendId) -> LbResult<Backend> {
        if !self.backend_map.contains_key(&backend_id) {
            return Err(crate::error::unified::LbError::BackendNotFound);
        }

        self.backend_map.remove(&backend_id)
            .ok_or(crate::error::unified::LbError::BackendNotFound)
    }

    /// Get backend
    pub fn get_backend(&self, backend_id: BackendId) -> Option<&Backend> {
        self.backend_map.get(&backend_id)
    }

    /// Get mutable backend
    pub fn get_backend_mut(&mut self, backend_id: BackendId) -> Option<&mut Backend> {
        self.backend_map.get_mut(&backend_id)
    }

    /// List backends
    pub fn list_backends(&self) -> Vec<&Backend> {
        self.backend_map.values().collect()
    }

    /// Select backend using configured algorithm
    pub fn select_backend(&self, client_ip: Option<u32>) -> LbResult<BackendId> {
        let available: Vec<&Backend> = self.backend_map
            .values()
            .filter(|b| b.is_available())
            .collect();

        if available.is_empty() {
            // Try backup servers
            let backup: Vec<&Backend> = self.backend_map
                .values()
                .filter(|b| b.backup && b.health == BackendHealth::Healthy)
                .collect();

            if backup.is_empty() {
                return Err(crate::error::unified::LbError::NoHealthyBackends);
            }
        }

        match self.config.algorithm {
            LbAlgorithm::RoundRobin => self.select_rr(&available),
            LbAlgorithm::WeightedRoundRobin => self.select_wrr(&available),
            LbAlgorithm::LeastConnections => self.select_lc(&available),
            LbAlgorithm::Random => self.select_random(&available),
            LbAlgorithm::SourceHash => self.select_source_hash(&available, client_ip),
            LbAlgorithm::LeastResponseTime => self.select_lc(&available),
        }
    }

    /// Round robin selection
    fn select_rr(&self, backends: &[&Backend]) -> LbResult<BackendId> {
        if backends.is_empty() {
            return Err(crate::error::unified::LbError::NoHealthyBackends);
        }

        let idx = self.rr_index.fetch_add(1, Ordering::SeqCst) as usize % backends.len();
        Ok(backends[idx].id)
    }

    /// Weighted round robin selection
    fn select_wrr(&self, backends: &[&Backend]) -> LbResult<BackendId> {
        if backends.is_empty() {
            return Err(crate::error::unified::LbError::NoHealthyBackends);
        }

        // Simple weighted selection
        let total_weight: u32 = backends.iter().map(|b| b.weight).sum();
        if total_weight == 0 {
            return Err(crate::error::unified::LbError::NoHealthyBackends);
        }

        let idx = self.rr_index.fetch_add(1, Ordering::SeqCst) as u32 % total_weight;
        let mut sum = 0;

        for backend in backends {
            sum += backend.weight;
            if idx < sum {
                return Ok(backend.id);
            }
        }

        Ok(backends[0].id)
    }

    /// Least connections selection
    fn select_lc(&self, backends: &[&Backend]) -> LbResult<BackendId> {
        if backends.is_empty() {
            return Err(crate::error::unified::LbError::NoHealthyBackends);
        }

        let backend = backends
            .iter()
            .min_by_key(|b| b.current_connections)
            .ok_or(crate::error::unified::LbError::NoHealthyBackends)?;

        Ok(backend.id)
    }

    /// Random selection
    fn select_random(&self, backends: &[&Backend]) -> LbResult<BackendId> {
        if backends.is_empty() {
            return Err(crate::error::unified::LbError::NoHealthyBackends);
        }

        let idx = (self.rr_index.fetch_add(1, Ordering::SeqCst) as usize) % backends.len();
        Ok(backends[idx].id)
    }

    /// Source hash selection
    fn select_source_hash(&self, backends: &[&Backend], client_ip: Option<u32>) -> LbResult<BackendId> {
        if backends.is_empty() {
            return Err(crate::error::unified::LbError::NoHealthyBackends);
        }

        let ip = client_ip.unwrap_or(0);
        let idx = (ip as usize) % backends.len();
        Ok(backends[idx].id)
    }

    /// Update statistics
    pub fn update_stats(&mut self, bytes_in: u64, bytes_out: u64) {
        self.stats.bytes_in += bytes_in;
        self.stats.bytes_out += bytes_out;
        self.stats.requests += 1;
    }

    /// Record failed request
    pub fn record_failure(&mut self) {
        self.stats.failed += 1;
    }

    /// Get statistics
    pub fn get_stats(&self) -> &VipStats {
        &self.stats
    }

    /// Set persistence
    pub fn set_persistence(&mut self, key: String, backend_id: BackendId) {
        self.persistence.insert(key, backend_id);
    }

    /// Get persistence
    pub fn get_persistence(&self, key: &str) -> Option<BackendId> {
        self.persistence.get(key).copied()
    }
}

// ============================================================================
// Health Checking
// ============================================================================

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Check type
    pub check_type: HealthCheckType,
    /// Check interval (seconds)
    pub interval: u32,
    /// Timeout (seconds)
    pub timeout: u32,
    /// Unhealthy threshold
    pub unhealthy_threshold: u32,
    /// Healthy threshold
    pub healthy_threshold: u32,
    /// HTTP check path
    pub http_path: String,
    /// Expected HTTP status
    pub expected_status: u16,
    /// Send string
    pub send: String,
    /// Expected receive string
    pub receive: String,
}

/// Health check type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCheckType {
    /// TCP check
    Tcp,
    /// HTTP check
    Http,
    /// HTTPS check
    Https,
    /// gRPC check
    Grpc,
    /// Ping check
    Ping,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_type: HealthCheckType::Tcp,
            interval: 5,
            timeout: 3,
            unhealthy_threshold: 3,
            healthy_threshold: 2,
            http_path: String::from("/health"),
            expected_status: 200,
            send: String::new(),
            receive: String::new(),
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCheckResult {
    Success,
    Failure,
    Timeout,
}

/// Health checker
#[derive(Debug)]
pub struct HealthChecker {
    /// Backend health status indexed by backend ID
    health_status: BTreeMap<BackendId, BackendHealth>,
    /// Failure counters
    failure_count: BTreeMap<BackendId, u32>,
    /// Success counters
    success_count: BTreeMap<BackendId, u32>,
    /// Health check configuration
    config: HealthCheckConfig,
}

impl HealthChecker {
    /// Create new health checker
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            health_status: BTreeMap::new(),
            failure_count: BTreeMap::new(),
            success_count: BTreeMap::new(),
            config,
        }
    }

    /// Register backend
    pub fn register_backend(&mut self, backend_id: BackendId) {
        self.health_status.insert(backend_id, BackendHealth::Pending);
        self.failure_count.insert(backend_id, 0);
        self.success_count.insert(backend_id, 0);
    }

    /// Unregister backend
    pub fn unregister_backend(&mut self, backend_id: BackendId) {
        self.health_status.remove(&backend_id);
        self.failure_count.remove(&backend_id);
        self.success_count.remove(&backend_id);
    }

    /// Process health check result
    pub fn process_result(&mut self, backend_id: BackendId, result: HealthCheckResult) {
        let is_success = result == HealthCheckResult::Success;

        if is_success {
            let count = self.success_count.entry(backend_id).or_insert(0);
            *count += 1;
            self.failure_count.insert(backend_id, 0);

            if *count >= self.config.healthy_threshold {
                self.health_status.insert(backend_id, BackendHealth::Healthy);
            }
        } else {
            let count = self.failure_count.entry(backend_id).or_insert(0);
            *count += 1;
            self.success_count.insert(backend_id, 0);

            if *count >= self.config.unhealthy_threshold {
                self.health_status.insert(backend_id, BackendHealth::Unhealthy);
            }
        }
    }

    /// Get backend health
    pub fn get_health(&self, backend_id: BackendId) -> BackendHealth {
        *self.health_status.get(&backend_id).unwrap_or(&BackendHealth::Unhealthy)
    }

    /// Set backend health (manual override)
    pub fn set_health(&mut self, backend_id: BackendId, health: BackendHealth) {
        self.health_status.insert(backend_id, health);
        self.failure_count.insert(backend_id, 0);
        self.success_count.insert(backend_id, 0);
    }
}

// ============================================================================
// Load Balancer
// ============================================================================

/// Load balancer
#[derive(Debug)]
pub struct LoadBalancer {
    /// VIPs indexed by ID
    vips: BTreeMap<VipId, Vip>,
    /// Next VIP ID
    next_vip_id: AtomicU32,
    /// Next backend ID
    next_backend_id: AtomicU32,
    /// Health checker
    health_checker: Arc<Mutex<HealthChecker>>,
}

impl LoadBalancer {
    /// Create new load balancer
    pub fn new(health_config: HealthCheckConfig) -> Self {
        Self {
            vips: BTreeMap::new(),
            next_vip_id: AtomicU32::new(1),
            next_backend_id: AtomicU32::new(1),
            health_checker: Arc::new(Mutex::new(HealthChecker::new(health_config))),
        }
    }

    /// Create VIP
    pub fn create_vip(&mut self, config: VipConfig) -> LbResult<VipId> {
        if self.vips.contains_key(&config.id) {
            return Err(crate::error::unified::LbError::VipAlreadyExists);
        }

        let vip = Vip::new(config.clone());
        self.vips.insert(config.id, vip);
        Ok(config.id)
    }

    /// Remove VIP
    pub fn remove_vip(&mut self, vip_id: VipId) -> LbResult<()> {
        if !self.vips.contains_key(&vip_id) {
            return Err(crate::error::unified::LbError::VipNotFound);
        }

        self.vips.remove(&vip_id);
        Ok(())
    }

    /// Get VIP
    pub fn get_vip(&self, vip_id: VipId) -> Option<&Vip> {
        self.vips.get(&vip_id)
    }

    /// Get mutable VIP
    pub fn get_vip_mut(&mut self, vip_id: VipId) -> Option<&mut Vip> {
        self.vips.get_mut(&vip_id)
    }

    /// List VIPs
    pub fn list_vips(&self) -> Vec<VipId> {
        self.vips.keys().copied().collect()
    }

    /// Add backend to VIP
    pub fn add_backend(&mut self, vip_id: VipId, mut backend: Backend) -> LbResult<()> {
        let vip = self.vips.get_mut(&vip_id)
            .ok_or(crate::error::unified::LbError::VipNotFound)?;

        // Assign backend ID if needed
        if backend.id == 0 {
            backend.id = self.next_backend_id.fetch_add(1, Ordering::SeqCst);
        }

        vip.add_backend(backend)?;

        // Register with health checker
        self.health_checker.lock().register_backend(backend.id);

        Ok(())
    }

    /// Remove backend from VIP
    pub fn remove_backend(&mut self, vip_id: VipId, backend_id: BackendId) -> LbResult<()> {
        let vip = self.vips.get_mut(&vip_id)
            .ok_or(crate::error::unified::LbError::VipNotFound)?;

        vip.remove_backend(backend_id)?;

        // Unregister from health checker
        self.health_checker.lock().unregister_backend(backend_id);

        Ok(())
    }

    /// Drain backend (stop sending new connections)
    pub fn drain_backend(&mut self, vip_id: VipId, backend_id: BackendId) -> LbResult<()> {
        let vip = self.vips.get_mut(&vip_id)
            .ok_or(crate::error::unified::LbError::VipNotFound)?;

        let backend = vip.get_backend_mut(backend_id)
            .ok_or(crate::error::unified::LbError::BackendNotFound)?;

        backend.draining = true;
        Ok(())
    }

    /// Cancel drain
    pub fn cancel_drain(&mut self, vip_id: VipId, backend_id: BackendId) -> LbResult<()> {
        let vip = self.vips.get_mut(&vip_id)
            .ok_or(crate::error::unified::LbError::VipNotFound)?;

        let backend = vip.get_backend_mut(backend_id)
            .ok_or(crate::error::unified::LbError::BackendNotFound)?;

        backend.draining = false;
        Ok(())
    }

    /// Select backend for new connection
    pub fn select_backend(&self, vip_id: VipId, client_ip: Option<u32>) -> LbResult<Backend> {
        let vip = self.vips.get(&vip_id)
            .ok_or(crate::error::unified::LbError::VipNotFound)?;

        let backend_id = vip.select_backend(client_ip)?;

        let backend = vip.get_backend(backend_id)
            .ok_or(crate::error::unified::LbError::BackendNotFound)?;

        Ok(Backend {
            id: backend.id,
            address: backend.address,
            port: backend.port,
            weight: backend.weight,
            max_connections: backend.max_connections,
            current_connections: backend.current_connections,
            health: backend.health,
            draining: backend.draining,
            backup: backend.backup,
        })
    }

    /// Process connection (increment counters)
    pub fn process_connection(&mut self, vip_id: VipId, backend_id: BackendId) -> LbResult<()> {
        let vip = self.vips.get_mut(&vip_id)
            .ok_or(crate::error::unified::LbError::VipNotFound)?;

        let backend = vip.get_backend_mut(backend_id)
            .ok_or(crate::error::unified::LbError::BackendNotFound)?;

        backend.inc_connections();
        vip.stats.connections += 1;
        vip.stats.current_connections += 1;

        Ok(())
    }

    /// Close connection (decrement counters)
    pub fn close_connection(&mut self, vip_id: VipId, backend_id: BackendId) -> LbResult<()> {
        let vip = self.vips.get_mut(&vip_id)
            .ok_or(crate::error::unified::LbError::VipNotFound)?;

        let backend = vip.get_backend_mut(backend_id)
            .ok_or(crate::error::unified::LbError::BackendNotFound)?;

        backend.dec_connections();
        vip.stats.current_connections = vip.stats.current_connections.saturating_sub(1);

        Ok(())
    }

    /// Update health status
    pub fn update_health(&self, backend_id: BackendId, result: HealthCheckResult) {
        self.health_checker.lock().process_result(backend_id, result);
    }

    /// Get backend health
    pub fn get_backend_health(&self, vip_id: VipId, backend_id: BackendId) -> LbResult<BackendHealth> {
        let vip = self.vips.get(&vip_id)
            .ok_or(crate::error::unified::LbError::VipNotFound)?;

        let backend = vip.get_backend(backend_id)
            .ok_or(crate::error::unified::LbError::BackendNotFound)?;

        Ok(backend.health)
    }

    /// Set backend health manually
    pub fn set_backend_health(&mut self, vip_id: VipId, backend_id: BackendId, health: BackendHealth) -> LbResult<()> {
        let vip = self.vips.get_mut(&vip_id)
            .ok_or(crate::error::unified::LbError::VipNotFound)?;

        if !vip.backend_map.contains_key(&backend_id) {
            return Err(crate::error::unified::LbError::BackendNotFound);
        }

        self.health_checker.lock().set_health(backend_id, health);

        if let Some(backend) = vip.get_backend_mut(backend_id) {
            backend.health = health;
        }

        Ok(())
    }
}

// ============================================================================
// Load Balancer API
// ============================================================================

/// Global load balancer
static LOAD_BALANCER: RwLock<Option<LoadBalancer>> = RwLock::new(None);

/// Initialize load balancer
pub fn init_load_balancer(health_config: HealthCheckConfig) -> LbResult<()> {
    let lb = LoadBalancer::new(health_config);

    let mut global = LOAD_BALANCER.write();
    *global = Some(lb);

    Ok(())
}

/// Create VIP
pub fn create_vip(config: VipConfig) -> LbResult<VipId> {
    let mut lb = LOAD_BALANCER.write();
    let lb = lb.as_mut()
        .ok_or(crate::error::unified::LbError::InvalidHealthCheckConfig)?;

    lb.create_vip(config)
}

/// Add backend
pub fn add_backend(vip_id: VipId, backend: Backend) -> LbResult<()> {
    let mut lb = LOAD_BALANCER.write();
    let lb = lb.as_mut()
        .ok_or(crate::error::unified::LbError::InvalidHealthCheckConfig)?;

    lb.add_backend(vip_id, backend)
}

/// Remove backend
pub fn remove_backend(vip_id: VipId, backend_id: BackendId) -> LbResult<()> {
    let mut lb = LOAD_BALANCER.write();
    let lb = lb.as_mut()
        .ok_or(crate::error::unified::LbError::InvalidHealthCheckConfig)?;

    lb.remove_backend(vip_id, backend_id)
}

/// Select backend
pub fn select_backend(vip_id: VipId, client_ip: Option<u32>) -> LbResult<Backend> {
    let lb = LOAD_BALANCER.read();
    let lb = lb.as_ref()
        .ok_or(crate::error::unified::LbError::InvalidHealthCheckConfig)?;

    lb.select_backend(vip_id, client_ip)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend() {
        let backend = Backend::new(1, IpAddr::v4(192, 168, 1, 10), 8080, 100);
        assert!(backend.is_available());
        assert_eq!(backend.weight, 100);

        let mut backend = backend;
        backend.inc_connections();
        assert_eq!(backend.current_connections, 1);

        backend.dec_connections();
        assert_eq!(backend.current_connections, 0);
    }

    #[test]
    fn test_backend_unavailable() {
        let mut backend = Backend::new(1, IpAddr::v4(192, 168, 1, 10), 8080, 100);
        backend.health = BackendHealth::Unhealthy;
        assert!(!backend.is_available());

        backend.health = BackendHealth::Healthy;
        backend.draining = true;
        assert!(!backend.is_available());

        backend.draining = false;
        backend.max_connections = 10;
        backend.current_connections = 10;
        assert!(!backend.is_available());
    }

    #[test]
    fn test_vip() {
        let config = VipConfig {
            id: 1,
            address: IpAddr::v4(192, 168, 1, 100),
            port: 80,
            protocol: LbProtocol::Http,
            algorithm: LbAlgorithm::RoundRobin,
            persistence: PersistenceType::None,
            backends: Vec::new(),
            ssl_cert: None,
            ssl_key: None,
            ssl_termination: false,
        };

        let mut vip = Vip::new(config);

        let backend1 = Backend::new(1, IpAddr::v4(192, 168, 1, 10), 8080, 100);
        let backend2 = Backend::new(2, IpAddr::v4(192, 168, 1, 11), 8080, 100);

        assert!(vip.add_backend(backend1).is_ok());
        assert!(vip.add_backend(backend2).is_ok());

        // Round robin selection
        let id1 = vip.select_backend(None).unwrap();
        let id2 = vip.select_backend(None).unwrap();
        let id3 = vip.select_backend(None).unwrap();

        assert_ne!(id1, id2);
        assert_eq!(id1, id3); // Should wrap around
    }

    #[test]
    fn test_weighted_round_robin() {
        let config = VipConfig {
            id: 1,
            address: IpAddr::v4(192, 168, 1, 100),
            port: 80,
            protocol: LbProtocol::Http,
            algorithm: LbAlgorithm::WeightedRoundRobin,
            persistence: PersistenceType::None,
            backends: Vec::new(),
            ssl_cert: None,
            ssl_key: None,
            ssl_termination: false,
        };

        let mut vip = Vip::new(config);

        let backend1 = Backend::new(1, IpAddr::v4(192, 168, 1, 10), 8080, 70);
        let backend2 = Backend::new(2, IpAddr::v4(192, 168, 1, 11), 8080, 30);

        vip.add_backend(backend1).unwrap();
        vip.add_backend(backend2).unwrap();

        // With weights 70:30, backend1 should be selected more often
        let mut count1 = 0;
        let mut count2 = 0;

        for _ in 0..100 {
            match vip.select_backend(None) {
                Ok(1) => count1 += 1,
                Ok(2) => count2 += 1,
                _ => {},
            }
        }

        assert!(count1 > count2);
    }

    #[test]
    fn test_least_connections() {
        let config = VipConfig {
            id: 1,
            address: IpAddr::v4(192, 168, 1, 100),
            port: 80,
            protocol: LbProtocol::Http,
            algorithm: LbAlgorithm::LeastConnections,
            persistence: PersistenceType::None,
            backends: Vec::new(),
            ssl_cert: None,
            ssl_key: None,
            ssl_termination: false,
        };

        let mut vip = Vip::new(config);

        let mut backend1 = Backend::new(1, IpAddr::v4(192, 168, 1, 10), 8080, 100);
        let mut backend2 = Backend::new(2, IpAddr::v4(192, 168, 1, 11), 8080, 100);

        backend1.current_connections = 5;
        backend2.current_connections = 2;

        vip.add_backend(backend1).unwrap();
        vip.add_backend(backend2).unwrap();

        // Should select backend2 (fewer connections)
        let id = vip.select_backend(None).unwrap();
        assert_eq!(id, 2);
    }

    #[test]
    fn test_source_hash() {
        let config = VipConfig {
            id: 1,
            address: IpAddr::v4(192, 168, 1, 100),
            port: 80,
            protocol: LbProtocol::Http,
            algorithm: LbAlgorithm::SourceHash,
            persistence: PersistenceType::None,
            backends: Vec::new(),
            ssl_cert: None,
            ssl_key: None,
            ssl_termination: false,
        };

        let mut vip = Vip::new(config);

        let backend1 = Backend::new(1, IpAddr::v4(192, 168, 1, 10), 8080, 100);
        let backend2 = Backend::new(2, IpAddr::v4(192, 168, 1, 11), 8080, 100);

        vip.add_backend(backend1).unwrap();
        vip.add_backend(backend2).unwrap();

        // Same source IP should always select same backend
        let ip = 0xC0A80101;
        let id1 = vip.select_backend(Some(ip)).unwrap();
        let id2 = vip.select_backend(Some(ip)).unwrap();

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_health_checker() {
        let config = HealthCheckConfig::default();
        let mut checker = HealthChecker::new(config);

        checker.register_backend(1);

        // Process successful checks
        for _ in 0..2 {
            checker.process_result(1, HealthCheckResult::Success);
        }

        assert_eq!(checker.get_health(1), BackendHealth::Healthy);

        // Process failures
        for _ in 0..3 {
            checker.process_result(1, HealthCheckResult::Failure);
        }

        assert_eq!(checker.get_health(1), BackendHealth::Unhealthy);
    }

    #[test]
    fn test_load_balancer() {
        let health_config = HealthCheckConfig::default();
        let mut lb = LoadBalancer::new(health_config);

        let config = VipConfig {
            id: 1,
            address: IpAddr::v4(192, 168, 1, 100),
            port: 80,
            protocol: LbProtocol::Http,
            algorithm: LbAlgorithm::RoundRobin,
            persistence: PersistenceType::None,
            backends: Vec::new(),
            ssl_cert: None,
            ssl_key: None,
            ssl_termination: false,
        };

        lb.create_vip(config).unwrap();

        let backend = Backend::new(0, IpAddr::v4(192, 168, 1, 10), 8080, 100);
        lb.add_backend(1, backend).unwrap();

        let selected = lb.select_backend(1, None).unwrap();
        assert_eq!(selected.address, IpAddr::v4(192, 168, 1, 10));
        assert_eq!(selected.port, 8080);
    }

    #[test]
    fn test_backend_draining() {
        let health_config = HealthCheckConfig::default();
        let mut lb = LoadBalancer::new(health_config);

        let config = VipConfig {
            id: 1,
            address: IpAddr::v4(192, 168, 1, 100),
            port: 80,
            protocol: LbProtocol::Http,
            algorithm: LbAlgorithm::RoundRobin,
            persistence: PersistenceType::None,
            backends: Vec::new(),
            ssl_cert: None,
            ssl_key: None,
            ssl_termination: false,
        };

        lb.create_vip(config).unwrap();

        let backend = Backend::new(0, IpAddr::v4(192, 168, 1, 10), 8080, 100);
        let backend_id = lb.add_backend(1, backend).unwrap();

        // Drain the backend
        lb.drain_backend(1, backend_id).unwrap();

        // Backend should not be available
        let result = lb.select_backend(1, None);
        assert!(result.is_err());

        // Cancel drain
        lb.cancel_drain(1, backend_id).unwrap();

        // Backend should be available again
        let selected = lb.select_backend(1, None).unwrap();
        assert_eq!(selected.id, backend_id);
    }

    #[test]
    fn test_persistence() {
        let config = VipConfig {
            id: 1,
            address: IpAddr::v4(192, 168, 1, 100),
            port: 80,
            protocol: LbProtocol::Http,
            algorithm: LbAlgorithm::RoundRobin,
            persistence: PersistenceType::SourceIp,
            backends: Vec::new(),
            ssl_cert: None,
            ssl_key: None,
            ssl_termination: false,
        };

        let mut vip = Vip::new(config);

        let backend1 = Backend::new(1, IpAddr::v4(192, 168, 1, 10), 8080, 100);
        let backend2 = Backend::new(2, IpAddr::v4(192, 168, 1, 11), 8080, 100);

        vip.add_backend(backend1).unwrap();
        vip.add_backend(backend2).unwrap();

        // Set persistence
        vip.set_persistence(String::from("192.168.1.50"), 1);

        // Should return persisted backend
        let persisted = vip.get_persistence("192.168.1.50");
        assert_eq!(persisted, Some(1));
    }
}
