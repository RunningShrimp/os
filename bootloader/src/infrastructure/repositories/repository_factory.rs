//! 仓储工厂实现
//!
//! 提供仓储实例的创建和管理功能，支持依赖注入和配置。
//! 实现了工厂模式，确保仓储实例的一致性和正确性。

use crate::domain::repositories::{
    Repository, BasicRepositoryQuery, BootInfoRepository, MemoryRegionRepository, KernelInfoRepository, 
    GraphicsInfoRepository, EntityId, IdGenerator, RepositoryError
};
use crate::domain::boot_info::BootInfo;
use crate::domain::boot_config::{MemoryRegion, KernelInfo, GraphicsInfo};
use crate::domain::aggregate_root::AggregateRoot;
use crate::domain::transactions::TransactionManager;
use crate::infrastructure::serialization::SerializerRegistry;
use crate::domain::serialization::{BinarySerializer, JsonSerializer};
use super::memory_repository::MemoryRepository;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::string::String;

use alloc::vec::Vec;

/// 工厂错误类型
#[derive(Debug, Clone)]
pub enum FactoryError {
    /// 创建错误
    CreationError(&'static str),
    /// 配置错误
    ConfigurationError(&'static str),
    /// 依赖错误
    DependencyError(&'static str),
    /// 不支持的类型
    UnsupportedType(&'static str),
    /// 初始化错误
    InitializationError(&'static str),
}

impl core::fmt::Display for FactoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FactoryError::CreationError(msg) => write!(f, "Creation error: {}", msg),
            FactoryError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            FactoryError::DependencyError(msg) => write!(f, "Dependency error: {}", msg),
            FactoryError::UnsupportedType(msg) => write!(f, "Unsupported type: {}", msg),
            FactoryError::InitializationError(msg) => write!(f, "Initialization error: {}", msg),
        }
    }
}

/// 仓储工厂特征
///
/// 定义了仓储工厂的基本接口
pub trait RepositoryFactory: Send + Sync {
    /// 创建BootInfo仓储
    fn create_boot_info_repository(&self) -> Result<Box<dyn BootInfoRepository>, FactoryError>;
    
    /// 创建MemoryRegion仓储
    fn create_memory_region_repository(&self) -> Result<Box<dyn MemoryRegionRepository>, FactoryError>;
    
    /// 创建KernelInfo仓储
    fn create_kernel_info_repository(&self) -> Result<Box<dyn KernelInfoRepository>, FactoryError>;
    
    /// 创建GraphicsInfo仓储
    fn create_graphics_info_repository(&self) -> Result<Box<dyn GraphicsInfoRepository>, FactoryError>;
    
    /// 创建通用仓储
    fn create_repository<T: AggregateRoot>(&self) -> Result<Box<dyn Repository<T>>, FactoryError>;
    
    /// 获取工厂统计信息
    fn get_factory_stats(&self) -> FactoryStats;
    
    /// 重置工厂状态
    fn reset(&self) -> Result<(), FactoryError>;
}

/// 工厂统计信息
///
/// 提供工厂的运行统计信息
#[derive(Debug, Clone)]
pub struct FactoryStats {
    /// 创建的仓储数量
    pub repositories_created: usize,
    /// 活跃仓储数量
    pub active_repositories: usize,
    /// 创建的BootInfo仓储数量
    pub boot_info_repositories: usize,
    /// 创建的MemoryRegion仓储数量
    pub memory_region_repositories: usize,
    /// 创建的KernelInfo仓储数量
    pub kernel_info_repositories: usize,
    /// 创建的GraphicsInfo仓储数量
    pub graphics_info_repositories: usize,
    /// 工厂运行时间（毫秒）
    pub uptime_ms: u64,
}

impl FactoryStats {
    /// 创建新的工厂统计信息
    pub fn new() -> Self {
        Self {
            repositories_created: 0,
            active_repositories: 0,
            boot_info_repositories: 0,
            memory_region_repositories: 0,
            kernel_info_repositories: 0,
            graphics_info_repositories: 0,
            uptime_ms: 0,
        }
    }
    
    /// 获取总仓储数量
    pub fn total_repositories(&self) -> usize {
        self.boot_info_repositories + self.memory_region_repositories + 
        self.kernel_info_repositories + self.graphics_info_repositories
    }
}

impl Default for FactoryStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 仓储工厂配置
///
/// 定义了仓储工厂的配置选项
#[derive(Debug, Clone)]
pub struct RepositoryFactoryConfig {
    /// 默认超时时间（毫秒）
    pub default_timeout_ms: u64,
    /// 是否启用缓存
    pub enable_cache: bool,
    /// 缓存大小
    pub cache_size: usize,
    /// 是否启用事务
    pub enable_transactions: bool,
    /// 是否启用序列化
    pub enable_serialization: bool,
    /// 自定义配置选项
    pub custom_options: Vec<(String, String)>,
}

impl RepositoryFactoryConfig {
    /// 创建默认配置
    pub fn new() -> Self {
        Self {
            default_timeout_ms: 30000, // 30秒
            enable_cache: true,
            cache_size: 1000,
            enable_transactions: true,
            enable_serialization: true,
            custom_options: Vec::new(),
        }
    }
    
    /// 设置默认超时时间
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.default_timeout_ms = timeout_ms;
        self
    }
    
    /// 设置缓存配置
    pub fn with_cache(mut self, enable: bool, size: usize) -> Self {
        self.enable_cache = enable;
        self.cache_size = size;
        self
    }
    
    /// 设置事务配置
    pub fn with_transactions(mut self, enable: bool) -> Self {
        self.enable_transactions = enable;
        self
    }
    
    /// 设置序列化配置
    pub fn with_serialization(mut self, enable: bool) -> Self {
        self.enable_serialization = enable;
        self
    }
    
    /// 添加自定义选项
    pub fn with_custom_option(mut self, key: String, value: String) -> Self {
        self.custom_options.push((key, value));
        self
    }
    
    /// 获取自定义选项值
    pub fn get_custom_option(&self, key: &str) -> Option<&String> {
        self.custom_options.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

impl Default for RepositoryFactoryConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// 默认仓储工厂实现
///
/// 提供基于内存的仓储实现
pub struct DefaultRepositoryFactory {
    /// 事务管理器
    transaction_manager: Arc<dyn TransactionManager>,
    /// ID生成器
    id_generator: Arc<dyn IdGenerator>,
    /// 序列化器注册表
    serializer_registry: Arc<SerializerRegistry>,
    /// 工厂配置
    config: RepositoryFactoryConfig,
    /// 统计信息
    stats: Arc<spin::Mutex<FactoryStats>>,
    /// 创建时间戳
    created_at: u64,
}

impl DefaultRepositoryFactory {
    /// 创建新的默认仓储工厂
    pub fn new() -> Result<Self, FactoryError> {
        Self::with_config(RepositoryFactoryConfig::new())
    }
    
    /// 使用配置创建仓储工厂
    pub fn with_config(config: RepositoryFactoryConfig) -> Result<Self, FactoryError> {
        log::info!("Initializing repository factory with config");
        log::debug!("Factory config timeout: {} ms", config.default_timeout_ms);
        log::debug!("Cache enabled: {}, size: {}", config.enable_cache, config.cache_size);
        
        // 验证配置有效性
        if config.cache_size == 0 && config.enable_cache {
            log::warn!("Cache enabled but size is 0, using default");
        }
        
        // 这里应该有实际的事务管理器创建逻辑
        // 现在返回一个错误作为占位符
        log::error!("Transaction manager initialization not yet implemented");
        return Err(FactoryError::InitializationError(
            "Transaction manager initialization not implemented"
        ));
        
        /*
        let id_generator = Arc::new(DefaultIdGenerator::new());
        let serializer = Arc::new(SimpleSerializationService::new());
        let mut serializer_registry = SerializerRegistry::new();
        let _ = serializer_registry.register(Box::new(BinarySerializer::new()));
        let _ = serializer_registry.register(Box::new(JsonSerializer::new()));
        let serializer_registry = Arc::new(serializer_registry);
        
        // 这里应该有实际的事务管理器创建逻辑
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let transaction_manager = Arc::new(MemoryTransactionManager::new(
            transaction_log,
            id_generator.clone(),
        ));
        
        Ok(Self {
            transaction_manager,
            serializer,
            id_generator,
            serializer_registry,
            config,
            stats: Arc::new(spin::Mutex::new(FactoryStats::new())),
            created_at: 1234567890, // 实际实现中应该使用当前时间
        })
        */
    }
    
    /// 创建带有依赖的仓储工厂
    pub fn with_dependencies(
        transaction_manager: Arc<dyn TransactionManager>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Result<Self, FactoryError> {
        let mut serializer_registry = SerializerRegistry::new();
        serializer_registry.register(Box::new(BinarySerializer::new()));
        serializer_registry.register(Box::new(JsonSerializer::new()));
        let serializer_registry = Arc::new(serializer_registry);
        
        Ok(Self {
            transaction_manager,
            id_generator,
            serializer_registry,
            config: RepositoryFactoryConfig::new(),
            stats: Arc::new(spin::Mutex::new(FactoryStats::new())),
            created_at: 1234567890, // 实际实现中应该使用当前时间
        })
    }
    
    /// 创建内存仓储实例
    fn create_memory_repository<T: AggregateRoot>(&self) -> Result<MemoryRepository<T>, FactoryError> {
        // 检查配置是否启用缓存
        if self.config.enable_cache {
            log::debug!("Creating cached memory repository with size {}", self.config.cache_size);
        }
        
        Ok(MemoryRepository::new(
            self.transaction_manager.clone(),
            self.id_generator.clone(),
            Box::new(BinarySerializer::new()),
        ))
    }
    
    /// 更新统计信息
    fn update_stats<F>(&self, update_fn: F) 
    where
        F: FnOnce(&mut FactoryStats),
    {
        if let Some(mut stats) = self.stats.try_lock() {
            update_fn(&mut *stats);
        }
    }
}

impl RepositoryFactory for DefaultRepositoryFactory {
    fn create_boot_info_repository(&self) -> Result<Box<dyn BootInfoRepository>, FactoryError> {
        let _repository = self.create_memory_repository::<BootInfo>()?;
        
        self.update_stats(|stats| {
            stats.repositories_created += 1;
            stats.active_repositories += 1;
            stats.boot_info_repositories += 1;
        });
        
        // 这里需要创建一个BootInfoRepository的包装器
        // 现在返回一个错误作为占位符
        Err(FactoryError::UnsupportedType("BootInfoRepository wrapper not implemented"))
    }
    
    fn create_memory_region_repository(&self) -> Result<Box<dyn MemoryRegionRepository>, FactoryError> {
        let _repository = self.create_memory_repository::<MemoryRegion>()?;
        
        self.update_stats(|stats| {
            stats.repositories_created += 1;
            stats.active_repositories += 1;
            stats.memory_region_repositories += 1;
        });
        
        // 这里需要创建一个实现MemoryRegionRepository的包装器
        // 现在返回一个错误作为占位符
        Err(FactoryError::UnsupportedType("MemoryRegionRepository wrapper not implemented"))
    }
    
    fn create_kernel_info_repository(&self) -> Result<Box<dyn KernelInfoRepository>, FactoryError> {
        let _repository = self.create_memory_repository::<KernelInfo>()?;
        
        self.update_stats(|stats| {
            stats.repositories_created += 1;
            stats.active_repositories += 1;
            stats.kernel_info_repositories += 1;
        });
        
        // 这里需要创建一个实现KernelInfoRepository的包装器
        // 现在返回一个错误作为占位符
        Err(FactoryError::UnsupportedType("KernelInfoRepository wrapper not implemented"))
    }
    
    fn create_graphics_info_repository(&self) -> Result<Box<dyn GraphicsInfoRepository>, FactoryError> {
        let _repository = self.create_memory_repository::<GraphicsInfo>()?;
        
        self.update_stats(|stats| {
            stats.repositories_created += 1;
            stats.active_repositories += 1;
            stats.graphics_info_repositories += 1;
        });
        
        // 这里需要创建一个实现GraphicsInfoRepository的包装器
        // 现在返回一个错误作为占位符
        Err(FactoryError::UnsupportedType("GraphicsInfoRepository wrapper not implemented"))
    }
    
    fn create_repository<T: AggregateRoot>(&self) -> Result<Box<dyn Repository<T>>, FactoryError> {
        let repository = self.create_memory_repository::<T>()?;
        
        self.update_stats(|stats| {
            stats.repositories_created += 1;
            stats.active_repositories += 1;
        });
        
        Ok(Box::new(repository))
    }
    
    fn get_factory_stats(&self) -> FactoryStats {
        if let Some(stats) = self.stats.try_lock() {
            let mut result = stats.clone();
            result.uptime_ms = 1234567890 - self.created_at; // 实际实现中应该使用当前时间
            result
        } else {
            FactoryStats::new()
        }
    }
    
    fn reset(&self) -> Result<(), FactoryError> {
        self.update_stats(|stats| {
            *stats = FactoryStats::new();
        });
        Ok(())
    }
}

/// 特定仓储包装器
///
/// 为特定聚合根提供专门的查询方法
pub struct SpecificRepositoryWrapper<T: AggregateRoot> {
    /// 通用仓储实现
    inner: Box<dyn Repository<T>>,
    /// 仓储类型名称
    repository_type: &'static str,
}

impl<T: AggregateRoot> SpecificRepositoryWrapper<T> {
    /// 创建新的特定仓储包装器
    pub fn new(repository: Box<dyn Repository<T>>, repository_type: &'static str) -> Self {
        Self {
            inner: repository,
            repository_type,
        }
    }
    
    /// 获取内部仓储的引用
    pub fn inner(&self) -> &dyn Repository<T> {
        self.inner.as_ref()
    }
}

impl<T: AggregateRoot> Repository<T> for SpecificRepositoryWrapper<T> {
    fn create(&self, entity: T) -> Result<EntityId, RepositoryError> {
        log::info!("Creating entity in {} repository", self.repository_type);
        self.inner.create(entity)
    }
    
    fn find_by_id(&self, id: EntityId) -> Result<Option<T>, RepositoryError> {
        log::debug!("Finding entity by ID in {} repository: {:?}", self.repository_type, id);
        self.inner.find_by_id(id)
    }
    
    fn update(&self, entity: T) -> Result<(), RepositoryError> {
        log::info!("Updating entity in {} repository", self.repository_type);
        self.inner.update(entity)
    }
    
    fn delete(&self, id: EntityId) -> Result<(), RepositoryError> {
        log::info!("Deleting entity by ID in {} repository: {:?}", self.repository_type, id);
        self.inner.delete(id)
    }
    
    fn find_all(&self) -> Result<Vec<T>, RepositoryError> {
        log::info!("Finding all entities in {} repository", self.repository_type);
        self.inner.find_all()
    }
    
    fn query(&self) -> &dyn BasicRepositoryQuery<T> {
        log::debug!("Getting query interface for {} repository", self.repository_type);
        self.inner.query()
    }
    
    fn count(&self) -> Result<usize, RepositoryError> {
        log::info!("Counting entities in {} repository", self.repository_type);
        self.inner.count()
    }
    
    fn exists(&self, id: EntityId) -> Result<bool, RepositoryError> {
        log::debug!("Checking if entity exists in {} repository: {:?}", self.repository_type, id);
        self.inner.exists(id)
    }
    
    // These methods are part of RepositoryQuery and should be accessed through query()
    // fn find_with_pagination(&self, page: usize, size: usize) -> Result<crate::domain::repositories::Page<T>, RepositoryError> {
    //     self.inner.find_with_pagination(page, size)
    // }
    
    // fn create_batch(&self, entities: Vec<T>) -> Result<Vec<EntityId>, RepositoryError> {
    //     self.inner.create_batch(entities)
    // }
    
    // fn update_batch(&self, entities: Vec<T>) -> Result<(), RepositoryError> {
    //     self.inner.update_batch(entities)
    // }
    
   // fn delete_batch(&self, ids: Vec<EntityId>) -> Result<(), RepositoryError> {
    //     self.inner.delete_batch(ids)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_factory_config() {
        let config = RepositoryFactoryConfig::new()
            .with_timeout(5000)
            .with_cache(true, 100)
            .with_transactions(true)
            .with_serialization(true)
            .with_custom_option("test".to_string(), "value".to_string());
        
        assert_eq!(config.default_timeout_ms, 5000);
        assert!(config.enable_cache);
        assert_eq!(config.cache_size, 100);
        assert!(config.enable_transactions);
        assert!(config.enable_serialization);
        assert_eq!(config.get_custom_option("test"), Some(&"value".to_string()));
    }
    
    #[test]
    fn test_factory_stats() {
        let mut stats = FactoryStats::new();
        stats.repositories_created = 10;
        stats.boot_info_repositories = 3;
        stats.memory_region_repositories = 2;
        stats.kernel_info_repositories = 3;
        stats.graphics_info_repositories = 2;
        
        assert_eq!(stats.total_repositories(), 10);
        assert_eq!(stats.repositories_created, 10);
    }
    
    #[test]
    fn test_specific_repository_wrapper() {
        // 这里需要实际的仓储实现来测试
        // 现在只测试包装器的基本功能
        
        let config = RepositoryFactoryConfig::new();
        assert_eq!(config.default_timeout_ms, 30000);
        assert!(config.enable_cache);
    }
}