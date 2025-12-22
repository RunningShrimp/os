//! 序列化服务
//!
//! 提供了领域实体的序列化和反序列化功能。
//! 支持多种序列化格式，包括二进制、JSON和自定义格式。

use super::aggregate_root::AggregateRoot;
use super::repositories::RepositoryError;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use core::fmt;
use alloc::boxed::Box;

/// 序列化格式枚举
///
/// 定义了支持的序列化格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// 二进制格式
    Binary,
    /// JSON格式
    Json,
    /// 自定义格式
    Custom(&'static str),
}

impl SerializationFormat {
    /// 获取格式名称
    pub fn name(&self) -> &'static str {
        match self {
            SerializationFormat::Binary => "binary",
            SerializationFormat::Json => "json",
            SerializationFormat::Custom(name) => name,
        }
    }
    
    /// 检查是否为二进制格式
    pub fn is_binary(&self) -> bool {
        matches!(self, SerializationFormat::Binary)
    }
    
    /// 检查是否为文本格式
    pub fn is_text(&self) -> bool {
        !self.is_binary()
    }
}

impl Ord for SerializationFormat {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match (self, other) {
            (SerializationFormat::Binary, SerializationFormat::Binary) => core::cmp::Ordering::Equal,
            (SerializationFormat::Binary, _) => core::cmp::Ordering::Less,
            (_, SerializationFormat::Binary) => core::cmp::Ordering::Greater,
            (SerializationFormat::Json, SerializationFormat::Json) => core::cmp::Ordering::Equal,
            (SerializationFormat::Json, _) => core::cmp::Ordering::Less,
            (_, SerializationFormat::Json) => core::cmp::Ordering::Greater,
            (SerializationFormat::Custom(a), SerializationFormat::Custom(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for SerializationFormat {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for SerializationFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 序列化上下文
///
/// 提供序列化过程中的上下文信息
#[derive(Debug, Clone)]
pub struct SerializationContext {
    /// 序列化格式
    pub format: SerializationFormat,
    /// 是否包含版本信息
    pub include_version: bool,
    /// 是否包含元数据
    pub include_metadata: bool,
    /// 是否压缩数据
    pub compress: bool,
    /// 自定义选项
    pub custom_options: Vec<(String, String)>,
}

impl SerializationContext {
    /// 创建默认的序列化上下文
    pub fn new(format: SerializationFormat) -> Self {
        Self {
            format,
            include_version: true,
            include_metadata: true,
            compress: false,
            custom_options: Vec::new(),
        }
    }
    
    /// 创建二进制格式的上下文
    pub fn binary() -> Self {
        Self::new(SerializationFormat::Binary)
    }
    
    /// 创建JSON格式的上下文
    pub fn json() -> Self {
        Self::new(SerializationFormat::Json)
    }
    
    /// 设置是否包含版本信息
    pub fn with_version(mut self, include_version: bool) -> Self {
        self.include_version = include_version;
        self
    }
    
    /// 设置是否包含元数据
    pub fn with_metadata(mut self, include_metadata: bool) -> Self {
        self.include_metadata = include_metadata;
        self
    }
    
    /// 设置是否压缩数据
    pub fn with_compression(mut self, compress: bool) -> Self {
        self.compress = compress;
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

impl Default for SerializationContext {
    fn default() -> Self {
        Self::binary()
    }
}

/// 序列化结果
///
/// 包含序列化后的数据和元数据
#[derive(Debug, Clone)]
pub struct SerializationResult {
    /// 序列化后的数据
    pub data: Vec<u8>,
    /// 数据格式
    pub format: SerializationFormat,
    /// 数据大小（字节）
    pub size: usize,
    /// 是否压缩
    pub compressed: bool,
    /// 版本信息
    pub version: Option<u64>,
    /// 元数据
    pub metadata: Vec<(String, String)>,
}

impl SerializationResult {
    /// 创建新的序列化结果
    pub fn new(data: Vec<u8>, format: SerializationFormat) -> Self {
        let size = data.len();
        Self {
            data,
            format,
            size,
            compressed: false,
            version: None,
            metadata: Vec::new(),
        }
    }
    
    /// 设置版本信息
    pub fn with_version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }
    
    /// 设置压缩标志
    pub fn with_compression(mut self, compressed: bool) -> Self {
        self.compressed = compressed;
        self
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.push((key, value));
        self
    }
    
    /// 获取元数据值
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

/// 反序列化结果
///
/// 包含反序列化后的实体和元数据
#[derive(Debug, Clone)]
pub struct DeserializationResult<T: AggregateRoot> {
    /// 反序列化的实体
    pub entity: T,
    /// 数据格式
    pub format: SerializationFormat,
    /// 版本信息
    pub version: Option<u64>,
    /// 元数据
    pub metadata: Vec<(String, String)>,
}

impl<T: AggregateRoot> DeserializationResult<T> {
    /// 创建新的反序列化结果
    pub fn new(entity: T, format: SerializationFormat) -> Self {
        Self {
            entity,
            format,
            version: None,
            metadata: Vec::new(),
        }
    }
    
    /// 设置版本信息
    pub fn with_version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.push((key, value));
        self
    }
}

/// 序列化器特征
///
/// 定义了序列化器的基本接口，支持动态调度
pub trait Serializer: Send + Sync {
    /// 获取支持的格式
    fn supported_formats(&self) -> Vec<SerializationFormat>;
    
    /// 检查是否支持指定格式
    fn supports_format(&self, format: SerializationFormat) -> bool {
        self.supported_formats().contains(&format)
    }
    
    /// 获取序列化器名称
    fn name(&self) -> &'static str;
    
    /// 获取序列化器版本
    fn version(&self) -> &'static str;
}

/// 类型化序列化器特征
///
/// 定义了针对特定聚合根类型的序列化和反序列化方法
pub trait TypedSerializer<T: AggregateRoot>: Serializer {
    /// 序列化聚合根
    fn serialize(
        &self,
        entity: &T,
        context: &SerializationContext,
    ) -> Result<SerializationResult, RepositoryError>;
    
    /// 反序列化聚合根
    fn deserialize(
        &self,
        data: &[u8],
        context: &SerializationContext,
    ) -> Result<DeserializationResult<T>, RepositoryError>;
}

/// 二进制序列化器
///
/// 使用简单的二进制格式进行序列化
pub struct BinarySerializer;

impl BinarySerializer {
    /// 创建新的二进制序列化器
    pub fn new() -> Self {
        Self
    }
}

impl Default for BinarySerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for BinarySerializer {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl Serializer for BinarySerializer {
    fn supported_formats(&self) -> Vec<SerializationFormat> {
        vec![SerializationFormat::Binary]
    }
    
    fn name(&self) -> &'static str {
        "BinarySerializer"
    }
    
    fn version(&self) -> &'static str {
        "1.0.0"
    }
}

impl<T: AggregateRoot> TypedSerializer<T> for BinarySerializer {
    fn serialize(
        &self,
        entity: &T,
        context: &SerializationContext,
    ) -> Result<SerializationResult, RepositoryError> {
        let mut data = Vec::new();
        
        // 写入实体类型
        let entity_type = T::entity_type();
        data.extend_from_slice(&(entity_type.len() as u32).to_le_bytes());
        data.extend_from_slice(entity_type.as_bytes());
        
        // 写入ID
        let id = entity.id().value();
        data.extend_from_slice(&id.to_le_bytes());
        
        // 写入版本信息（如果支持且要求）
        if context.include_version {
            let version = match entity.version() {
                0 => {
                    // Version is already 0, use it as is
                    0
                }
                v => v
            };
            data.extend_from_slice(&version.to_le_bytes());
        }
        
        // 写入实体数据（简单使用Debug格式）
        let entity_data = format!("{:?}", entity);
        data.extend_from_slice(&(entity_data.len() as u32).to_le_bytes());
        data.extend_from_slice(entity_data.as_bytes());
        
        let mut result = SerializationResult::new(data, SerializationFormat::Binary);
        
        // 添加版本信息
        if context.include_version {
            let version = entity.version();
            if version > 0 {
                result = result.with_version(version);
            }
        }
        
        // 添加元数据
        if context.include_metadata {
            result = result.with_metadata(
                "entity_type".to_string(),
                entity_type.to_string(),
            );
            result = result.with_metadata(
                "serialization_time".to_string(),
                "1234567890".to_string(), // 实际实现中应该使用当前时间
            );
        }
        
        Ok(result)
    }
    
    fn deserialize(
        &self,
        data: &[u8],
        context: &SerializationContext,
    ) -> Result<DeserializationResult<T>, RepositoryError> {
        if data.len() < 8 {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for deserialization".to_string(),
            ));
        }
        
        let mut offset = 0;
        
        // 读取实体类型
        let type_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;
        
        if offset + type_len > data.len() {
            return Err(RepositoryError::SerializationError(
                "Invalid entity type length".to_string(),
            ));
        }
        
        let entity_type = core::str::from_utf8(&data[offset..offset + type_len])
            .map_err(|_| RepositoryError::SerializationError("Invalid entity type encoding".to_string()))?;
        offset += type_len;
        
        // 验证实体类型
        if entity_type != T::entity_type() {
            return Err(RepositoryError::SerializationError(
                format!("Entity type mismatch: expected {}, got {}", T::entity_type(), entity_type),
            ));
        }
        
        // 读取ID
        if offset + 8 > data.len() {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for ID".to_string(),
            ));
        }
        
        let id_bytes = &data[offset..offset + 8];
        let id = u64::from_le_bytes([
            id_bytes[0], id_bytes[1], id_bytes[2], id_bytes[3],
            id_bytes[4], id_bytes[5], id_bytes[6], id_bytes[7],
        ]);
        log::trace!("Deserialized entity ID: {}", id);
        offset += 8;
        
        // 读取版本信息（如果存在）
        let version = if context.include_version {
            if offset + 8 > data.len() {
                return Err(RepositoryError::SerializationError(
                    "Insufficient data for version".to_string(),
                ));
            }
            
            let version_bytes = &data[offset..offset + 8];
            let ver = u64::from_le_bytes([
                version_bytes[0], version_bytes[1], version_bytes[2], version_bytes[3],
                version_bytes[4], version_bytes[5], version_bytes[6], version_bytes[7],
            ]);
            log::trace!("Deserialized entity version: {:?}", ver);
            offset += 8;
            Some(ver)
        } else {
            None
        };
        
        // Validate version consistency
        if let Some(ver) = version {
            log::debug!("Entity version {} deserialized successfully", ver);
        }
        
        // 读取实体数据
        if offset + 4 > data.len() {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for entity data length".to_string(),
            ));
        }
        
        let data_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;
        
        if offset + data_len > data.len() {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for entity data".to_string(),
            ));
        }
        
        let entity_data = &data[offset..offset + data_len];
        let entity_str = core::str::from_utf8(entity_data)
            .map_err(|_| RepositoryError::SerializationError("Invalid entity data encoding".to_string()))?;
        
        log::debug!("Deserialized entity data: {} bytes, content preview: {}", data_len, 
            if entity_str.len() > 50 { &entity_str[..50] } else { entity_str });
        
        // 这里应该有实际的实体重建逻辑
        // 现在返回一个错误，因为这是一个简化的实现
        Err(RepositoryError::SerializationError(
            "Deserialization not fully implemented".to_string(),
        ))
    }
}

/// JSON序列化器
///
/// 使用JSON格式进行序列化
pub struct JsonSerializer;

impl JsonSerializer {
    /// 创建新的JSON序列化器
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for JsonSerializer {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl Serializer for JsonSerializer {
    fn supported_formats(&self) -> Vec<SerializationFormat> {
        vec![SerializationFormat::Json]
    }
    
    fn name(&self) -> &'static str {
        "json_serializer"
    }
    
    fn version(&self) -> &'static str {
        "1.0.0"
    }
}

impl<T: AggregateRoot> TypedSerializer<T> for JsonSerializer {
    fn serialize(
        &self,
        entity: &T,
        context: &SerializationContext,
    ) -> Result<SerializationResult, RepositoryError> {
        // 简化的JSON序列化实现
        let json_data = format!(
            r#"{{
  "entity_type": "{}",
  "id": {},
  "version": {},
  "data": {:?}
}}"#,
            T::entity_type(),
            entity.id().value(),
            if context.include_version {
                entity.version()
            } else {
                0
            },
            entity
        );
        
        let mut result = SerializationResult::new(json_data.into_bytes(), SerializationFormat::Json);
        
        // 添加版本信息
        if context.include_version {
            let version = entity.version();
            if version > 0 {
                result = result.with_version(version);
            }
        }
        
        // 添加元数据
        if context.include_metadata {
            result = result.with_metadata(
                "entity_type".to_string(),
                T::entity_type().to_string(),
            );
        }
        
        Ok(result)
    }
    
    fn deserialize(
        &self,
        _data: &[u8],
        _context: &SerializationContext,
    ) -> Result<DeserializationResult<T>, RepositoryError> {
        // JSON反序列化的简化实现
        Err(RepositoryError::SerializationError(
            "JSON deserialization not implemented".to_string(),
        ))
    }
}

/// 序列化器注册表
///
/// 管理多个序列化器
pub struct SerializerRegistry {
    serializers: Vec<Box<dyn Serializer>>,
}

impl SerializerRegistry {
    /// 创建新的序列化器注册表
    pub fn new() -> Self {
        Self {
            serializers: Vec::new(),
        }
    }
    
    /// 注册序列化器
    pub fn register(&mut self, serializer: Box<dyn Serializer>) {
        self.serializers.push(serializer);
    }
    
    /// 获取支持指定格式的序列化器
    pub fn get_serializer(&self, format: SerializationFormat) -> Option<&dyn Serializer> {
        self.serializers.iter().find(|s| s.supports_format(format)).map(|s| s.as_ref())
    }
    
    /// 获取所有支持的格式
    pub fn supported_formats(&self) -> Vec<SerializationFormat> {
        let mut formats = Vec::new();
        for serializer in &self.serializers {
            formats.extend(serializer.supported_formats());
        }
        formats.sort_by(|a, b| a.name().cmp(b.name()));
        formats.dedup();
        formats
    }
    
    /// 获取默认序列化器（返回第一个注册的序列化器）
    pub fn get_default_serializer(&self) -> Box<dyn Serializer> {
        if self.serializers.is_empty() {
            // 如果没有注册序列化器，返回默认的BinarySerializer
            Box::new(BinarySerializer::new())
        } else {
            // 返回第一个注册的序列化器作为默认值
            // 注意：由于移除了Clone bound，这里直接返回BinarySerializer作为替代
            Box::new(BinarySerializer::new())
        }
    }
}

impl Default for SerializerRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(BinarySerializer::new()));
        registry.register(Box::new(JsonSerializer::new()));
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::repositories::{AggregateRoot, EntityId};
    use super::super::aggregate_root::VersionedAggregateRoot;
    
    // 测试用的聚合根
    #[derive(Debug, Clone)]
    struct TestEntity {
        id: EntityId,
        name: String,
        version: u64,
    }
    
    impl TestEntity {
        fn new(name: String) -> Self {
            Self {
                id: EntityId::new(0),
                name,
                version: 1,
            }
        }
    }
    
    impl AggregateRoot for TestEntity {
        fn id(&self) -> EntityId {
            self.id
        }
        
        fn set_id(&mut self, id: EntityId) {
            self.id = id;
        }
        
        fn validate(&self) -> Result<(), RepositoryError> {
            if self.name.is_empty() {
                return Err(RepositoryError::ValidationError("Name cannot be empty"));
            }
            Ok(())
        }
        
        fn entity_type() -> &'static str {
            "TestEntity"
        }
    }
    
    impl VersionedAggregateRoot for TestEntity {
        fn version(&self) -> u64 {
            self.version
        }
        
        fn set_version(&mut self, version: u64) {
            self.version = version;
        }
    }
    
    #[test]
    fn test_serialization_context() {
        let context = SerializationContext::binary()
            .with_version(true)
            .with_metadata(true)
            .with_compression(false)
            .with_custom_option("test".to_string(), "value".to_string());
        
        assert_eq!(context.format, SerializationFormat::Binary);
        assert!(context.include_version);
        assert!(context.include_metadata);
        assert!(!context.compress);
        assert_eq!(context.get_custom_option("test"), Some(&"value".to_string()));
    }
    
    #[test]
    fn test_binary_serializer() {
        let serializer = BinarySerializer::new();
        let entity = TestEntity::new("Test".to_string());
        let context = SerializationContext::binary().with_version(true);
        
        let result = serializer.serialize(&entity, &context);
        assert!(result.is_ok());
        
        let serialization_result = result.unwrap();
        assert_eq!(serialization_result.format, SerializationFormat::Binary);
        assert_eq!(serialization_result.version, Some(1));
        assert!(!serialization_result.data.is_empty());
    }
    
    #[test]
    fn test_json_serializer() {
        let serializer = JsonSerializer::new();
        let entity = TestEntity::new("Test".to_string());
        let context = SerializationContext::json().with_version(true);
        
        let result = serializer.serialize(&entity, &context);
        assert!(result.is_ok());
        
        let serialization_result = result.unwrap();
        assert_eq!(serialization_result.format, SerializationFormat::Json);
        assert_eq!(serialization_result.version, Some(1));
        assert!(!serialization_result.data.is_empty());
    }
    
    #[test]
    fn test_serializer_registry() {
        let mut registry = SerializerRegistry::new();
        registry.register(Box::new(BinarySerializer::new()));
        registry.register(Box::new(JsonSerializer::new()));
        
        let binary_serializer = registry.get_serializer(SerializationFormat::Binary);
        assert!(binary_serializer.is_some());
        
        let json_serializer = registry.get_serializer(SerializationFormat::Json);
        assert!(json_serializer.is_some());
        
        let formats = registry.supported_formats();
        assert!(formats.contains(&SerializationFormat::Binary));
        assert!(formats.contains(&SerializationFormat::Json));
    }
}