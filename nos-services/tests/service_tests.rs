//! Service tests

use nos_services::*;

#[test]
fn test_service_init() {
    // Test service initialization
    assert!(init_services().is_ok());
}

#[test]
fn test_service_stats() {
    // Test service statistics
    let stats = get_service_stats();
    assert_eq!(stats.total_services, 0);
    assert_eq!(stats.running_services, 0);
    assert_eq!(stats.error_count, 0);
    assert_eq!(stats.avg_startup_time, 0);
}

#[test]
fn test_service_registry() {
    use crate::registry;
    
    // Test service registry
    let mut registry = registry::ServiceRegistry::new();
    
    // Register a test service
    let service = TestService::new("test_service");
    let id = registry.register("test_service", Box::new(service)).unwrap();
    
    // Get service
    let info = registry.get(id).unwrap();
    assert_eq!(info.name, "test_service");
    
    // Get by name
    let info = registry.get_by_name("test_service").unwrap();
    assert_eq!(info.id, id);
}

#[test]
fn test_service_discovery() {
    use crate::discovery;
    
    // Test service discovery
    let mut discovery = discovery::ServiceDiscovery::new();
    
    // Add a test service
    let descriptor = discovery::ServiceDescriptor {
        name: "test_service".to_string(),
        service_type: 1,
        version: "1.0.0".to_string(),
        description: "Test service".to_string(),
        endpoint: "local://test_service".to_string(),
        metadata: alloc::collections::BTreeMap::new(),
    };
    discovery.add_service(descriptor.clone());
    
    // Get service
    let retrieved = discovery.get_service("test_service").unwrap();
    assert_eq!(retrieved.name, descriptor.name);
    
    // List services
    let services = discovery.list_services();
    assert_eq!(services.len(), 1);
    assert_eq!(services[0].name, descriptor.name);
}

struct TestService {
    name: String,
}

impl TestService {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl crate::core::Service for TestService {
    fn start(&self) -> nos_api::Result<()> {
        Ok(())
    }
    
    fn stop(&self) -> nos_api::Result<()> {
        Ok(())
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn service_type(&self) -> u32 {
        1
    }
}