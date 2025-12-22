//! 聚合根特征
//! 
//! 定义了聚合根的基本特征，为所有领域实体提供统一的接口。
//! 聚合根是领域驱动设计中的核心概念，表示数据修改的单一入口点。

use super::repositories::{EntityId, RepositoryError};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::format;
use core::fmt;
use core::any::Any;

/// 聚合根特征
///
/// 所有可以存储在仓储中的实体都必须实现此特征
/// 聚合根确保了领域对象的一致性和完整性
pub trait AggregateRoot: Send + Sync + fmt::Debug + Any {
    /// 克隆聚合根
    ///
    /// 提供了一个与Clone trait功能相同的方法，但不要求Self: Sized
    fn clone_aggregate(&self) -> Box<dyn AggregateRoot>;
    /// 获取实体ID
    ///
    /// 返回实体的唯一标识符
    fn id(&self) -> EntityId;
    
    /// 设置实体ID
    ///
    /// 设置实体的唯一标识符
    /// 注意：某些聚合根可能是不可变的，此时此方法可能不适用
    fn set_id(&mut self, id: EntityId);
    
    /// 验证实体
    ///
    /// 验证实体的业务规则和不变量
    /// 返回Ok(())表示验证通过，Err表示验证失败
    fn validate(&self) -> Result<(), RepositoryError>;
    
    /// 获取实体类型名称
    ///
    /// 返回实体的类型名称，用于日志记录和调试
    fn entity_type() -> &'static str
    where
        Self: Sized;
    
    /// 获取动态实体类型名称
    ///
    /// 用于dyn AggregateRoot类型
    fn entity_type_dyn(&self) -> &'static str;
    
    /// 检查实体是否有效
    ///
    /// 快速检查实体的基本有效性
    /// 不进行完整的业务规则验证
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
    
    /// 获取实体的字符串表示
    ///
    /// 返回实体的可读字符串表示
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
    
    /// 检查实体是否为新创建的
    ///
    /// 新创建的实体通常ID为0或无效值
    fn is_new(&self) -> bool {
        !self.id().is_valid()
    }
    
    /// 获取实体的版本号
    ///
    /// 用于乐观并发控制
    /// 默认实现返回0，表示不支持版本控制
    fn version(&self) -> u64 {
        0
    }
    
    /// 设置实体的版本号
    ///
    /// 用于乐观并发控制
    /// 默认实现为空操作
    fn set_version(&mut self, _version: u64) {
        // 默认实现不做任何操作
    }
}

/// 可版本化的聚合根特征
///
/// 扩展了基本的聚合根特征，添加了版本控制支持
pub trait VersionedAggregateRoot: AggregateRoot {
    /// 获取实体的版本号
    fn current_version(&self) -> u64;
    
    /// 更新实体的版本号
    fn update_version(&mut self, version: u64);
    
    /// 增加版本号
    ///
    /// 将版本号增加1，用于实体更新时
    fn increment_version(&mut self) {
        self.update_version(self.current_version() + 1);
    }
    
    /// 检查版本是否匹配
    ///
    /// 用于乐观并发控制，检查期望版本是否匹配当前版本
    fn version_matches(&self, expected_version: u64) -> bool {
        self.current_version() == expected_version
    }
}

/// 可审计的聚合根特征
///
/// 扩展了聚合根特征，添加了审计支持
pub trait AuditableAggregateRoot: AggregateRoot {
    /// 获取创建时间戳
    fn created_at(&self) -> Option<u64>;
    
    /// 获取更新时间戳
    fn updated_at(&self) -> Option<u64>;
    
    /// 设置创建时间戳
    fn set_created_at(&mut self, timestamp: u64);
    
    /// 设置更新时间戳
    fn set_updated_at(&mut self, timestamp: u64);
    
    /// 更新时间戳
    ///
    /// 将更新时间戳设置为当前时间
    fn touch(&mut self) {
        // 这里应该使用实际的时间获取机制
        // 暂时使用一个固定值作为示例
        self.set_updated_at(1234567890);
    }
    
    /// 检查实体是否已被修改
    ///
    /// 比较创建时间和更新时间
    fn is_modified(&self) -> bool {
        match (self.created_at(), self.updated_at()) {
            (Some(created), Some(updated)) => updated > created,
            (None, Some(_)) => true,
            _ => false,
        }
    }
}

/// 软删除的聚合根特征
///
/// 扩展了聚合根特征，添加了软删除支持
pub trait SoftDeletableAggregateRoot: AggregateRoot {
    /// 检查实体是否被删除
    fn is_deleted(&self) -> bool;
    
    /// 标记实体为已删除
    fn mark_as_deleted(&mut self);
    
    /// 恢复已删除的实体
    fn restore(&mut self);
    
    /// 获取删除时间戳
    fn deleted_at(&self) -> Option<u64>;
    
    /// 设置删除时间戳
    fn set_deleted_at(&mut self, timestamp: u64);
}

/// 聚合根构建器特征
///
/// 用于构建聚合根实例的特征
pub trait AggregateRootBuilder<T: AggregateRoot> {
    /// 构建聚合根实例
    fn build(self) -> Result<T, RepositoryError>;
    
    /// 设置ID
    fn with_id(self, id: EntityId) -> Self;
    
    /// 验证构建参数
    fn validate(&self) -> Result<(), RepositoryError>;
}

/// 聚合根工厂特征
///
/// 用于创建聚合根实例的特征
pub trait AggregateRootFactory<T: AggregateRoot>: Send + Sync {
    /// 创建新的聚合根实例
    fn create(&self) -> Result<T, RepositoryError>;
    
    /// 从数据重建聚合根实例
    fn from_data(&self, data: &[u8]) -> Result<T, RepositoryError>;
    
    /// 克隆聚合根实例
    fn clone(&self, entity: &T) -> Result<T, RepositoryError>;
}

/// 聚合根验证器特征
///
/// 用于验证聚合根的特征
pub trait AggregateRootValidator<T: AggregateRoot>: Send + Sync {
    /// 验证聚合根
    fn validate(&self, entity: &T) -> Result<(), RepositoryError>;
    
    /// 验证聚合根的业务规则
    fn validate_business_rules(&self, entity: &T) -> Result<(), RepositoryError>;
    
    /// 验证聚合根的不变量
    fn validate_invariants(&self, entity: &T) -> Result<(), RepositoryError>;
}

/// 聚合根事件特征
///
/// 为聚合根添加领域事件支持
pub trait EventSourcedAggregateRoot: AggregateRoot {
    /// 领域事件类型
    type DomainEvent;
    
    /// 获取未提交的事件
    fn get_uncommitted_events(&self) -> &[Self::DomainEvent];
    
    /// 标记事件为已提交
    fn mark_events_as_committed(&mut self);
    
    /// 添加领域事件
    fn add_domain_event(&mut self, event: Self::DomainEvent);
    
    /// 清除所有事件
    fn clear_events(&mut self);
    
    /// 检查是否有未提交的事件
    fn has_uncommitted_events(&self) -> bool {
        !self.get_uncommitted_events().is_empty()
    }
    
    /// 获取未提交事件的数量
    fn uncommitted_event_count(&self) -> usize {
        self.get_uncommitted_events().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::repositories::EntityId;
    
    // 测试用的简单聚合根实现
    #[derive(Debug, Clone)]
    struct TestAggregateRoot {
        id: EntityId,
        name: String,
        version: u64,
    }
    
    impl TestAggregateRoot {
        fn new(name: String) -> Self {
            Self {
                id: EntityId::new(0),
                name,
                version: 0,
            }
        }
    }
    
    impl AggregateRoot for TestAggregateRoot {
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
            "TestAggregateRoot"
        }
        
        fn clone_aggregate(&self) -> Box<dyn AggregateRoot> {
            Box::new(self.clone())
        }
    }
    
    impl VersionedAggregateRoot for TestAggregateRoot {
        fn version(&self) -> u64 {
            self.version
        }
        
        fn set_version(&mut self, version: u64) {
            self.version = version;
        }
    }
    
    #[test]
    fn test_aggregate_root_basic() {
        let mut entity = TestAggregateRoot::new("Test".to_string());
        
        // 测试新实体
        assert!(entity.is_new());
        
        // 设置ID
        entity.set_id(EntityId::new(1));
        assert_eq!(entity.id(), EntityId::new(1));
        assert!(!entity.is_new());
        
        // 测试验证
        assert!(entity.is_valid());
        
        // 测试实体类型
        assert_eq!(TestAggregateRoot::entity_type(), "TestAggregateRoot");
    }
    
    #[test]
    fn test_versioned_aggregate_root() {
        let mut entity = TestAggregateRoot::new("Test".to_string());
        
        // 初始版本
        assert_eq!(entity.version(), 0);
        
        // 增加版本
        entity.increment_version();
        assert_eq!(entity.version(), 1);
        
        // 版本匹配
        assert!(entity.version_matches(1));
        assert!(!entity.version_matches(0));
    }
    
    #[test]
    fn test_aggregate_root_validation() {
        let valid_entity = TestAggregateRoot::new("Valid".to_string());
        assert!(valid_entity.is_valid());
        
        let invalid_entity = TestAggregateRoot::new("".to_string());
        assert!(!invalid_entity.is_valid());
        assert!(invalid_entity.validate().is_err());
    }
    
    #[test]
    fn test_aggregate_root_to_string() {
        let entity = TestAggregateRoot::new("Test".to_string());
        let string_repr = entity.to_string();
        
        // 字符串表示应该包含实体信息
        assert!(string_repr.contains("Test"));
    }
}