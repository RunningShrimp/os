//! 内存仓储实现
//!
//! 提供基于内存的仓储实现，支持事务操作和并发访问。
//! 使用BTreeMap进行数据存储，提供高效的查找和排序功能。

use crate::domain::aggregate_root::AggregateRoot;
use crate::domain::repositories::{
    Repository, BasicRepositoryQuery, RepositoryQuery, EntityId, RepositoryError, Page,
    IdGenerator
};
use crate::domain::transactions::{TransactionOperation, TransactionManager};
use crate::domain::serialization::{TypedSerializer, SerializationContext};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::any::Any;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

/// 内存仓储实现
///
/// 提供基于内存的仓储实现，支持事务和并发访问
pub struct MemoryRepository<T: AggregateRoot> {
    /// 存储的数据
    data: RwLock<BTreeMap<EntityId, T>>,
    /// 事务管理器
    transaction_manager: Arc<dyn TransactionManager>,
    /// ID生成器
    id_generator: Arc<dyn IdGenerator>,
    /// 序列化器
    serializer_impl: Box<dyn TypedSerializer<T>>,
    /// 实体计数器（用于统计）
    entity_count: AtomicU64,
}

impl<T: AggregateRoot> MemoryRepository<T> {
    /// 创建新的内存仓储
    pub fn new(
        transaction_manager: Arc<dyn TransactionManager>,
        id_generator: Arc<dyn IdGenerator>,
        serializer_impl: Box<dyn TypedSerializer<T>>,
    ) -> Self {
        Self {
            data: RwLock::new(BTreeMap::new()),
            transaction_manager,
            id_generator,
            serializer_impl,
            entity_count: AtomicU64::new(0),
        }
    }
    
    /// 生成新的实体ID
    fn generate_id(&self) -> EntityId {
        self.id_generator.generate_entity_id()
    }
    
    /// 序列化实体
    fn serialize_entity(&self, entity: &T) -> Result<Vec<u8>, RepositoryError> {
        let context = SerializationContext::binary()
            .with_version(true)
            .with_metadata(true);
        
        let result = self.serializer_impl.serialize(entity, &context)?;
        Ok(result.data)
    }
    
    /// 验证实体
    fn validate_entity(&self, entity: &T) -> Result<(), RepositoryError> {
        entity.validate()
    }
    
    /// 检查实体是否已存在
    fn entity_exists(&self, id: EntityId) -> bool {
        let data = self.data.read();
        data.contains_key(&id)
    }
    
    /// 执行分页查询
    fn paginate_entities(&self, entities: Vec<T>, page: usize, size: usize) -> Page<T> {
        let total_items = entities.len();
        let _total_pages = if size == 0 { 0 } else { (total_items + size - 1) / size };
        
        let start_index = page * size;
        let end_index = core::cmp::min(start_index + size, total_items);
        
        // Create a new Vec for items instead of using to_vec() which requires Clone
        let mut items = Vec::new();
        if start_index < total_items {
            for entity in entities.into_iter().skip(start_index).take(end_index - start_index) {
                items.push(entity);
            }
        }
        
        Page::new(items, page, size, total_items)
    }
}

impl<T: AggregateRoot> Repository<T> for MemoryRepository<T> {
    fn create(&self, mut entity: T) -> Result<EntityId, RepositoryError> {
        // 验证实体
        self.validate_entity(&entity)?;
        
        // 生成ID
        let id = self.generate_id();
        entity.set_id(id);
        
        // 序列化实体（用于事务日志）
        let serialized_data = self.serialize_entity(&entity)?;
        
        // 开始事务
        let mut transaction = self.transaction_manager.begin_transaction()?;
        
        // 添加创建操作到事务
        transaction.add_operation(TransactionOperation::Create {
            entity_type: T::entity_type(),
            entity_id: id,
            data: serialized_data,
        })?;
        
        // 提交事务
        transaction.commit()?;
        
        // 存储实体
        {
            let mut data = self.data.write();
            if data.contains_key(&id) {
                return Err(RepositoryError::EntityAlreadyExists(id));
            }
            data.insert(id, entity);
            self.entity_count.fetch_add(1, Ordering::SeqCst);
        }
        
        Ok(id)
    }
    
    fn find_by_id(&self, id: EntityId) -> Result<Option<T>, RepositoryError> {
        let data = self.data.read();
        if let Some(entity) = data.get(&id) {
            // Use clone_aggregate and downcast to get T
            let cloned_box = entity.clone_aggregate();
            let any_box = cloned_box as Box<dyn Any>;
            if let Ok(cloned) = any_box.downcast::<T>() {
                Ok(Some(*cloned))
            } else {
                Err(RepositoryError::StorageError("Failed to downcast cloned aggregate"))
            }
        } else {
            Ok(None)
        }
    }
    
    fn update(&self, entity: T) -> Result<(), RepositoryError> {
        // 验证实体
        self.validate_entity(&entity)?;
        
        let id = entity.id();
        if !id.is_valid() {
            return Err(RepositoryError::ValidationError("Invalid entity ID"));
        }
        
        // 序列化新实体数据
        let new_serialized_data = self.serialize_entity(&entity)?;
        
        // 获取旧实体数据（用于回滚）
        let old_serialized_data = {
            let data = self.data.read();
            if let Some(old_entity) = data.get(&id) {
                self.serialize_entity(old_entity)?
            } else {
                return Err(RepositoryError::EntityNotFound(id));
            }
        };
        
        // 开始事务
        let mut transaction = self.transaction_manager.begin_transaction()?;
        
        // 添加更新操作到事务
        transaction.add_operation(TransactionOperation::Update {
            entity_type: T::entity_type(),
            entity_id: id,
            old_data: old_serialized_data,
            new_data: new_serialized_data,
        })?;
        
        // 提交事务
        transaction.commit()?;
        
        // 更新实体
        {
            let mut data = self.data.write();
            if !data.contains_key(&id) {
                return Err(RepositoryError::EntityNotFound(id));
            }
            data.insert(id, entity);
        }
        
        Ok(())
    }
    
    fn delete(&self, id: EntityId) -> Result<(), RepositoryError> {
        // 获取要删除的实体数据（用于回滚）
        let old_serialized_data = {
            let data = self.data.read();
            if let Some(entity) = data.get(&id) {
                self.serialize_entity(entity)?
            } else {
                return Err(RepositoryError::EntityNotFound(id));
            }
        };
        
        // 开始事务
        let mut transaction = self.transaction_manager.begin_transaction()?;
        
        // 添加删除操作到事务
        transaction.add_operation(TransactionOperation::Delete {
            entity_type: T::entity_type(),
            entity_id: id,
            old_data: old_serialized_data,
        })?;
        
        // 提交事务
        transaction.commit()?;
        
        // 删除实体
        {
            let mut data = self.data.write();
            if data.remove(&id).is_none() {
                return Err(RepositoryError::EntityNotFound(id));
            }
            self.entity_count.fetch_sub(1, Ordering::SeqCst);
        }
        
        Ok(())
    }
    
    fn find_all(&self) -> Result<Vec<T>, RepositoryError> {
        let data = self.data.read();
        let mut entities: Vec<T> = Vec::new();
        
        for entity in data.values() {
            // Use clone_aggregate and downcast to get T
            let cloned_box = entity.clone_aggregate();
            let any_box = cloned_box as Box<dyn Any>;
            if let Ok(cloned) = any_box.downcast::<T>() {
                entities.push(*cloned);
            } else {
                return Err(RepositoryError::StorageError("Failed to downcast cloned aggregate"));
            }
        }
        
        Ok(entities)
    }
    
    fn query(&self) -> &dyn BasicRepositoryQuery<T> {
        self
    }
    
    fn count(&self) -> Result<usize, RepositoryError> {
        Ok(self.entity_count.load(Ordering::SeqCst) as usize)
    }
}

impl<T: AggregateRoot> BasicRepositoryQuery<T> for MemoryRepository<T> {
    fn count(&self) -> Result<usize, RepositoryError> {
        Ok(self.entity_count.load(Ordering::SeqCst) as usize)
    }
    
    fn exists(&self, id: EntityId) -> Result<bool, RepositoryError> {
        Ok(self.entity_exists(id))
    }
    
    fn find_with_pagination(&self, page: usize, size: usize) -> Result<Page<T>, RepositoryError> {
        let data = self.data.read();
        let mut entities: Vec<T> = Vec::new();
        
        for entity in data.values() {
            // Use clone_aggregate and downcast to get T
            let cloned_box = entity.clone_aggregate();
            // Convert to Any first
            let any_box = cloned_box as Box<dyn Any>;
            if let Ok(cloned) = any_box.downcast::<T>() {
                entities.push(*cloned);
            } else {
                return Err(RepositoryError::StorageError("Failed to downcast cloned aggregate"));
            }
        }
        
        Ok(self.paginate_entities(entities, page, size))
    }
    
    fn create_batch(&self, entities: Vec<T>) -> Result<Vec<EntityId>, RepositoryError> {
        if entities.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut ids = Vec::with_capacity(entities.len());
        let mut transaction = self.transaction_manager.begin_transaction()?;
        let mut entities_with_ids: Vec<T> = Vec::with_capacity(entities.len());
        
        // First pass: validate, generate IDs, and prepare for transaction
        for mut entity in entities {
            // 验证实体
            self.validate_entity(&entity)?;
            
            // 生成ID
            let id = self.generate_id();
            entity.set_id(id);
            
            // 序列化实体
            let serialized_data = self.serialize_entity(&entity)?;
            
            // 添加创建操作到事务
            transaction.add_operation(TransactionOperation::Create {
                entity_type: T::entity_type(),
                entity_id: id,
                data: serialized_data,
            })?;
            
            ids.push(id);
            entities_with_ids.push(entity);
        }
        
        // 提交事务
        transaction.commit()?;
        
        // 批量存储实体
        {
            let mut data = self.data.write();
            for (i, entity) in entities_with_ids.into_iter().enumerate() {
                let id = ids[i];
                if data.contains_key(&id) {
                    return Err(RepositoryError::EntityAlreadyExists(id));
                }
                data.insert(id, entity);
            }
            self.entity_count.fetch_add(ids.len() as u64, Ordering::SeqCst);
        }
        
        Ok(ids)
    }
    
    fn update_batch(&self, entities: Vec<T>) -> Result<(), RepositoryError> {
        if entities.is_empty() {
            return Ok(());
        }
        
        let mut transaction = self.transaction_manager.begin_transaction()?;
        
        for entity in &entities {
            // 验证实体
            self.validate_entity(entity)?;
            
            let id = entity.id();
            if !id.is_valid() {
                return Err(RepositoryError::ValidationError("Invalid entity ID"));
            }
            
            // 获取旧实体数据
            let old_serialized_data = {
                let data = self.data.read();
                if let Some(old_entity) = data.get(&id) {
                    self.serialize_entity(old_entity)?
                } else {
                    return Err(RepositoryError::EntityNotFound(id));
                }
            };
            
            // 序列化新实体数据
            let new_serialized_data = self.serialize_entity(entity)?;
            
            // 添加更新操作到事务
            transaction.add_operation(TransactionOperation::Update {
                entity_type: T::entity_type(),
                entity_id: id,
                old_data: old_serialized_data,
                new_data: new_serialized_data,
            })?;
        }
        
        // 提交事务
        transaction.commit()?;
        
        // 批量更新实体
        {
            let mut data = self.data.write();
            for entity in entities {
                let id = entity.id();
                if !data.contains_key(&id) {
                    return Err(RepositoryError::EntityNotFound(id));
                }
                data.insert(id, entity);
            }
        }
        
        Ok(())
    }
    
    fn delete_batch(&self, ids: Vec<EntityId>) -> Result<(), RepositoryError> {
        if ids.is_empty() {
            return Ok(());
        }
        
        // Save the length of ids before moving it
        let ids_len = ids.len();
        
        let mut transaction = self.transaction_manager.begin_transaction()?;
        
        // 获取要删除的实体数据
        let old_data_list = {
            let data = self.data.read();
            let mut old_data = Vec::with_capacity(ids.len());
            for &id in &ids {
                if let Some(entity) = data.get(&id) {
                    let serialized = self.serialize_entity(entity)?;
                    old_data.push(serialized);
                } else {
                    return Err(RepositoryError::EntityNotFound(id));
                }
            }
            old_data
        };
        
        // 添加删除操作到事务
        for (&id, old_data) in ids.iter().zip(old_data_list) {
            transaction.add_operation(TransactionOperation::Delete {
                entity_type: T::entity_type(),
                entity_id: id,
                old_data,
            })?;
        }
        
        // 提交事务
        transaction.commit()?;
        
        // 批量删除实体
        {
            let mut data = self.data.write();
            for id in ids {
                data.remove(&id);
            }
            self.entity_count.fetch_sub(ids_len as u64, Ordering::SeqCst);
        }
        
        Ok(())
    }
}

impl<T: AggregateRoot> RepositoryQuery<T> for MemoryRepository<T> {
    fn find_by_predicate<P>(&self, predicate: P) -> Result<Vec<T>, RepositoryError>
    where
        P: Fn(&T) -> bool + Send + Sync,
    {
        let data = self.data.read();
        let mut filtered: Vec<T> = Vec::new();
        
        for entity in data.values().filter(|entity| predicate(entity)) {
            // Use clone_aggregate and downcast to get T
            let cloned_box = entity.clone_aggregate();
            // Convert to Any first
            let any_box = cloned_box as Box<dyn Any>;
            if let Ok(cloned) = any_box.downcast::<T>() {
                filtered.push(*cloned);
            } else {
                return Err(RepositoryError::StorageError("Failed to downcast cloned aggregate"));
            }
        }
        
        Ok(filtered)
    }
    

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::{DefaultIdGenerator, SimpleSerializationService};
    use crate::domain::transactions::{MemoryTransactionManager, MemoryTransactionLog};
    use crate::domain::serialization::{BinarySerializer, SerializerRegistry};
    use alloc::sync::Arc;
    
    // 测试用的简单聚合根
    #[derive(Debug, Clone, PartialEq)]
    struct TestEntity {
        id: EntityId,
        name: String,
    }
    
    impl TestEntity {
        fn new(name: String) -> Self {
            Self {
                id: EntityId::new(0),
                name,
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
    
    fn create_test_repository() -> MemoryRepository<TestEntity> {
        let id_generator = Arc::new(DefaultIdGenerator::new());
        let serializer = Arc::new(SimpleSerializationService::new());
        let serializer_impl = Arc::new(BinarySerializer::new());
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let transaction_manager = Arc::new(MemoryTransactionManager::new(
            transaction_log,
            id_generator.clone(),
        ));
        
        MemoryRepository::new(
            transaction_manager,
            serializer,
            id_generator,
            serializer_impl,
        )
    }
    
    #[test]
    fn test_create_entity() {
        let repo = create_test_repository();
        let entity = TestEntity::new("Test".to_string());
        
        let id = repo.create(entity.clone()).unwrap();
        assert!(id.is_valid());
        
        let found = repo.find_by_id(id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, entity.name);
    }
    
    #[test]
    fn test_create_invalid_entity() {
        let repo = create_test_repository();
        let entity = TestEntity::new("".to_string()); // 无效实体
        
        let result = repo.create(entity);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepositoryError::ValidationError(_)));
    }
    
    #[test]
    fn test_update_entity() {
        let repo = create_test_repository();
        let mut entity = TestEntity::new("Test".to_string());
        
        let id = repo.create(entity.clone()).unwrap();
        entity.set_id(id);
        
        let mut updated_entity = entity.clone();
        updated_entity.name = "Updated".to_string();
        
        let result = repo.update(updated_entity);
        assert!(result.is_ok());
        
        let found = repo.find_by_id(id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Updated");
    }
    
    #[test]
    fn test_delete_entity() {
        let repo = create_test_repository();
        let entity = TestEntity::new("Test".to_string());
        
        let id = repo.create(entity).unwrap();
        
        let result = repo.delete(id);
        assert!(result.is_ok());
        
        let found = repo.find_by_id(id).unwrap();
        assert!(found.is_none());
    }
    
    #[test]
    fn test_find_by_predicate() {
        let repo = create_test_repository();
        
        let entity1 = TestEntity::new("Test1".to_string());
        let entity2 = TestEntity::new("Test2".to_string());
        let entity3 = TestEntity::new("Other".to_string());
        
        repo.create(entity1).unwrap();
        repo.create(entity2).unwrap();
        repo.create(entity3).unwrap();
        
        let results = repo.find_by_predicate(|e| e.name.starts_with("Test")).unwrap();
        assert_eq!(results.len(), 2);
    }
    
    #[test]
    fn test_pagination() {
        let repo = create_test_repository();
        
        for i in 0..10 {
            let entity = TestEntity::new(format!("Test{}", i));
            repo.create(entity).unwrap();
        }
        
        let page1 = repo.find_with_pagination(0, 3).unwrap();
        assert_eq!(page1.items.len(), 3);
        assert_eq!(page1.page_number, 0);
        assert_eq!(page1.total_pages, 4);
        
        let page2 = repo.find_with_pagination(1, 3).unwrap();
        assert_eq!(page2.items.len(), 3);
        assert_eq!(page2.page_number, 1);
        
        let page4 = repo.find_with_pagination(3, 3).unwrap();
        assert_eq!(page4.items.len(), 1);
        assert_eq!(page4.page_number, 3);
    }
    
    #[test]
    fn test_batch_operations() {
        let repo = create_test_repository();
        
        let entities: Vec<TestEntity> = (0..5)
            .map(|i| TestEntity::new(format!("Batch{}", i)))
            .collect();
        
        let ids = repo.create_batch(entities.clone()).unwrap();
        assert_eq!(ids.len(), 5);
        
        for (i, entity) in entities.iter().enumerate() {
            let found = repo.find_by_id(ids[i]).unwrap();
            assert!(found.is_some());
            assert_eq!(found.unwrap().name, entity.name);
        }
    }
    
    #[test]
    fn test_count() {
        let repo = create_test_repository();
        
        assert_eq!(repo.count().unwrap(), 0);
        
        for i in 0..5 {
            let entity = TestEntity::new(format!("Test{}", i));
            repo.create(entity).unwrap();
        }
        
        assert_eq!(repo.count().unwrap(), 5);
    }
}