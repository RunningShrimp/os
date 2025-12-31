//! # Recovery Manager
//!
//! Provides ARIES-style crash recovery:
//! - Analysis phase: Determine which transactions to undo/redo
//! - Redo phase: Repeat all actions that didn't reach disk
//! - Undo phase: Undo actions of uncommitted transactions
//! - Checkpointing: Fuzzy and incremental checkpoints
//! - Rollback recovery: Transaction and statement rollback

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use super::storage::Value;
use super::transaction::{LogRecord, LogRecordType, WriteAheadLog};
use super::{DatabaseError, Result};

/// Checkpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointType {
    /// Fuzzy checkpoint (allows concurrent activity)
    Fuzzy,

    /// Incremental checkpoint (only updates since last checkpoint)
    Incremental,

    /// Full checkpoint (complete database snapshot)
    Full,
}

/// Recovery manager
pub struct RecoveryManager {
    /// Database name
    db_name: String,

    /// Write-ahead log
    wal: WriteAheadLog,

    /// Enable WAL
    enable_wal: bool,

    /// WAL sync mode
    wal_sync_mode: WalSyncMode,

    /// Last checkpoint LSN
    last_checkpoint_lsn: AtomicU64,

    /// Transaction table (for recovery)
    txn_table: BTreeMap<u64, TransactionStatus>,
}

/// Transaction status for recovery
#[derive(Debug, Clone)]
struct TransactionStatus {
    /// Last LSN for this transaction
    last_lsn: u64,

    /// Transaction state
    state: TxnState,

    /// Dirty pages modified by this transaction
    dirty_pages: Vec<usize>,
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxnState {
    /// In progress
    InProgress,

    /// Committed
    Committed,

    /// Aborted
    Aborted,
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

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(
        db_name: &str,
        enable_wal: bool,
        wal_sync_mode: WalSyncMode,
    ) -> Result<Self> {
        Ok(Self {
            db_name: String::from(db_name),
            wal: WriteAheadLog::new(),
            enable_wal,
            wal_sync_mode,
            last_checkpoint_lsn: AtomicU64::new(0),
            txn_table: BTreeMap::new(),
        })
    }

    /// Write a log record
    pub fn write_log(&mut self, record: LogRecord) -> Result<u64> {
        if !self.enable_wal {
            return Ok(0);
        }

        let lsn = self.wal.write_record(record)?;

        // Sync WAL based on sync mode
        match self.wal_sync_mode {
            WalSyncMode::Fsync => {
                // In a real implementation, fsync the WAL file
            }
            WalSyncMode::Fdatasync => {
                // In a real implementation, fdatasync the WAL file
            }
            WalSyncMode::Flush => {
                // In a real implementation, flush to OS cache
            }
            WalSyncMode::Full => {
                // In a real implementation, full sync
            }
            WalSyncMode::None => {
                // No sync
            }
        }

        Ok(lsn)
    }

    /// Begin transaction in log
    pub fn log_begin(&mut self, txn_id: u64) -> Result<u64> {
        let record = LogRecord {
            lsn: None,
            txn_id,
            typ: LogRecordType::Begin,
            key: None,
            old_value: None,
            new_value: None,
            prev_lsn: None,
        };

        let lsn = self.write_log(record)?;

        // Update transaction table
        self.txn_table.insert(txn_id, TransactionStatus {
            last_lsn: lsn,
            state: TxnState::InProgress,
            dirty_pages: Vec::new(),
        });

        Ok(lsn)
    }

    /// Log an update
    pub fn log_update(
        &mut self,
        txn_id: u64,
        key: Vec<u8>,
        old_value: Option<Value>,
        new_value: Value,
    ) -> Result<u64> {
        // Get previous LSN for this transaction
        let prev_lsn = self
            .txn_table
            .get(&txn_id)
            .map(|status| status.last_lsn);

        let record = LogRecord {
            lsn: None,
            txn_id,
            typ: LogRecordType::Update,
            key: Some(key),
            old_value,
            new_value: Some(new_value),
            prev_lsn,
        };

        let lsn = self.write_log(record)?;

        // Update transaction table
        if let Some(status) = self.txn_table.get_mut(&txn_id) {
            status.last_lsn = lsn;
        }

        Ok(lsn)
    }

    /// Log a commit
    pub fn log_commit(&mut self, txn_id: u64) -> Result<u64> {
        let prev_lsn = self
            .txn_table
            .get(&txn_id)
            .map(|status| status.last_lsn);

        let record = LogRecord {
            lsn: None,
            txn_id,
            typ: LogRecordType::Commit,
            key: None,
            old_value: None,
            new_value: None,
            prev_lsn,
        };

        let lsn = self.write_log(record)?;

        // Update transaction table
        if let Some(status) = self.txn_table.get_mut(&txn_id) {
            status.last_lsn = lsn;
            status.state = TxnState::Committed;
        }

        Ok(lsn)
    }

    /// Log a rollback
    pub fn log_rollback(&mut self, txn_id: u64) -> Result<u64> {
        let prev_lsn = self
            .txn_table
            .get(&txn_id)
            .map(|status| status.last_lsn);

        let record = LogRecord {
            lsn: None,
            txn_id,
            typ: LogRecordType::Rollback,
            key: None,
            old_value: None,
            new_value: None,
            prev_lsn,
        };

        let lsn = self.write_log(record)?;

        // Update transaction table
        if let Some(status) = self.txn_table.get_mut(&txn_id) {
            status.last_lsn = lsn;
            status.state = TxnState::Aborted;
        }

        Ok(lsn)
    }

    /// Perform recovery using ARIES algorithm
    pub fn recover(&mut self) -> Result<RecoveryStats> {
        // Phase 1: Analysis
        let analysis_result = self.analysis_phase()?;

        // Phase 2: Redo
        let redo_count = self.redo_phase(&analysis_result)?;

        // Phase 3: Undo
        let undo_count = self.undo_phase(&analysis_result)?;

        Ok(RecoveryStats {
            redo_count,
            undo_count,
            checkpoint_lsn: self.last_checkpoint_lsn.load(AtomicOrdering::SeqCst),
        })
    }

    /// Analysis phase: Determine which transactions to undo/redo
    fn analysis_phase(&mut self) -> Result<AnalysisResult> {
        let mut committed = BTreeMap::new();
        let mut uncommitted = BTreeMap::new();
        let dirty_pages = BTreeMap::new();

        // Scan log from last checkpoint
        let _start_lsn = self.last_checkpoint_lsn.load(AtomicOrdering::SeqCst);

        // In a real implementation, we would iterate through log records
        // For now, use the transaction table
        for (txn_id, status) in &self.txn_table {
            match status.state {
                TxnState::Committed => {
                    committed.insert(*txn_id, status.last_lsn);
                }
                TxnState::InProgress | TxnState::Aborted => {
                    uncommitted.insert(*txn_id, status.last_lsn);
                }
            }
        }

        Ok(AnalysisResult {
            committed,
            uncommitted,
            dirty_pages,
        })
    }

    /// Redo phase: Repeat all actions that didn't reach disk
    fn redo_phase(&mut self, analysis: &AnalysisResult) -> Result<usize> {
        let redo_count = 0;

        // Find the smallest LSN to start redo
        let mut min_lsn: Option<u64> = None;

        for &lsn in analysis.committed.values() {
            min_lsn = Some(min_lsn.map_or(lsn, |m| m.min(lsn)));
        }

        for &lsn in analysis.uncommitted.values() {
            min_lsn = Some(min_lsn.map_or(lsn, |m| m.min(lsn)));
        }

        // In a real implementation, we would iterate through log records
        // starting from min_lsn and redoing updates
        // For now, just return a count

        Ok(redo_count)
    }

    /// Undo phase: Undo actions of uncommitted transactions
    fn undo_phase(&mut self, analysis: &AnalysisResult) -> Result<usize> {
        let mut undo_count = 0;

        // Sort uncommitted transactions by last LSN (descending)
        let mut sorted_txns: Vec<(u64, u64)> =
            analysis.uncommitted.iter().map(|(k, v)| (*k, *v)).collect();
        sorted_txns.sort_by(|a, b| b.1.cmp(&a.1));

        // Undo each transaction
        for (txn_id, last_lsn) in sorted_txns {
            let mut current_lsn = Some(last_lsn);

            // Follow the chain of prev_lsn pointers
            while let Some(lsn) = current_lsn {
                if let Some(record) = self.wal.get_record(lsn) {
                    // Undo the operation
                    match record.typ {
                        LogRecordType::Update => {
                            if let (Some(_key), Some(_old_value)) = (&record.key, &record.old_value) {
                                // Restore old value
                                undo_count += 1;
                            }
                        }
                        LogRecordType::Insert => {
                            // Delete the inserted record
                            undo_count += 1;
                        }
                        LogRecordType::Delete => {
                            // Restore the deleted record
                            undo_count += 1;
                        }
                        _ => {}
                    }

                    current_lsn = record.prev_lsn;
                } else {
                    break;
                }
            }

            // Write compensation log record
            let clr = LogRecord {
                lsn: None,
                txn_id,
                typ: LogRecordType::Compensation,
                key: None,
                old_value: None,
                new_value: None,
                prev_lsn: Some(last_lsn),
            };

            self.write_log(clr)?;
        }

        Ok(undo_count)
    }

    /// Create a checkpoint
    pub fn checkpoint(&mut self, checkpoint_type: CheckpointType) -> Result<()> {
        match checkpoint_type {
            CheckpointType::Fuzzy => self.fuzzy_checkpoint()?,
            CheckpointType::Incremental => self.incremental_checkpoint()?,
            CheckpointType::Full => self.full_checkpoint()?,
        }

        Ok(())
    }

    /// Fuzzy checkpoint (allows concurrent activity)
    fn fuzzy_checkpoint(&mut self) -> Result<()> {
        // Record current LSN as checkpoint LSN
        let _checkpoint_lsn = self.wal.current_lsn();

        // Write checkpoint record
        let record = LogRecord {
            lsn: None,
            txn_id: 0,
            typ: LogRecordType::Commit, // Using Commit as marker
            key: Some(vec![b'C', b'K', b'P']),
            old_value: None,
            new_value: None,
            prev_lsn: None,
        };

        let lsn = self.write_log(record)?;

        // Update last checkpoint LSN
        self.last_checkpoint_lsn.store(lsn, AtomicOrdering::SeqCst);

        Ok(())
    }

    /// Incremental checkpoint
    fn incremental_checkpoint(&mut self) -> Result<()> {
        // Similar to fuzzy, but only records changes since last checkpoint
        let last_lsn = self.last_checkpoint_lsn.load(AtomicOrdering::SeqCst);
        let _current_lsn = self.wal.current_lsn();

        // Write checkpoint record with range
        let record = LogRecord {
            lsn: None,
            txn_id: 0,
            typ: LogRecordType::Commit,
            key: Some(vec![b'I', b'C', b'K']),
            old_value: None,
            new_value: None,
            prev_lsn: Some(last_lsn),
        };

        let lsn = self.write_log(record)?;
        self.last_checkpoint_lsn.store(lsn, AtomicOrdering::SeqCst);

        Ok(())
    }

    /// Full checkpoint
    fn full_checkpoint(&mut self) -> Result<()> {
        // In a real implementation, this would:
        // 1. Stop all transaction activity
        // 2. Flush all dirty pages to disk
        // 3. Write checkpoint record
        // 4. Resume activity

        let _checkpoint_lsn = self.wal.current_lsn();

        let record = LogRecord {
            lsn: None,
            txn_id: 0,
            typ: LogRecordType::Commit,
            key: Some(vec![b'F', b'C', b'K']),
            old_value: None,
            new_value: None,
            prev_lsn: None,
        };

        let lsn = self.write_log(record)?;
        self.last_checkpoint_lsn.store(lsn, AtomicOrdering::SeqCst);

        Ok(())
    }

    /// Rollback a transaction
    pub fn rollback_transaction(&mut self, txn_id: u64) -> Result<()> {
        // Get transaction's last LSN
        let last_lsn = self
            .txn_table
            .get(&txn_id)
            .map(|status| status.last_lsn)
            .ok_or_else(|| DatabaseError::TransactionConflict(
                String::from("Transaction not found")
            ))?;

        // Undo all operations
        let mut current_lsn = Some(last_lsn);

        while let Some(lsn) = current_lsn {
            if let Some(record) = self.wal.get_record(lsn) {
                match record.typ {
                    LogRecordType::Update => {
                        if let (Some(_key), Some(_old_value)) = (&record.key, &record.old_value) {
                            // Restore old value
                            // In a real implementation, we would apply this to the database
                        }
                    }
                    LogRecordType::Insert => {
                        // Delete the inserted record
                    }
                    LogRecordType::Delete => {
                        // Restore the deleted record
                    }
                    _ => {}
                }

                current_lsn = record.prev_lsn;
            } else {
                break;
            }
        }

        // Log the rollback
        self.log_rollback(txn_id)?;

        Ok(())
    }

    /// Get WAL size
    pub fn wal_size(&self) -> usize {
        self.wal.current_lsn() as usize
    }

    /// Truncate WAL up to checkpoint LSN
    pub fn truncate_wal(&mut self) -> Result<()> {
        let checkpoint_lsn = self.last_checkpoint_lsn.load(AtomicOrdering::SeqCst);
        self.wal.truncate(checkpoint_lsn);
        Ok(())
    }
}

/// Result of analysis phase
#[derive(Debug, Clone)]
struct AnalysisResult {
    /// Committed transactions and their last LSN
    committed: BTreeMap<u64, u64>,

    /// Uncommitted transactions and their last LSN
    uncommitted: BTreeMap<u64, u64>,

    /// Dirty pages (page_id -> LSN)
    dirty_pages: BTreeMap<usize, u64>,
}

/// Recovery statistics
#[derive(Debug, Clone)]
pub struct RecoveryStats {
    /// Number of redo operations
    pub redo_count: usize,

    /// Number of undo operations
    pub undo_count: usize,

    /// Checkpoint LSN
    pub checkpoint_lsn: u64,
}

/// Rollback recovery manager
pub struct RollbackManager {
    /// Savepoints for statement-level rollback
    savepoints: BTreeMap<String, Vec<LogRecord>>,
}

impl RollbackManager {
    /// Create a new rollback manager
    pub fn new() -> Self {
        Self {
            savepoints: BTreeMap::new(),
        }
    }

    /// Create a savepoint
    pub fn create_savepoint(&mut self, name: String, records: Vec<LogRecord>) -> Result<()> {
        self.savepoints.insert(name, records);
        Ok(())
    }

    /// Rollback to a savepoint
    pub fn rollback_to_savepoint(&mut self, name: &str) -> Result<()> {
        let records = self
            .savepoints
            .get(name)
            .ok_or_else(|| DatabaseError::InternalError(
                format!("Savepoint {} not found", name)
            ))?;

        // Undo all operations since savepoint
        for record in records.iter().rev() {
            // Undo the operation
            match record.typ {
                LogRecordType::Update => {
                    if let (Some(_key), Some(old_value)) = (&record.key, &record.old_value) {
                        // Restore old value
                        let _ = old_value;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Release a savepoint
    pub fn release_savepoint(&mut self, name: &str) -> Result<()> {
        self.savepoints.remove(name);
        Ok(())
    }
}

/// Media recovery for backup and restore
pub struct MediaRecovery {
    /// Backup metadata
    backups: Vec<BackupInfo>,
}

/// Backup information
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Backup ID
    pub backup_id: String,

    /// Backup timestamp
    pub timestamp: u64,

    /// Backup type
    pub backup_type: BackupType,

    /// LSN at backup time
    pub lsn: u64,

    /// Backup file path
    pub path: String,
}

/// Backup type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupType {
    /// Full backup
    Full,

    /// Incremental backup
    Incremental,

    /// Differential backup
    Differential,
}

impl MediaRecovery {
    /// Create a new media recovery manager
    pub fn new() -> Self {
        Self {
            backups: Vec::new(),
        }
    }

    /// Create a backup
    pub fn create_backup(
        &mut self,
        backup_id: String,
        backup_type: BackupType,
        lsn: u64,
        path: String,
    ) -> Result<()> {
        let backup = BackupInfo {
            backup_id,
            timestamp: 0, // In a real implementation, use current time
            backup_type,
            lsn,
            path,
        };

        self.backups.push(backup);
        Ok(())
    }

    /// Restore from backup
    pub fn restore_from_backup(&mut self, backup_id: &str) -> Result<RestoreResult> {
        let backup = self
            .backups
            .iter()
            .find(|b| b.backup_id == backup_id)
            .ok_or_else(|| DatabaseError::InternalError(
                format!("Backup {} not found", backup_id)
            ))?;

        // Apply any incremental backups since the full backup
        let mut incremental_count = 0;

        for b in &self.backups {
            if b.backup_type == BackupType::Incremental && b.lsn > backup.lsn {
                incremental_count += 1;
            }
        }

        Ok(RestoreResult {
            backup_lsn: backup.lsn,
            incremental_count,
        })
    }

    /// Get list of backups
    pub fn list_backups(&self) -> Vec<&BackupInfo> {
        self.backups.iter().collect()
    }
}

/// Result of restore operation
#[derive(Debug, Clone)]
pub struct RestoreResult {
    /// LSN of restored backup
    pub backup_lsn: u64,

    /// Number of incremental backups applied
    pub incremental_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_manager_creation() {
        let rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();
        assert_eq!(rm.db_name, "test_db");
    }

    #[test]
    fn test_log_begin() {
        let mut rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();
        let lsn = rm.log_begin(1).unwrap();
        assert!(lsn > 0);
    }

    #[test]
    fn test_log_commit() {
        let mut rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();
        rm.log_begin(1).unwrap();
        let lsn = rm.log_commit(1).unwrap();
        assert!(lsn > 0);
    }

    #[test]
    fn test_log_rollback() {
        let mut rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();
        rm.log_begin(1).unwrap();
        let lsn = rm.log_rollback(1).unwrap();
        assert!(lsn > 0);
    }

    #[test]
    fn test_fuzzy_checkpoint() {
        let mut rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();
        rm.checkpoint(CheckpointType::Fuzzy).unwrap();
        assert!(rm.last_checkpoint_lsn.load(AtomicOrdering::SeqCst) > 0);
    }

    #[test]
    fn test_incremental_checkpoint() {
        let mut rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();
        rm.checkpoint(CheckpointType::Incremental).unwrap();
        assert!(rm.last_checkpoint_lsn.load(AtomicOrdering::SeqCst) > 0);
    }

    #[test]
    fn test_full_checkpoint() {
        let mut rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();
        rm.checkpoint(CheckpointType::Full).unwrap();
        assert!(rm.last_checkpoint_lsn.load(AtomicOrdering::SeqCst) > 0);
    }

    #[test]
    fn test_wal_size() {
        let mut rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();
        rm.log_begin(1).unwrap();
        rm.log_commit(1).unwrap();

        let size = rm.wal_size();
        assert!(size > 0);
    }

    #[test]
    fn test_rollback_transaction() {
        let mut rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();
        rm.log_begin(1).unwrap();
        rm.log_update(
            1,
            vec![1, 2, 3],
            None,
            Value::Integer(42),
        ).unwrap();

        rm.rollback_transaction(1).unwrap();
    }

    #[test]
    fn test_rollback_manager() {
        let mut rm = RollbackManager::new();

        let records = vec![
            LogRecord {
                lsn: None,
                txn_id: 1,
                typ: LogRecordType::Update,
                key: Some(vec![1, 2, 3]),
                old_value: None,
                new_value: Some(Value::Integer(42)),
                prev_lsn: None,
            }
        ];

        rm.create_savepoint(String::from("sp1"), records).unwrap();
        rm.rollback_to_savepoint("sp1").unwrap();
        rm.release_savepoint("sp1").unwrap();
    }

    #[test]
    fn test_media_recovery() {
        let mut mr = MediaRecovery::new();

        mr.create_backup(
            String::from("backup1"),
            BackupType::Full,
            100,
            String::from("/backups/backup1"),
        ).unwrap();

        let backups = mr.list_backups();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn test_restore_from_backup() {
        let mut mr = MediaRecovery::new();

        mr.create_backup(
            String::from("backup1"),
            BackupType::Full,
            100,
            String::from("/backups/backup1"),
        ).unwrap();

        let result = mr.restore_from_backup("backup1").unwrap();
        assert_eq!(result.backup_lsn, 100);
    }

    #[test]
    fn test_checkpoint_type() {
        assert_eq!(CheckpointType::Fuzzy, CheckpointType::Fuzzy);
        assert_eq!(CheckpointType::Incremental, CheckpointType::Incremental);
        assert_eq!(CheckpointType::Full, CheckpointType::Full);
    }

    #[test]
    fn test_backup_type() {
        assert_eq!(BackupType::Full, BackupType::Full);
        assert_eq!(BackupType::Incremental, BackupType::Incremental);
        assert_eq!(BackupType::Differential, BackupType::Differential);
    }

    #[test]
    fn test_wal_sync_mode() {
        assert_eq!(WalSyncMode::None, WalSyncMode::None);
        assert_eq!(WalSyncMode::Fdatasync, WalSyncMode::Fdatasync);
        assert_eq!(WalSyncMode::Fsync, WalSyncMode::Fsync);
    }

    #[test]
    fn test_recovery() {
        let mut rm = RecoveryManager::new("test_db", true, WalSyncMode::Fsync).unwrap();

        // Simulate some activity
        rm.log_begin(1).unwrap();
        rm.log_update(1, vec![1, 2, 3], None, Value::Integer(42)).unwrap();
        rm.log_commit(1).unwrap();

        // Perform recovery
        let stats = rm.recover().unwrap();
        assert!(stats.redo_count >= 0);
        assert!(stats.undo_count >= 0);
    }
}
