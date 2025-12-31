//! Disk Quota System
//!
//! POSIX-compliant disk quota management.
//!
//! ## Overview
//!
//! This module provides comprehensive disk quota support:
//! - **User quotas**: Per-user limits on blocks and inodes
//! - **Group quotas**: Per-group limits
//! - **Project quotas**: Per-project limits
//! - **Grace periods**: Time before soft limits become hard
//! - **Enforcement**: Block quota operations that exceed limits
//!
//! ## Key Concepts
//!
//! - **Hard limit**: Absolute maximum, cannot be exceeded
//! - **Soft limit**: Warning threshold, with grace period
//! - **Grace period**: Time soft limit can be exceeded
//! - **Block quota**: Limit on disk space (bytes or blocks)
//! - **Inode quota**: Limit on file count
//!
//! ## Architecture
//!
//! ```
//! Quota Manager
//! ├── User Quotas (UID -> quota info)
//! ├── Group Quotas (GID -> quota info)
//! ├── Project Quotas (Project ID -> quota info)
//! └── Enforcement
//!     ├── Check before allocation
//!     ├── Update after allocation
//!     └── Warn on soft limit exceeded
//! ```
//!
//! ## Quota Types
//!
//! - **User quota**: Tracks per-user usage
//! - **Group quota**: Tracks per-group usage
//! - **Project quota**: Tracks per-project usage
//!
//! ## Enforcement
//!
//! Quotas are enforced at allocation time:
//! 1. Check if operation would exceed hard limit
//! 2. Update usage counters
//! 3. Check grace period if soft limit exceeded
//! 4. Warn user if needed

#![allow(dead_code)]

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::subsystems::sync::Mutex;
use crate::filesystem::error::{FsError, FsResult};

/// Default soft limit (1 GB)
pub const DEFAULT_SOFT_LIMIT: u64 = 1024 * 1024 * 1024;

/// Default hard limit (2 GB)
pub const DEFAULT_HARD_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

/// Default inode soft limit (10000 files)
pub const DEFAULT_INODE_SOFT: u64 = 10000;

/// Default inode hard limit (20000 files)
pub const DEFAULT_INODE_HARD: u64 = 20000;

/// Default grace period (7 days in seconds)
pub const DEFAULT_GRACE_PERIOD: u64 = 7 * 24 * 60 * 60;

/// Quota type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaType {
    /// User quota
    User,
    /// Group quota
    Group,
    /// Project quota
    Project,
}

/// Quota ID (UID, GID, or project ID)
pub type QuotaId = u32;

/// Quota limits
#[derive(Debug, Clone)]
pub struct QuotaLimits {
    /// Soft block limit
    pub block_soft: u64,
    /// Hard block limit
    pub block_hard: u64,
    /// Soft inode limit
    pub inode_soft: u64,
    /// Hard inode limit
    pub inode_hard: u64,
    /// Block grace period (seconds)
    pub block_grace: u64,
    /// Inode grace period (seconds)
    pub inode_grace: u64,
}

impl QuotaLimits {
    /// Create default limits
    pub fn new() -> Self {
        Self {
            block_soft: DEFAULT_SOFT_LIMIT,
            block_hard: DEFAULT_HARD_LIMIT,
            inode_soft: DEFAULT_INODE_SOFT,
            inode_hard: DEFAULT_INODE_HARD,
            block_grace: DEFAULT_GRACE_PERIOD,
            inode_grace: DEFAULT_GRACE_PERIOD,
        }
    }

    /// Create custom limits
    pub fn custom(block_soft: u64, block_hard: u64, inode_soft: u64, inode_hard: u64) -> Self {
        Self {
            block_soft,
            block_hard,
            inode_soft,
            inode_hard,
            block_grace: DEFAULT_GRACE_PERIOD,
            inode_grace: DEFAULT_GRACE_PERIOD,
        }
    }

    /// Check if block quota exceeded
    pub fn block_exceeded(&self, used: u64) -> bool {
        used >= self.block_hard
    }

    /// Check if block soft limit exceeded
    pub fn block_soft_exceeded(&self, used: u64) -> bool {
        used >= self.block_soft
    }

    /// Check if inode quota exceeded
    pub fn inode_exceeded(&self, used: u64) -> bool {
        used >= self.inode_hard
    }

    /// Check if inode soft limit exceeded
    pub fn inode_soft_exceeded(&self, used: u64) -> bool {
        used >= self.inode_soft
    }
}

impl Default for QuotaLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// Quota usage information
#[derive(Debug)]
pub struct QuotaUsage {
    /// Blocks used (in bytes)
    pub blocks_used: AtomicU64,
    /// Inodes used
    pub inodes_used: AtomicU64,
    /// Block soft limit exceeded time
    pub block_exceed_time: Mutex<Option<u64>>,
    /// Inode soft limit exceeded time
    pub inode_exceed_time: Mutex<Option<u64>>,
}

impl QuotaUsage {
    /// Create new usage tracking
    pub fn new() -> Self {
        Self {
            blocks_used: AtomicU64::new(0),
            inodes_used: AtomicU64::new(0),
            block_exceed_time: Mutex::new(None),
            inode_exceed_time: Mutex::new(None),
        }
    }

    /// Get blocks used
    pub fn get_blocks(&self) -> u64 {
        self.blocks_used.load(Ordering::SeqCst)
    }

    /// Get inodes used
    pub fn get_inodes(&self) -> u64 {
        self.inodes_used.load(Ordering::SeqCst)
    }

    /// Add blocks
    pub fn add_blocks(&self, bytes: u64) {
        self.blocks_used.fetch_add(bytes, Ordering::SeqCst);
    }

    /// Subtract blocks
    pub fn sub_blocks(&self, bytes: u64) {
        self.blocks_used.fetch_sub(bytes, Ordering::SeqCst);
    }

    /// Add inode
    pub fn add_inode(&self) {
        self.inodes_used.fetch_add(1, Ordering::SeqCst);
    }

    /// Subtract inode
    pub fn sub_inode(&self) {
        self.inodes_used.fetch_sub(1, Ordering::SeqCst);
    }

    /// Check if block grace period expired
    pub fn block_grace_expired(&self, grace_period: u64) -> bool {
        let exceed_time = self.block_exceed_time.lock();
        if let Some(time) = *exceed_time {
            let current = 0u64; // TODO: Use actual time
            current.saturating_sub(time) > grace_period
        } else {
            false
        }
    }

    /// Check if inode grace period expired
    pub fn inode_grace_expired(&self, grace_period: u64) -> bool {
        let exceed_time = self.inode_exceed_time.lock();
        if let Some(time) = *exceed_time {
            let current = 0u64; // TODO: Use actual time
            current.saturating_sub(time) > grace_period
        } else {
            false
        }
    }

    /// Record block soft limit exceed time
    pub fn mark_block_exceeded(&self) {
        let mut exceed_time = self.block_exceed_time.lock();
        if exceed_time.is_none() {
            *exceed_time = Some(0u64); // TODO: Use actual time
        }
    }

    /// Record inode soft limit exceed time
    pub fn mark_inode_exceeded(&self) {
        let mut exceed_time = self.inode_exceed_time.lock();
        if exceed_time.is_none() {
            *exceed_time = Some(0u64); // TODO: Use actual time
        }
    }

    /// Clear block exceed time
    pub fn clear_block_exceeded(&self) {
        *self.block_exceed_time.lock() = None;
    }

    /// Clear inode exceed time
    pub fn clear_inode_exceeded(&self) {
        *self.inode_exceed_time.lock() = None;
    }
}

impl Default for QuotaUsage {
    fn default() -> Self {
        Self::new()
    }
}

/// Quota information
#[derive(Debug)]
pub struct QuotaInfo {
    /// Quota ID
    pub id: QuotaId,
    /// Quota limits
    pub limits: QuotaLimits,
    /// Quota usage
    pub usage: QuotaUsage,
}

impl QuotaInfo {
    /// Create new quota info
    pub fn new(id: QuotaId, limits: QuotaLimits) -> Self {
        Self {
            id,
            limits,
            usage: QuotaUsage::new(),
        }
    }

    /// Check if block allocation allowed
    pub fn check_block_allocation(&self, bytes: u64) -> FsResult<()> {
        let current = self.usage.get_blocks();
        let new_total = current.saturating_add(bytes);

        // Check hard limit
        if self.limits.block_exceeded(new_total) {
            return Err(FsError::QuotaExceeded);
        }

        // Check soft limit with grace period
        if self.limits.block_soft_exceeded(new_total) {
            if self.usage.block_grace_expired(self.limits.block_grace) {
                return Err(FsError::QuotaExceeded);
            }
        }

        Ok(())
    }

    /// Check if inode allocation allowed
    pub fn check_inode_allocation(&self) -> FsResult<()> {
        let current = self.usage.get_inodes();
        let new_total = current.saturating_add(1);

        // Check hard limit
        if self.limits.inode_exceeded(new_total) {
            return Err(FsError::QuotaExceeded);
        }

        // Check soft limit with grace period
        if self.limits.inode_soft_exceeded(new_total) {
            if self.usage.inode_grace_expired(self.limits.inode_grace) {
                return Err(FsError::QuotaExceeded);
            }
        }

        Ok(())
    }

    /// Allocate blocks
    pub fn alloc_blocks(&self, bytes: u64) -> FsResult<()> {
        self.check_block_allocation(bytes)?;

        self.usage.add_blocks(bytes);

        // Update exceed time if soft limit exceeded
        if self.limits.block_soft_exceeded(self.usage.get_blocks()) {
            self.usage.mark_block_exceeded();
        } else {
            self.usage.clear_block_exceeded();
        }

        Ok(())
    }

    /// Free blocks
    pub fn free_blocks(&self, bytes: u64) {
        self.usage.sub_blocks(bytes);

        // Clear exceed time if below soft limit
        if !self.limits.block_soft_exceeded(self.usage.get_blocks()) {
            self.usage.clear_block_exceeded();
        }
    }

    /// Allocate inode
    pub fn alloc_inode(&self) -> FsResult<()> {
        self.check_inode_allocation()?;

        self.usage.add_inode();

        // Update exceed time if soft limit exceeded
        if self.limits.inode_soft_exceeded(self.usage.get_inodes()) {
            self.usage.mark_inode_exceeded();
        } else {
            self.usage.clear_inode_exceeded();
        }

        Ok(())
    }

    /// Free inode
    pub fn free_inode(&self) {
        self.usage.sub_inode();

        // Clear exceed time if below soft limit
        if !self.limits.inode_soft_exceeded(self.usage.get_inodes()) {
            self.usage.clear_inode_exceeded();
        }
    }

    /// Get quota status
    pub fn get_status(&self) -> QuotaStatus {
        let blocks_used = self.usage.get_blocks();
        let inodes_used = self.usage.get_inodes();

        QuotaStatus {
            id: self.id,
            blocks_used,
            blocks_soft: self.limits.block_soft,
            blocks_hard: self.limits.block_hard,
            inodes_used,
            inodes_soft: self.limits.inode_soft,
            inodes_hard: self.limits.inode_hard,
            block_soft_exceeded: self.limits.block_soft_exceeded(blocks_used),
            inode_soft_exceeded: self.limits.inode_soft_exceeded(inodes_used),
            block_grace_expired: self.usage.block_grace_expired(self.limits.block_grace),
            inode_grace_expired: self.usage.inode_grace_expired(self.limits.inode_grace),
        }
    }
}

/// Quota status information
#[derive(Debug, Clone)]
pub struct QuotaStatus {
    /// Quota ID
    pub id: QuotaId,
    /// Blocks used
    pub blocks_used: u64,
    /// Block soft limit
    pub blocks_soft: u64,
    /// Block hard limit
    pub blocks_hard: u64,
    /// Inodes used
    pub inodes_used: u64,
    /// Inode soft limit
    pub inodes_soft: u64,
    /// Inode hard limit
    pub inodes_hard: u64,
    /// Block soft limit exceeded
    pub block_soft_exceeded: bool,
    /// Inode soft limit exceeded
    pub inode_soft_exceeded: bool,
    /// Block grace period expired
    pub block_grace_expired: bool,
    /// Inode grace period expired
    pub inode_grace_expired: bool,
}

/// Quota manager
pub struct QuotaManager {
    /// User quotas
    pub user_quotas: Mutex<BTreeMap<QuotaId, QuotaInfo>>,
    /// Group quotas
    pub group_quotas: Mutex<BTreeMap<QuotaId, QuotaInfo>>,
    /// Project quotas
    pub project_quotas: Mutex<BTreeMap<QuotaId, QuotaInfo>>,
    /// Quota enabled flags
    pub enabled: Mutex<QuotaEnabled>,
}

/// Quota enabled flags
#[derive(Debug, Clone, Copy, Default)]
pub struct QuotaEnabled {
    /// User quota enabled
    pub user: bool,
    /// Group quota enabled
    pub group: bool,
    /// Project quota enabled
    pub project: bool,
}

impl QuotaManager {
    /// Create a new quota manager
    pub fn new() -> Self {
        Self {
            user_quotas: Mutex::new(BTreeMap::new()),
            group_quotas: Mutex::new(BTreeMap::new()),
            project_quotas: Mutex::new(BTreeMap::new()),
            enabled: Mutex::new(QuotaEnabled::default()),
        }
    }

    /// Enable quota type
    pub fn enable(&self, quota_type: QuotaType) {
        let mut enabled = self.enabled.lock();
        match quota_type {
            QuotaType::User => enabled.user = true,
            QuotaType::Group => enabled.group = true,
            QuotaType::Project => enabled.project = true,
        }
    }

    /// Disable quota type
    pub fn disable(&self, quota_type: QuotaType) {
        let mut enabled = self.enabled.lock();
        match quota_type {
            QuotaType::User => enabled.user = false,
            QuotaType::Group => enabled.group = false,
            QuotaType::Project => enabled.project = false,
        }
    }

    /// Check if quota type is enabled
    pub fn is_enabled(&self, quota_type: QuotaType) -> bool {
        let enabled = self.enabled.lock();
        match quota_type {
            QuotaType::User => enabled.user,
            QuotaType::Group => enabled.group,
            QuotaType::Project => enabled.project,
        }
    }

    /// Set quota limits
    pub fn set_quota(&self, quota_type: QuotaType, id: QuotaId, limits: QuotaLimits) -> FsResult<()> {
        let quotas = match quota_type {
            QuotaType::User => &self.user_quotas,
            QuotaType::Group => &self.group_quotas,
            QuotaType::Project => &self.project_quotas,
        };

        let mut quotas = quotas.lock();
        quotas.insert(id, QuotaInfo::new(id, limits));

        Ok(())
    }

    /// Get quota information
    pub fn get_quota(&self, quota_type: QuotaType, id: QuotaId) -> FsResult<QuotaInfo> {
        let quotas = match quota_type {
            QuotaType::User => &self.user_quotas,
            QuotaType::Group => &self.group_quotas,
            QuotaType::Project => &self.project_quotas,
        };

        let quotas = quotas.lock();
        quotas.get(&id).map(|info| {
            // Manual clone implementation for QuotaInfo
            QuotaInfo {
                id: info.id,
                limits: QuotaLimits {
                    block_soft: info.limits.block_soft,
                    block_hard: info.limits.block_hard,
                    inode_soft: info.limits.inode_soft,
                    inode_hard: info.limits.inode_hard,
                    block_grace: info.limits.block_grace,
                    inode_grace: info.limits.inode_grace,
                },
                usage: QuotaUsage {
                    blocks_used: AtomicU64::new(info.usage.blocks_used.load(Ordering::SeqCst)),
                    inodes_used: AtomicU64::new(info.usage.inodes_used.load(Ordering::SeqCst)),
                    block_exceed_time: Mutex::new(*info.usage.block_exceed_time.lock()),
                    inode_exceed_time: Mutex::new(*info.usage.inode_exceed_time.lock()),
                },
            }
        }).ok_or(FsError::NotFound)
    }

    /// Check and allocate blocks
    pub fn alloc_blocks(&self, uid: QuotaId, gid: QuotaId, bytes: u64) -> FsResult<()> {
        let enabled = self.enabled.lock();

        if enabled.user {
            if let Ok(quota) = self.get_quota(QuotaType::User, uid) {
                quota.alloc_blocks(bytes)?;
            }
        }

        if enabled.group {
            if let Ok(quota) = self.get_quota(QuotaType::Group, gid) {
                quota.alloc_blocks(bytes)?;
            }
        }

        Ok(())
    }

    /// Free blocks
    pub fn free_blocks(&self, uid: QuotaId, gid: QuotaId, bytes: u64) {
        let enabled = self.enabled.lock();

        if enabled.user {
            if let Ok(quota) = self.get_quota(QuotaType::User, uid) {
                quota.free_blocks(bytes);
            }
        }

        if enabled.group {
            if let Ok(quota) = self.get_quota(QuotaType::Group, gid) {
                quota.free_blocks(bytes);
            }
        }
    }

    /// Check and allocate inode
    pub fn alloc_inode(&self, uid: QuotaId, gid: QuotaId) -> FsResult<()> {
        let enabled = self.enabled.lock();

        if enabled.user {
            if let Ok(quota) = self.get_quota(QuotaType::User, uid) {
                quota.alloc_inode()?;
            }
        }

        if enabled.group {
            if let Ok(quota) = self.get_quota(QuotaType::Group, gid) {
                quota.alloc_inode()?;
            }
        }

        Ok(())
    }

    /// Free inode
    pub fn free_inode(&self, uid: QuotaId, gid: QuotaId) {
        let enabled = self.enabled.lock();

        if enabled.user {
            if let Ok(quota) = self.get_quota(QuotaType::User, uid) {
                quota.free_inode();
            }
        }

        if enabled.group {
            if let Ok(quota) = self.get_quota(QuotaType::Group, gid) {
                quota.free_inode();
            }
        }
    }

    /// Get quota status
    pub fn get_status(&self, quota_type: QuotaType, id: QuotaId) -> FsResult<QuotaStatus> {
        let quota = self.get_quota(quota_type, id)?;
        Ok(quota.get_status())
    }

    /// List all quotas of a type
    pub fn list_quotas(&self, quota_type: QuotaType) -> Vec<QuotaStatus> {
        let quotas = match quota_type {
            QuotaType::User => &self.user_quotas,
            QuotaType::Group => &self.group_quotas,
            QuotaType::Project => &self.project_quotas,
        };

        let quotas = quotas.lock();
        quotas.values().map(|q| q.get_status()).collect()
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize quota system
pub fn init_quota_system() -> FsResult<()> {
    crate::println!("[quota] Quota system initialized");
    Ok(())
}

/// Shutdown quota system
pub fn shutdown_quota_system() -> FsResult<()> {
    crate::println!("[quota] Quota system shutdown");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_limits() {
        let limits = QuotaLimits::new();
        assert!(!limits.block_exceeded(0));
        assert!(limits.block_exceeded(DEFAULT_HARD_LIMIT));
    }

    #[test]
    fn test_quota_info() {
        let limits = QuotaLimits::new();
        let info = QuotaInfo::new(1000, limits);

        assert!(info.alloc_blocks(1024).is_ok());
        assert_eq!(info.usage.get_blocks(), 1024);

        info.free_blocks(1024);
        assert_eq!(info.usage.get_blocks(), 0);
    }

    #[test]
    fn test_quota_exceeded() {
        let limits = QuotaLimits::custom(100, 200, 10, 20);
        let info = QuotaInfo::new(1000, limits);

        assert!(info.alloc_blocks(300).is_err());
    }

    #[test]
    fn test_quota_manager() {
        let manager = QuotaManager::new();
        manager.enable(QuotaType::User);

        let limits = QuotaLimits::new();
        assert!(manager.set_quota(QuotaType::User, 1000, limits).is_ok());

        assert!(manager.alloc_blocks(1000, 1000, 1024).is_ok());
    }

    #[test]
    fn test_inode_quota() {
        let limits = QuotaLimits::new();
        let info = QuotaInfo::new(1000, limits);

        assert!(info.alloc_inode().is_ok());
        assert_eq!(info.usage.get_inodes(), 1);

        info.free_inode();
        assert_eq!(info.usage.get_inodes(), 0);
    }
}
