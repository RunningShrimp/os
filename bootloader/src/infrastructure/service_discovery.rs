//! 服务发现机制
//!
//! 提供自动发现和注册服务的功能，支持编译时和运行时发现。
//! 包含服务元数据管理和注解驱动的注册。

use crate::infrastructure::di_container::{
    DIContainer, ServiceCondition
};
use crate::protocol::BootProtocolType;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::any::TypeId;

/// 服务注册表接口
///
/// 定义服务注册的标准接口
pub trait ServiceRegistry: Send + Sync {
    /// 注册服务到容器
    fn register_services(&self, container: &mut DIContainer) -> Result<(), &'static str>;
    
    /// 获取注册表名称
    fn registry_name(&self) -> &'static str;
    
    /// 获取注册表优先级（数值越小优先级越高）
    fn priority(&self) -> u32 { 100 }
}

/// 服务元数据
///
/// 包含服务的描述信息和发现数据
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    /// 服务类型名称
    pub service_type: &'static str,
    /// 服务描述
    pub description: &'static str,
    /// 服务版本
    pub version: &'static str,
    /// 服务作者
    pub author: &'static str,
    /// 服务标签
    pub tags: Vec<&'static str>,
    /// 服务依赖
    pub dependencies: Vec<&'static str>,
    /// 支持的协议类型
    pub supported_protocols: Vec<BootProtocolType>,
    /// 是否为核心服务
    pub is_core_service: bool,
    /// 是否为实验性功能
    pub is_experimental: bool,
}

impl ServiceMetadata {
    /// 创建新的服务元数据
    pub fn new(service_type: &'static str) -> Self {
        Self {
            service_type,
            description: "",
            version: "1.0.0",
            author: "Unknown",
            tags: Vec::new(),
            dependencies: Vec::new(),
            supported_protocols: vec![
                BootProtocolType::Bios,
                BootProtocolType::Uefi,
                BootProtocolType::Multiboot2,
            ],
            is_core_service: false,
            is_experimental: false,
        }
    }
    
    /// 设置描述
    pub fn with_description(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }
    
    /// 设置版本
    pub fn with_version(mut self, version: &'static str) -> Self {
        self.version = version;
        self
    }
    
    /// 设置作者
    pub fn with_author(mut self, author: &'static str) -> Self {
        self.author = author;
        self
    }
    
    /// 添加标签
    pub fn with_tag(mut self, tag: &'static str) -> Self {
        self.tags.push(tag);
        self
    }
    
    /// 添加依赖
    pub fn with_dependency(mut self, dependency: &'static str) -> Self {
        self.dependencies.push(dependency);
        self
    }
    
    /// 设置支持的协议
    pub fn with_supported_protocols(mut self, protocols: Vec<BootProtocolType>) -> Self {
        self.supported_protocols = protocols;
        self
    }
    
    /// 设置为核心服务
    pub fn as_core_service(mut self) -> Self {
        self.is_core_service = true;
        self
    }
    
    /// 设置为实验性功能
    pub fn as_experimental(mut self) -> Self {
        self.is_experimental = true;
        self
    }
}

/// 服务发现器
///
/// 负责发现和注册所有可用的服务
pub struct ServiceDiscovery {
    /// 注册表列表
    registries: Vec<Box<dyn ServiceRegistry>>,
    /// 服务元数据缓存
    metadata_cache: BTreeMap<&'static str, ServiceMetadata>,
    /// 发现的服务类型
    discovered_services: BTreeMap<TypeId, &'static str>,
}

impl ServiceDiscovery {
    /// 创建新的服务发现器
    pub fn new() -> Self {
        Self {
            registries: Vec::new(),
            metadata_cache: BTreeMap::new(),
            discovered_services: BTreeMap::new(),
        }
    }
    
    /// 添加服务注册表
    pub fn add_registry(&mut self, registry: Box<dyn ServiceRegistry>) {
        self.registries.push(registry);
    }
    
    /// 发现并注册所有服务
    pub fn discover_and_register(&mut self, container: &mut DIContainer) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();
        
        // 按优先级排序注册表
        self.registries.sort_by(|a, b| a.priority().cmp(&b.priority()));
        
        // 执行每个注册表
        for registry in &self.registries {
            if let Err(e) = registry.register_services(container) {
                errors.push(e);
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// 注册服务元数据
    pub fn register_metadata(&mut self, metadata: ServiceMetadata) {
        self.metadata_cache.insert(metadata.service_type, metadata);
    }
    
    /// 获取服务元数据
    pub fn get_metadata(&self, service_type: &str) -> Option<&ServiceMetadata> {
        self.metadata_cache.get(service_type)
    }
    
    /// 获取所有服务元数据
    pub fn get_all_metadata(&self) -> Vec<&ServiceMetadata> {
        self.metadata_cache.values().collect()
    }
    
    /// 按标签查找服务
    pub fn find_services_by_tag(&self, tag: &str) -> Vec<&ServiceMetadata> {
        self.metadata_cache
            .values()
            .filter(|metadata| metadata.tags.contains(&tag))
            .collect()
    }
    
    /// 按协议查找服务
    pub fn find_services_by_protocol(&self, protocol: BootProtocolType) -> Vec<&ServiceMetadata> {
        self.metadata_cache
            .values()
            .filter(|metadata| metadata.supported_protocols.contains(&protocol))
            .collect()
    }
    
    /// 查找核心服务
    pub fn find_core_services(&self) -> Vec<&ServiceMetadata> {
        self.metadata_cache
            .values()
            .filter(|metadata| metadata.is_core_service)
            .collect()
    }
    
    /// 查找实验性服务
    pub fn find_experimental_services(&self) -> Vec<&ServiceMetadata> {
        self.metadata_cache
            .values()
            .filter(|metadata| metadata.is_experimental)
            .collect()
    }
    
    /// 注册服务类型信息
    pub fn register_service_type<T: 'static>(&mut self, service_name: &'static str) {
        self.discovered_services.insert(TypeId::of::<T>(), service_name);
    }
    
    /// 根据服务类型ID获取服务名称
    pub fn get_service_name_by_type(&self, type_id: TypeId) -> Option<&'static str> {
        self.discovered_services.get(&type_id).copied()
    }
    
    /// 根据服务名称获取服务类型ID
    pub fn get_service_type_id_by_name(&self, service_name: &str) -> Option<TypeId> {
        self.discovered_services
            .iter()
            .find(|(_, name)| **name == service_name)
            .map(|(&type_id, _)| type_id)
    }
    
    /// 获取所有发现的服务类型
    pub fn get_all_service_types(&self) -> Vec<(TypeId, &'static str)> {
        self.discovered_services.iter().map(|(k, v)| (*k, *v)).collect()
    }
    
    /// 生成服务发现报告
    pub fn generate_discovery_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# Service Discovery Report\n\n");
        
        // 总体统计
        report.push_str(&format!("Total Services: {}\n", self.metadata_cache.len()));
        report.push_str(&format!("Discovered Service Types: {}\n", self.discovered_services.len()));
        
        let core_count = self.metadata_cache
            .values()
            .filter(|m| m.is_core_service)
            .count();
        report.push_str(&format!("Core Services: {}\n", core_count));
        
        let experimental_count = self.metadata_cache
            .values()
            .filter(|m| m.is_experimental)
            .count();
        report.push_str(&format!("Experimental Services: {}\n\n", experimental_count));
        
        // 服务类型列表
        if !self.discovered_services.is_empty() {
            report.push_str("## Discovered Service Types\n\n");
            for (type_id, service_name) in &self.discovered_services {
                report.push_str(&format!("- {}: {:?}\n", service_name, type_id));
            }
            report.push_str("\n");
        }
        
        // 服务列表
        report.push_str("## Services\n\n");
        
        for metadata in self.metadata_cache.values() {
            report.push_str(&format!("### {}\n", metadata.service_type));
            report.push_str(&format!("- Description: {}\n", metadata.description));
            report.push_str(&format!("- Version: {}\n", metadata.version));
            report.push_str(&format!("- Author: {}\n", metadata.author));
            
            if !metadata.tags.is_empty() {
                report.push_str(&format!("- Tags: {}\n", metadata.tags.join(", ")));
            }
            
            if !metadata.dependencies.is_empty() {
                report.push_str(&format!("- Dependencies: {}\n", metadata.dependencies.join(", ")));
            }
            
            let protocols: Vec<String> = metadata.supported_protocols
                .iter()
                .map(|p| format!("{:?}", p))
                .collect();
            report.push_str(&format!("- Supported Protocols: {}\n", protocols.join(", ")));
            
            if metadata.is_core_service {
                report.push_str("- Type: Core Service\n");
            }
            
            if metadata.is_experimental {
                report.push_str("- Status: Experimental\n");
            }
            
            report.push_str("\n");
        }
        
        report
    }
}

impl Default for ServiceDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// 核心服务注册表
pub struct CoreServiceRegistry;

impl ServiceRegistry for CoreServiceRegistry {
    fn register_services(&self, container: &mut DIContainer) -> Result<(), &'static str> {
        // 注册硬件检测服务
        let _ = container.register_singleton(
            "HardwareDetectionService",
            crate::infrastructure::di_container::HardwareDetectionServiceFactory::new(container.protocol_type()),
            Some(ServiceCondition::Always),
        );
        
        // 注册配置仓库
        let _ = container.register_singleton(
            "BootConfigRepository",
            crate::infrastructure::di_container::DefaultBootConfigRepositoryFactory,
            Some(ServiceCondition::Always),
        );
        
        // 注册事件发布器
        let _ = container.register_singleton(
            "DomainEventPublisher",
            crate::infrastructure::di_container::SimpleEventPublisherFactory,
            Some(ServiceCondition::Always),
        );
        
        // 注册图形后端（条件性）
        let graphics_condition = ServiceCondition::ProtocolType(container.protocol_type());
        let _ = container.register_singleton(
            "GraphicsBackend",
            crate::infrastructure::di_container::GraphicsBackendFactory::new(container.protocol_type()),
            Some(graphics_condition),
        );
        
        Ok(())
    }
    
    fn registry_name(&self) -> &'static str {
        "CoreServiceRegistry"
    }
    
    fn priority(&self) -> u32 {
        0 // 最高优先级
    }
}

/// BIOS特定服务注册表
pub struct BiosServiceRegistry;

impl ServiceRegistry for BiosServiceRegistry {
    fn register_services(&self, container: &mut DIContainer) -> Result<(), &'static str> {
        // 只在BIOS协议下注册
        if container.protocol_type() != BootProtocolType::Bios {
            return Ok(()); // 不是BIOS协议，跳过注册
        }
        
        // 注册BIOS特定的服务
        let _ = container.register_singleton(
            "BiosGraphicsBackend",
            crate::infrastructure::di_container::GraphicsBackendFactory::new(BootProtocolType::Bios),
            Some(ServiceCondition::ProtocolType(BootProtocolType::Bios)),
        );
        
        Ok(())
    }
    
    fn registry_name(&self) -> &'static str {
        "BiosServiceRegistry"
    }
    
    fn priority(&self) -> u32 {
        10
    }
}

/// UEFI特定服务注册表
pub struct UefiServiceRegistry;

impl ServiceRegistry for UefiServiceRegistry {
    fn register_services(&self, container: &mut DIContainer) -> Result<(), &'static str> {
        // 只在UEFI协议下注册
        if container.protocol_type() != BootProtocolType::Uefi {
            return Ok(()); // 不是UEFI协议，跳过注册
        }
        
        // 注册UEFI特定的服务
        let _ = container.register_singleton(
            "UefiGraphicsBackend",
            crate::infrastructure::di_container::GraphicsBackendFactory::new(BootProtocolType::Uefi),
            Some(ServiceCondition::ProtocolType(BootProtocolType::Uefi)),
        );
        
        Ok(())
    }
    
    fn registry_name(&self) -> &'static str {
        "UefiServiceRegistry"
    }
    
    fn priority(&self) -> u32 {
        10
    }
}

/// 服务发现扩展
impl DIContainer {
    /// 使用服务发现自动注册服务
    pub fn auto_discover_services(&mut self) -> Result<(), Vec<&'static str>> {
        let mut discovery = ServiceDiscovery::new();
        
        // 添加核心服务注册表
        discovery.add_registry(Box::new(CoreServiceRegistry));
        
        // 添加协议特定注册表
        discovery.add_registry(Box::new(BiosServiceRegistry));
        discovery.add_registry(Box::new(UefiServiceRegistry));
        
        // 执行发现和注册
        discovery.discover_and_register(self)
    }
    
    /// 创建服务发现器并注册所有服务
    pub fn create_service_discovery(&self) -> ServiceDiscovery {
        let mut discovery = ServiceDiscovery::new();
        
        // 添加注册表
        discovery.add_registry(Box::new(CoreServiceRegistry));
        discovery.add_registry(Box::new(BiosServiceRegistry));
        discovery.add_registry(Box::new(UefiServiceRegistry));
        
        // 注册核心服务元数据
        discovery.register_metadata(ServiceMetadata::new("HardwareDetectionService")
            .with_description("Hardware detection and information service")
            .with_author("Bootloader Team")
            .with_version("1.0.0")
            .with_tag("hardware")
            .with_tag("detection")
            .as_core_service());
        
        discovery.register_metadata(ServiceMetadata::new("BootConfigRepository")
            .with_description("Boot configuration repository")
            .with_author("Bootloader Team")
            .with_version("1.0.0")
            .with_tag("configuration")
            .with_tag("repository")
            .as_core_service());
        
        discovery.register_metadata(ServiceMetadata::new("GraphicsBackend")
            .with_description("Graphics output backend")
            .with_author("Bootloader Team")
            .with_version("1.0.0")
            .with_tag("graphics")
            .with_tag("output")
            .with_dependency("HardwareDetectionService"));
        
        discovery.register_metadata(ServiceMetadata::new("DomainEventPublisher")
            .with_description("Domain event publishing service")
            .with_author("Bootloader Team")
            .with_version("1.0.0")
            .with_tag("events")
            .with_tag("messaging")
            .as_core_service());
        
        discovery
    }
    
    /// 获取服务发现统计信息
    pub fn get_discovery_stats(&self) -> DiscoveryStats {
        let discovery = self.create_service_discovery();
        
        let total_services = discovery.get_all_metadata().len();
        let core_services = discovery.find_core_services().len();
        let experimental_services = discovery.find_experimental_services().len();
        
        let mut protocol_distribution = BTreeMap::new();
        for metadata in discovery.get_all_metadata() {
            for protocol in &metadata.supported_protocols {
                *protocol_distribution.entry(format!("{:?}", protocol)).or_insert(0) += 1;
            }
        }
        
        DiscoveryStats {
            total_services,
            core_services,
            experimental_services,
            protocol_distribution,
        }
    }
}

/// 服务发现统计信息
#[derive(Debug, Clone)]
pub struct DiscoveryStats {
    /// 总服务数
    pub total_services: usize,
    /// 核心服务数
    pub core_services: usize,
    /// 实验性服务数
    pub experimental_services: usize,
    /// 协议分布
    pub protocol_distribution: BTreeMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_service_metadata() {
        let metadata = ServiceMetadata::new("TestService")
            .with_description("Test service for unit testing")
            .with_author("Test Author")
            .with_version("1.0.0")
            .with_tag("test")
            .with_dependency("DependencyService")
            .as_core_service();
        
        assert_eq!(metadata.service_type, "TestService");
        assert_eq!(metadata.description, "Test service for unit testing");
        assert_eq!(metadata.author, "Test Author");
        assert_eq!(metadata.version, "1.0.0");
        assert!(metadata.tags.contains(&"test"));
        assert!(metadata.dependencies.contains(&"DependencyService"));
        assert!(metadata.is_core_service);
        assert!(!metadata.is_experimental);
    }
    
    #[test]
    fn test_service_discovery() {
        let mut discovery = ServiceDiscovery::new();
        
        // 添加测试注册表
        discovery.add_registry(Box::new(CoreServiceRegistry));
        
        // 注册元数据
        discovery.register_metadata(ServiceMetadata::new("TestService")
            .with_description("Test service")
            .with_tag("test"));
        
        // 测试元数据查找
        let metadata = discovery.get_metadata("TestService");
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().description, "Test service");
        
        // 测试按标签查找
        let test_services = discovery.find_services_by_tag("test");
        assert_eq!(test_services.len(), 1);
        assert_eq!(test_services[0].service_type, "TestService");
    }
    
    #[test]
    fn test_core_service_registry() {
        let registry = CoreServiceRegistry;
        assert_eq!(registry.registry_name(), "CoreServiceRegistry");
        assert_eq!(registry.priority(), 0);
        
        let mut container = DIContainer::new(BootProtocolType::Bios);
        let result = registry.register_services(&mut container);
        assert!(result.is_ok());
        assert!(container.is_service_registered("HardwareDetectionService"));
        assert!(container.is_service_registered("BootConfigRepository"));
        assert!(container.is_service_registered("DomainEventPublisher"));
    }
    
    #[test]
    fn test_protocol_specific_registries() {
        let bios_registry = BiosServiceRegistry;
        let uefi_registry = UefiServiceRegistry;
        
        let mut bios_container = DIContainer::new(BootProtocolType::Bios);
        let mut uefi_container = DIContainer::new(BootProtocolType::Uefi);
        
        // BIOS注册表应该在BIOS容器中注册服务
        let bios_result = bios_registry.register_services(&mut bios_container);
        assert!(bios_result.is_ok());
        assert!(bios_container.is_service_registered("BiosGraphicsBackend"));
        
        // BIOS注册表不应该在UEFI容器中注册服务
        let bios_in_uefi_result = bios_registry.register_services(&mut uefi_container);
        assert!(bios_in_uefi_result.is_ok());
        assert!(!uefi_container.is_service_registered("BiosGraphicsBackend"));
        
        // UEFI注册表应该在UEFI容器中注册服务
        let uefi_result = uefi_registry.register_services(&mut uefi_container);
        assert!(uefi_result.is_ok());
        assert!(uefi_container.is_service_registered("UefiGraphicsBackend"));
    }
    
    #[test]
    fn test_auto_discovery() {
        let mut container = DIContainer::new(BootProtocolType::Bios);
        let result = container.auto_discover_services();
        assert!(result.is_ok());
        
        // 验证核心服务已注册
        assert!(container.is_service_registered("HardwareDetectionService"));
        assert!(container.is_service_registered("BootConfigRepository"));
        assert!(container.is_service_registered("DomainEventPublisher"));
        
        // 验证协议特定服务已注册
        assert!(container.is_service_registered("BiosGraphicsBackend"));
    }
    
    #[test]
    fn test_discovery_stats() {
        let container = DIContainer::new(BootProtocolType::Bios);
        let stats = container.get_discovery_stats();
        
        assert!(stats.total_services > 0);
        assert!(stats.core_services > 0);
        assert!(!stats.protocol_distribution.is_empty());
    }
    
    #[test]
    fn test_discovery_report() {
        let mut discovery = ServiceDiscovery::new();
        
        discovery.register_metadata(ServiceMetadata::new("TestService")
            .with_description("Test service for discovery")
            .with_author("Test Author")
            .with_version("1.0.0")
            .with_tag("test")
            .as_core_service());
        
        let report = discovery.generate_discovery_report();
        assert!(report.contains("Service Discovery Report"));
        assert!(report.contains("TestService"));
        assert!(report.contains("Test service for discovery"));
        assert!(report.contains("Core Service"));
    }
}