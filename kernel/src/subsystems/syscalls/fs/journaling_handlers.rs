//! Journaling File System System Call Handlers
//!
//! This module contains system call handlers for journaling file system operations.

use super::types::*;
use nos_nos_error_handling::unified::KernelError;
use alloc::string::ToString;
use crate::subsystems::fs::{get_jfs_wrapper, JournalStats};

/// Handle journal_begin system call - begin a new transaction
pub fn handle_journal_begin(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 0 {
        return Err(KernelError::InvalidArgument);
    }

    let jfs_wrapper = get_jfs_wrapper().ok_or(KernelError::NotSupported)?;
    
    match jfs_wrapper.begin_transaction() {
        Ok(tx_id) => Ok(tx_id),
        Err(_) => Err(KernelError::IoError),
    }
}

/// Handle journal_commit system call - commit a transaction
pub fn handle_journal_commit(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let tx_id = args[0];
    let jfs_wrapper = get_jfs_wrapper().ok_or(KernelError::NotSupported)?;
    
    match jfs_wrapper.commit_transaction(tx_id) {
        Ok(()) => Ok(0),
        Err(_) => Err(KernelError::IoError),
    }
}

/// Handle journal_abort system call - abort a transaction
pub fn handle_journal_abort(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let tx_id = args[0];
    let jfs_wrapper = get_jfs_wrapper().ok_or(KernelError::NotSupported)?;
    
    match jfs_wrapper.abort_transaction(tx_id) {
        Ok(()) => Ok(0),
        Err(_) => Err(KernelError::IoError),
    }
}

/// Handle journal_enable system call - enable or disable journaling
pub fn handle_journal_enable(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let enabled = args[0] != 0;
    let jfs_wrapper = get_jfs_wrapper().ok_or(KernelError::NotSupported)?;
    
    jfs_wrapper.set_journaling(enabled);
    Ok(0)
}

/// Handle journal_status system call - get journaling status
pub fn handle_journal_status(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 0 {
        return Err(KernelError::InvalidArgument);
    }

    let jfs_wrapper = get_jfs_wrapper().ok_or(KernelError::NotSupported)?;
    
    let status = if jfs_wrapper.is_journaling_enabled() { 1 } else { 0 };
    Ok(status)
}

/// Handle journal_stats system call - get journal statistics
pub fn handle_journal_stats(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 1 {
        return Err(KernelError::InvalidArgument);
    }

    let stats_ptr = args[0] as *mut JournalStats;
    if stats_ptr.is_null() {
        return Err(KernelError::BadAddress);
    }

    let jfs_wrapper = get_jfs_wrapper().ok_or(KernelError::NotSupported)?;
    let stats = jfs_wrapper.get_journal_stats();
    
    // Copy stats to user space
    unsafe {
        *stats_ptr = stats;
    }
    
    Ok(0)
}

/// Handle journal_checkpoint system call - checkpoint the journal
pub fn handle_journal_checkpoint(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 0 {
        return Err(KernelError::InvalidArgument);
    }

    let jfs_wrapper = get_jfs_wrapper().ok_or(KernelError::NotSupported)?;
    
    match jfs_wrapper.checkpoint() {
        Ok(()) => Ok(0),
        Err(_) => Err(KernelError::IoError),
    }
}

/// Handle journal_recovery_status system call - check if system is in recovery mode
pub fn handle_journal_recovery_status(args: &[u64]) -> Result<u64, KernelError> {
    if args.len() != 0 {
        return Err(KernelError::InvalidArgument);
    }

    let jfs_wrapper = get_jfs_wrapper().ok_or(KernelError::NotSupported)?;
    
    let status = if jfs_wrapper.is_in_recovery() { 1 } else { 0 };
    Ok(status)
}