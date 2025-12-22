//! 事务日志实现
//!
//! 提供基于内存的事务日志实现，支持事务操作记录、
//! 历史查询和统计信息收集。

use crate::domain::repositories::TransactionId;
use crate::domain::transactions::{
    TransactionLog, TransactionOperation, TransactionError,
    TransactionLogEntry, TransactionLogType, TransactionLogStats
};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::format;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

/// 内存事务日志实现
///
/// 提供基于内存的事务日志功能
pub struct MemoryTransactionLog {
    /// 事务操作日志
    operation_logs: RwLock<BTreeMap<TransactionId, Vec<TransactionOperation>>>,
    /// 事务历史日志
    history_logs: RwLock<Vec<TransactionLogEntry>>,
    /// 日志统计信息
    stats: RwLock<TransactionLogStats>,
    /// 最大历史记录数
    max_history_entries: usize,
    /// 操作计数器
    operation_counter: AtomicU64,
    /// 历史计数器
    history_counter: AtomicU64,
}

impl MemoryTransactionLog {
    /// 创建新的内存事务日志
    pub fn new() -> Self {
        Self {
            operation_logs: RwLock::new(BTreeMap::new()),
            history_logs: RwLock::new(Vec::new()),
            stats: RwLock::new(TransactionLogStats::new()),
            max_history_entries: 10000, // 最大保存10000条历史记录
            operation_counter: AtomicU64::new(0),
            history_counter: AtomicU64::new(0),
        }
    }
    
    /// 创建带限制的内存事务日志
    pub fn with_max_history(max_history_entries: usize) -> Self {
        Self {
            operation_logs: RwLock::new(BTreeMap::new()),
            history_logs: RwLock::new(Vec::new()),
            stats: RwLock::new(TransactionLogStats::new()),
            max_history_entries,
            operation_counter: AtomicU64::new(0),
            history_counter: AtomicU64::new(0),
        }
    }
    
    /// 获取当前时间戳
    fn current_timestamp() -> u64 {
        // 实际实现中应该使用系统时间
        // 这里使用一个固定值作为示例
        1234567890
    }
    
    /// 添加历史日志条目
    fn add_history_entry(&self, entry: TransactionLogEntry) {
        let mut history = self.history_logs.write();
        history.push(entry);
        let log_type = history.last().unwrap().log_type;
        let timestamp = history.last().unwrap().timestamp;
        
        // 限制历史记录数量
        if history.len() > self.max_history_entries {
            history.remove(0);
        }
        
        // 更新计数器
        self.history_counter.fetch_add(1, Ordering::SeqCst);
        
        // 更新统计信息
        self.update_stats(|stats| {
            stats.total_entries += 1;
            
            // 更新最早和最晚时间戳
            if stats.earliest_timestamp.is_none() {
                stats.earliest_timestamp = Some(timestamp);
            }
            stats.latest_timestamp = Some(timestamp);
            
            // 更新类型统计
            match log_type {
                TransactionLogType::Start => stats.start_entries += 1,
                TransactionLogType::Operation => stats.operation_entries += 1,
                TransactionLogType::Commit => stats.commit_entries += 1,
                TransactionLogType::Rollback => stats.rollback_entries += 1,
                TransactionLogType::Timeout => stats.timeout_entries += 1,
                TransactionLogType::Failure => stats.failure_entries += 1,
            }
        });
    }
    
    /// 更新统计信息
    fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut TransactionLogStats),
    {
        let mut stats = self.stats.write();
        update_fn(&mut *stats);
    }
}

impl TransactionLog for MemoryTransactionLog {
    fn log_operation(&self, transaction_id: TransactionId, operation: &TransactionOperation) -> Result<(), TransactionError> {
        // 添加操作到事务日志
        {
            let mut logs = self.operation_logs.write();
            let entry = logs.entry(transaction_id).or_insert_with(Vec::new);
            entry.push(operation.clone());
        }
        
        // 更新计数器
        self.operation_counter.fetch_add(1, Ordering::SeqCst);
        
        // 更新统计信息
        self.update_stats(|stats| {
            stats.total_entries += 1;
            stats.operation_entries += 1;
        });
        
        Ok(())
    }
    
    fn log_transaction_start(&self, transaction_id: TransactionId) -> Result<(), TransactionError> {
        let entry = TransactionLogEntry {
            transaction_id,
            log_type: TransactionLogType::Start,
            timestamp: Self::current_timestamp(),
            message: format!("Transaction {} started", transaction_id),
            operation: None,
        };
        
        self.add_history_entry(entry);
        Ok(())
    }
    
    fn log_transaction_commit(&self, transaction_id: TransactionId) -> Result<(), TransactionError> {
        let entry = TransactionLogEntry {
            transaction_id,
            log_type: TransactionLogType::Commit,
            timestamp: Self::current_timestamp(),
            message: format!("Transaction {} committed", transaction_id),
            operation: None,
        };
        
        self.add_history_entry(entry);
        Ok(())
    }
    
    fn log_transaction_rollback(&self, transaction_id: TransactionId) -> Result<(), TransactionError> {
        let entry = TransactionLogEntry {
            transaction_id,
            log_type: TransactionLogType::Rollback,
            timestamp: Self::current_timestamp(),
            message: format!("Transaction {} rolled back", transaction_id),
            operation: None,
        };
        
        self.add_history_entry(entry);
        Ok(())
    }
    
    fn log_transaction_timeout(&self, transaction_id: TransactionId) -> Result<(), TransactionError> {
        let entry = TransactionLogEntry {
            transaction_id,
            log_type: TransactionLogType::Timeout,
            timestamp: Self::current_timestamp(),
            message: format!("Transaction {} timed out", transaction_id),
            operation: None,
        };
        
        self.add_history_entry(entry);
        Ok(())
    }
    
    fn log_transaction_failure(&self, transaction_id: TransactionId, error: &TransactionError) -> Result<(), TransactionError> {
        let entry = TransactionLogEntry {
            transaction_id,
            log_type: TransactionLogType::Failure,
            timestamp: Self::current_timestamp(),
            message: format!("Transaction {} failed: {}", transaction_id, error),
            operation: None,
        };
        
        self.add_history_entry(entry);
        Ok(())
    }
    
    fn get_transaction_operations(&self, transaction_id: TransactionId) -> Result<Vec<TransactionOperation>, TransactionError> {
        let logs = self.operation_logs.read();
        Ok(logs.get(&transaction_id).cloned().unwrap_or_default())
    }
    
    fn get_transaction_history(&self, limit: Option<usize>) -> Result<Vec<TransactionLogEntry>, TransactionError> {
        let history = self.history_logs.read();
        
        let result = if let Some(limit) = limit {
            let len = history.len();
            if len > limit {
                history[(len - limit)..].to_vec()
            } else {
                history.clone()
            }
        } else {
            history.clone()
        };
        
        Ok(result)
    }
    
    fn clear_transaction_log(&self, transaction_id: TransactionId) -> Result<(), TransactionError> {
        // 清除操作日志
        {
            let mut logs = self.operation_logs.write();
            logs.remove(&transaction_id);
        }
        
        // 添加清除记录到历史
        let entry = TransactionLogEntry {
            transaction_id,
            log_type: TransactionLogType::Operation,
            timestamp: Self::current_timestamp(),
            message: format!("Transaction {} log cleared", transaction_id),
            operation: None,
        };
        
        self.add_history_entry(entry);
        Ok(())
    }
    
    fn clear_all_logs(&self) -> Result<(), TransactionError> {
        // 清除所有操作日志
        {
            let mut logs = self.operation_logs.write();
            logs.clear();
        }
        
        // 清除所有历史日志
        {
            let mut history = self.history_logs.write();
            history.clear();
        }
        
        // 重置统计信息
        self.update_stats(|stats| {
            *stats = TransactionLogStats::new();
        });
        
        Ok(())
    }
    
    fn get_log_stats(&self) -> Result<TransactionLogStats, TransactionError> {
        let mut stats = self.stats.read().clone();
        
        // 更新实时统计信息
        {
            let _logs = self.operation_logs.read();
            let history = self.history_logs.read();
            log::trace!("Collecting transaction statistics from {} entries", history.len());
            
            stats.total_entries = self.operation_counter.load(Ordering::SeqCst) as usize + history.len();
            stats.operation_entries = self.operation_counter.load(Ordering::SeqCst) as usize;
        }
        
        Ok(stats)
    }
}

impl Default for MemoryTransactionLog {
    fn default() -> Self {
        Self::new()
    }
}

/// 事务日志查询器
///
/// 提供事务日志的查询功能
pub struct TransactionLogQuery {
    /// 事务日志引用
    log: Arc<dyn TransactionLog>,
}

impl TransactionLogQuery {
    /// 创建新的事务日志查询器
    pub fn new(log: Arc<dyn TransactionLog>) -> Self {
        Self { log }
    }
    
    /// 查询指定事务的所有操作
    pub fn get_transaction_operations(&self, transaction_id: TransactionId) -> Result<Vec<TransactionOperation>, TransactionError> {
        self.log.get_transaction_operations(transaction_id)
    }
    
    /// 查询最近的事务历史
    pub fn get_recent_history(&self, count: usize) -> Result<Vec<TransactionLogEntry>, TransactionError> {
        self.log.get_transaction_history(Some(count))
    }
    
    /// 查询所有事务历史
    pub fn get_all_history(&self) -> Result<Vec<TransactionLogEntry>, TransactionError> {
        self.log.get_transaction_history(None)
    }
    
    /// 查询指定时间范围的历史
    pub fn get_history_by_time_range(
        &self,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<TransactionLogEntry>, TransactionError> {
        let all_history = self.log.get_transaction_history(None)?;
        let filtered: Vec<TransactionLogEntry> = all_history
            .into_iter()
            .filter(|entry| entry.timestamp >= start_time && entry.timestamp <= end_time)
            .collect();
        
        Ok(filtered)
    }
    
    /// 查询指定类型的历史
    pub fn get_history_by_type(
        &self,
        log_type: TransactionLogType,
    ) -> Result<Vec<TransactionLogEntry>, TransactionError> {
        let all_history = self.log.get_transaction_history(None)?;
        let filtered: Vec<TransactionLogEntry> = all_history
            .into_iter()
            .filter(|entry| entry.log_type == log_type)
            .collect();
        
        Ok(filtered)
    }
    
    /// 查询指定事务的历史
    pub fn get_transaction_history(
        &self,
        transaction_id: TransactionId,
    ) -> Result<Vec<TransactionLogEntry>, TransactionError> {
        let all_history = self.log.get_transaction_history(None)?;
        let filtered: Vec<TransactionLogEntry> = all_history
            .into_iter()
            .filter(|entry| entry.transaction_id == transaction_id)
            .collect();
        
        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    
    #[test]
    fn test_memory_transaction_log_creation() {
        let log = MemoryTransactionLog::new();
        let stats = log.get_log_stats().unwrap();
        
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.operation_entries, 0);
    }
    
    #[test]
    fn test_log_transaction_start() {
        let log = MemoryTransactionLog::new();
        let transaction_id = TransactionId::new(1);
        
        let result = log.log_transaction_start(transaction_id);
        assert!(result.is_ok());
        
        let history = log.get_transaction_history(None).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].log_type, TransactionLogType::Start);
        assert_eq!(history[0].transaction_id, transaction_id);
    }
    
    #[test]
    fn test_log_operation() {
        let log = MemoryTransactionLog::new();
        let transaction_id = TransactionId::new(1);
        let operation = TransactionOperation::Create {
            entity_type: "TestEntity",
            entity_id: crate::domain::repositories::EntityId::new(1),
            data: vec![1, 2, 3],
        };
        
        let result = log.log_operation(transaction_id, &operation);
        assert!(result.is_ok());
        
        let operations = log.get_transaction_operations(transaction_id).unwrap();
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].entity_type, "TestEntity");
    }
    
    #[test]
    fn test_log_transaction_commit() {
        let log = MemoryTransactionLog::new();
        let transaction_id = TransactionId::new(1);
        
        let result = log.log_transaction_commit(transaction_id);
        assert!(result.is_ok());
        
        let history = log.get_transaction_history(None).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].log_type, TransactionLogType::Commit);
    }
    
    #[test]
    fn test_log_transaction_rollback() {
        let log = MemoryTransactionLog::new();
        let transaction_id = TransactionId::new(1);
        
        let result = log.log_transaction_rollback(transaction_id);
        assert!(result.is_ok());
        
        let history = log.get_transaction_history(None).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].log_type, TransactionLogType::Rollback);
    }
    
    #[test]
    fn test_clear_transaction_log() {
        let log = MemoryTransactionLog::new();
        let transaction_id = TransactionId::new(1);
        let operation = TransactionOperation::Create {
            entity_type: "TestEntity",
            entity_id: crate::domain::repositories::EntityId::new(1),
            data: vec![1, 2, 3],
        };
        
        // 添加操作
        let _ = log.log_operation(transaction_id, &operation);
        assert_eq!(log.get_transaction_operations(transaction_id).unwrap().len(), 1);
        
        // 清除日志
        let result = log.clear_transaction_log(transaction_id);
        assert!(result.is_ok());
        
        // 验证已清除
        assert_eq!(log.get_transaction_operations(transaction_id).unwrap().len(), 0);
    }
    
    #[test]
    fn test_transaction_log_query() {
        let log = Arc::new(MemoryTransactionLog::new());
        let query = TransactionLogQuery::new(log.clone());
        
        let transaction_id = TransactionId::new(1);
        let operation = TransactionOperation::Create {
            entity_type: "TestEntity",
            entity_id: crate::domain::repositories::EntityId::new(1),
            data: vec![1, 2, 3],
        };
        
        // 添加操作
        let _ = log.log_operation(transaction_id, &operation);
        
        // 查询操作
        let operations = query.get_transaction_operations(transaction_id).unwrap();
        assert_eq!(operations.len(), 1);
        
        // 查询历史
        let history = query.get_all_history().unwrap();
        assert_eq!(history.len(), 1);
    }
    
    #[test]
    fn test_log_stats() {
        let log = MemoryTransactionLog::new();
        let transaction_id = TransactionId::new(1);
        
        // 添加多个操作
        for i in 0..5 {
            let operation = TransactionOperation::Create {
                entity_type: "TestEntity",
                entity_id: crate::domain::repositories::EntityId::new(i),
                data: vec![i as u8],
            };
            let _ = log.log_operation(transaction_id, &operation);
        }
        
        let stats = log.get_log_stats().unwrap();
        assert_eq!(stats.operation_entries, 5);
        assert_eq!(stats.total_entries, 5);
    }
    
    #[test]
    fn test_max_history_limit() {
        let log = MemoryTransactionLog::with_max_history(3);
        let log = MemoryTransactionLog::with_max_history(3);
        let transaction_id = TransactionId::new(1);
        
        // 添加超过限制的历史记录
        let _ = log.log_transaction_start(transaction_id);
        for i in 1..5 {
            let _ = log.log_transaction_start(TransactionId::new(i));
        }
        
        let history = log.get_transaction_history(None).unwrap();
        assert_eq!(history.len(), 3); // 应该只保留最新的3条
        
        // 验证保留的是最新的记录
        assert_eq!(history[0].transaction_id, TransactionId::new(2));
        assert_eq!(history[1].transaction_id, TransactionId::new(3));
        assert_eq!(history[2].transaction_id, TransactionId::new(4));
    }
}