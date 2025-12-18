//! 序列化基础设施模块
//!
//! 提供序列化和反序列化的基础设施实现，包括多种格式支持
//! 和序列化器注册表。

pub mod simple_serializer;
pub mod serializer_registry;

// 重新导出主要类型
pub use simple_serializer::SimpleSerializer;
pub use serializer_registry::SerializerRegistry;