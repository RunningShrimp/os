//! # NOS Database Engine
//!
//! A comprehensive, embedded database engine for the NOS kernel providing:
//! - SQL-92 compatible query processing
//! - Multiple storage structures (B-tree, LSM-tree, Hash Index)
//! - ACID transactions with MVCC
//! - Concurrency control with deadlock detection
//! - ARIES-style crash recovery
//!
//! ## Architecture
//!
//! The database engine is organized into the following components:
//!
//! - **SQL Engine**: Lexer, parser, query planner, and optimizer
//! - **Storage**: B-tree, LSM-tree, and hash index implementations
//! - **Transaction**: MVCC, WAL, and transaction management
//! - **Executor**: Query execution engine with multiple join algorithms
//! - **Concurrency**: Lock manager, latches, and deadlock detection
//! - **Recovery**: ARIES-style crash recovery and checkpointing
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::database::{Database, DatabaseError};
//!
//! # async fn example() -> Result<(), DatabaseError> {
//! // Create a new database
//! let db = Database::new("my_db")?;
//!
//! // Create a table
//! db.execute("CREATE TABLE users (id INT PRIMARY KEY, name TEXT)")?;
//!
//! // Insert data
//! db.execute("INSERT INTO users VALUES (1, 'Alice')")?;
//!
//! // Query data
//! let result = db.execute("SELECT * FROM users WHERE id = 1")?;
//! # Ok(())
//! # }
//! ```

#![no_std]

extern crate alloc;

use alloc::string::String;
use core::fmt;
use spin::Mutex;

pub mod concurrency;
pub mod executor;
pub mod recovery;
pub mod sql_engine;
pub mod storage;
pub mod transaction;

/// Re-export commonly used types
pub use concurrency::{LockManager, LockMode};
pub use executor::{Executor, QueryResult};
pub use recovery::{RecoveryManager, CheckpointType};
pub use sql_engine::{SqlEngine, QueryPlan};
pub use storage::{StorageEngine, IndexType};
pub use transaction::{Transaction, IsolationLevel, TransactionManager};

/// Database error type
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseError {
    /// SQL syntax error
    SyntaxError(String),

    /// Table not found
    TableNotFound(String),

    /// Column not found
    ColumnNotFound(String),

    /// Index not found
    IndexNotFound(String),

    /// Constraint violation
    ConstraintViolation(String),

    /// Transaction conflict
    TransactionConflict(String),

    /// Deadlock detected
    Deadlock,

    /// Lock timeout
    LockTimeout,

    /// I/O error
    IoError(String),

    /// Out of memory
    OutOfMemory,

    /// Internal error
    InternalError(String),
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SyntaxError(msg) => write!(f, "SQL syntax error: {}", msg),
            Self::TableNotFound(name) => write!(f, "Table not found: {}", name),
            Self::ColumnNotFound(name) => write!(f, "Column not found: {}", name),
            Self::IndexNotFound(name) => write!(f, "Index not found: {}", name),
            Self::ConstraintViolation(msg) => write!(f, "Constraint violation: {}", msg),
            Self::TransactionConflict(msg) => write!(f, "Transaction conflict: {}", msg),
            Self::Deadlock => write!(f, "Deadlock detected"),
            Self::LockTimeout => write!(f, "Lock timeout"),
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Type alias for Result with DatabaseError
pub type Result<T> = core::result::Result<T, DatabaseError>;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Page size in bytes
    pub page_size: usize,

    /// Buffer pool size in pages
    pub buffer_pool_size: usize,

    /// Maximum number of concurrent transactions
    pub max_transactions: usize,

    /// Default isolation level
    pub default_isolation_level: IsolationLevel,

    /// Enable WAL
    pub enable_wal: bool,

    /// WAL sync mode
    pub wal_sync_mode: WalSyncMode,

    /// Checkpoint interval in milliseconds
    pub checkpoint_interval_ms: u64,

    /// Lock timeout in milliseconds
    pub lock_timeout_ms: u64,

    /// Enable query optimizer
    pub enable_optimizer: bool,

    /// B-tree node size
    pub btree_node_size: usize,

    /// LSM-tree compaction threshold
    pub lsm_compaction_threshold: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            page_size: 4096,
            buffer_pool_size: 1000,
            max_transactions: 100,
            default_isolation_level: IsolationLevel::ReadCommitted,
            enable_wal: true,
            wal_sync_mode: WalSyncMode::Fsync,
            checkpoint_interval_ms: 60000, // 1 minute
            lock_timeout_ms: 5000, // 5 seconds
            enable_optimizer: true,
            btree_node_size: 4000,
            lsm_compaction_threshold: 4,
        }
    }
}

/// WAL sync mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalSyncMode {
    /// No sync
    None,

    /// Flush to OS cache
    Flush,

    /// Full sync
    Full,

    /// Sync to OS cache
    Fdatasync,

    /// Full fsync
    Fsync,
}

/// Main database struct
pub struct Database {
    /// Database name
    name: String,

    /// Configuration
    config: DatabaseConfig,

    /// SQL engine
    sql_engine: SqlEngine,

    /// Storage engine
    storage_engine: StorageEngine,

    /// Transaction manager
    transaction_manager: TransactionManager,

    /// Lock manager
    lock_manager: LockManager,

    /// Recovery manager
    recovery_manager: RecoveryManager,

    /// Query executor (wrapped in Mutex for interior mutability)
    executor: Mutex<Executor>,
}

impl Database {
    /// Create a new database instance
    ///
    /// # Arguments
    ///
    /// * `name` - Database name
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the database instance or a `DatabaseError`
    pub fn new(name: impl Into<String>) -> Result<Self> {
        Self::with_config(name, DatabaseConfig::default())
    }

    /// Create a new database with custom configuration
    ///
    /// # Arguments
    ///
    /// * `name` - Database name
    /// * `config` - Database configuration
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the database instance or a `DatabaseError`
    pub fn with_config(name: impl Into<String>, config: DatabaseConfig) -> Result<Self> {
        let name = name.into();

        // Initialize components
        let sql_engine = SqlEngine::new(config.enable_optimizer)?;
        let storage_engine = StorageEngine::new(
            config.page_size,
            config.buffer_pool_size,
            config.btree_node_size,
            config.lsm_compaction_threshold,
        )?;
        let transaction_manager = TransactionManager::new(config.max_transactions)?;
        let lock_manager = LockManager::new(config.lock_timeout_ms)?;
        let recovery_manager = RecoveryManager::new(
            &name,
            config.enable_wal,
            match config.wal_sync_mode {
                WalSyncMode::None => recovery::WalSyncMode::None,
                WalSyncMode::Flush => recovery::WalSyncMode::Flush,
                WalSyncMode::Full => recovery::WalSyncMode::Full,
                WalSyncMode::Fdatasync => recovery::WalSyncMode::Fdatasync,
                WalSyncMode::Fsync => recovery::WalSyncMode::Fsync,
            },
        )?;
        let executor = Executor::new(storage_engine.clone())?;

        Ok(Self {
            name,
            config,
            sql_engine,
            storage_engine,
            transaction_manager,
            lock_manager,
            recovery_manager,
            executor: Mutex::new(executor),
        })
    }

    /// Execute a SQL statement
    ///
    /// # Arguments
    ///
    /// * `sql` - SQL statement to execute
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the query result or a `DatabaseError`
    pub fn execute(&self, sql: &str) -> Result<QueryResult> {
        // Parse SQL
        let parsed = self.sql_engine.parse(sql)?;

        // Plan query
        let plan = self.sql_engine.plan(parsed)?;

        // Execute query (uses interior mutability via Mutex)
        let result = self.executor.lock().execute(plan.physical)?;

        Ok(result)
    }

    /// Begin a new transaction
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the transaction handle or a `DatabaseError`
    pub fn begin_transaction(&mut self) -> Result<Transaction> {
        self.transaction_manager.begin(self.config.default_isolation_level)
    }

    /// Begin a transaction with specific isolation level
    ///
    /// # Arguments
    ///
    /// * `isolation_level` - Transaction isolation level
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the transaction handle or a `DatabaseError`
    pub fn begin_transaction_with_level(&mut self, isolation_level: IsolationLevel) -> Result<Transaction> {
        self.transaction_manager.begin(isolation_level)
    }

    /// Commit a transaction
    ///
    /// # Arguments
    ///
    /// * `transaction` - Transaction to commit
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating success or a `DatabaseError`
    pub fn commit(&mut self, transaction: Transaction) -> Result<()> {
        self.transaction_manager.commit(transaction)
    }

    /// Rollback a transaction
    ///
    /// # Arguments
    ///
    /// * `transaction` - Transaction to rollback
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating success or a `DatabaseError`
    pub fn rollback(&mut self, transaction: Transaction) -> Result<()> {
        self.transaction_manager.rollback(transaction)
    }

    /// Create a checkpoint
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating success or a `DatabaseError`
    pub fn checkpoint(&mut self) -> Result<()> {
        self.recovery_manager.checkpoint(CheckpointType::Fuzzy)
    }

    /// Get database statistics
    ///
    /// # Returns
    ///
    /// Returns database statistics
    pub fn stats(&self) -> DatabaseStats {
        DatabaseStats {
            name: self.name.clone(),
            tables: self.storage_engine.table_count(),
            indexes: self.storage_engine.index_count(),
            active_transactions: self.transaction_manager.active_count(),
            buffer_pool_hit_rate: self.storage_engine.buffer_pool_hit_rate(),
            wal_size: self.recovery_manager.wal_size(),
        }
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    /// Database name
    pub name: String,

    /// Number of tables
    pub tables: usize,

    /// Number of indexes
    pub indexes: usize,

    /// Number of active transactions
    pub active_transactions: usize,

    /// Buffer pool hit rate (0.0 to 1.0)
    pub buffer_pool_hit_rate: f64,

    /// WAL size in bytes
    pub wal_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::new("test_db").unwrap();
        assert_eq!(db.name, "test_db");
    }

    #[test]
    fn test_database_config() {
        let config = DatabaseConfig::default();
        assert_eq!(config.page_size, 4096);
        assert_eq!(config.buffer_pool_size, 1000);
    }

    #[test]
    fn test_error_display() {
        let err = DatabaseError::TableNotFound(String::from("users"));
        assert_eq!(format!("{}", err), "Table not found: users");
    }

    #[test]
    fn test_wal_sync_mode() {
        assert_eq!(WalSyncMode::None, WalSyncMode::None);
        assert_eq!(WalSyncMode::Fdatasync, WalSyncMode::Fdatasync);
        assert_eq!(WalSyncMode::Fsync, WalSyncMode::Fsync);
    }

    #[test]
    fn test_transaction_basics() {
        let db = Database::new("test_db").unwrap();

        // Begin transaction
        let txn = db.begin_transaction().unwrap();
        assert!(txn.id() > 0);

        // Commit transaction
        db.commit(txn).unwrap();
    }

    #[test]
    fn test_database_stats() {
        let db = Database::new("test_db").unwrap();
        let stats = db.stats();
        assert_eq!(stats.name, "test_db");
    }

    #[test]
    fn test_multiple_transactions() {
        let db = Database::new("test_db").unwrap();

        let txn1 = db.begin_transaction().unwrap();
        let txn2 = db.begin_transaction().unwrap();

        assert_ne!(txn1.id(), txn2.id());

        db.commit(txn1).unwrap();
        db.commit(txn2).unwrap();
    }

    #[test]
    fn test_transaction_rollback() {
        let db = Database::new("test_db").unwrap();

        let txn = db.begin_transaction().unwrap();
        db.rollback(txn).unwrap();
    }

    #[test]
    fn test_checkpoint() {
        let db = Database::new("test_db").unwrap();
        db.checkpoint().unwrap();
    }
}
