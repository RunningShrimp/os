//! 简单序列化器实现
//!
//! 提供基本的序列化和反序列化功能，使用简单的字节操作。
//! 适用于快速原型开发和测试场景。

use crate::domain::serialization::{
    Serializer, TypedSerializer, SerializationContext, SerializationResult, DeserializationResult,
    SerializationFormat
};
use crate::domain::aggregate_root::AggregateRoot;
use crate::domain::repositories::RepositoryError;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;

/// 简单序列化器
///
/// 提供基本的序列化功能，使用简单的字节格式
pub struct SimpleSerializer;

impl SimpleSerializer {
    /// 创建新的简单序列化器
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer for SimpleSerializer {
    fn supported_formats(&self) -> Vec<SerializationFormat> {
        vec![
            SerializationFormat::Binary,
            SerializationFormat::Json,
        ]
    }
    
    fn name(&self) -> &'static str {
        "SimpleSerializer"
    }
    
    fn version(&self) -> &'static str {
        "1.0.0"
    }
}

impl<T: AggregateRoot> TypedSerializer<T> for SimpleSerializer {
    fn serialize(
        &self,
        entity: &T,
        context: &SerializationContext,
    ) -> Result<SerializationResult, RepositoryError> {
        match context.format {
            SerializationFormat::Binary => self.serialize_binary(entity, context),
            SerializationFormat::Json => self.serialize_json(entity, context),
            SerializationFormat::Custom(name) => Err(RepositoryError::SerializationError(
                format!("Unsupported serialization format: {}", name)
            )),
        }
    }
    
    fn deserialize(
        &self,
        data: &[u8],
        context: &SerializationContext,
    ) -> Result<DeserializationResult<T>, RepositoryError> {
        match context.format {
            SerializationFormat::Binary => self.deserialize_binary::<T>(data, context),
            SerializationFormat::Json => self.deserialize_json::<T>(data, context),
            SerializationFormat::Custom(name) => Err(RepositoryError::SerializationError(
                format!("Unsupported deserialization format: {}", name)
            )),
        }
    }
}

impl SimpleSerializer {
    /// 二进制序列化
    fn serialize_binary<T: AggregateRoot>(
        &self,
        entity: &T,
        context: &SerializationContext,
    ) -> Result<SerializationResult, RepositoryError> {
        let mut data = Vec::new();
        
        // 写入魔数（用于标识格式）
        data.extend_from_slice(b"SSER"); // Simple Serializer
        
        // 写入版本信息
        if context.include_version {
            let version = entity.version();
            data.extend_from_slice(&version.to_le_bytes());
        }
        
        // 写入实体类型
        let entity_type = T::entity_type();
        data.extend_from_slice(&(entity_type.len() as u16).to_le_bytes());
        data.extend_from_slice(entity_type.as_bytes());
        
        // 写入实体ID
        let id = entity.id().value();
        data.extend_from_slice(&id.to_le_bytes());
        
        // 写入实体数据长度
        let entity_data = format!("{:?}", entity);
        let entity_data_bytes = entity_data.as_bytes();
        data.extend_from_slice(&(entity_data_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(entity_data_bytes);
        
        // 写入元数据
        if context.include_metadata {
            let metadata = self.create_metadata(entity);
            let metadata_bytes = metadata.as_bytes();
            data.extend_from_slice(&(metadata_bytes.len() as u16).to_le_bytes());
            data.extend_from_slice(metadata_bytes);
        }
        
        let mut result = SerializationResult::new(data, SerializationFormat::Binary);
        
        // 添加版本信息
        if context.include_version {
            result = result.with_version(entity.version());
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
    
    /// JSON序列化
    fn serialize_json<T: AggregateRoot>(
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
  "data": {:?},
  "metadata": {{
    "type": "{}",
    "serialized_at": "1234567890"
  }}
}}"#,
            T::entity_type(),
            entity.id().value(),
            if context.include_version {
                entity.version()
            } else {
                0
            },
            entity,
            T::entity_type()
        );
        
        let mut result = SerializationResult::new(json_data.into_bytes(), SerializationFormat::Json);
        
        // 添加版本信息
        if context.include_version {
            result = result.with_version(entity.version());
        }
        
        // 添加元数据
        if context.include_metadata {
            result = result.with_metadata(
                "entity_type".to_string(),
                T::entity_type().to_string(),
            );
            result = result.with_metadata(
                "serialization_time".to_string(),
                "1234567890".to_string(), // 实际实现中应该使用当前时间
            );
        }
        
        Ok(result)
    }
    
    /// 二进制反序列化
    fn deserialize_binary<T: AggregateRoot>(
        &self,
        data: &[u8],
        context: &SerializationContext,
    ) -> Result<DeserializationResult<T>, RepositoryError> {
        if data.len() < 4 {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for deserialization".to_string()
            ));
        }
        
        // 检查魔数
        if &data[0..4] != b"SSER" {
            return Err(RepositoryError::SerializationError(
                "Invalid serialization format".to_string()
            ));
        }
        
        let mut offset = 4;
        
        // 读取版本信息
        let version = if context.include_version {
            if offset + 8 > data.len() {
                return Err(RepositoryError::SerializationError(
                    "Insufficient data for version".to_string()
                ));
            }
            
            let version_bytes = &data[offset..offset + 8];
            let ver = u64::from_le_bytes([
                version_bytes[0], version_bytes[1], version_bytes[2], version_bytes[3],
                version_bytes[4], version_bytes[5], version_bytes[6], version_bytes[7],
            ]);
            log::trace!("Deserialized version: {}", ver);
            offset += 8;
            Some(ver)
        } else {
            None
        };
        
        // Validate version information
        if let Some(ver) = version {
            log::debug!("Version {} loaded from serialized data", ver);
        }
        
        // 读取实体类型
        if offset + 2 > data.len() {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for entity type".to_string()
            ));
        }
        
        let type_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        
        if offset + type_len > data.len() {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for entity type".to_string()
            ));
        }
        
        let entity_type = core::str::from_utf8(&data[offset..offset + type_len])
            .map_err(|_| RepositoryError::SerializationError("Invalid entity type encoding".to_string()))?;
        offset += type_len;
        
        // 验证实体类型
        if entity_type != T::entity_type() {
            return Err(RepositoryError::SerializationError(
                format!("Entity type mismatch: expected {}, got {}", T::entity_type(), entity_type)
            ));
        }
        
        // 读取实体ID
        if offset + 8 > data.len() {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for entity ID".to_string()
            ));
        }
        
        let id_bytes = &data[offset..offset + 8];
        let _id = u64::from_le_bytes([
            id_bytes[0], id_bytes[1], id_bytes[2], id_bytes[3],
            id_bytes[4], id_bytes[5], id_bytes[6], id_bytes[7],
        ]);
        log::trace!("Deserialized entity ID: {}", _id);
        offset += 8;
        
        // 读取实体数据长度
        if offset + 4 > data.len() {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for entity data length".to_string()
            ));
        }
        
        let data_len = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
        offset += 4;
        
        if offset + data_len > data.len() {
            return Err(RepositoryError::SerializationError(
                "Insufficient data for entity data".to_string()
            ));
        }
        
        let entity_data = &data[offset..offset + data_len];
        let _entity_str = core::str::from_utf8(entity_data)
            .map_err(|_| RepositoryError::SerializationError("Invalid entity data encoding".to_string()))?;
        log::trace!("Deserialized entity with {} bytes of data", data_len);
        
        // 这里应该有实际的实体重建逻辑
        // 现在返回一个错误，因为这是一个简化的实现
        Err(RepositoryError::SerializationError(
            "Binary deserialization not fully implemented".to_string()
        ))
    }
    
    /// JSON反序列化
    fn deserialize_json<T: AggregateRoot>(
        &self,
        _data: &[u8],
        _context: &SerializationContext,
    ) -> Result<DeserializationResult<T>, RepositoryError> {
        // JSON反序列化的简化实现
        Err(RepositoryError::SerializationError(
            "JSON deserialization not implemented".to_string()
        ))
    }
    
    /// 创建元数据字符串
    fn create_metadata<T: AggregateRoot>(&self, entity: &T) -> String {
        format!(
            r#"{{"type": "{}", "id": {}, "class": "{:?}"}}"#,
            T::entity_type(),
            entity.id().value(),
            entity
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::{AggregateRoot, EntityId, DefaultIdGenerator};
    use crate::domain::aggregate_root::VersionedAggregateRoot;
    use alloc::sync::Arc;
    
    // 测试用的简单聚合根
    #[derive(Debug, Clone, PartialEq)]
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
    fn test_simple_serializer_creation() {
        let serializer = SimpleSerializer::new();
        assert_eq!(serializer.name(), "SimpleSerializer");
        assert_eq!(serializer.version(), "1.0.0");
        
        let formats = serializer.supported_formats();
        assert!(formats.contains(&SerializationFormat::Binary));
        assert!(formats.contains(&SerializationFormat::Json));
    }
    
    #[test]
    fn test_binary_serialization() {
        let serializer = SimpleSerializer::new();
        let entity = TestEntity::new("Test".to_string());
        let context = SerializationContext::binary().with_version(true).with_metadata(true);
        
        let result = serializer.serialize(&entity, &context);
        assert!(result.is_ok());
        
        let serialization_result = result.unwrap();
        assert_eq!(serialization_result.format, SerializationFormat::Binary);
        assert_eq!(serialization_result.version, Some(1));
        assert!(!serialization_result.data.is_empty());
        assert!(serialization_result.get_metadata("entity_type").is_some());
    }
    
    #[test]
    fn test_json_serialization() {
        let serializer = SimpleSerializer::new();
        let entity = TestEntity::new("Test".to_string());
        let context = SerializationContext::json().with_version(true).with_metadata(true);
        
        let result = serializer.serialize(&entity, &context);
        assert!(result.is_ok());
        
        let serialization_result = result.unwrap();
        assert_eq!(serialization_result.format, SerializationFormat::Json);
        assert_eq!(serialization_result.version, Some(1));
        assert!(!serialization_result.data.is_empty());
        assert!(serialization_result.get_metadata("entity_type").is_some());
    }
    
    #[test]
    fn test_binary_deserialization() {
        let serializer = SimpleSerializer::new();
        let entity = TestEntity::new("Test".to_string());
        let context = SerializationContext::binary().with_version(true);
        
        // 序列化
        let serialized = serializer.serialize(&entity, &context).unwrap();
        
        // 反序列化（应该失败，因为实现不完整）
        let result = serializer.deserialize::<TestEntity>(&serialized.data, &context);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepositoryError::SerializationError(_)));
    }
    
    #[test]
    fn test_unsupported_format() {
        let serializer = SimpleSerializer::new();
        let entity = TestEntity::new("Test".to_string());
        let context = SerializationContext {
            format: SerializationFormat::Custom("custom"),
            include_version: true,
            include_metadata: true,
            compress: false,
            custom_options: Vec::new(),
        };
        
        let result = serializer.serialize(&entity, &context);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepositoryError::SerializationError(_)));
    }
}