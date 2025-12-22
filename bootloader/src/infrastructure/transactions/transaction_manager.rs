//! 事务管理器实现
//!
//! 提供基于内存的事务管理器实现，支持事务生命周期管理、
//! 超时控制、并发管理和统计信息收集。

use crate::domain::repositories::TransactionId;
use crate::domain::transactions::{
    TransactionManager, Transaction, TransactionStatus, TransactionError,
    TransactionStats, TransactionLog
};
use crate::domain::repositories::IdGenerator;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

use super::memory_transaction::{MemoryTransaction, TransactionPool};

/// 内存事务管理器
///
/// 提供基于内存的事务管理功能
pub struct MemoryTransactionManager {
    /// 事务池
    transaction_pool: TransactionPool,
    /// 事务日志
    transaction_log: Arc<dyn TransactionLog>,
    /// ID生成器
    id_generator: Arc<dyn IdGenerator>,
    /// 默认超时时间
    default_timeout: RwLock<u64>,
    /// 统计信息
    stats: RwLock<TransactionStats>,
    /// 创建时间
    created_at: u64,
}

impl MemoryTransactionManager {
    /// 创建新的内存事务管理器
    pub fn new(
        transaction_log: Arc<dyn TransactionLog>,
        id_generator: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            transaction_pool: TransactionPool::new(100), // 最大100个活跃事务
            transaction_log,
            id_generator,
            default_timeout: RwLock::new(30000), // 默认30秒超时
            stats: RwLock::new(TransactionStats::new()),
            created_at: Self::current_time(),
        }
    }
    
    /// 获取当前时间（毫秒）
    fn current_time() -> u64 {
        // 实际实现中应该使用系统时间
        // 这里使用一个固定值作为示例
        1234567890
    }
    
    /// 更新统计信息
    fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut TransactionStats),
    {
        let mut stats = self.stats.write();
        update_fn(&mut *stats);
    }
    
    /// 清理超时事务
    fn cleanup_timeout_transactions(&self) -> Result<usize, TransactionError> {
        let timeout_ids = self.transaction_pool.check_timeouts();
        let mut cleaned_count = 0;
        
        for id in timeout_ids {
            if let Some(_transaction) = self.transaction_pool.get_transaction(id) {
                // 将事务移动到已完成列表
                log::debug!("Cleaning up timeout transaction: {:?}", id);
                let _ = self.transaction_pool.move_to_completed(id);
                cleaned_count += 1;
                
                // 更新统计信息
                self.update_stats(|stats| {
                    stats.active_transactions -= 1;
                    stats.timeout_transactions += 1;
                });
            }
        }
        
        Ok(cleaned_count)
    }
    
    /// 执行事务提交
    fn commit_transaction_internal(&self, id: TransactionId) -> Result<(), TransactionError> {
        // 获取事务
        let mut transaction = self.transaction_pool.get_transaction(id)
            .ok_or_else(|| TransactionError::TransactionNotFound(id))?;
        
        // 提交事务
        transaction.commit()?;
        
        // 移动到已完成列表
        self.transaction_pool.move_to_completed(id)?;
        
        // 更新统计信息
        self.update_stats(|stats| {
            stats.active_transactions -= 1;
            stats.committed_transactions += 1;
        });
        
        Ok(())
    }
    
    /// 执行事务回滚
    fn rollback_transaction_internal(&self, id: TransactionId) -> Result<(), TransactionError> {
        // 获取事务
        let mut transaction = self.transaction_pool.get_transaction(id)
            .ok_or_else(|| TransactionError::TransactionNotFound(id))?;
        
        // 回滚事务
        transaction.rollback()?;
        
        // 移动到已完成列表
        self.transaction_pool.move_to_completed(id)?;
        
        // 更新统计信息
        self.update_stats(|stats| {
            stats.active_transactions -= 1;
            stats.rolled_back_transactions += 1;
        });
        
        Ok(())
    }
}

impl TransactionManager for MemoryTransactionManager {
    fn begin_transaction(&self) -> Result<Box<dyn Transaction>, TransactionError> {
        let timeout = *self.default_timeout.read();
        self.begin_transaction_with_timeout(timeout)
    }
    
    fn begin_transaction_with_timeout(&self, timeout_ms: u64) -> Result<Box<dyn Transaction>, TransactionError> {
        // 清理超时事务
        self.cleanup_timeout_transactions()?;
        
        // 生成事务ID
        let id = self.id_generator.generate_transaction_id();
        
        // 创建事务
        let transaction = MemoryTransaction::new(id, self.transaction_log.clone(), Some(timeout_ms))?;
        let transaction_id = transaction.id();
        
        // 添加到事务池
        self.transaction_pool.add_transaction(Box::new(transaction))?;
        
        // 更新统计信息
        self.update_stats(|stats| {
            stats.active_transactions += 1;
            stats.total_transactions += 1;
        });
        
        // 从事务池获取事务的克隆
        if let Some(cloned_transaction) = self.transaction_pool.get_transaction(transaction_id) {
            Ok(cloned_transaction)
        } else {
            Err(TransactionError::TransactionNotFound(transaction_id))
        }
    }
    
    fn commit_transaction(&self, transaction_id: TransactionId) -> Result<(), TransactionError> {
        // 清理超时事务
        self.cleanup_timeout_transactions()?;
        
        // 执行提交
        self.commit_transaction_internal(transaction_id)
    }
    
    fn rollback_transaction(&self, transaction_id: TransactionId) -> Result<(), TransactionError> {
        // 清理超时事务
        self.cleanup_timeout_transactions()?;
        
        // 执行回滚
        self.rollback_transaction_internal(transaction_id)
    }
    
    fn get_transaction_status(&self, transaction_id: TransactionId) -> Result<TransactionStatus, TransactionError> {
        // 清理超时事务
        self.cleanup_timeout_transactions()?;
        
        // 获取事务状态
        if let Some(transaction) = self.transaction_pool.get_transaction(transaction_id) {
            Ok(transaction.status())
        } else {
            // 检查已完成事务
            if self.transaction_pool.get_transaction(transaction_id).is_some() {
                Ok(TransactionStatus::Committed)
            } else {
                Err(TransactionError::TransactionNotFound(transaction_id))
            }
        }
    }
    
    fn get_active_transactions(&self) -> Result<Vec<TransactionId>, TransactionError> {
        // 清理超时事务
        self.cleanup_timeout_transactions()?;
        
        Ok(self.transaction_pool.get_active_transaction_ids())
    }
    
    fn cleanup_completed_transactions(&self) -> Result<usize, TransactionError> {
        let cleaned_count = self.transaction_pool.cleanup_completed_transactions();
        
        // 更新统计信息（活跃事务数已经在清理时更新）
        Ok(cleaned_count)
    }
    
    fn set_default_timeout(&mut self, timeout_ms: u64) {
        *self.default_timeout.write() = timeout_ms;
    }
    
    fn default_timeout(&self) -> u64 {
        *self.default_timeout.read()
    }
    
    fn get_transaction_stats(&self) -> Result<TransactionStats, TransactionError> {
        // 清理超时事务
        self.cleanup_timeout_transactions()?;
        
        // 获取当前统计信息
        let mut stats = self.stats.read().clone();
        
        // 计算事务管理器运行时间
        stats.uptime_ms = Self::current_time() - self.created_at;
        
        Ok(stats)
    }
}

impl Drop for MemoryTransactionManager {
    fn drop(&mut self) {
        // 清理所有活跃事务
        let active_ids = self.transaction_pool.get_active_transaction_ids();
        for id in active_ids {
            let _ = self.rollback_transaction_internal(id);
        }
        
        // 清理已完成事务
        let _ = self.transaction_pool.cleanup_completed_transactions();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::DefaultIdGenerator;
    use crate::domain::transactions::MemoryTransactionLog;
    use alloc::sync::Arc;
    
    #[test]
    fn test_transaction_manager_creation() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id_generator = Arc::new(DefaultIdGenerator::new());
        let manager = MemoryTransactionManager::new(transaction_log, id_generator);
        
        assert_eq!(manager.default_timeout(), 30000);
        
        let stats = manager.get_transaction_stats().unwrap();
        assert_eq!(stats.total_transactions, 0);
        assert_eq!(stats.active_transactions, 0);
    }
    
    #[test]
    fn test_begin_transaction() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id_generator = Arc::new(DefaultIdGenerator::new());
        let manager = MemoryTransactionManager::new(transaction_log, id_generator);
        
        let transaction = manager.begin_transaction();
        assert!(transaction.is_ok());
        
        let stats = manager.get_transaction_stats().unwrap();
        assert_eq!(stats.active_transactions, 1);
        assert_eq!(stats.total_transactions, 1);
    }
    
    #[test]
    fn test_commit_transaction() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id_generator = Arc::new(DefaultIdGenerator::new());
        let manager = MemoryTransactionManager::new(transaction_log, id_generator);
        
        let mut transaction = manager.begin_transaction().unwrap();
        let id = transaction.id();
        
        let result = manager.commit_transaction(id);
        assert!(result.is_ok());
        
        let stats = manager.get_transaction_stats().unwrap();
        assert_eq!(stats.active_transactions, 0);
        assert_eq!(stats.committed_transactions, 1);
    }
    
    #[test]
    fn test_rollback_transaction() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id_generator = Arc::new(DefaultIdGenerator::new());
        let manager = MemoryTransactionManager::new(transaction_log, id_generator);
        
        let mut transaction = manager.begin_transaction().unwrap();
        let id = transaction.id();
        
        let result = manager.rollback_transaction(id);
        assert!(result.is_ok());
        
        let stats = manager.get_transaction_stats().unwrap();
        assert_eq!(stats.active_transactions, 0);
        assert_eq!(stats.rolled_back_transactions, 1);
    }
    
    #[test]
    fn test_transaction_timeout() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id_generator = Arc::new(DefaultIdGenerator::new());
        let mut manager = MemoryTransactionManager::new(transaction_log, id_generator);
        
        // 设置很短的超时时间
        manager.set_default_timeout(1);
        
        let transaction = manager.begin_transaction().unwrap();
        let id = transaction.id();
        
        // 尝试提交（应该因为超时而失败）
        let result = manager.commit_transaction(id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransactionError::TransactionTimeout(_)));
    }
    
    #[test]
    fn test_cleanup_completed_transactions() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id_generator = Arc::new(DefaultIdGenerator::new());
        let manager = MemoryTransactionManager::new(transaction_log, id_generator);
        
        // 创建并提交几个事务
        for _ in 0..5 {
            let mut transaction = manager.begin_transaction().unwrap();
            let id = transaction.id();
            let _ = manager.commit_transaction(id);
        }
        
        let cleaned_count = manager.cleanup_completed_transactions().unwrap();
        assert_eq!(cleaned_count, 5);
        
        let stats = manager.get_transaction_stats().unwrap();
        assert_eq!(stats.committed_transactions, 5);
        assert_eq!(stats.active_transactions, 0);
    }
    
    #[test]
    fn test_default_timeout() {
        let transaction_log = Arc::new(MemoryTransactionLog::new());
        let id_generator = Arc::new(DefaultIdGenerator::new());
        let mut manager = MemoryTransactionManager::new(transaction_log, id_generator);
        
        assert_eq!(manager.default_timeout(), 30000);
        
        manager.set_default_timeout(5000);
        assert_eq!(manager.default_timeout(), 5000);
    }
}