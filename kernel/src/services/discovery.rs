// Service discovery and registration for hybrid architecture
//
// Provides dynamic service discovery, load balancing, and failover mechanisms
// for the service-oriented architecture.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::sync::Mutex;
use crate::reliability::errno::{EINVAL, ENOENT, EAGAIN, ETIMEDOUT};
use crate::microkernel::service_registry::{ServiceRegistry, ServiceId, ServiceCategory, ServiceInfo, ServiceStatus, InterfaceVersion};

/// Service discovery endpoint
#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    pub service_id: ServiceId,
    pub endpoint_type: EndpointType,
    pub address: String,
    pub port: u16,
    pub protocol: String,
    pub weight: u32,        // Load balancing weight
    pub health_score: f64,  // Health score (0.0 to 1.0)
    pub last_health_check: u64,
    pub request_count: AtomicU64,
    pub error_count: AtomicU64,
    pub response_time_sum: AtomicU64,
}

impl ServiceEndpoint {
    pub fn new(service_id: ServiceId, endpoint_type: EndpointType, address: String, port: u16, protocol: String) -> Self {
        Self {
            service_id,
            endpoint_type,
            address,
            port,
            protocol,
            weight: 100,
            health_score: 1.0,
            last_health_check: crate::time::get_time_ns(),
            request_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            response_time_sum: AtomicU64::new(0),
        }
    }

    pub fn is_healthy(&self) -> bool {
        let current_time = crate::time::get_time_ns();
        let health_check_interval = 5_000_000_000; // 5 seconds

        // Check if recent health check
        if current_time - self.last_health_check > health_check_interval {
            return false;
        }

        // Check health score
        self.health_score >= 0.5
    }

    pub fn get_average_response_time(&self) -> f64 {
        let request_count = self.request_count.load(Ordering::SeqCst);
        if request_count == 0 {
            0.0
        } else {
            self.response_time_sum.load(Ordering::SeqCst) as f64 / request_count as f64
        }
    }

    pub fn get_error_rate(&self) -> f64 {
        let request_count = self.request_count.load(Ordering::SeqCst);
        if request_count == 0 {
            0.0
        } else {
            self.error_count.load(Ordering::SeqCst) as f64 / request_count as f64
        }
    }

    pub fn update_health(&mut self, response_time_ns: u64, success: bool) {
        let current_time = crate::time::get_time_ns();

        self.last_health_check = current_time;
        self.request_count.fetch_add(1, Ordering::SeqCst);
        self.response_time_sum.fetch_add(response_time_ns, Ordering::SeqCst);

        if !success {
            self.error_count.fetch_add(1, Ordering::SeqCst);
        }

        // Update health score based on response time and error rate
        let error_rate = self.get_error_rate();
        let avg_response_time = self.get_average_response_time();

        // Calculate health score (0.0 to 1.0)
        let response_time_score = if avg_response_time < 1_000_000.0 { // < 1ms
            1.0
        } else if avg_response_time < 10_000_000.0 { // < 10ms
            0.8
        } else if avg_response_time < 100_000_000.0 { // < 100ms
            0.6
        } else {
            0.4
        };

        let error_score = 1.0 - error_rate;

        self.health_score = (response_time_score + error_score) / 2.0;
    }
}

/// Endpoint types for different communication mechanisms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    Local,      // Local process communication
    Socket,     // Network socket
    SharedMem,  // Shared memory
    MessageQueue, // Message queue
    Rpc,        // Remote procedure call
}

/// Load balancing strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingStrategy {
    RoundRobin,     // Simple round-robin
    WeightedRoundRobin, // Weighted round-robin
    LeastConnections, // Endpoint with fewest active connections
    ResponseTime,   // Endpoint with best response time
    HealthScore,    // Endpoint with highest health score
    Random,         // Random selection
}

/// Service discovery cache entry
#[derive(Debug, Clone)]
pub struct DiscoveryCacheEntry {
    pub service_info: ServiceInfo,
    pub endpoints: Vec<ServiceEndpoint>,
    pub timestamp: u64,
    pub ttl: u64, // Time-to-live in nanoseconds
}

impl DiscoveryCacheEntry {
    pub fn new(service_info: ServiceInfo, endpoints: Vec<ServiceEndpoint>, ttl_ns: u64) -> Self {
        let current_time = crate::time::get_time_ns();
        Self {
            service_info,
            endpoints,
            timestamp: current_time,
            ttl: ttl_ns,
        }
    }

    pub fn is_expired(&self) -> bool {
        let current_time = crate::time::get_time_ns();
        current_time - self.timestamp > self.ttl
    }

    pub fn get_healthy_endpoints(&self) -> Vec<&ServiceEndpoint> {
        self.endpoints.iter().filter(|e| e.is_healthy()).collect()
    }
}

/// Service discovery manager
pub struct ServiceDiscovery {
    pub registry: Arc<ServiceRegistry>,
    pub cache: Mutex<BTreeMap<String, DiscoveryCacheEntry>>,
    pub load_balancer: LoadBalancer,
    pub discovery_interval: u64,
    pub next_discovery_time: AtomicU64,
}

impl ServiceDiscovery {
    pub fn new(registry: Arc<ServiceRegistry>) -> Self {
        Self {
            registry,
            cache: Mutex::new(BTreeMap::new()),
            load_balancer: LoadBalancer::new(LoadBalancingStrategy::HealthScore),
            discovery_interval: 30_000_000_000, // 30 seconds
            next_discovery_time: AtomicU64::new(crate::time::get_time_ns()),
        }
    }

    pub fn discover_service(&self, name: &str) -> Result<Option<DiscoveryCacheEntry>, i32> {
        // Check cache first
        {
            let mut cache = self.cache.lock();
            if let Some(entry) = cache.get(name) {
                if !entry.is_expired() {
                    return Ok(Some(entry.clone()));
                }
                // Remove expired entry
                cache.remove(name);
            }
        }

        // Discover from registry
        if let Some(service_info) = self.registry.find_service_by_name(name) {
            // Get endpoints for this service
            let endpoints = self.get_service_endpoints(service_info.id)?;

            let cache_entry = DiscoveryCacheEntry::new(service_info, endpoints, self.discovery_interval);

            // Update cache
            {
                let mut cache = self.cache.lock();
                cache.insert(name.to_string(), cache_entry.clone());
            }

            Ok(Some(cache_entry))
        } else {
            Ok(None)
        }
    }

    pub fn discover_services_by_category(&self, category: ServiceCategory) -> Result<Vec<DiscoveryCacheEntry>, i32> {
        let services = self.registry.find_services_by_category(category);
        let mut results = Vec::new();

        for service in services {
            let endpoints = self.get_service_endpoints(service.id)?;
            let cache_entry = DiscoveryCacheEntry::new(service, endpoints, self.discovery_interval);

            // Update cache
            {
                let mut cache = self.cache.lock();
                cache.insert(cache_entry.service_info.name.clone(), cache_entry.clone());
            }

            results.push(cache_entry);
        }

        Ok(results)
    }

    pub fn get_service_endpoints(&self, service_id: ServiceId) -> Result<Vec<ServiceEndpoint>, i32> {
        // In a real implementation, this would query the service for its endpoints
        // For now, we'll create a default local endpoint
        let service_info = self.registry.find_service_by_id(service_id).ok_or(ENOENT)?;

        let mut endpoints = Vec::new();

        // Add local endpoint
        let local_endpoint = ServiceEndpoint::new(
            service_id,
            EndpointType::Local,
            "local".to_string(),
            0,
            "local".to_string(),
        );
        endpoints.push(local_endpoint);

        // Add network endpoints if available (placeholder)
        if service_info.status == ServiceStatus::Running {
            let network_endpoint = ServiceEndpoint::new(
                service_id,
                EndpointType::Socket,
                "127.0.0.1".to_string(),
                8080 + (service_id % 1000) as u16,
                "tcp".to_string(),
            );
            endpoints.push(network_endpoint);
        }

        Ok(endpoints)
    }

    pub fn register_service_endpoint(&self, endpoint: ServiceEndpoint) -> Result<(), i32> {
        // Verify service exists
        let _service_info = self.registry.find_service_by_id(endpoint.service_id).ok_or(ENOENT)?;

        // Update cache entry for the service
        let service_name = {
            let cache = self.cache.lock();
            cache.keys().next().cloned().unwrap_or_default()
        };

        if !service_name.is_empty() {
            let mut cache = self.cache.lock();
            if let Some(entry) = cache.get_mut(&service_name) {
                // Add or update endpoint
                if let Some(existing_endpoint) = entry.endpoints.iter_mut().find(|e| e.service_id == endpoint.service_id) {
                    *existing_endpoint = endpoint;
                } else {
                    entry.endpoints.push(endpoint);
                }
            }
        }

        Ok(())
    }

    pub fn select_endpoint(&self, service_name: &str) -> Result<Option<ServiceEndpoint>, i32> {
        let cache_entry = self.discover_service(service_name)?;

        if let Some(entry) = cache_entry {
            let healthy_endpoints: Vec<_> = entry.get_healthy_endpoints().into_iter().cloned().collect();

            if healthy_endpoints.is_empty() {
                return Ok(None);
            }

            let selected = self.load_balancer.select_endpoint(&healthy_endpoints)?;
            Ok(Some(selected))
        } else {
            Ok(None)
        }
    }

    pub fn update_endpoint_health(&self, service_id: ServiceId, response_time_ns: u64, success: bool) -> Result<(), i32> {
        let mut cache = self.cache.lock();

        for entry in cache.values_mut() {
            if let Some(endpoint) = entry.endpoints.iter_mut().find(|e| e.service_id == service_id) {
                endpoint.update_health(response_time_ns, success);
                break;
            }
        }

        Ok(())
    }

    pub fn refresh_discovery_cache(&self) -> Result<(), i32> {
        let current_time = crate::time::get_time_ns();

        // Check if it's time for discovery
        let next_discovery = self.next_discovery_time.load(Ordering::SeqCst);
        if current_time < next_discovery {
            return Ok(());
        }

        // Clear expired entries
        {
            let mut cache = self.cache.lock();
            cache.retain(|_, entry| !entry.is_expired());
        }

        // Update next discovery time
        self.next_discovery_time.store(current_time + self.discovery_interval, Ordering::SeqCst);

        Ok(())
    }

    pub fn set_load_balancing_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.load_balancer.strategy = strategy;
    }

    pub fn get_service_health(&self, service_name: &str) -> Option<f64> {
        let cache = self.cache.lock();
        cache.get(service_name).map(|entry| {
            let endpoints = &entry.endpoints;
            if endpoints.is_empty() {
                0.0
            } else {
                let total_health: f64 = endpoints.iter().map(|e| e.health_score).sum();
                total_health / endpoints.len() as f64
            }
        })
    }

    pub fn get_service_metrics(&self, service_name: &str) -> Option<ServiceMetrics> {
        let cache = self.cache.lock();
        cache.get(service_name).map(|entry| {
            let endpoints = &entry.endpoints;
            let total_requests: u64 = endpoints.iter().map(|e| e.request_count.load(Ordering::SeqCst)).sum();
            let total_errors: u64 = endpoints.iter().map(|e| e.error_count.load(Ordering::SeqCst)).sum();
            let total_response_time: u64 = endpoints.iter().map(|e| e.response_time_sum.load(Ordering::SeqCst)).sum();

            ServiceMetrics {
                total_requests,
                total_errors,
                average_response_time: if total_requests > 0 {
                    total_response_time as f64 / total_requests as f64
                } else {
                    0.0
                },
                error_rate: if total_requests > 0 {
                    total_errors as f64 / total_requests as f64
                } else {
                    0.0
                },
            }
        })
    }
}

/// Service metrics for monitoring
#[derive(Debug, Clone)]
pub struct ServiceMetrics {
    pub total_requests: u64,
    pub total_errors: u64,
    pub average_response_time: f64,
    pub error_rate: f64,
}

/// Load balancer for service endpoints
#[derive(Debug)]
pub struct LoadBalancer {
    pub strategy: LoadBalancingStrategy,
    round_robin_counter: AtomicUsize,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            strategy,
            round_robin_counter: AtomicUsize::new(0),
        }
    }

    pub fn select_endpoint(&self, endpoints: &[ServiceEndpoint]) -> Result<ServiceEndpoint, i32> {
        if endpoints.is_empty() {
            return Err(ENOENT);
        }

        let index = match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                let counter = self.round_robin_counter.fetch_add(1, Ordering::SeqCst);
                counter % endpoints.len()
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                self.select_weighted_round_robin(endpoints)
            }
            LoadBalancingStrategy::LeastConnections => {
                self.select_least_connections(endpoints)
            }
            LoadBalancingStrategy::ResponseTime => {
                self.select_by_response_time(endpoints)
            }
            LoadBalancingStrategy::HealthScore => {
                self.select_by_health_score(endpoints)
            }
            LoadBalancingStrategy::Random => {
                use crate::time::get_time_ns;
                let hash = get_time_ns() as usize;
                hash % endpoints.len()
            }
        };

        Ok(endpoints[index].clone())
    }

    fn select_weighted_round_robin(&self, endpoints: &[ServiceEndpoint]) -> usize {
        let total_weight: u32 = endpoints.iter().map(|e| e.weight).sum();
        let mut counter = self.round_robin_counter.fetch_add(1, Ordering::SeqCst) as u32;
        counter %= total_weight;

        let mut accumulated_weight = 0;
        for (i, endpoint) in endpoints.iter().enumerate() {
            accumulated_weight += endpoint.weight;
            if counter < accumulated_weight {
                return i;
            }
        }

        0 // Fallback to first endpoint
    }

    fn select_least_connections(&self, endpoints: &[ServiceEndpoint]) -> usize {
        let mut best_index = 0;
        let mut min_requests = endpoints[0].request_count.load(Ordering::SeqCst);

        for (i, endpoint) in endpoints.iter().enumerate().skip(1) {
            let requests = endpoint.request_count.load(Ordering::SeqCst);
            if requests < min_requests {
                min_requests = requests;
                best_index = i;
            }
        }

        best_index
    }

    fn select_by_response_time(&self, endpoints: &[ServiceEndpoint]) -> usize {
        let mut best_index = 0;
        let mut best_time = endpoints[0].get_average_response_time();

        for (i, endpoint) in endpoints.iter().enumerate().skip(1) {
            let avg_time = endpoint.get_average_response_time();
            if avg_time < best_time {
                best_time = avg_time;
                best_index = i;
            }
        }

        best_index
    }

    fn select_by_health_score(&self, endpoints: &[ServiceEndpoint]) -> usize {
        let mut best_index = 0;
        let mut best_score = endpoints[0].health_score;

        for (i, endpoint) in endpoints.iter().enumerate().skip(1) {
            if endpoint.health_score > best_score {
                best_score = endpoint.health_score;
                best_index = i;
            }
        }

        best_index
    }
}

/// Service discovery client for applications
pub struct ServiceClient {
    discovery: Arc<ServiceDiscovery>,
}

impl ServiceClient {
    pub fn new(discovery: Arc<ServiceDiscovery>) -> Self {
        Self { discovery }
    }

    pub fn connect_to_service(&self, service_name: &str) -> Result<ServiceConnection, i32> {
        let endpoint = self.discovery.select_endpoint(service_name)?
            .ok_or(EAGAIN)?;

        let start_time = crate::time::get_time_ns();

        // Establish connection based on endpoint type
        let connection = match endpoint.endpoint_type {
            EndpointType::Local => self.connect_local(endpoint.service_id)?,
            EndpointType::Socket => self.connect_socket(&endpoint.address, endpoint.port)?,
            EndpointType::SharedMem => self.connect_shared_memory(endpoint.service_id)?,
            EndpointType::MessageQueue => self.connect_message_queue(endpoint.service_id)?,
            EndpointType::Rpc => self.connect_rpc(&endpoint.address, endpoint.port)?,
        };

        let end_time = crate::time::get_time_ns();
        let response_time = end_time - start_time;

        // Update endpoint health
        let _ = self.discovery.update_endpoint_health(endpoint.service_id, response_time, true);

        Ok(ServiceConnection {
            endpoint,
            connection,
            last_activity: end_time,
        })
    }

    fn connect_local(&self, service_id: ServiceId) -> Result<ConnectionHandle, i32> {
        // Establish local connection to service
        Ok(ConnectionHandle::Local(service_id))
    }

    fn connect_socket(&self, address: &str, port: u16) -> Result<ConnectionHandle, i32> {
        // Establish socket connection
        // This would use the network stack
        Ok(ConnectionHandle::Socket(format!("{}:{}", address, port)))
    }

    fn connect_shared_memory(&self, service_id: ServiceId) -> Result<ConnectionHandle, i32> {
        // Establish shared memory connection
        Ok(ConnectionHandle::SharedMemory(service_id))
    }

    fn connect_message_queue(&self, service_id: ServiceId) -> Result<ConnectionHandle, i32> {
        // Establish message queue connection
        Ok(ConnectionHandle::MessageQueue(service_id))
    }

    fn connect_rpc(&self, address: &str, port: u16) -> Result<ConnectionHandle, i32> {
        // Establish RPC connection
        Ok(ConnectionHandle::Rpc(format!("{}:{}", address, port)))
    }
}

/// Active connection to a service
#[derive(Debug)]
pub struct ServiceConnection {
    pub endpoint: ServiceEndpoint,
    pub connection: ConnectionHandle,
    pub last_activity: u64,
}

/// Connection handle for different endpoint types
#[derive(Debug)]
pub enum ConnectionHandle {
    Local(ServiceId),
    Socket(String),
    SharedMemory(ServiceId),
    MessageQueue(ServiceId),
    Rpc(String),
}

/// Global service discovery instance
static mut GLOBAL_SERVICE_DISCOVERY: Option<Arc<ServiceDiscovery>> = None;
static DISCOVERY_INIT: AtomicBool = AtomicBool::new(false);

/// Initialize service discovery
pub fn init(registry: Arc<ServiceRegistry>) -> Result<(), i32> {
    if DISCOVERY_INIT.load(Ordering::SeqCst) {
        return Ok(());
    }

    let discovery = Arc::new(ServiceDiscovery::new(registry));

    unsafe {
        GLOBAL_SERVICE_DISCOVERY = Some(discovery);
    }

    DISCOVERY_INIT.store(true, Ordering::SeqCst);
    Ok(())
}

/// Get global service discovery
pub fn get_service_discovery() -> Option<Arc<ServiceDiscovery>> {
    unsafe {
        GLOBAL_SERVICE_DISCOVERY.clone()
    }
}

/// Get service discovery client
pub fn get_service_client() -> Option<ServiceClient> {
    get_service_discovery().map(ServiceClient::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microkernel::service_registry::{InterfaceVersion, ServiceInfo};

    #[test]
    fn test_service_endpoint() {
        let mut endpoint = ServiceEndpoint::new(
            1,
            EndpointType::Local,
            "local".to_string(),
            0,
            "local".to_string(),
        );

        assert!(endpoint.is_healthy());
        assert_eq!(endpoint.get_average_response_time(), 0.0);
        assert_eq!(endpoint.get_error_rate(), 0.0);

        endpoint.update_health(1_000_000, true);
        assert_eq!(endpoint.request_count.load(Ordering::SeqCst), 1);
        assert_eq!(endpoint.response_time_sum.load(Ordering::SeqCst), 1_000_000);

        endpoint.update_health(2_000_000, false);
        assert_eq!(endpoint.request_count.load(Ordering::SeqCst), 2);
        assert_eq!(endpoint.error_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_discovery_cache_entry() {
        let version = InterfaceVersion::new(1, 0, 0);
        let service_info = ServiceInfo::new(
            1,
            "test-service".to_string(),
            "Test service".to_string(),
            ServiceCategory::System,
            version,
            100,
        );

        let endpoint = ServiceEndpoint::new(
            1,
            EndpointType::Local,
            "local".to_string(),
            0,
            "local".to_string(),
        );

        let cache_entry = DiscoveryCacheEntry::new(service_info, vec![endpoint], 1_000_000_000);

        assert!(!cache_entry.is_expired());
        assert_eq!(cache_entry.get_healthy_endpoints().len(), 1);
    }

    #[test]
    fn test_load_balancer() {
        let mut lb = LoadBalancer::new(LoadBalancingStrategy::RoundRobin);

        let endpoints = vec![
            ServiceEndpoint::new(1, EndpointType::Local, "local1".to_string(), 0, "local".to_string()),
            ServiceEndpoint::new(2, EndpointType::Local, "local2".to_string(), 0, "local".to_string()),
        ];

        // Test round-robin
        let endpoint1 = lb.select_endpoint(&endpoints).unwrap();
        let endpoint2 = lb.select_endpoint(&endpoints).unwrap();
        let endpoint3 = lb.select_endpoint(&endpoints).unwrap();

        assert_ne!(endpoint1.service_id, endpoint2.service_id);
        assert_eq!(endpoint1.service_id, endpoint3.service_id);
    }

    #[test]
    fn test_load_balancing_strategies() {
        let endpoints = vec![
            ServiceEndpoint::new(1, EndpointType::Local, "fast".to_string(), 0, "local".to_string()),
            ServiceEndpoint::new(2, EndpointType::Local, "slow".to_string(), 0, "local".to_string()),
        ];

        let strategies = [
            LoadBalancingStrategy::RoundRobin,
            LoadBalancingStrategy::Random,
            LoadBalancingStrategy::HealthScore,
        ];

        for strategy in strategies {
            let lb = LoadBalancer::new(strategy);
            let endpoint = lb.select_endpoint(&endpoints);
            assert!(endpoint.is_ok());
        }
    }
}