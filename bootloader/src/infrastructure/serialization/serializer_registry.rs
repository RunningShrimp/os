//! 序列化器注册表实现
//!
//! 提供序列化器的注册、查找和管理功能。
//! 支持多种序列化格式的动态选择。

use crate::domain::serialization::{Serializer, SerializationFormat};
use crate::domain::RepositoryError;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::format;
use alloc::vec::Vec;
use spin::RwLock;

/// 序列化器注册表
///
/// 管理多个序列化器实例，支持动态格式选择
pub struct SerializerRegistry {
    /// 注册的序列化器
    serializers: RwLock<BTreeMap<String, Box<dyn Serializer>>>,
    /// 格式到名称的映射
    format_to_name: RwLock<BTreeMap<SerializationFormat, String>>,
}

impl SerializerRegistry {
    /// 创建新的序列化器注册表
    pub fn new() -> Self {
        Self {
            serializers: RwLock::new(BTreeMap::new()),
            format_to_name: RwLock::new(BTreeMap::new()),
        }
    }
    
    /// 注册序列化器
    pub fn register(&mut self, serializer: Box<dyn Serializer>) -> Result<(), RepositoryError> {
        let name = serializer.name().to_string();
        let formats = serializer.supported_formats();
        
        // 检查名称是否已存在
        {
            let serializers = self.serializers.read();
            if serializers.contains_key(&name) {
                return Err(RepositoryError::SerializationError(
                    format!("Serializer with name '{}' already registered", name)
                ));
            }
        }
        
        // 注册序列化器
        {
            let mut serializers = self.serializers.write();
            serializers.insert(name.clone(), serializer);
        }
        
        // 注册格式映射
        {
            let mut format_map = self.format_to_name.write();
            for format in formats {
                if format_map.contains_key(&format) {
                    return Err(RepositoryError::SerializationError(
                        format!("Format '{:?}' already registered by serializer '{}'", 
                               format, format_map.get(&format).unwrap())
                    ));
                }
                format_map.insert(format, name.clone());
            }
        }
        
        Ok(())
    }
    
    /// 注销序列化器
    pub fn unregister(&mut self, name: &str) -> Result<(), RepositoryError> {
        // 获取要注销的序列化器
        let serializer = {
            let mut serializers = self.serializers.write();
            serializers.remove(name)
        };
        
        if let Some(serializer) = serializer {
            // 移除格式映射
            let formats = serializer.supported_formats();
            let mut format_map = self.format_to_name.write();
            for format in formats {
                format_map.remove(&format);
            }
            
            Ok(())
        } else {
            Err(RepositoryError::SerializationError(
                format!("Serializer with name '{}' not found", name)
            ))
        }
    }
    
    /// 获取指定格式的序列化器
    pub fn get_serializer(&self, format: &SerializationFormat) -> Option<Box<dyn Serializer>> {
        let format_map = self.format_to_name.read();
        if let Some(name) = format_map.get(format) {
            let serializers = self.serializers.read();
            if let Some(_serializer) = serializers.get(name) {
            // 这里需要克隆序列化器，但Box<dyn Serializer>不能直接克隆
            // 实际实现中可能需要使用Arc或其他共享机制
            // 现在返回None作为占位符
            None
        } else {
            None
        }
        } else {
            None
        }
    }
    
    /// 获取指定名称的序列化器
    pub fn get_serializer_by_name(&self, name: &str) -> Option<Box<dyn Serializer>> {
        let serializers = self.serializers.read();
        if let Some(_serializer) = serializers.get(name) {
            // 同样，这里需要解决克隆问题
            None
        } else {
            None
        }
    }
    
    /// 获取所有支持的格式
    pub fn supported_formats(&self) -> Vec<SerializationFormat> {
        let format_map = self.format_to_name.read();
        format_map.keys().copied().collect()
    }
    
    /// 获取所有注册的序列化器名称
    pub fn registered_serializers(&self) -> Vec<String> {
        let serializers = self.serializers.read();
        serializers.keys().cloned().collect()
    }
    
    /// 检查是否支持指定格式
    pub fn supports_format(&self, format: &SerializationFormat) -> bool {
        let format_map = self.format_to_name.read();
        format_map.contains_key(format)
    }
    
    /// 获取序列化器信息
    pub fn get_serializer_info(&self, name: &str) -> Option<SerializerInfo> {
        let serializers = self.serializers.read();
        if let Some(serializer) = serializers.get(name) {
            Some(SerializerInfo {
                name: serializer.name().to_string(),
                version: serializer.version().to_string(),
                supported_formats: serializer.supported_formats(),
            })
        } else {
            None
        }
    }
    
    /// 获取所有序列化器信息
    pub fn get_all_serializer_info(&self) -> Vec<SerializerInfo> {
        let serializers = self.serializers.read();
        serializers.values().map(|serializer| SerializerInfo {
            name: serializer.name().to_string(),
            version: serializer.version().to_string(),
            supported_formats: serializer.supported_formats(),
        }).collect()
    }
    
    /// 清除所有序列化器
    pub fn clear(&mut self) {
        {
            let mut serializers = self.serializers.write();
            serializers.clear();
        }
        {
            let mut format_map = self.format_to_name.write();
            format_map.clear();
        }
    }
    
    /// 获取注册表统计信息
    pub fn get_registry_stats(&self) -> RegistryStats {
        let serializers = self.serializers.read();
        let format_map = self.format_to_name.read();
        
        let mut format_counts = BTreeMap::new();
        for (_, serializer) in serializers.iter() {
            for format in serializer.supported_formats() {
                *format_counts.entry(format).or_insert(0) += 1;
            }
        }
        
        RegistryStats {
            total_serializers: serializers.len(),
            total_formats: format_map.len(),
            format_counts,
        }
    }
}

/// 序列化器信息
///
/// 包含序列化器的元数据信息
#[derive(Debug, Clone)]
pub struct SerializerInfo {
    /// 序列化器名称
    pub name: String,
    /// 序列化器版本
    pub version: String,
    /// 支持的格式
    pub supported_formats: Vec<SerializationFormat>,
}

/// 注册表统计信息
///
/// 提供序列化器注册表的统计数据
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// 总序列化器数量
    pub total_serializers: usize,
    /// 总格式数量
    pub total_formats: usize,
    /// 格式计数映射
    pub format_counts: BTreeMap<SerializationFormat, usize>,
}

impl RegistryStats {
    /// 获取最常用的格式
    pub fn most_popular_format(&self) -> Option<&SerializationFormat> {
        self.format_counts
            .iter()
            .max_by_key(|(_, count)| **count)
            .map(|(format, _)| format)
    }
    
    /// 获取支持指定格式的序列化器数量
    pub fn serializer_count_for_format(&self, format: &SerializationFormat) -> usize {
        self.format_counts.get(format).copied().unwrap_or(0)
    }
}

impl Default for SerializerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::serialization::{BinarySerializer, JsonSerializer};
    use alloc::sync::Arc;
    
    #[test]
    fn test_serializer_registry_creation() {
        let registry = SerializerRegistry::new();
        assert_eq!(registry.registered_serializers().len(), 0);
        assert_eq!(registry.supported_formats().len(), 0);
    }
    
    #[test]
    fn test_register_serializer() {
        let mut registry = SerializerRegistry::new();
        let binary_serializer = Box::new(BinarySerializer::new());
        
        let result = registry.register(binary_serializer);
        assert!(result.is_ok());
        
        assert_eq!(registry.registered_serializers().len(), 1);
        assert!(registry.supports_format(&SerializationFormat::Binary));
    }
    
    #[test]
    fn test_register_duplicate_serializer() {
        let mut registry = SerializerRegistry::new();
        let binary_serializer1 = Box::new(BinarySerializer::new());
        let binary_serializer2 = Box::new(BinarySerializer::new());
        
        // 注册第一个
        let result1 = registry.register(binary_serializer1);
        assert!(result1.is_ok());
        
        // 尝试注册同名序列化器
        let result2 = registry.register(binary_serializer2);
        assert!(result2.is_err());
        assert!(matches!(result2.unwrap_err(), RepositoryError::SerializationError(_)));
    }
    
    #[test]
    fn test_unregister_serializer() {
        let mut registry = SerializerRegistry::new();
        let binary_serializer = Box::new(BinarySerializer::new());
        
        // 注册序列化器
        let _ = registry.register(binary_serializer).unwrap();
        assert_eq!(registry.registered_serializers().len(), 1);
        
        // 注销序列化器
        let result = registry.unregister("BinarySerializer");
        assert!(result.is_ok());
        assert_eq!(registry.registered_serializers().len(), 0);
        assert!(!registry.supports_format(&SerializationFormat::Binary));
    }
    
    #[test]
    fn test_get_serializer_info() {
        let mut registry = SerializerRegistry::new();
        let binary_serializer = Box::new(BinarySerializer::new());
        
        // 注册序列化器
        let _ = registry.register(binary_serializer).unwrap();
        
        // 获取序列化器信息
        let info = registry.get_serializer_info("BinarySerializer");
        assert!(info.is_some());
        
        let serializer_info = info.unwrap();
        assert_eq!(serializer_info.name, "BinarySerializer");
        assert_eq!(serializer_info.version, "1.0.0");
        assert!(serializer_info.supported_formats.contains(&SerializationFormat::Binary));
    }
    
    #[test]
    fn test_registry_stats() {
        let mut registry = SerializerRegistry::new();
        let binary_serializer = Box::new(BinarySerializer::new());
        let json_serializer = Box::new(JsonSerializer::new());
        
        // 注册序列化器
        let _ = registry.register(binary_serializer).unwrap();
        let _ = registry.register(json_serializer).unwrap();
        
        // 获取统计信息
        let stats = registry.get_registry_stats();
        assert_eq!(stats.total_serializers, 2);
        assert_eq!(stats.total_formats, 2);
        assert_eq!(stats.serializer_count_for_format(&SerializationFormat::Binary), 1);
        assert_eq!(stats.serializer_count_for_format(&SerializationFormat::Json), 1);
    }
    
    #[test]
    fn test_clear_registry() {
        let mut registry = SerializerRegistry::new();
        let binary_serializer = Box::new(BinarySerializer::new());
        
        // 注册序列化器
        let _ = registry.register(binary_serializer).unwrap();
        assert_eq!(registry.registered_serializers().len(), 1);
        
        // 清除注册表
        registry.clear();
        assert_eq!(registry.registered_serializers().len(), 0);
        assert_eq!(registry.supported_formats().len(), 0);
    }
}