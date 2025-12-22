//! 仓储基础设施模块
//!
//! 提供仓储模式的基础设施实现，包括内存存储、工厂模式和序列化支持。

pub mod memory_repository;
pub mod repository_factory;

// 重新导出主要类型
pub use memory_repository::MemoryRepository;
pub use repository_factory::{
    RepositoryFactory, DefaultRepositoryFactory, FactoryError, FactoryStats,
    RepositoryFactoryConfig, SpecificRepositoryWrapper
};