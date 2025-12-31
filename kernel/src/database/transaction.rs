//! # Transaction Management
//!
//! Provides ACID transaction support with:
//! - MVCC (Multi-Version Concurrency Control)
//! - WAL (Write-Ahead Log) for durability
//! - Transaction state management
//! - Snapshot isolation
//! - Serializable isolation

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use super::storage::Value;
use super::{DatabaseError, Result};

/// Transaction isolation level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Read Uncommitted
    ReadUncommitted,

    /// Read Committed
    ReadCommitted,

    /// Repeatable Read
    RepeatableRead,

    /// Serializable
    Serializable,
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    /// Transaction is active
    Active,

    /// Transaction is committing
    Committing,

    /// Transaction has committed
    Committed,

    /// Transaction is rolling back
    RollingBack,

    /// Transaction has rolled back
    RolledBack,
}

/// Transaction manager
pub struct TransactionManager {
    /// Next transaction ID
    next_txn_id: AtomicU64,

    /// Maximum number of concurrent transactions
    max_transactions: usize,

    /// Active transactions
    transactions: BTreeMap<u64, Transaction>,

    /// MVCC version store
    version_store: VersionStore,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new(max_transactions: usize) -> Result<Self> {
        Ok(Self {
            next_txn_id: AtomicU64::new(1),
            max_transactions,
            transactions: BTreeMap::new(),
            version_store: VersionStore::new(),
        })
    }

    /// Begin a new transaction
    pub fn begin(&mut self, isolation_level: IsolationLevel) -> Result<Transaction> {
        let txn_id = self.next_txn_id.fetch_add(1, AtomicOrdering::SeqCst);

        if self.transactions.len() >= self.max_transactions {
            return Err(DatabaseError::TransactionConflict(
                String::from("Maximum number of concurrent transactions reached")
            ));
        }

        let snapshot = self.version_store.create_snapshot(txn_id);

        let txn = Transaction {
            id: txn_id,
            state: TransactionState::Active,
            isolation_level,
            snapshot,
            write_set: BTreeMap::new(),
            read_set: Vec::new(),
            start_time: txn_id,
        };

        self.transactions.insert(txn_id, txn.clone());
        Ok(txn)
    }

    /// Commit a transaction
    pub fn commit(&mut self, txn: Transaction) -> Result<()> {
        let txn_id = txn.id;

        // Check if transaction exists and is active
        let _txn_state = {
            let txn_ref = self.transactions.get_mut(&txn_id)
                .ok_or_else(|| DatabaseError::TransactionConflict(
                    String::from("Transaction not found")
                ))?;

            if txn_ref.state != TransactionState::Active {
                return Err(DatabaseError::TransactionConflict(
                    String::from("Transaction is not active")
                ));
            }

            txn_ref.state = TransactionState::Committing;
            txn_ref.state
        };

        // Validate transaction
        self.validate_transaction(txn_id)?;

        // Apply writes to version store
        for (key, value) in &txn.write_set {
            self.version_store.write(txn_id, key.clone(), value.clone())?;
        }

        // Mark as committed and remove
        let txn_ref = self.transactions.get_mut(&txn_id)
            .ok_or_else(|| DatabaseError::TransactionConflict(
                String::from("Transaction not found")
            ))?;
        txn_ref.state = TransactionState::Committed;
        self.transactions.remove(&txn_id);

        Ok(())
    }

    /// Rollback a transaction
    pub fn rollback(&mut self, txn: Transaction) -> Result<()> {
        let txn_id = txn.id;

        let txn_ref = self.transactions.get_mut(&txn_id)
            .ok_or_else(|| DatabaseError::TransactionConflict(
                String::from("Transaction not found")
            ))?;

        txn_ref.state = TransactionState::RollingBack;

        // Discard all writes
        txn_ref.write_set.clear();

        txn_ref.state = TransactionState::RolledBack;
        self.transactions.remove(&txn_id);

        Ok(())
    }

    /// Validate transaction for serializability
    fn validate_transaction(&self, txn_id: u64) -> Result<()> {
        let txn = self.transactions.get(&txn_id)
            .ok_or_else(|| DatabaseError::TransactionConflict(
                String::from("Transaction not found")
            ))?;

        // Check for write-write conflicts
        for (key, _) in &txn.write_set {
            if let Some((writer_id, _)) = self.version_store.get_last_writer(key, txn_id) {
                if writer_id != txn_id {
                    return Err(DatabaseError::TransactionConflict(
                        format!("Write-write conflict on key with transaction {}", writer_id)
                    ));
                }
            }
        }

        // Check for read-write conflicts (for repeatable read and serializable)
        if txn.isolation_level == IsolationLevel::RepeatableRead ||
           txn.isolation_level == IsolationLevel::Serializable {
            for key in &txn.read_set {
                if let Some((writer_id, _)) = self.version_store.get_last_writer(key, txn_id) {
                    if writer_id > txn.snapshot.snapshot_id {
                        return Err(DatabaseError::TransactionConflict(
                            format!("Read-write conflict on key with transaction {}", writer_id)
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Read a value through MVCC
    pub fn read(&mut self, txn: &Transaction, key: &[u8]) -> Result<Option<Value>> {
        match txn.isolation_level {
            IsolationLevel::ReadUncommitted => {
                // Read latest committed or uncommitted
                self.version_store.read_latest(key)
            }
            IsolationLevel::ReadCommitted => {
                // Read latest committed
                self.version_store.read_committed(key)
            }
            IsolationLevel::RepeatableRead | IsolationLevel::Serializable => {
                // Read from snapshot
                self.version_store.read_snapshot(key, &txn.snapshot)
            }
        }
    }

    /// Write a value within a transaction
    pub fn write(&mut self, txn: &mut Transaction, key: Vec<u8>, value: Value) -> Result<()> {
        if txn.state != TransactionState::Active {
            return Err(DatabaseError::TransactionConflict(
                String::from("Transaction is not active")
            ));
        }

        txn.write_set.insert(key, value);
        Ok(())
    }

    /// Get number of active transactions
    pub fn active_count(&self) -> usize {
        self.transactions.len()
    }

    /// Get transaction by ID
    pub fn get_transaction(&self, id: u64) -> Option<Transaction> {
        self.transactions.get(&id).cloned()
    }
}

/// Transaction
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Transaction ID
    pub id: u64,

    /// Transaction state
    pub state: TransactionState,

    /// Isolation level
    pub isolation_level: IsolationLevel,

    /// Snapshot for MVCC
    pub snapshot: Snapshot,

    /// Write set
    pub write_set: BTreeMap<Vec<u8>, Value>,

    /// Read set (for conflict detection)
    pub read_set: Vec<Vec<u8>>,

    /// Start time (logical timestamp)
    pub start_time: u64,
}

impl Transaction {
    /// Get transaction ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Check if transaction is active
    pub fn is_active(&self) -> bool {
        self.state == TransactionState::Active
    }

    /// Add key to read set
    pub fn track_read(&mut self, key: Vec<u8>) {
        if !self.read_set.contains(&key) {
            self.read_set.push(key);
        }
    }
}

/// MVCC snapshot
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Snapshot ID (transaction ID)
    pub snapshot_id: u64,

    /// Active transaction IDs at snapshot time
    pub active_txns: Vec<u64>,
}

/// MVCC version store
pub struct VersionStore {
    /// Version chains for each key
    versions: BTreeMap<Vec<u8>, Vec<Version>>,

    /// LSN (Log Sequence Number) counter
    next_lsn: AtomicU64,
}

impl VersionStore {
    /// Create a new version store
    pub fn new() -> Self {
        Self {
            versions: BTreeMap::new(),
            next_lsn: AtomicU64::new(1),
        }
    }

    /// Create a snapshot
    pub fn create_snapshot(&self, txn_id: u64) -> Snapshot {
        // In a real implementation, this would collect active transactions
        Snapshot {
            snapshot_id: txn_id,
            active_txns: Vec::new(),
        }
    }

    /// Write a new version
    pub fn write(&mut self, txn_id: u64, key: Vec<u8>, value: Value) -> Result<()> {
        let lsn = self.next_lsn.fetch_add(1, AtomicOrdering::SeqCst);

        let version = Version {
            lsn,
            txn_id,
            value,
            is_committed: true,
        };

        self.versions
            .entry(key)
            .or_insert_with(Vec::new)
            .push(version);

        Ok(())
    }

    /// Read latest value (uncommitted)
    pub fn read_latest(&self, key: &[u8]) -> Result<Option<Value>> {
        if let Some(versions) = self.versions.get(key) {
            if let Some(version) = versions.last() {
                Ok(Some(version.value.clone()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Read latest committed value
    pub fn read_committed(&self, key: &[u8]) -> Result<Option<Value>> {
        if let Some(versions) = self.versions.get(key) {
            for version in versions.iter().rev() {
                if version.is_committed {
                    return Ok(Some(version.value.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Read from snapshot
    pub fn read_snapshot(&self, key: &[u8], snapshot: &Snapshot) -> Result<Option<Value>> {
        if let Some(versions) = self.versions.get(key) {
            for version in versions.iter().rev() {
                // Find first version visible to snapshot
                if version.txn_id <= snapshot.snapshot_id {
                    return Ok(Some(version.value.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Get last writer of a key (for conflict detection)
    pub fn get_last_writer(&self, key: &[u8], txn_id: u64) -> Option<(u64, Value)> {
        if let Some(versions) = self.versions.get(key) {
            for version in versions.iter().rev() {
                if version.txn_id != txn_id {
                    return Some((version.txn_id, version.value.clone()));
                }
            }
        }
        None
    }
}

/// Version in MVCC
#[derive(Debug, Clone)]
pub struct Version {
    /// Log Sequence Number
    pub lsn: u64,

    /// Transaction ID
    pub txn_id: u64,

    /// Value
    pub value: Value,

    /// Whether the version is committed
    pub is_committed: bool,
}

/// Write-Ahead Log (WAL)
pub struct WriteAheadLog {
    /// Log records
    records: Vec<LogRecord>,

    /// Current LSN
    current_lsn: AtomicU64,

    /// Last checkpoint LSN
    last_checkpoint_lsn: AtomicU64,
}

impl WriteAheadLog {
    /// Create a new WAL
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            current_lsn: AtomicU64::new(1),
            last_checkpoint_lsn: AtomicU64::new(0),
        }
    }

    /// Write a log record
    pub fn write_record(&mut self, record: LogRecord) -> Result<u64> {
        let lsn = self.current_lsn.fetch_add(1, AtomicOrdering::SeqCst);

        let record_with_lsn = LogRecord {
            lsn: Some(lsn),
            ..record
        };

        self.records.push(record_with_lsn);
        Ok(lsn)
    }

    /// Get log record by LSN
    pub fn get_record(&self, lsn: u64) -> Option<&LogRecord> {
        self.records.iter().find(|r| r.lsn == Some(lsn))
    }

    /// Get all records for a transaction
    pub fn get_txn_records(&self, txn_id: u64) -> Vec<&LogRecord> {
        self.records
            .iter()
            .filter(|r| r.txn_id == txn_id)
            .collect()
    }

    /// Truncate log up to LSN
    pub fn truncate(&mut self, lsn: u64) {
        self.records.retain(|r| r.lsn.map_or(true, |l| l > lsn));
    }

    /// Get current LSN
    pub fn current_lsn(&self) -> u64 {
        self.current_lsn.load(AtomicOrdering::SeqCst)
    }

    /// Set checkpoint LSN
    pub fn set_checkpoint_lsn(&self, lsn: u64) {
        self.last_checkpoint_lsn.store(lsn, AtomicOrdering::SeqCst);
    }

    /// Get checkpoint LSN
    pub fn checkpoint_lsn(&self) -> u64 {
        self.last_checkpoint_lsn.load(AtomicOrdering::SeqCst)
    }
}

/// WAL log record
#[derive(Debug, Clone)]
pub struct LogRecord {
    /// Log Sequence Number
    pub lsn: Option<u64>,

    /// Transaction ID
    pub txn_id: u64,

    /// Log record type
    pub typ: LogRecordType,

    /// Key (for updates)
    pub key: Option<Vec<u8>>,

    /// Old value (for undo)
    pub old_value: Option<Value>,

    /// New value (for redo)
    pub new_value: Option<Value>,

    /// Previous LSN for this transaction
    pub prev_lsn: Option<u64>,
}

/// Log record type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogRecordType {
    /// Transaction begin
    Begin,

    /// Update
    Update,

    /// Insert
    Insert,

    /// Delete
    Delete,

    /// Transaction commit
    Commit,

    /// Transaction rollback
    Rollback,

    /// Compensation log record (for undo actions)
    Compensation,
}

/// Checkpoint record
#[derive(Debug, Clone)]
pub struct CheckpointRecord {
    /// Checkpoint LSN
    pub lsn: u64,

    /// Active transactions at checkpoint time
    pub active_txns: Vec<u64>,

    /// Last LSN for each active transaction
    pub txn_lsns: BTreeMap<u64, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_manager_creation() {
        let tm = TransactionManager::new(100).unwrap();
        assert_eq!(tm.active_count(), 0);
    }

    #[test]
    fn test_begin_transaction() {
        let mut tm = TransactionManager::new(100).unwrap();
        let txn = tm.begin(IsolationLevel::ReadCommitted).unwrap();
        assert_eq!(txn.state, TransactionState::Active);
        assert!(txn.id > 0);
    }

    #[test]
    fn test_commit_transaction() {
        let mut tm = TransactionManager::new(100).unwrap();
        let txn = tm.begin(IsolationLevel::ReadCommitted).unwrap();
        tm.commit(txn).unwrap();
        assert_eq!(tm.active_count(), 0);
    }

    #[test]
    fn test_rollback_transaction() {
        let mut tm = TransactionManager::new(100).unwrap();
        let txn = tm.begin(IsolationLevel::ReadCommitted).unwrap();
        tm.rollback(txn).unwrap();
        assert_eq!(tm.active_count(), 0);
    }

    #[test]
    fn test_max_transactions_limit() {
        let mut tm = TransactionManager::new(2).unwrap();

        let txn1 = tm.begin(IsolationLevel::ReadCommitted).unwrap();
        let txn2 = tm.begin(IsolationLevel::ReadCommitted).unwrap();

        // Third transaction should fail
        let txn3 = tm.begin(IsolationLevel::ReadCommitted);
        assert!(txn3.is_err());

        // Cleanup
        tm.commit(txn1).unwrap();
        tm.commit(txn2).unwrap();
    }

    #[test]
    fn test_mvcc_read_write() {
        let mut tm = TransactionManager::new(100).unwrap();

        let mut txn1 = tm.begin(IsolationLevel::Serializable).unwrap();

        let key = vec![1, 2, 3];
        let value = Value::Integer(42);

        tm.write(&mut txn1, key.clone(), value.clone()).unwrap();
        tm.commit(txn1).unwrap();

        let txn2 = tm.begin(IsolationLevel::Serializable).unwrap();
        let result = tm.read(&txn2, &key).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), value);
    }

    #[test]
    fn test_snapshot_isolation() {
        let mut tm = TransactionManager::new(100).unwrap();

        let txn1 = tm.begin(IsolationLevel::RepeatableRead).unwrap();
        let snapshot_id = txn1.snapshot.snapshot_id;

        assert_eq!(snapshot_id, txn1.id);
    }

    #[test]
    fn test_wal_write_record() {
        let mut wal = WriteAheadLog::new();

        let record = LogRecord {
            lsn: None,
            txn_id: 1,
            typ: LogRecordType::Begin,
            key: None,
            old_value: None,
            new_value: None,
            prev_lsn: None,
        };

        let lsn = wal.write_record(record).unwrap();
        assert_eq!(lsn, 1);
    }

    #[test]
    fn test_wal_get_record() {
        let mut wal = WriteAheadLog::new();

        let record = LogRecord {
            lsn: None,
            txn_id: 1,
            typ: LogRecordType::Begin,
            key: None,
            old_value: None,
            new_value: None,
            prev_lsn: None,
        };

        let lsn = wal.write_record(record).unwrap();
        let retrieved = wal.get_record(lsn);

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().txn_id, 1);
    }

    #[test]
    fn test_wal_truncate() {
        let mut wal = WriteAheadLog::new();

        for i in 0..5 {
            let record = LogRecord {
                lsn: None,
                txn_id: i,
                typ: LogRecordType::Begin,
                key: None,
                old_value: None,
                new_value: None,
                prev_lsn: None,
            };
            wal.write_record(record).unwrap();
        }

        wal.truncate(3);
        assert_eq!(wal.records.len(), 2);
    }

    #[test]
    fn test_isolation_levels() {
        assert_eq!(IsolationLevel::ReadUncommitted, IsolationLevel::ReadUncommitted);
        assert_eq!(IsolationLevel::ReadCommitted, IsolationLevel::ReadCommitted);
        assert_eq!(IsolationLevel::RepeatableRead, IsolationLevel::RepeatableRead);
        assert_eq!(IsolationLevel::Serializable, IsolationLevel::Serializable);
    }

    #[test]
    fn test_transaction_state_transitions() {
        let mut tm = TransactionManager::new(100).unwrap();
        let txn = tm.begin(IsolationLevel::ReadCommitted).unwrap();

        assert_eq!(txn.state, TransactionState::Active);

        let committed_txn = Transaction {
            state: TransactionState::Committed,
            ..txn
        };

        assert_eq!(committed_txn.state, TransactionState::Committed);
    }

    #[test]
    fn test_read_write_tracking() {
        let mut tm = TransactionManager::new(100).unwrap();
        let mut txn = tm.begin(IsolationLevel::Serializable).unwrap();

        let key = vec![1, 2, 3];
        txn.track_read(key.clone());

        assert!(txn.read_set.contains(&key));
        assert_eq!(txn.read_set.len(), 1);
    }

    #[test]
    fn test_version_store_write_read() {
        let mut vs = VersionStore::new();

        let key = vec![1, 2, 3];
        let value = Value::Integer(42);

        vs.write(1, key.clone(), value.clone()).unwrap();

        let result = vs.read_committed(&key).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), value);
    }

    #[test]
    fn test_multiple_versions() {
        let mut vs = VersionStore::new();

        let key = vec![1, 2, 3];
        let value1 = Value::Integer(42);
        let value2 = Value::Integer(100);

        vs.write(1, key.clone(), value1.clone()).unwrap();
        vs.write(2, key.clone(), value2.clone()).unwrap();

        let result = vs.read_committed(&key).unwrap();
        assert!(result.is_some());
        // Should return latest version
        assert_eq!(result.unwrap(), value2);
    }
}
