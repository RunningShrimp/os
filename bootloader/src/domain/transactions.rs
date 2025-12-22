//! 事务管理接口
//!
//! 定义了事务管理的核心接口，包括事务操作、事务状态和事务管理器。
//! 提供ACID事务特性支持，确保数据一致性和完整性。

use super::repositories::{EntityId, TransactionId, RepositoryError};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// 事务操作类型
///
/// 定义了事务中可以执行的操作类型
#[derive(Debug, Clone)]
pub enum TransactionOperation {
    /// 创建实体操作
    Create {
        /// 实体类型
        entity_type: &'static str,
        /// 实体ID
        entity_id: EntityId,
        /// 序列化的实体数据
        data: Vec<u8>,
    },
    /// 更新实体操作
    Update {
        /// 实体类型
        entity_type: &'static str,
        /// 实体ID
        entity_id: EntityId,
        /// 原始数据（用于回滚）
        old_data: Vec<u8>,
        /// 新数据
        new_data: Vec<u8>,
    },
    /// 删除实体操作
    Delete {
        /// 实体类型
        entity_type: &'static str,
        /// 实体ID
        entity_id: EntityId,
        /// 被删除实体的数据（用于回滚）
        old_data: Vec<u8>,
    },
}

impl TransactionOperation {
    /// 获取操作类型名称
    pub fn operation_type(&self) -> &'static str {
        match self {
            TransactionOperation::Create { .. } => "Create",
            TransactionOperation::Update { .. } => "Update",
            TransactionOperation::Delete { .. } => "Delete",
        }
    }
    
    /// 获取实体类型
    pub fn entity_type(&self) -> &'static str {
        match self {
            TransactionOperation::Create { entity_type, .. } => entity_type,
            TransactionOperation::Update { entity_type, .. } => entity_type,
            TransactionOperation::Delete { entity_type, .. } => entity_type,
        }
    }
    
    /// 获取实体ID
    pub fn entity_id(&self) -> EntityId {
        match self {
            TransactionOperation::Create { entity_id, .. } => *entity_id,
            TransactionOperation::Update { entity_id, .. } => *entity_id,
            TransactionOperation::Delete { entity_id, .. } => *entity_id,
        }
    }
}

/// 事务状态
///
/// 定义了事务的生命周期状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    /// 活跃状态 - 事务正在进行中
    Active,
    /// 已提交 - 事务已成功完成
    Committed,
    /// 已回滚 - 事务被撤销
    RolledBack,
    /// 超时 - 事务执行超时
    Timeout,
    /// 失败 - 事务执行失败
    Failed,
}

impl TransactionStatus {
    /// 检查事务是否已完成（提交或回滚）
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Committed | Self::RolledBack | Self::Timeout | Self::Failed)
    }
    
    /// 检查事务是否可以执行操作
    pub fn can_execute_operations(&self) -> bool {
        matches!(self, Self::Active)
    }
}

impl fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionStatus::Active => write!(f, "Active"),
            TransactionStatus::Committed => write!(f, "Committed"),
            TransactionStatus::RolledBack => write!(f, "RolledBack"),
            TransactionStatus::Timeout => write!(f, "Timeout"),
            TransactionStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// 事务错误类型
///
/// 定义了事务操作可能出现的错误
#[derive(Debug, Clone)]
pub enum TransactionError {
    /// 事务未找到
    TransactionNotFound(TransactionId),
    /// 事务已提交
    TransactionAlreadyCommitted(TransactionId),
    /// 事务已回滚
    TransactionAlreadyRolledBack(TransactionId),
    /// 事务超时
    TransactionTimeout(TransactionId),
    /// 事务冲突
    TransactionConflict(TransactionId),
    /// 操作错误
    OperationError(&'static str),
    /// 验证错误
    ValidationError(&'static str),
    /// 存储错误
    StorageError(String),
    /// 并发错误
    ConcurrencyError(&'static str),
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::TransactionNotFound(id) => write!(f, "Transaction not found: {}", id),
            TransactionError::TransactionAlreadyCommitted(id) => write!(f, "Transaction already committed: {}", id),
            TransactionError::TransactionAlreadyRolledBack(id) => write!(f, "Transaction already rolled back: {}", id),
            TransactionError::TransactionTimeout(id) => write!(f, "Transaction timeout: {}", id),
            TransactionError::TransactionConflict(id) => write!(f, "Transaction conflict: {}", id),
            TransactionError::OperationError(msg) => write!(f, "Operation error: {}", msg),
            TransactionError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            TransactionError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            TransactionError::ConcurrencyError(msg) => write!(f, "Concurrency error: {}", msg),
        }
    }
}

impl From<RepositoryError> for TransactionError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::StorageError(msg) => TransactionError::StorageError(msg.to_string()),
            RepositoryError::ConcurrencyError(msg) => TransactionError::ConcurrencyError(msg),
            RepositoryError::ValidationError(msg) => TransactionError::ValidationError(msg),
            RepositoryError::TransactionError(msg) => TransactionError::StorageError(msg),
            _ => TransactionError::OperationError("Repository error occurred"),
        }
    }
}

/// 事务接口
///
/// 定义了事务的基本操作和属性
pub trait Transaction: Send + Sync {
    /// 获取事务ID
    fn id(&self) -> TransactionId;
    
    /// 获取事务状态
    fn status(&self) -> TransactionStatus;
    
    /// 添加操作到事务
    fn add_operation(&mut self, operation: TransactionOperation) -> Result<(), TransactionError>;
    
    /// 提交事务
    fn commit(&mut self) -> Result<(), TransactionError>;
    
    /// 回滚事务
    fn rollback(&mut self) -> Result<(), TransactionError>;
    
    /// 获取事务操作列表
    fn operations(&self) -> Vec<TransactionOperation>;
    
    /// 获取事务开始时间
    fn start_time(&self) -> u64;
    
    /// 获取事务超时时间（毫秒）
    fn timeout(&self) -> Option<u64>;
    
    /// 设置事务超时时间
    fn set_timeout(&mut self, timeout_ms: u64);
    
    /// 检查事务是否超时
    fn is_timeout(&self) -> bool;
    
    /// 获取事务操作数量
    fn operation_count(&self) -> usize;
    
    /// 检查事务是否为空（没有操作）
    fn is_empty(&self) -> bool;
    
    /// 克隆事务
    fn clone_box(&self) -> Box<dyn Transaction>;
}

/// 事务管理器接口
///
/// 定义了事务管理的核心功能
pub trait TransactionManager: Send + Sync {
    /// 开始新事务
    fn begin_transaction(&self) -> Result<Box<dyn Transaction>, TransactionError>;
    
    /// 开始带超时的事务
    fn begin_transaction_with_timeout(&self, timeout_ms: u64) -> Result<Box<dyn Transaction>, TransactionError>;
    
    /// 提交事务
    fn commit_transaction(&self, transaction_id: TransactionId) -> Result<(), TransactionError>;
    
    /// 回滚事务
    fn rollback_transaction(&self, transaction_id: TransactionId) -> Result<(), TransactionError>;
    
    /// 获取当前事务状态
    fn get_transaction_status(&self, transaction_id: TransactionId) -> Result<TransactionStatus, TransactionError>;
    
    /// 获取活跃事务列表
    fn get_active_transactions(&self) -> Result<Vec<TransactionId>, TransactionError>;
    
    /// 清理已完成的事务
    fn cleanup_completed_transactions(&self) -> Result<usize, TransactionError>;
    
    /// 设置默认事务超时时间
    fn set_default_timeout(&mut self, timeout_ms: u64);
    
    /// 获取默认事务超时时间
    fn default_timeout(&self) -> u64;
    
    /// 获取事务统计信息
    fn get_transaction_stats(&self) -> Result<TransactionStats, TransactionError>;
}

/// 事务统计信息
///
/// 提供事务管理的统计数据
#[derive(Debug, Clone)]
pub struct TransactionStats {
    /// 活跃事务数量
    pub active_transactions: usize,
    /// 已提交事务数量
    pub committed_transactions: usize,
    /// 已回滚事务数量
    pub rolled_back_transactions: usize,
    /// 超时事务数量
    pub timeout_transactions: usize,
    /// 失败事务数量
    pub failed_transactions: usize,
    /// 总事务数量
    pub total_transactions: usize,
    /// 平均事务执行时间（毫秒）
    pub average_execution_time_ms: u64,
    /// 最长事务执行时间（毫秒）
    pub max_execution_time_ms: u64,
    /// 事务管理器运行时间（毫秒）
    pub uptime_ms: u64,
}

impl TransactionStats {
    /// 创建新的事务统计信息
    pub fn new() -> Self {
        Self {
            active_transactions: 0,
            committed_transactions: 0,
            rolled_back_transactions: 0,
            timeout_transactions: 0,
            failed_transactions: 0,
            total_transactions: 0,
            average_execution_time_ms: 0,
            max_execution_time_ms: 0,
            uptime_ms: 0,
        }
    }
    
    /// 获取成功事务数量（提交的事务）
    pub fn successful_transactions(&self) -> usize {
        self.committed_transactions
    }
    
    /// 获取失败事务数量（回滚、超时、失败的事务）
    pub fn unsuccessful_transactions(&self) -> usize {
        self.rolled_back_transactions + self.timeout_transactions + self.failed_transactions
    }
    
    /// 获取成功率（百分比）
    pub fn success_rate(&self) -> f64 {
        if self.total_transactions == 0 {
            0.0
        } else {
            (self.committed_transactions as f64 / self.total_transactions as f64) * 100.0
        }
    }
}

impl Default for TransactionStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 事务日志接口
///
/// 定义了事务日志记录的功能
pub trait TransactionLog: Send + Sync {
    /// 记录事务操作
    fn log_operation(&self, transaction_id: TransactionId, operation: &TransactionOperation) -> Result<(), TransactionError>;
    
    /// 记录事务开始
    fn log_transaction_start(&self, transaction_id: TransactionId) -> Result<(), TransactionError>;
    
    /// 记录事务提交
    fn log_transaction_commit(&self, transaction_id: TransactionId) -> Result<(), TransactionError>;
    
    /// 记录事务回滚
    fn log_transaction_rollback(&self, transaction_id: TransactionId) -> Result<(), TransactionError>;
    
    /// 记录事务超时
    fn log_transaction_timeout(&self, transaction_id: TransactionId) -> Result<(), TransactionError>;
    
    /// 记录事务失败
    fn log_transaction_failure(&self, transaction_id: TransactionId, error: &TransactionError) -> Result<(), TransactionError>;
    
    /// 获取事务操作日志
    fn get_transaction_operations(&self, transaction_id: TransactionId) -> Result<Vec<TransactionOperation>, TransactionError>;
    
    /// 获取事务历史记录
    fn get_transaction_history(&self, limit: Option<usize>) -> Result<Vec<TransactionLogEntry>, TransactionError>;
    
    /// 清理事务日志
    fn clear_transaction_log(&self, transaction_id: TransactionId) -> Result<(), TransactionError>;
    
    /// 清理所有事务日志
    fn clear_all_logs(&self) -> Result<(), TransactionError>;
    
    /// 获取日志统计信息
    fn get_log_stats(&self) -> Result<TransactionLogStats, TransactionError>;
}

/// 事务日志条目
///
/// 表示事务日志中的一个条目
#[derive(Debug, Clone)]
pub struct TransactionLogEntry {
    /// 事务ID
    pub transaction_id: TransactionId,
    /// 日志类型
    pub log_type: TransactionLogType,
    /// 时间戳
    pub timestamp: u64,
    /// 消息内容
    pub message: String,
    /// 相关的操作（可选）
    pub operation: Option<TransactionOperation>,
}

/// 事务日志类型
///
/// 事务日志类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionLogType {
    /// 事务开始
    Start,
    /// 操作记录
    Operation,
    /// 事务提交
    Commit,
    /// 事务回滚
    Rollback,
    /// 事务超时
    Timeout,
    /// 事务失败
    Failure,
}

/// 事务日志统计信息
///
/// 提供事务日志的统计数据
#[derive(Debug, Clone)]
pub struct TransactionLogStats {
    /// 总日志条目数
    pub total_entries: usize,
    /// 开始事务日志数
    pub start_entries: usize,
    /// 操作日志数
    pub operation_entries: usize,
    /// 提交日志数
    pub commit_entries: usize,
    /// 回滚日志数
    pub rollback_entries: usize,
    /// 超时日志数
    pub timeout_entries: usize,
    /// 失败日志数
    pub failure_entries: usize,
    /// 最早日志时间戳
    pub earliest_timestamp: Option<u64>,
    /// 最新日志时间戳
    pub latest_timestamp: Option<u64>,
}

impl TransactionLogStats {
    /// 创建新的日志统计信息
    pub fn new() -> Self {
        Self {
            total_entries: 0,
            start_entries: 0,
            operation_entries: 0,
            commit_entries: 0,
            rollback_entries: 0,
            timeout_entries: 0,
            failure_entries: 0,
            earliest_timestamp: None,
            latest_timestamp: None,
        }
    }
}

impl Default for TransactionLogStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transaction_operation() {
        let operation = TransactionOperation::Create {
            entity_type: "TestEntity",
            entity_id: EntityId::new(1),
            data: vec![1, 2, 3],
        };
        
        assert_eq!(operation.operation_type(), "Create");
        assert_eq!(operation.entity_type(), "TestEntity");
        assert_eq!(operation.entity_id(), EntityId::new(1));
    }
    
    #[test]
    fn test_transaction_status() {
        assert!(TransactionStatus::Active.can_execute_operations());
        assert!(!TransactionStatus::Committed.can_execute_operations());
        assert!(TransactionStatus::Committed.is_completed());
        assert!(!TransactionStatus::Active.is_completed());
    }
    
    #[test]
    fn test_transaction_stats() {
        let mut stats = TransactionStats::new();
        stats.committed_transactions = 10;
        stats.rolled_back_transactions = 3;
        stats.total_transactions = 13;
        
        assert_eq!(stats.successful_transactions(), 10);
        assert_eq!(stats.unsuccessful_transactions(), 3);
        assert!((stats.success_rate() - 76.92).abs() < 0.01);
    }
    
    #[test]
    fn test_transaction_log_entry() {
        let entry = TransactionLogEntry {
            transaction_id: TransactionId::new(1),
            log_type: TransactionLogType::Start,
            timestamp: 123456789,
            message: "Transaction started".to_string(),
            operation: None,
        };
        
        assert_eq!(entry.transaction_id, TransactionId::new(1));
        assert_eq!(entry.log_type, TransactionLogType::Start);
        assert_eq!(entry.timestamp, 123456789);
        assert_eq!(entry.message, "Transaction started");
        assert!(entry.operation.is_none());
    }
}