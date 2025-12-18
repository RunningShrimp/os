//! 事务基础设施模块
//!
//! 提供事务管理的基础设施实现，包括内存事务、事务管理器和事务日志。

pub mod memory_transaction;
pub mod transaction_manager;
pub mod transaction_log;

// 重新导出主要类型
pub use memory_transaction::{MemoryTransaction, TransactionPool};
pub use transaction_manager::MemoryTransactionManager;
pub use transaction_log::{MemoryTransactionLog, TransactionLogQuery};