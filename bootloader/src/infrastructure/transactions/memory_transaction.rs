//! 内存事务实现
//!
//! 提供基于内存的事务实现，支持ACID事务特性。
//! 包括事务操作管理、超时控制和回滚机制。

use crate::domain::transactions::{
    Transaction, TransactionStatus, TransactionOperation, 
    TransactionError, TransactionLog
};
use crate::domain::repositories::TransactionId;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

/// 内存事务实现
///
/// 提供基于内存的事务功能，支持操作记录和回滚
pub struct MemoryTransaction {
    /// 事务ID
    id: TransactionId,
    /// 事务状态
    status: RwLock<TransactionStatus>,
    /// 事务操作列表
    operations: RwLock<Vec<TransactionOperation>>,
    /// 开始时间
    start_time: u64,
    /// 超时时间（毫秒）
    timeout: RwLock<Option<u64>>,
    /// 事务日志
    transaction_log: Arc<dyn TransactionLog>,
    /// 操作计数器
    operation_count: AtomicU64,
}

impl MemoryTransaction {
    /// 创建新的内存事务
    pub fn new(
        id: TransactionId,
        transaction_log: Arc<dyn TransactionLog>,
        timeout_ms: Option<u64>,
    ) -> Result<Self, TransactionError> {
        let transaction = Self {
            id,
            status: RwLock::new(TransactionStatus::Active),
            operations: RwLock::new(Vec::new()),
            start_time: Self::current_time(),
            timeout: RwLock::new(timeout_ms),
            transaction_log,
            operation_count: AtomicU64::new(0),
        };
        
        // 记录事务开始
        transaction.transaction_log.log_transaction_start(id)?;
        
        Ok(transaction)
    }
    
    /// 获取当前时间（毫秒）
    fn current_time() -> u64 {
        // 实际实现中应该使用系统时间
        // 这里使用一个固定值作为示例
        1234567890
    }
    
    /// 检查事务是否超时
    fn check_timeout(&self) -> Result<(), TransactionError> {
        let timeout = self.timeout.read();
        if let Some(timeout_ms) = *timeout {
            let elapsed = Self::current_time() - self.start_time;
            if elapsed > timeout_ms {
                let mut status = self.status.write();
                *status = TransactionStatus::Timeout;
                
                // 记录超时
                self.transaction_log.log_transaction_timeout(self.id)?;
                
                return Err(TransactionError::TransactionTimeout(self.id));
            }
        }
        Ok(())
    }
    
    /// 验证事务状态
    fn validate_transaction_state(&self) -> Result<(), TransactionError> {
        let status = self.status.read();
        match *status {
            TransactionStatus::Active => Ok(()),
            TransactionStatus::Committed => Err(TransactionError::TransactionAlreadyCommitted(self.id)),
            TransactionStatus::RolledBack => Err(TransactionError::TransactionAlreadyRolledBack(self.id)),
            TransactionStatus::Timeout => Err(TransactionError::TransactionTimeout(self.id)),
            TransactionStatus::Failed => Err(TransactionError::OperationError("Transaction has failed")),
        }
    }
    
    /// 设置事务状态
    fn set_status(&self, new_status: TransactionStatus) -> Result<(), TransactionError> {
        let mut status = self.status.write();
        
        // 检查状态转换是否合法
        let current_status = *status;
        match (current_status, new_status) {
            (TransactionStatus::Active, TransactionStatus::Committed) => Ok(()),
            (TransactionStatus::Active, TransactionStatus::RolledBack) => Ok(()),
            (TransactionStatus::Active, TransactionStatus::Timeout) => Ok(()),
            (TransactionStatus::Active, TransactionStatus::Failed) => Ok(()),
            _ => Err(TransactionError::OperationError("Invalid status transition")),
        }?;
        
        *status = new_status;
        Ok(())
    }
}

impl Transaction for MemoryTransaction {
    fn id(&self) -> TransactionId {
        self.id
    }
    
    fn status(&self) -> TransactionStatus {
        *self.status.read()
    }
    
    fn add_operation(&mut self, operation: TransactionOperation) -> Result<(), TransactionError> {
        // 验证事务状态
        self.validate_transaction_state()?;
        
        // 检查超时
        self.check_timeout()?;
        
        // 先记录操作
        self.transaction_log.log_operation(self.id, &operation)?;
        
        // 再添加操作
        {
            let mut operations = self.operations.write();
            operations.push(operation);
        }
        
        // 更新计数器
        self.operation_count.fetch_add(1, Ordering::SeqCst);
        
        Ok(())
    }
    
    fn commit(&mut self) -> Result<(), TransactionError> {
        // 验证事务状态
        self.validate_transaction_state()?;
        
        // 检查超时
        self.check_timeout()?;
        
        // 设置状态为已提交
        self.set_status(TransactionStatus::Committed)?;
        
        // 记录提交
        self.transaction_log.log_transaction_commit(self.id)?;
        
        Ok(())
    }
    
    fn rollback(&mut self) -> Result<(), TransactionError> {
        // 验证事务状态
        self.validate_transaction_state()?;
        
        // 设置状态为已回滚
        self.set_status(TransactionStatus::RolledBack)?;
        
        // 记录回滚
        self.transaction_log.log_transaction_rollback(self.id)?;
        
        Ok(())
    }
    
    fn operations(&self) -> Vec<TransactionOperation> {
        self.operations.read().clone()
    }
    
    fn start_time(&self) -> u64 {
        self.start_time
    }
    
    fn timeout(&self) -> Option<u64> {
        *self.timeout.read()
    }
    
    fn set_timeout(&mut self, timeout_ms: u64) {
        *self.timeout.write() = Some(timeout_ms);
    }
    
    fn is_timeout(&self) -> bool {
        matches!(self.status(), TransactionStatus::Timeout)
    }
    
    fn operation_count(&self) -> usize {
        self.operation_count.load(Ordering::SeqCst) as usize
    }
    
    fn is_empty(&self) -> bool {
        self.operations.read().is_empty()
    }
    
    fn clone_box(&self) -> Box<dyn Transaction> {
        Box::new(Self {
            id: self.id,
            status: RwLock::new(*self.status.read()),
            operations: RwLock::new(self.operations.read().clone()),
            start_time: self.start_time,
            timeout: RwLock::new(*self.timeout.read()),
            transaction_log: Arc::clone(&self.transaction_log),
            operation_count: AtomicU64::new(self.operation_count.load(Ordering::SeqCst)),
        })
    }
}

/// 事务池
///
/// 管理多个事务实例
pub struct TransactionPool {
    /// 活跃事务
    active_transactions: RwLock<BTreeMap<TransactionId, Box<dyn Transaction>>>,
    /// 已完成事务
    completed_transactions: RwLock<BTreeMap<TransactionId, Box<dyn Transaction>>>,
    /// 最大活跃事务数
    max_active_transactions: usize,
}

impl TransactionPool {
    /// 创建新的事务池
    pub fn new(max_active_transactions: usize) -> Self {
        Self {
            active_transactions: RwLock::new(BTreeMap::new()),
            completed_transactions: RwLock::new(BTreeMap::new()),
            max_active_transactions,
        }
    }
    
    /// 添加事务到池中
    pub fn add_transaction(&self, transaction: Box<dyn Transaction>) -> Result<(), TransactionError> {
        let id = transaction.id();
        
        // 检查活跃事务数量限制
        {
            let active = self.active_transactions.read();
            if active.len() >= self.max_active_transactions {
                return Err(TransactionError::OperationError("Too many active transactions"));
            }
        }
        
        // 添加到活跃事务
        {
            let mut active = self.active_transactions.write();
            if active.contains_key(&id) {
                return Err(TransactionError::TransactionConflict(id));
            }
            active.insert(id, transaction);
        }
        
        Ok(())
    }
    
    /// 从池中获取事务
    pub fn get_transaction(&self, id: TransactionId) -> Option<Box<dyn Transaction>> {
        // 首先检查活跃事务
        {
            let active = self.active_transactions.read();
            if let Some(transaction) = active.get(&id) {
                return Some(transaction.clone_box());
            }
        }
        
        // 然后检查已完成事务
        {
            let completed = self.completed_transactions.read();
            if let Some(transaction) = completed.get(&id) {
                return Some(transaction.clone_box());
            }
        }
        
        None
    }
    
    /// 移除活跃事务
    pub fn remove_active_transaction(&self, id: TransactionId) -> Option<Box<dyn Transaction>> {
        let mut active = self.active_transactions.write();
        active.remove(&id)
    }
    
    /// 移动事务到已完成列表
    pub fn move_to_completed(&self, id: TransactionId) -> Result<(), TransactionError> {
        let transaction = {
            let mut active = self.active_transactions.write();
            active.remove(&id)
        };
        
        if let Some(transaction) = transaction {
            let mut completed = self.completed_transactions.write();
            completed.insert(id, transaction);
            Ok(())
        } else {
            Err(TransactionError::TransactionNotFound(id))
        }
    }
    
    /// 获取活跃事务数量
    pub fn active_transaction_count(&self) -> usize {
        self.active_transactions.read().len()
    }
    
    /// 获取已完成事务数量
    pub fn completed_transaction_count(&self) -> usize {
        self.completed_transactions.read().len()
    }
    
    /// 清理已完成的事务
    pub fn cleanup_completed_transactions(&self) -> usize {
        let mut completed = self.completed_transactions.write();
        let count = completed.len();
        completed.clear();
        count
    }
    
    /// 获取所有活跃事务ID
    pub fn get_active_transaction_ids(&self) -> Vec<TransactionId> {
        self.active_transactions.read().keys().copied().collect()
    }
    
    /// 检查事务超时
    pub fn check_timeouts(&self) -> Vec<TransactionId> {
        let mut timeout_ids = Vec::new();
        let active = self.active_transactions.read();
        
        for (id, transaction) in active.iter() {
            if transaction.is_timeout() {
                timeout_ids.push(*id);
            }
        }
        
        timeout_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::transactions::MemoryTransactionLog;
    use alloc::sync::Arc;
    
    #[test]
    fn test_memory_transaction_creation() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id = TransactionId::new(1);
        let timeout = Some(5000);
        
        let result = MemoryTransaction::new(id, transaction_log, timeout);
        assert!(result.is_ok());
        
        let transaction = result.unwrap();
        assert_eq!(transaction.id(), id);
        assert_eq!(transaction.status(), TransactionStatus::Active);
        assert_eq!(transaction.timeout(), Some(5000));
        assert!(transaction.is_empty());
    }
    
    #[test]
    fn test_transaction_operations() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id = TransactionId::new(1);
        let mut transaction = MemoryTransaction::new(id, transaction_log, None).unwrap();
        
        // 添加操作
        let operation = TransactionOperation::Create {
            entity_type: "TestEntity",
            entity_id: EntityId::new(1),
            data: vec![1, 2, 3],
        };
        
        let result = transaction.add_operation(operation);
        assert!(result.is_ok());
        assert_eq!(transaction.operation_count(), 1);
        assert!(!transaction.is_empty());
        
        // 获取操作列表
        let operations = transaction.operations();
        assert_eq!(operations.len(), 1);
    }
    
    #[test]
    fn test_transaction_commit() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id = TransactionId::new(1);
        let mut transaction = MemoryTransaction::new(id, transaction_log, None).unwrap();
        
        // 提交事务
        let result = transaction.commit();
        assert!(result.is_ok());
        assert_eq!(transaction.status(), TransactionStatus::Committed);
    }
    
    #[test]
    fn test_transaction_rollback() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id = TransactionId::new(1);
        let mut transaction = MemoryTransaction::new(id, transaction_log, None).unwrap();
        
        // 回滚事务
        let result = transaction.rollback();
        assert!(result.is_ok());
        assert_eq!(transaction.status(), TransactionStatus::RolledBack);
    }
    
    #[test]
    fn test_transaction_pool() {
        let pool = TransactionPool::new(10);
        
        assert_eq!(pool.active_transaction_count(), 0);
        assert_eq!(pool.completed_transaction_count(), 0);
        
        // 测试添加事务
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let transaction = MemoryTransaction::new(TransactionId::new(1), transaction_log, None).unwrap();
        
        let result = pool.add_transaction(Box::new(transaction));
        assert!(result.is_ok());
        assert_eq!(pool.active_transaction_count(), 1);
        
        // 测试获取事务
        let found = pool.get_transaction(TransactionId::new(1));
        assert!(found.is_some());
        
        // 测试移动到已完成
        let result = pool.move_to_completed(TransactionId::new(1));
        assert!(result.is_ok());
        assert_eq!(pool.active_transaction_count(), 0);
        assert_eq!(pool.completed_transaction_count(), 1);
    }
}