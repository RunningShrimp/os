//! Transaction Management with MVCC
//!
//! Implements ACID transactions with Multi-Version Concurrency Control.

use super::types::{TxId, Value, RowId, IsolationLevel, TxStatus};
use super::{DbError, DbResult};
use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// Transaction manager
pub struct TransactionManager {
    next_tx_id: AtomicU64,
    transactions: Mutex<BTreeMap<TxId, Arc<Transaction>>>,
    active_count: Mutex<usize>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            next_tx_id: AtomicU64::new(1),
            transactions: Mutex::new(BTreeMap::new()),
            active_count: Mutex::new(0),
        }
    }

    /// Begin a new transaction
    pub fn begin(&self, isolation_level: IsolationLevel) -> DbResult<Transaction> {
        let tx_id = self.next_tx_id.fetch_add(1, Ordering::SeqCst);

        let tx = Transaction {
            id: tx_id,
            isolation_level,
            status: Mutex::new(TxStatus::Active),
            read_set: Mutex::new(Vec::new()),
            write_set: Mutex::new(BTreeMap::new()),
            start_timestamp: self.current_timestamp(),
            mvcc_versions: Mutex::new(BTreeMap::new()),
        };

        let tx_arc = Arc::new(tx);

        {
            let mut transactions = self.transactions.lock();
            transactions.insert(tx_id, tx_arc.clone());
        }

        {
            let mut active_count = self.active_count.lock();
            *active_count += 1;
        }

        // Drop Arc to get owned Transaction
        let tx = Arc::try_unwrap(tx_arc)
            .map_err(|_| DbError::Internal("Failed to unwrap transaction".into()))?;

        Ok(tx)
    }

    /// Commit a transaction
    pub fn commit(&self, tx: &Transaction) -> DbResult<()> {
        let mut status = tx.status.lock();

        if *status != TxStatus::Active {
            return Err(DbError::Transaction("Transaction not active".into()));
        }

        // Perform validation for serializable isolation
        if tx.isolation_level == IsolationLevel::Serializable {
            self.validate_serializable(tx)?;
        }

        // Apply writes
        self.apply_writes(tx)?;

        *status = TxStatus::Committed;

        // Cleanup
        {
            let mut transactions = self.transactions.lock();
            transactions.remove(&tx.id);
        }

        {
            let mut active_count = self.active_count.lock();
            *active_count -= 1;
        }

        Ok(())
    }

    /// Rollback a transaction
    pub fn rollback(&self, tx: &Transaction) -> DbResult<()> {
        let mut status = tx.status.lock();

        if *status != TxStatus::Active {
            return Err(DbError::Transaction("Transaction not active".into()));
        }

        // Discard writes
        tx.write_set.lock().clear();

        *status = TxStatus::RolledBack;

        // Cleanup
        {
            let mut transactions = self.transactions.lock();
            transactions.remove(&tx.id);
        }

        {
            let mut active_count = self.active_count.lock();
            *active_count -= 1;
        }

        Ok(())
    }

    /// Validate serializable isolation
    fn validate_serializable(&self, tx: &Transaction) -> DbResult<()> {
        let transactions = self.transactions.lock();
        let read_set = tx.read_set.lock();

        // Check for write-write conflicts
        for other_tx in transactions.values() {
            if other_tx.id == tx.id {
                continue;
            }

            let other_write_set = other_tx.write_set.lock();

            for (key, _) in read_set.iter() {
                if other_write_set.contains_key(key) {
                    return Err(DbError::Transaction(
                        "Serialization conflict".into()
                    ));
                }
            }
        }

        Ok(())
    }

    /// Apply transaction writes
    fn apply_writes(&self, _tx: &Transaction) -> DbResult<()> {
        // In a real implementation, this would apply writes to storage
        Ok(())
    }

    fn current_timestamp(&self) -> u64 {
        // Simple timestamp based on transaction ID
        self.next_tx_id.load(Ordering::SeqCst)
    }

    /// Get transaction by ID
    pub fn get_transaction(&self, tx_id: TxId) -> Option<Arc<Transaction>> {
        let transactions = self.transactions.lock();
        transactions.get(&tx_id).cloned()
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction
pub struct Transaction {
    pub id: TxId,
    pub isolation_level: IsolationLevel,
    status: Mutex<TxStatus>,
    read_set: Mutex<Vec<RowId>>,
    write_set: Mutex<BTreeMap<RowId, Value>>,
    start_timestamp: u64,
    mvcc_versions: Mutex<BTreeMap<RowId, MvccVersion>>,
}

impl Transaction {
    /// Read a value with MVCC
    pub fn read(&self, row_id: RowId, storage: &MvccStorage) -> DbResult<Option<Value>> {
        self.check_active()?;

        // Add to read set
        {
            let mut read_set = self.read_set.lock();
            read_set.push(row_id);
        }

        // Get visible version
        let version = storage.get_visible_version(row_id, self.id, self.isolation_level)?;

        Ok(version.map(|v| v.value))
    }

    /// Write a value
    pub fn write(&self, row_id: RowId, value: Value) -> DbResult<()> {
        self.check_active()?;

        let mut write_set = self.write_set.lock();
        write_set.insert(row_id, value);

        Ok(())
    }

    /// Check if transaction is active
    fn check_active(&self) -> DbResult<()> {
        let status = self.status.lock();
        if *status != TxStatus::Active {
            return Err(DbError::Transaction("Transaction not active".into()));
        }
        Ok(())
    }

    /// Get transaction status
    pub fn status(&self) -> TxStatus {
        *self.status.lock()
    }
}

/// MVCC version of a value
#[derive(Debug, Clone)]
pub struct MvccVersion {
    pub value: Value,
    pub tx_id: TxId,
    pub timestamp: u64,
    pub is_deleted: bool,
}

/// MVCC Storage
pub struct MvccStorage {
    versions: Mutex<BTreeMap<RowId, Vec<MvccVersion>>>,
}

impl MvccStorage {
    pub fn new() -> Self {
        Self {
            versions: Mutex::new(BTreeMap::new()),
        }
    }

    /// Get visible version for transaction
    pub fn get_visible_version(
        &self,
        row_id: RowId,
        tx_id: TxId,
        isolation_level: IsolationLevel,
    ) -> DbResult<Option<MvccVersion>> {
        let versions = self.versions.lock();

        if let Some(row_versions) = versions.get(&row_id) {
            // Find the latest visible version
            for version in row_versions.iter().rev() {
                if self.is_visible(version, tx_id, isolation_level) {
                    if version.is_deleted {
                        return Ok(None);
                    }
                    return Ok(Some(version.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Check if version is visible to transaction
    fn is_visible(&self, version: &MvccVersion, tx_id: TxId, isolation_level: IsolationLevel) -> bool {
        match isolation_level {
            IsolationLevel::ReadCommitted => {
                // See only committed versions
                // In real implementation, would check transaction status
                version.tx_id < tx_id
            }
            IsolationLevel::RepeatableRead | IsolationLevel::Serializable => {
                // See only versions committed before transaction start
                // In real implementation, would use timestamps
                version.tx_id < tx_id
            }
            IsolationLevel::ReadUncommitted => {
                // See all versions including uncommitted
                true
            }
        }
    }

    /// Write a new version
    pub fn write(&self, row_id: RowId, value: Value, tx_id: TxId) -> DbResult<()> {
        let mut versions = self.versions.lock();
        let row_versions = versions.entry(row_id).or_insert_with(Vec::new);

        row_versions.push(MvccVersion {
            value,
            tx_id,
            timestamp: self.current_timestamp(),
            is_deleted: false,
        });

        Ok(())
    }

    /// Delete a row (tombstone)
    pub fn delete(&self, row_id: RowId, tx_id: TxId) -> DbResult<()> {
        let mut versions = self.versions.lock();
        let row_versions = versions.entry(row_id).or_insert_with(Vec::new);

        row_versions.push(MvccVersion {
            value: Value::Null,
            tx_id,
            timestamp: self.current_timestamp(),
            is_deleted: true,
        });

        Ok(())
    }

    /// Cleanup old versions
    pub fn cleanup(&self, _before_timestamp: u64) -> DbResult<()> {
        // In a real implementation, would remove old versions
        Ok(())
    }

    fn current_timestamp(&self) -> u64 {
        // Simple timestamp
        use core::time::Duration;
        // This is a placeholder - would use actual time in real implementation
        0
    }
}

impl Default for MvccStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction log entry
#[derive(Debug, Clone)]
pub struct TxLogEntry {
    pub tx_id: TxId,
    pub operation: TxOperation,
    pub row_id: RowId,
    pub old_value: Option<Value>,
    pub new_value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxOperation {
    Insert,
    Update,
    Delete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_begin_commit() {
        let tm = TransactionManager::new();
        let tx = tm.begin(IsolationLevel::ReadCommitted).unwrap();
        assert_eq!(tx.status(), TxStatus::Active);

        tm.commit(&tx).unwrap();
        assert_eq!(tx.status(), TxStatus::Committed);
    }

    #[test]
    fn test_transaction_rollback() {
        let tm = TransactionManager::new();
        let tx = tm.begin(IsolationLevel::ReadCommitted).unwrap();

        tm.rollback(&tx).unwrap();
        assert_eq!(tx.status(), TxStatus::RolledBack);
    }
}
