//! 依赖注入容器配置支持
//!
//! 提供TOML和JSON格式的配置解析功能，
//! 支持基于配置文件的服务注册和条件评估。

use crate::infrastructure::di_container::{
    DIContainer, ServiceLifecycle, ServiceCondition
};
use crate::infrastructure::ServiceFactory;
use crate::protocol::BootProtocolType;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::any::Any;

/// 配置错误类型
#[derive(Debug)]
pub enum ConfigError {
    /// 解析错误
    ParseError(String),
    /// 验证错误
    ValidationError(String),
    /// 依赖错误
    DependencyError(String),
    /// IO错误
    IoError(String),
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ConfigError::DependencyError(msg) => write!(f, "Dependency error: {}", msg),
            ConfigError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

/// 配置结构
#[derive(Debug, Clone)]
pub struct DIContainerConfig {
    /// 默认协议类型
    pub default_protocol: BootProtocolType,
    /// 是否启用延迟加载
    pub enable_lazy_loading: bool,
    /// 是否启用循环依赖检查
    pub enable_circular_dependency_check: bool,
    /// 服务配置列表
    pub services: BTreeMap<String, ServiceConfig>,
    /// 配置值
    pub config_values: BTreeMap<String, String>,
}

impl Default for DIContainerConfig {
    fn default() -> Self {
        Self {
            default_protocol: BootProtocolType::Bios,
            enable_lazy_loading: true,
            enable_circular_dependency_check: true,
            services: BTreeMap::new(),
            config_values: BTreeMap::new(),
        }
    }
}

/// 服务配置
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// 服务类型
    pub service_type: String,
    /// 生命周期
    pub lifecycle: ServiceLifecycle,
    /// 实现类型
    pub implementation: Option<String>,
    /// 注册条件
    pub condition: Option<ConditionConfig>,
    /// 依赖列表
    pub dependencies: Vec<String>,
}

/// 条件配置
#[derive(Debug, Clone)]
pub enum ConditionConfig {
    /// 总是条件
    Always,
    /// 协议类型条件
    ProtocolType(BootProtocolType),
    /// 功能启用条件
    FeatureEnabled(String),
    /// 自定义条件
    Custom(String),
}

impl ConditionConfig {
    /// 转换为ServiceCondition
    pub fn to_service_condition(&self) -> ServiceCondition {
        match self {
            ConditionConfig::Always => ServiceCondition::Always,
            ConditionConfig::ProtocolType(protocol) => ServiceCondition::ProtocolType(*protocol),
            ConditionConfig::FeatureEnabled(feature) => {
                let feature = feature.clone().leak();
                ServiceCondition::FeatureEnabled(feature)
            }
            ConditionConfig::Custom(_func_name) => {
                // 在实际实现中，这里会查找并调用自定义条件函数
                // 现在使用默认的Always条件
                ServiceCondition::Always
            }
        }
    }
}

/// 配置解析器
pub struct ConfigParser;

impl ConfigParser {
    /// 从TOML字符串解析配置
    pub fn parse_toml(toml_str: &str) -> Result<DIContainerConfig, ConfigError> {
        // 简化的TOML解析实现
        // 在实际项目中，应该使用专门的TOML解析库
        Self::parse_config_str(toml_str, "toml")
    }
    
    /// 从JSON字符串解析配置
    pub fn parse_json(json_str: &str) -> Result<DIContainerConfig, ConfigError> {
        // 简化的JSON解析实现
        // 在实际项目中，应该使用专门的JSON解析库
        Self::parse_config_str(json_str, "json")
    }
    
    /// 从配置字符串解析（内部方法）
    fn parse_config_str(config_str: &str, _format: &str) -> Result<DIContainerConfig, ConfigError> {
        // 这里是一个简化的解析实现
        // 实际项目中应该使用专业的解析库
        
        let mut config = DIContainerConfig::default();
        
        // 简单的键值对解析（仅用于演示）
        for line in config_str.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                
                match key.as_str() {
                    "default_protocol" => {
                        config.default_protocol = match value.as_str() {
                            "Bios" => BootProtocolType::Bios,
                            "Uefi" => BootProtocolType::Uefi,
                            "Multiboot2" => BootProtocolType::Multiboot2,
                            _ => BootProtocolType::Bios,
                        };
                    }
                    "enable_lazy_loading" => {
                        config.enable_lazy_loading = value == "true";
                    }
                    "enable_circular_dependency_check" => {
                        config.enable_circular_dependency_check = value == "true";
                    }
                    _ => {
                        // 其他配置值
                        config.config_values.insert(key, value);
                    }
                }
            }
        }
        
        Ok(config)
    }
    
    /// 验证配置
    pub fn validate_config(config: &DIContainerConfig) -> Result<(), ConfigError> {
        // 验证服务配置
        for (name, service_config) in &config.services {
            // 验证服务类型
            if service_config.service_type.is_empty() {
                return Err(ConfigError::ValidationError(
                    format!("Service '{}' has empty type", name)
                ));
            }
            
            // 验证依赖
            for dep in &service_config.dependencies {
                if !config.services.contains_key(dep) {
                    return Err(ConfigError::DependencyError(
                        format!("Service '{}' depends on non-existent service '{}'", name, dep)
                    ));
                }
            }
        }
        
        Ok(())
    }
}

/// 配置驱动的服务工厂
pub struct ConfigurableServiceFactory {
    service_config: ServiceConfig,
}

impl ConfigurableServiceFactory {
    /// 创建新的可配置服务工厂
    pub fn new(service_config: ServiceConfig) -> Self {
        Self { service_config }
    }
}

impl ServiceFactory for ConfigurableServiceFactory {
    fn create_instance(&self, _container: &DIContainer) -> Result<Box<dyn Any>, &'static str> {
        // 在实际实现中，这里会根据配置创建相应的服务实例
        // 现在返回一个简单的占位符
        match self.service_config.service_type.as_str() {
            "BootConfigRepository" => {
                Ok(Box::new(crate::domain::repositories::DefaultBootConfigRepository))
            }
            "DomainEventPublisher" => {
                Ok(Box::new(crate::domain::events::SimpleEventPublisher::new()))
            }
            _ => Err("Unknown service type in configuration")
        }
    }
    
    fn get_service_type(&self) -> &'static str {
        // Use Box::leak to convert String to &'static str
        Box::leak(Box::new(self.service_config.service_type.clone())) as &'static str
    }
    
    fn get_dependencies(&self) -> Vec<&'static str> {
        self.service_config.dependencies
            .iter()
            .map(|s| Box::leak(Box::new(s.clone())) as &'static str) // Convert String to &'static str
            .collect()
    }
}

/// DI容器配置扩展
impl DIContainer {
    /// 从配置文件创建容器
    pub fn from_config_file(_file_path: &str) -> Result<Self, ConfigError> {
        // 在实际实现中，这里会读取文件内容
        log::debug!("Loading DI configuration from file");
        // 现在返回一个默认容器
        Ok(Self::new(BootProtocolType::Bios))
    }
    
    /// 从TOML配置字符串创建容器
    pub fn from_toml_config(toml_str: &str) -> Result<Self, ConfigError> {
        let config = ConfigParser::parse_toml(toml_str)?;
        ConfigParser::validate_config(&config)?;
        
        let container = Self::new(config.default_protocol);
        
        // 设置配置值
        for (key, value) in &config.config_values {
            container.set_config_value(key.clone(), value.clone());
        }
        
        // 注册服务
        for (name, service_config) in config.services {
            let factory = ConfigurableServiceFactory::new(service_config.clone());
            let condition = service_config.condition
                .map(|c| c.to_service_condition());
            
            let result = match service_config.lifecycle {
                ServiceLifecycle::Singleton => {
                    container.register_singleton(
                    Box::leak(Box::new(service_config.service_type.clone())) as &'static str,
                    factory,
                    condition,
                )
            }
            ServiceLifecycle::Transient => {
                container.register_transient(
                    Box::leak(Box::new(service_config.service_type.clone())) as &'static str,
                    factory,
                    condition,
                )
            }
            ServiceLifecycle::Scoped => {
                container.register_scoped(
                    Box::leak(Box::new(service_config.service_type.clone())) as &'static str,
                    factory,
                    condition,
                )
            }
            };
            
            if let Err(e) = result {
                return Err(ConfigError::ValidationError(
                    format!("Failed to register service '{}': {}", name, e)
                ));
            }
        }
        
        Ok(container)
    }
    
    /// 从JSON配置字符串创建容器
    pub fn from_json_config(json_str: &str) -> Result<Self, ConfigError> {
        let config = ConfigParser::parse_json(json_str)?;
        ConfigParser::validate_config(&config)?;
        
        let container = Self::new(config.default_protocol);
        
        // 设置配置值
        for (key, value) in &config.config_values {
            container.set_config_value(key.clone(), value.clone());
        }
        
        // 注册服务
        for (name, service_config) in config.services {
            let factory = ConfigurableServiceFactory::new(service_config.clone());
            let condition = service_config.condition
                .map(|c| c.to_service_condition());
            
            let result = match service_config.lifecycle {
                ServiceLifecycle::Singleton => {
                    container.register_singleton(
                        service_config.service_type.leak(),
                        factory,
                        condition,
                    )
                }
                ServiceLifecycle::Transient => {
                    container.register_transient(
                        service_config.service_type.leak(),
                        factory,
                        condition,
                    )
                }
                ServiceLifecycle::Scoped => {
                    container.register_scoped(
                        service_config.service_type.leak(),
                        factory,
                        condition,
                    )
                }
            };
            
            if let Err(e) = result {
                return Err(ConfigError::ValidationError(
                    format!("Failed to register service '{}': {}", name, e)
                ));
            }
        }
        
        Ok(container)
    }
    
    /// 应用配置到现有容器
    pub fn apply_config(&mut self, config: DIContainerConfig) -> Result<(), ConfigError> {
        ConfigParser::validate_config(&config)?;
        
        // 设置配置值
        for (key, value) in &config.config_values {
            self.set_config_value(key.clone(), value.clone());
        }
        
        // 注册服务
        for (name, service_config) in config.services {
            let factory = ConfigurableServiceFactory::new(service_config.clone());
            let condition = service_config.condition
                .map(|c| c.to_service_condition());
            
            let result = match service_config.lifecycle {
                ServiceLifecycle::Singleton => {
                    self.register_singleton(
                    Box::leak(Box::new(service_config.service_type.clone())) as &'static str,
                    factory,
                    condition,
                )
            }
            ServiceLifecycle::Transient => {
                self.register_transient(
                    Box::leak(Box::new(service_config.service_type.clone())) as &'static str,
                    factory,
                    condition,
                )
            }
            ServiceLifecycle::Scoped => {
                self.register_scoped(
                    Box::leak(Box::new(service_config.service_type.clone())) as &'static str,
                    factory,
                    condition,
                )
            }
            };
            
            if let Err(e) = result {
                return Err(ConfigError::ValidationError(
                    format!("Failed to register service '{}': {}", name, e)
                ));
            }
        }
        
        Ok(())
    }
    
    /// 生成当前配置的TOML表示
    pub fn generate_toml_config(&self) -> String {
        let mut toml = String::new();
        
        // 容器配置
        toml.push_str("[di_container]\n");
        toml.push_str(&format!("default_protocol = \"{:?}\"\n", self.protocol_type()));
        toml.push_str("enable_lazy_loading = true\n");
        toml.push_str("enable_circular_dependency_check = true\n\n");
        
        // 服务配置
        toml.push_str("[services]\n");
        
        let services = self.get_registered_services();
        for service_name in services {
            toml.push_str(&format!("[services.{}]\n", service_name));
            toml.push_str(&format!("type = \"{}\"\n", service_name));
            // 在实际实现中，这里会包含更多配置信息
        }
        
        toml
    }
    
    /// 生成当前配置的JSON表示
    pub fn generate_json_config(&self) -> String {
        let mut json = String::new();
        
        json.push_str("{\n");
        json.push_str("  \"di_container\": {\n");
        json.push_str(&format!("    \"default_protocol\": \"{:?}\",\n", self.protocol_type()));
        json.push_str("    \"enable_lazy_loading\": true,\n");
        json.push_str("    \"enable_circular_dependency_check\": true\n");
        json.push_str("  },\n");
        
        json.push_str("  \"services\": {\n");
        
        let services = self.get_registered_services();
        for (i, service_name) in services.iter().enumerate() {
            json.push_str(&format!("    \"{}\": {{\n", service_name));
            json.push_str(&format!("      \"type\": \"{}\"\n", service_name));
            // 在实际实现中，这里会包含更多配置信息
            json.push_str("    }");
            if i < services.len() - 1 {
                json.push_str(",");
            }
            json.push_str("\n");
        }
        
        json.push_str("  }\n");
        json.push_str("}\n");
        
        json
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_parsing() {
        let toml_config = r#"
default_protocol = "Bios"
enable_lazy_loading = true

[services.test_service]
type = "TestService"
lifecycle = "Singleton"
"#;
        
        let result = ConfigParser::parse_toml(toml_config);
        assert!(result.is_ok());
        
        let config = result.unwrap();
        assert_eq!(config.default_protocol, BootProtocolType::Bios);
        assert!(config.enable_lazy_loading);
        assert!(config.services.contains_key("test_service"));
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = DIContainerConfig::default();
        
        // 添加有效服务
        config.services.insert("service1".to_string(), ServiceConfig {
            service_type: "Service1".to_string(),
            lifecycle: ServiceLifecycle::Singleton,
            implementation: None,
            condition: None,
            dependencies: Vec::new(),
        });
        
        // 验证应该成功
        let result = ConfigParser::validate_config(&config);
        assert!(result.is_ok());
        
        // 添加有依赖的服务
        config.services.insert("service2".to_string(), ServiceConfig {
            service_type: "Service2".to_string(),
            lifecycle: ServiceLifecycle::Singleton,
            implementation: None,
            condition: None,
            dependencies: vec!["service1".to_string()],
        });
        
        // 验证应该成功
        let result = ConfigParser::validate_config(&config);
        assert!(result.is_ok());
        
        // 添加有不存在的依赖的服务
        config.services.insert("service3".to_string(), ServiceConfig {
            service_type: "Service3".to_string(),
            lifecycle: ServiceLifecycle::Singleton,
            implementation: None,
            condition: None,
            dependencies: vec!["nonexistent".to_string()],
        });
        
        // 验证应该失败
        let result = ConfigParser::validate_config(&config);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_condition_config_conversion() {
        let always_condition = ConditionConfig::Always;
        let service_condition = always_condition.to_service_condition();
        match service_condition {
            ServiceCondition::Always => {}, // 成功
            _ => panic!("Expected Always condition"),
        }
        
        let protocol_condition = ConditionConfig::ProtocolType(BootProtocolType::Uefi);
        let service_condition = protocol_condition.to_service_condition();
        match service_condition {
            ServiceCondition::ProtocolType(BootProtocolType::Uefi) => {}, // 成功
            _ => panic!("Expected ProtocolType condition"),
        }
        
        let feature_condition = ConditionConfig::FeatureEnabled("test_feature".to_string());
        let service_condition = feature_condition.to_service_condition();
        match service_condition {
            ServiceCondition::FeatureEnabled(feature) => {
                assert_eq!(feature, "test_feature");
            }
            _ => panic!("Expected FeatureEnabled condition"),
        }
    }
    
    #[test]
    fn test_container_from_config() {
        let toml_config = r#"
default_protocol = "Bios"
enable_lazy_loading = true

[services.boot_config_repository]
type = "BootConfigRepository"
lifecycle = "Singleton"
"#;
        
        let result = DIContainer::from_toml_config(toml_config);
        assert!(result.is_ok());
        
        let container = result.unwrap();
        assert_eq!(container.protocol_type(), BootProtocolType::Bios);
        assert!(container.is_service_registered("BootConfigRepository"));
    }
    
    #[test]
    fn test_config_generation() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        // 注册一些服务
        let _ = container.register_singleton(
            "TestService1",
            crate::infrastructure::di_container::DefaultBootConfigRepositoryFactory,
            None,
        );
        let _ = container.register_transient(
            "TestService2",
            crate::infrastructure::di_container::DefaultBootConfigRepositoryFactory,
            None,
        );
        
        // 生成TOML配置
        let toml_config = container.generate_toml_config();
        assert!(toml_config.contains("default_protocol"));
        assert!(toml_config.contains("TestService1"));
        assert!(toml_config.contains("TestService2"));
        
        // 生成JSON配置
        let json_config = container.generate_json_config();
        assert!(json_config.contains("default_protocol"));
        assert!(json_config.contains("TestService1"));
        assert!(json_config.contains("TestService2"));
    }
    
    #[test]
    fn test_configurable_service_factory() {
        let service_config = ServiceConfig {
            service_type: "BootConfigRepository".to_string(),
            lifecycle: ServiceLifecycle::Singleton,
            implementation: None,
            condition: None,
            dependencies: Vec::new(),
        };
        
        let factory = ConfigurableServiceFactory::new(service_config);
        let container = DIContainer::new(BootProtocolType::Bios);
        
        let result = factory.create_instance(&container);
        assert!(result.is_ok());
        
        let instance = result.unwrap();
        // 验证实例类型
        let downcasted = instance.downcast::<crate::domain::repositories::DefaultBootConfigRepository>();
        assert!(downcasted.is_ok());
    }
}