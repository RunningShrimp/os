//! Core Database Engine
//!
//! Main database interface combining all components:
//! - Multiple storage engines (B+Tree, LSM-Tree)
//! - SQL processing
//! - Transaction management
//! - Query optimization
//! - Caching

use super::parser::parse_sql;
use super::executor::{Executor, ExecutionResult};
use super::transaction::{TransactionManager, Transaction, IsolationLevel};
use super::wal::WalManager;
use super::catalog::CatalogManager;
use super::btree::BTree;
use super::lsm::LsmTree;
use super::types::TxId;
use super::{DbError, DbResult};
use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;

/// Storage engine type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageEngine {
    BTree,
    LsmTree,
}

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub storage_engine: StorageEngine,
    pub cache_size: usize,
    pub enable_wal: bool,
    pub checkpoint_interval: u64,
    pub page_size: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            storage_engine: StorageEngine::BTree,
            cache_size: 1024,
            enable_wal: true,
            checkpoint_interval: 1000,
            page_size: 4096,
        }
    }
}

/// Main database engine
pub struct Database {
    name: String,
    config: DatabaseConfig,
    executor: Executor,
    tx_manager: Arc<TransactionManager>,
    wal: Option<Arc<WalManager>>,
    catalog: Arc<CatalogManager>,
    storage: Mutex<Storage>,
    query_cache: Mutex<BTreeMap<String, CachedResult>>,
    stats: Mutex<DatabaseStats>,
}

/// Storage abstraction
enum Storage {
    BTree(BTree),
    LsmTree(LsmTree),
}

/// Cached query result
#[derive(Debug, Clone)]
struct CachedResult {
    result: ExecutionResult,
    timestamp: u64,
    hit_count: usize,
}

/// Database statistics
#[derive(Debug, Clone, Default)]
struct DatabaseStats {
    queries_executed: u64,
    cache_hits: u64,
    cache_misses: u64,
    transactions_committed: u64,
    transactions_rolled_back: u64,
}

impl Database {
    /// Open a database
    pub fn open(name: impl Into<String>, config: DatabaseConfig) -> DbResult<Self> {
        let name = name.into();

        // Initialize storage
        let storage = match config.storage_engine {
            StorageEngine::BTree => {
                Storage::BTree(BTree::new(super::btree::BTreeConfig::default()))
            }
            StorageEngine::LsmTree => {
                Storage::LsmTree(LsmTree::new(super::lsm::LsmConfig::default()))
            }
        };

        // Initialize WAL if enabled
        let wal = if config.enable_wal {
            Some(Arc::new(WalManager::new(Some(format!("{}.wal", name)))?))
        } else {
            None
        };

        // Initialize catalog
        let catalog = Arc::new(CatalogManager::new());
        catalog.create_schema("public")?;

        Ok(Self {
            name,
            config,
            executor: Executor::new(),
            tx_manager: Arc::new(TransactionManager::new()),
            wal,
            catalog,
            storage: Mutex::new(storage),
            query_cache: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(DatabaseStats::default()),
        })
    }

    /// Execute a SQL query
    pub fn execute(&self, sql: &str) -> DbResult<ExecutionResult> {
        // Check cache for SELECT queries
        let cache_key = if self.config.cache_size > 0 && sql.to_uppercase().starts_with("SELECT") {
            Some(sql.to_string())
        } else {
            None
        };

        if let Some(key) = &cache_key {
            if let Some(cached) = self.check_cache(key) {
                let mut stats = self.stats.lock();
                stats.cache_hits += 1;
                stats.queries_executed += 1;
                return Ok(cached);
            }

            let mut stats = self.stats.lock();
            stats.cache_misses += 1;
        }

        // Parse SQL
        let ast = parse_sql(sql)?;

        // Execute
        let result = self.executor.execute(ast)?;

        // Update cache
        if let (Some(key), ExecutionResult::Query { .. }) = (&cache_key, &result) {
            self.update_cache(key, result.clone());
        }

        let mut stats = self.stats.lock();
        stats.queries_executed += 1;

        Ok(result)
    }

    /// Execute a SQL query with result rows
    pub fn query(&self, sql: &str) -> DbResult<Vec<Vec<super::types::Value>>> {
        let result = self.execute(sql)?;

        match result {
            ExecutionResult::Query { rows, .. } => Ok(rows),
            _ => Err(DbError::Query("Expected query result".into())),
        }
    }

    /// Begin a transaction
    pub fn begin_transaction(&self) -> DbResult<Transaction> {
        self.tx_manager.begin(IsolationLevel::ReadCommitted)
    }

    /// Begin a transaction with specified isolation level
    pub fn begin_transaction_with_level(
        &self,
        isolation_level: IsolationLevel,
    ) -> DbResult<Transaction> {
        self.tx_manager.begin(isolation_level)
    }

    /// Commit a transaction
    pub fn commit_transaction(&self, tx: &Transaction) -> DbResult<()> {
        self.tx_manager.commit(tx)?;

        // Write to WAL
        if let Some(wal) = &self.wal {
            wal.write_tx_commit(tx.id)?;
        }

        let mut stats = self.stats.lock();
        stats.transactions_committed += 1;

        Ok(())
    }

    /// Rollback a transaction
    pub fn rollback_transaction(&self, tx: &Transaction) -> DbResult<()> {
        self.tx_manager.rollback(tx)?;

        // Write to WAL
        if let Some(wal) = &self.wal {
            wal.write_tx_rollback(tx.id)?;
        }

        let mut stats = self.stats.lock();
        stats.transactions_rolled_back += 1;

        Ok(())
    }

    /// Create a checkpoint
    pub fn checkpoint(&self) -> DbResult<()> {
        if let Some(wal) = &self.wal {
            wal.checkpoint()?;
        }
        Ok(())
    }

    /// Get database statistics
    pub fn stats(&self) -> DatabaseStats {
        self.stats.lock().clone()
    }

    /// Get catalog
    pub fn catalog(&self) -> &Arc<CatalogManager> {
        &self.catalog
    }

    /// Clear query cache
    pub fn clear_cache(&self) -> DbResult<()> {
        let mut cache = self.query_cache.lock();
        cache.clear();
        Ok(())
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.query_cache.lock().len()
    }

    /// Close the database
    pub fn close(&self) -> DbResult<()> {
        // Flush WAL
        if let Some(wal) = &self.wal {
            wal.flush()?;
            wal.checkpoint()?;
        }

        Ok(())
    }

    // Cache management

    fn check_cache(&self, key: &str) -> Option<ExecutionResult> {
        let cache = self.query_cache.lock();

        if let Some(cached) = cache.get(key) {
            // Simple cache validation (no TTL)
            return Some(cached.result.clone());
        }

        None
    }

    fn update_cache(&self, key: String, result: ExecutionResult) {
        let mut cache = self.query_cache.lock();

        // Evict if cache is full
        if cache.len() >= self.config.cache_size {
            // Simple FIFO eviction
            if let Some(first_key) = cache.keys().next().cloned() {
                cache.remove(&first_key);
            }
        }

        cache.insert(key, CachedResult {
            result,
            timestamp: 0, // Would use actual timestamp
            hit_count: 0,
        });
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            config: self.config.clone(),
            executor: Executor::new(), // Create new executor
            tx_manager: Arc::clone(&self.tx_manager),
            wal: self.wal.as_ref().map(Arc::clone),
            catalog: Arc::clone(&self.catalog),
            storage: Mutex::new(Storage::BTree(BTree::new(super::btree::BTreeConfig::default()))),
            query_cache: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(DatabaseStats::default()),
        }
    }
}

/// Connection to the database
pub struct Connection {
    db: Arc<Database>,
    current_transaction: Option<Transaction>,
}

impl Connection {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            current_transaction: None,
        }
    }

    /// Execute SQL
    pub fn execute(&self, sql: &str) -> DbResult<ExecutionResult> {
        self.db.execute(sql)
    }

    /// Begin transaction
    pub fn begin(&mut self) -> DbResult<()> {
        if self.current_transaction.is_some() {
            return Err(DbError::Transaction("Transaction already active".into()));
        }

        let tx = self.db.begin_transaction()?;
        self.current_transaction = Some(tx);
        Ok(())
    }

    /// Commit transaction
    pub fn commit(&mut self) -> DbResult<()> {
        let tx = self.current_transaction
            .take()
            .ok_or_else(|| DbError::Transaction("No active transaction".into()))?;

        self.db.commit_transaction(&tx)
    }

    /// Rollback transaction
    pub fn rollback(&mut self) -> DbResult<()> {
        let tx = self.current_transaction
            .take()
            .ok_or_else(|| DbError::Transaction("No active transaction".into()))?;

        self.db.rollback_transaction(&tx)
    }
}

/// Connection pool
pub struct ConnectionPool {
    db: Arc<Database>,
    connections: Mutex<Vec<Connection>>,
    max_size: usize,
}

impl ConnectionPool {
    pub fn new(db: Arc<Database>, max_size: usize) -> Self {
        Self {
            db,
            connections: Mutex::new(Vec::new()),
            max_size,
        }
    }

    pub fn acquire(&self) -> DbResult<Connection> {
        let mut connections = self.connections.lock();

        if let Some(conn) = connections.pop() {
            Ok(conn)
        } else {
            Ok(Connection::new(Arc::clone(&self.db)))
        }
    }

    pub fn release(&self, conn: Connection) {
        let mut connections = self.connections.lock();

        if connections.len() < self.max_size {
            connections.push(conn);
        }
    }

    pub fn size(&self) -> usize {
        self.connections.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_open() {
        let db = Database::open("test", DatabaseConfig::default()).unwrap();
        assert_eq!(db.name, "test");
    }

    #[test]
    fn test_execute_create_table() {
        let db = Database::open("test", DatabaseConfig::default()).unwrap();
        let result = db.execute("CREATE TABLE users (id INT, name TEXT)").unwrap();
        assert!(matches!(result, ExecutionResult::Schema { .. }));
    }

    #[test]
    fn test_execute_insert_select() {
        let db = Database::open("test", DatabaseConfig::default()).unwrap();

        db.execute("CREATE TABLE users (id INT, name TEXT)").unwrap();
        db.execute("INSERT INTO users VALUES (1, 'Alice')").unwrap();

        let rows = db.query("SELECT * FROM users").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_transaction() {
        let db = Database::open("test", DatabaseConfig::default()).unwrap();

        db.execute("CREATE TABLE users (id INT, name TEXT)").unwrap();

        let mut conn = Connection::new(Arc::new(db));

        conn.begin().unwrap();
        conn.execute("INSERT INTO users VALUES (1, 'Alice')").unwrap();
        conn.commit().unwrap();

        let rows = conn.execute("SELECT * FROM users").unwrap();
        assert!(matches!(rows, ExecutionResult::Query { .. }));
    }
}
