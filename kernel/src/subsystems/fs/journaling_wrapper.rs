//! Journaling File System Wrapper
//!
//! This module provides a wrapper around the existing file system implementation
//! that adds journaling capabilities for transactional operations and crash recovery.

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::drivers::{BlockDevice, RamDisk};
use crate::subsystems::sync::Mutex;
// Sleeplock在当前文件中未使用，暂时注释掉
// use crate::subsystems::sync::Sleeplock;
use crate::subsystems::fs::{
    BSIZE, SuperBlock, InodeType, DiskInode, Dirent, BufFlags,
    Fs, Inode, NINODE, NDIRECT, IPB, DIRSIZ, ROOTINO, FS_MAGIC,
    BufCache, get_fs
};
use super::journaling_fs::{
    JournalingFileSystem, JournalTransaction, JfsError
};
// TransactionState在当前文件中未使用，暂时注释掉
// use super::journaling_fs::TransactionState;

/// Journaling file system wrapper
pub struct JournalingFsWrapper {
    /// Base file system
    base_fs: Fs,
    /// Journaling system
    journal: JournalingFileSystem,
    /// Active transactions
    active_transactions: Mutex<BTreeMap<u64, JournalingTransaction>>,
    /// Next transaction ID
    next_transaction_id: core::sync::atomic::AtomicU64,
    /// Journaling enabled flag
    journaling_enabled: core::sync::atomic::AtomicBool,
}

impl JournalingFsWrapper {
    /// Create a new journaling file system wrapper
    pub fn new(device: RamDisk) -> Self {
        let base_fs = Fs::new();
        let journal = JournalingFileSystem::new(Box::new(device.clone()));
        
        Self {
            base_fs,
            journal,
            active_transactions: Mutex::new(BTreeMap::new()),
            next_transaction_id: core::sync::atomic::AtomicU64::new(1),
            journaling_enabled: core::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Initialize the journaling file system
    pub fn init(&mut self) -> bool {
        // Initialize base file system
        if !self.base_fs.init() {
            crate::println!("jfs_wrapper: failed to initialize base file system");
            return false;
        }
        
        // Initialize journaling system
        if let Err(e) = self.journal.init() {
            crate::println!("jfs_wrapper: failed to initialize journal: {:?}", e);
            return false;
        }
        
        crate::println!("jfs_wrapper: journaling file system initialized");
        true
    }

    /// Begin a new transaction
    pub fn begin_transaction(&self) -> Result<u64, JfsError> {
        if !self.journaling_enabled.load(core::sync::atomic::Ordering::SeqCst) {
            return Err(JfsError::InvalidTransactionState);
        }
        
        let tx_id = self.next_transaction_id.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        
        // Begin transaction in journal
        self.journal.begin_transaction()?;
        
        // Create transaction record
        let tx = JournalTransaction::new(tx_id);
        let mut transactions = self.active_transactions.lock();
        transactions.insert(tx_id, tx);
        
        Ok(tx_id)
    }

    /// Commit a transaction
    pub fn commit_transaction(&self, tx_id: u64) -> Result<(), JfsError> {
        let mut transactions = self.active_transactions.lock();
        let tx = transactions.get_mut(&tx_id).ok_or(JfsError::TransactionNotFound)?;
        
        tx.commit();
        
        // Commit transaction in journal
        self.journal.commit_transaction(tx_id)?;
        
        // Remove from active transactions
        transactions.remove(&tx_id);
        
        Ok(())
    }

    /// Abort a transaction
    pub fn abort_transaction(&self, tx_id: u64) -> Result<(), JfsError> {
        let mut transactions = self.active_transactions.lock();
        let tx = transactions.get_mut(&tx_id).ok_or(JfsError::TransactionNotFound)?;
        
        tx.abort();
        
        // Abort transaction in journal
        self.journal.abort_transaction(tx_id)?;
        
        // Remove from active transactions
        transactions.remove(&tx_id);
        
        Ok(())
    }

    /// Enable or disable journaling
    pub fn set_journaling(&self, enabled: bool) {
        self.journaling_enabled.store(enabled, core::sync::atomic::Ordering::SeqCst);
    }

    /// Check if journaling is enabled
    pub fn is_journaling_enabled(&self) -> bool {
        self.journaling_enabled.load(core::sync::atomic::Ordering::SeqCst)
    }

    /// Allocate an inode with journaling
    pub fn ialloc(&self, itype: InodeType) -> Option<u32> {
        // Begin transaction
        let tx_id = match self.begin_transaction() {
            Ok(id) => id,
            Err(_) => {
                // Fall back to non-journaling operation
                return self.base_fs.ialloc(itype);
            }
        };
        
        // Perform the operation
        let result = self.base_fs.ialloc(itype);
        
        // Commit or abort based on result
        if result.is_some() {
            if let Err(e) = self.commit_transaction(tx_id) {
                crate::println!("jfs_wrapper: failed to commit transaction: {:?}", e);
            }
        } else {
            if let Err(e) = self.abort_transaction(tx_id) {
                crate::println!("jfs_wrapper: failed to abort transaction: {:?}", e);
            }
        }
        
        result
    }

    /// Get inode by number with journaling
    pub fn iget(&self, inum: u32) -> Option<usize> {
        // This is a read-only operation, no journaling needed
        self.base_fs.iget(inum)
    }

    /// Put (release) inode with journaling
    pub fn iput(&self, idx: usize) {
        // Begin transaction
        let tx_id = match self.begin_transaction() {
            Ok(id) => id,
            Err(_) => {
                // Fall back to non-journaling operation
                return self.base_fs.iput(idx);
            }
        };
        
        // Perform the operation
        self.base_fs.iput(idx);
        
        // Commit transaction
        if let Err(e) = self.commit_transaction(tx_id) {
            crate::println!("jfs_wrapper: failed to commit transaction: {:?}", e);
        }
    }

    /// Look up directory entry with journaling
    pub fn dirlookup(&self, dir_inum: u32, name: &str) -> Option<u32> {
        // This is a read-only operation, no journaling needed
        self.base_fs.dirlookup(dir_inum, name)
    }

    /// Create a new directory entry with journaling
    pub fn dirlink(&self, dir_inum: u32, name: &str, inum: u32) -> bool {
        // Begin transaction
        let tx_id = match self.begin_transaction() {
            Ok(id) => id,
            Err(_) => {
                // Fall back to non-journaling operation
                return self.base_fs.dirlink(dir_inum, name, inum);
            }
        };
        
        // Perform the operation
        let result = self.base_fs.dirlink(dir_inum, name, inum);
        
        // Commit or abort based on result
        if result {
            if let Err(e) = self.commit_transaction(tx_id) {
                crate::println!("jfs_wrapper: failed to commit transaction: {:?}", e);
            }
        } else {
            if let Err(e) = self.abort_transaction(tx_id) {
                crate::println!("jfs_wrapper: failed to abort transaction: {:?}", e);
            }
        }
        
        result
    }

    /// List directory contents with journaling
    pub fn list_dir(&self, dir_inum: u32) -> Vec<(String, u32)> {
        // This is a read-only operation, no journaling needed
        self.base_fs.list_dir(dir_inum)
    }

    /// List root directory with journaling
    pub fn list_root(&self) -> Vec<Inode> {
        // This is a read-only operation, no journaling needed
        self.base_fs.list_root()
    }

    /// Read data from inode with journaling
    pub fn read_inode(&self, inum: u32, dst: &mut [u8], off: usize) -> Option<usize> {
        // Get inode
        let idx = self.base_fs.iget(inum)?;
        let inodes = self.base_fs.inodes.lock();
        let inode = inodes.get(idx)?;
        
        // This is a read-only operation, no journaling needed
        let result = inode.read(&self.base_fs.dev, dst, off);
        
        Some(result)
    }

    /// Write data to inode with journaling
    pub fn write_inode(&self, inum: u32, src: &[u8], off: usize) -> Option<usize> {
        // Begin transaction
        let tx_id = match self.begin_transaction() {
            Ok(id) => id,
            Err(_) => {
                // Fall back to non-journaling operation
                let idx = self.base_fs.iget(inum)?;
                let inodes = self.base_fs.inodes.lock();
                let inode = inodes.get(idx)?;
                let mut inode_clone = Inode {
                    dev: inode.dev,
                    inum: inode.inum,
                    ref_count: inode.ref_count,
                    valid: inode.valid,
                    itype: inode.itype,
                    major: inode.major,
                    minor: inode.minor,
                    nlink: inode.nlink,
                    size: inode.size,
                    addrs: inode.addrs,
                };
                drop(inodes);
                
                let result = inode_clone.write(&self.base_fs.dev, src, off);
                Some(result)
            }
        };
        
        // Get inode and read original data
        let idx = self.base_fs.iget(inum)?;
        let inodes = self.base_fs.inodes.lock();
        let inode = inodes.get(idx)?;
        
        // Read original data for journaling
        let mut old_data = vec![0u8; src.len()];
        inode.read(&self.base_fs.dev, &mut old_data, off);
        
        drop(inodes);
        
        // Perform the write operation
        let idx = self.base_fs.iget(inum)?;
        let inodes = self.base_fs.inodes.lock();
        let inode = inodes.get(idx)?;
        let mut inode_clone = Inode {
            dev: inode.dev,
            inum: inode.inum,
            ref_count: inode.ref_count,
            valid: inode.valid,
            itype: inode.itype,
            major: inode.major,
            minor: inode.minor,
            nlink: inode.nlink,
            size: inode.size,
            addrs: inode.addrs,
        };
        drop(inodes);
        
        let result = inode_clone.write(&self.base_fs.dev, src, off);
        
        // Log the block modification to journal
        for i in 0..NDIRECT {
            if inode_clone.addrs[i] != 0 {
                let block_num = inode_clone.addrs[i];
                let mut block_data = vec![0u8; BSIZE];
                self.base_fs.dev.read(block_num as usize, &mut block_data);
                
                if let Err(e) = self.journal.log_block(tx_id, block_num, &old_data, &block_data) {
                    crate::println!("jfs_wrapper: failed to log block: {:?}", e);
                }
            }
        }
        
        // Commit or abort based on result
        if result > 0 {
            if let Err(e) = self.commit_transaction(tx_id) {
                crate::println!("jfs_wrapper: failed to commit transaction: {:?}", e);
            }
        } else {
            if let Err(e) = self.abort_transaction(tx_id) {
                crate::println!("jfs_wrapper: failed to abort transaction: {:?}", e);
            }
        }
        
        Some(result)
    }

    /// Create file system on device with journaling
    pub fn mkfs(&self) {
        // Begin transaction
        let tx_id = match self.begin_transaction() {
            Ok(id) => id,
            Err(_) => {
                // Fall back to non-journaling operation
                return self.base_fs.mkfs();
            }
        };
        
        // Perform the operation
        self.base_fs.mkfs();
        
        // Commit transaction
        if let Err(e) = self.commit_transaction(tx_id) {
            crate::println!("jfs_wrapper: failed to commit transaction: {:?}", e);
        }
    }

    /// Get journal statistics
    pub fn get_journal_stats(&self) -> super::journaling_fs::JournalStats {
        self.journal.get_stats()
    }

    /// Check if system is in recovery mode
    pub fn is_in_recovery(&self) -> bool {
        self.journal.is_in_recovery()
    }

    /// Checkpoint the journal
    pub fn checkpoint(&self) -> Result<(), JfsError> {
        self.journal.checkpoint()
    }
}

/// Global journaling file system wrapper instance
static mut JFS_WRAPPER: Option<JournalingFsWrapper> = None;

/// Initialize journaling file system wrapper
pub fn init(device: RamDisk) -> bool {
    unsafe {
        let mut jfs_wrapper = JournalingFsWrapper::new(device);
        if jfs_wrapper.init() {
            JFS_WRAPPER = Some(jfs_wrapper);
            true
        } else {
            false
        }
    }
}

/// Get the journaling file system wrapper instance
pub fn get_jfs_wrapper() -> Option<&'static JournalingFsWrapper> {
    unsafe { JFS_WRAPPER.as_ref() }
}

/// Initialize the file system with journaling support
pub fn init_fs_with_journaling(device: RamDisk) -> bool {
    // Try to initialize with journaling first
    if init(device) {
        crate::println!("fs: initialized with journaling support");
        return true;
    }
    
    // Fall back to regular file system
    crate::println!("fs: falling back to regular file system");
    super::init();
    true
}