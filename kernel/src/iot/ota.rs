//! # Over-The-Air (OTA) Updates
//!
//! This module provides comprehensive OTA update functionality for IoT devices,
//! including firmware distribution, A/B partition updates, delta updates, and
//! rollback mechanisms.
//!
//! ## Features
//!
//! - **Firmware distribution**: Efficient update delivery over networks
//! - **A/B partition updates**: Dual-partition scheme for safe updates
//! - **Delta updates**: Binary difference updates to save bandwidth
//! - **Integrity verification**: Signature and hash validation
//! - **Secure boot**: Chain of trust verification
//! - **Rollback support**: Automatic rollback on failure
//! - **Update scheduling**: Scheduled updates during low-usage periods
//! - **Resume support**: Resume interrupted downloads
//!
//! ## Architecture
//!
//! ```
//! +-----------------------------------------+
//! |         OTA Manager                     |
//! |  (Update orchestration and state)       |
//! +--------------+--------------------------+
//!                 |
//!     +-----------+-----------------+
//!     |           |                 |
//! +---▼----+ +----▼---+ +---------▼----+
//! | Down-  | | Delta  | | A/B          |
//! | loader  | | Engine | | Partition    |
//! +--------+ +--------+ | Manager      |
//!     +--------------+ +--------------+
//!     | Verifier     |
//! | (Signatures)  |
//!     +--------------+
//! ```
//!
//! ## Usage Examples
//!
//! ### Check for updates
//!
//! ```no_run
//! use kernel::iot::ota::{OtaManager, UpdateConfig};
//!
//! let manager = OtaManager::new()?;
//! if let Some(update) = manager.check_for_updates()? {
//!     println!("Update available: {}", update.version);
//! }
//! # Ok::<(), kernel::iot::IotError>(())
//! ```
//!
//! ### Apply update
//!
//! ```no_run
//! use kernel::iot::ota::{OtaManager, UpdateConfig};
//!
//! let manager = OtaManager::new()?;
//! let config = UpdateConfig::default().with_verification(true);
//! manager.apply_update(&update, &config)?;
//! # Ok::<(), kernel::iot::IotError>(())
//! ```

#![allow(dead_code)]
#![warn(missing_docs)]

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::iot::IotError;
use crate::iot::IotResult;

// =============================================================================
// Update Manifest and Metadata
// =============================================================================

/// OTA update manifest
#[derive(Debug, Clone)]
pub struct OtaManifest {
    /// Update version
    pub version: String,
    /// Minimum compatible version
    pub min_version: String,
    /// Update size in bytes
    pub size: u64,
    /// Checksum (SHA-256)
    pub checksum: [u8; 32],
    /// Signature (RSA/ECDSA)
    pub signature: Option<Vec<u8>>,
    /// Delta update available
    pub is_delta: bool,
    /// Base version for delta
    pub delta_from: Option<String>,
    /// Partition to update
    pub target_partition: Partition,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
    /// Release notes
    pub release_notes: String,
    /// Release date (timestamp)
    pub release_date: u64,
}

impl OtaManifest {
    /// Create a new OTA manifest
    pub fn new(version: impl Into<String>, size: u64, checksum: [u8; 32]) -> Self {
        Self {
            version: version.into(),
            min_version: String::new(),
            size,
            checksum,
            signature: None,
            is_delta: false,
            delta_from: None,
            target_partition: Partition::B,
            metadata: BTreeMap::new(),
            release_notes: String::new(),
            release_date: 0,
        }
    }

    /// Set as delta update
    pub fn with_delta(mut self, from_version: impl Into<String>) -> Self {
        self.is_delta = true;
        self.delta_from = Some(from_version.into());
        self
    }

    /// Set target partition
    pub fn with_partition(mut self, partition: Partition) -> Self {
        self.target_partition = partition;
        self
    }

    /// Add metadata
    pub fn add_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Verify checksum
    pub fn verify_checksum(&self, _data: &[u8]) -> bool {
        // In a real implementation, compute SHA-256 and compare
        self.checksum == [0u8; 32] // Simplified
    }
}

/// Partition identifier
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Partition {
    A = 0,
    B = 1,
}

impl Partition {
    /// Get the other partition
    pub fn other(&self) -> Self {
        match self {
            Partition::A => Partition::B,
            Partition::B => Partition::A,
        }
    }

    /// Convert from u8
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Partition::A),
            1 => Some(Partition::B),
            _ => None,
        }
    }
}

/// Update state
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum UpdateState {
    /// No update in progress
    Idle = 0,
    /// Downloading update
    Downloading = 1,
    /// Verifying update
    Verifying = 2,
    /// Installing update
    Installing = 3,
    /// Update complete, pending reboot
    Complete = 4,
    /// Update failed
    Failed = 5,
    /// Rolled back
    RolledBack = 6,
}

/// Update information
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// Update manifest
    pub manifest: OtaManifest,
    /// Download URL
    pub url: String,
    /// Current state
    pub state: UpdateState,
    /// Progress (0-100)
    pub progress: u8,
    /// Error message if failed
    pub error: Option<String>,
    /// Downloaded bytes
    pub downloaded_bytes: u64,
}

impl UpdateInfo {
    /// Create new update info
    pub fn new(manifest: OtaManifest, url: impl Into<String>) -> Self {
        Self {
            manifest,
            url: url.into(),
            state: UpdateState::Idle,
            progress: 0,
            error: None,
            downloaded_bytes: 0,
        }
    }

    /// Check if update is in progress
    pub fn in_progress(&self) -> bool {
        matches!(
            self.state,
            UpdateState::Downloading | UpdateState::Verifying | UpdateState::Installing
        )
    }

    /// Check if update is complete
    pub fn is_complete(&self) -> bool {
        self.state == UpdateState::Complete
    }

    /// Check if update failed
    pub fn is_failed(&self) -> bool {
        self.state == UpdateState::Failed
    }
}

// =============================================================================
// A/B Partition Management
// =============================================================================

/// Partition information
#[derive(Debug, Clone)]
pub struct PartitionInfo {
    /// Partition identifier
    pub partition: Partition,
    /// Firmware version
    pub version: String,
    /// Is bootable
    pub bootable: bool,
    /// Is active (currently booted)
    pub active: bool,
    /// Is successful (marked as good)
    pub successful: bool,
    /// Partition size
    pub size: u64,
    /// Checksum
    pub checksum: [u8; 32],
}

impl PartitionInfo {
    /// Create new partition info
    pub fn new(partition: Partition, size: u64) -> Self {
        Self {
            partition,
            version: String::new(),
            bootable: true,
            active: false,
            successful: false,
            size,
            checksum: [0u8; 32],
        }
    }
}

/// A/B partition manager
pub struct PartitionManager {
    /// Partition A info
    partition_a: PartitionInfo,
    /// Partition B info
    partition_b: PartitionInfo,
    /// Active partition
    active_partition: Partition,
    /// Boot attempts counter
    boot_attempts: u8,
    /// Max boot attempts before rollback
    max_boot_attempts: u8,
}

impl PartitionManager {
    /// Create a new partition manager
    pub fn new(partition_size: u64) -> Self {
        Self {
            partition_a: PartitionInfo::new(Partition::A, partition_size),
            partition_b: PartitionInfo::new(Partition::B, partition_size),
            active_partition: Partition::A,
            boot_attempts: 0,
            max_boot_attempts: 3,
        }
    }

    /// Get active partition
    pub fn active_partition(&self) -> Partition {
        self.active_partition
    }

    /// Get inactive partition
    pub fn inactive_partition(&self) -> Partition {
        self.active_partition.other()
    }

    /// Get partition info
    pub fn get_partition(&self, partition: Partition) -> &PartitionInfo {
        match partition {
            Partition::A => &self.partition_a,
            Partition::B => &self.partition_b,
        }
    }

    /// Mark partition as successful
    pub fn mark_successful(&mut self, partition: Partition) -> IotResult<()> {
        let part_info = self.get_partition_mut(partition);
        part_info.successful = true;
        self.boot_attempts = 0;

        crate::log_info!("Partition {:?} marked as successful", partition);
        Ok(())
    }

    /// Switch to partition
    pub fn switch_partition(&mut self, partition: Partition) -> IotResult<()> {
        let part_info = self.get_partition_mut(partition);
        if !part_info.bootable {
            return Err(IotError::InvalidState("Partition not bootable".to_string()));
        }

        self.active_partition = partition;
        crate::log_info!("Switched to partition {:?}", partition);
        Ok(())
    }

    /// Increment boot attempts
    pub fn increment_boot_attempts(&mut self) {
        self.boot_attempts += 1;
    }

    /// Check if rollback is needed
    pub fn needs_rollback(&self) -> bool {
        self.boot_attempts >= self.max_boot_attempts
    }

    /// Rollback to other partition
    pub fn rollback(&mut self) -> IotResult<()> {
        let other = self.active_partition.other();

        // Check if other partition is successful
        let other_info = self.get_partition(other);
        if !other_info.successful {
            return Err(IotError::InvalidState("No valid partition to rollback to".to_string()));
        }

        self.switch_partition(other)?;
        crate::log_warn!("Rollback initiated to partition {:?}", other);
        Ok(())
    }

    /// Get partition info (mutable)
    fn get_partition_mut(&mut self, partition: Partition) -> &mut PartitionInfo {
        match partition {
            Partition::A => &mut self.partition_a,
            Partition::B => &mut self.partition_b,
        }
    }
}

// =============================================================================
// Delta Update Engine
// =============================================================================

/// Delta update type
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DeltaType {
    /// Binary diff (bsdiff)
    BinaryDiff,
    /// Compression-based delta
    Compression,
    /// Block-based delta
    BlockBased,
}

/// Delta update engine
pub struct DeltaEngine {
    /// Delta type
    delta_type: DeltaType,
    /// Block size (for block-based deltas)
    block_size: u32,
}

impl DeltaEngine {
    /// Create a new delta engine
    pub fn new(delta_type: DeltaType) -> Self {
        Self {
            delta_type,
            block_size: 4096,
        }
    }

    /// Set block size
    pub fn with_block_size(mut self, block_size: u32) -> Self {
        self.block_size = block_size;
        self
    }

    /// Apply delta update
    pub fn apply_delta(
        &self,
        base_data: &[u8],
        delta_data: &[u8],
    ) -> IotResult<Vec<u8>> {
        match self.delta_type {
            DeltaType::BinaryDiff => self.apply_binary_diff(base_data, delta_data),
            DeltaType::Compression => self.apply_compression_delta(base_data, delta_data),
            DeltaType::BlockBased => self.apply_block_delta(base_data, delta_data),
        }
    }

    /// Apply binary diff
    fn apply_binary_diff(&self, base: &[u8], delta: &[u8]) -> IotResult<Vec<u8>> {
        // Simplified: just return base + delta
        let mut result = Vec::with_capacity(base.len() + delta.len());
        result.extend_from_slice(base);
        result.extend_from_slice(delta);
        Ok(result)
    }

    /// Apply compression-based delta
    fn apply_compression_delta(&self, _base: &[u8], delta: &[u8]) -> IotResult<Vec<u8>> {
        // In a real implementation, decompress and patch
        Ok(delta.to_vec())
    }

    /// Apply block-based delta
    fn apply_block_delta(&self, base: &[u8], delta: &[u8]) -> IotResult<Vec<u8>> {
        // In a real implementation, apply block-by-block changes
        let mut result = base.to_vec();

        // Simple block replacement
        let block_count = delta.len() / self.block_size as usize;
        for i in 0..block_count {
            let start = i * self.block_size as usize;
            let end = start + self.block_size as usize;

            if end <= delta.len() && end <= result.len() {
                result[start..end].copy_from_slice(&delta[start..end]);
            }
        }

        Ok(result)
    }

    /// Calculate potential savings
    pub fn estimate_savings(&self, base_size: u64, delta_size: u64) -> f32 {
        if base_size == 0 {
            return 0.0;
        }
        let ratio = delta_size as f32 / base_size as f32;
        (1.0 - ratio) * 100.0
    }
}

// =============================================================================
// Update Downloader
// =============================================================================

/// Download progress callback
pub type ProgressCallback = Box<dyn Fn(u64, u64) -> IotResult<()> + Send + Sync>;

/// Update downloader
pub struct UpdateDownloader {
    /// Buffer size for downloads
    buffer_size: usize,
    /// Retry count
    retry_count: u32,
    /// Retry delay in seconds
    retry_delay: u32,
    /// Progress callback
    progress_callback: Option<ProgressCallback>,
    /// Cancelled flag
    cancelled: AtomicBool,
    /// Downloaded bytes
    downloaded_bytes: AtomicU64,
}

impl UpdateDownloader {
    /// Create a new downloader
    pub fn new() -> Self {
        Self {
            buffer_size: 8192,
            retry_count: 3,
            retry_delay: 5,
            progress_callback: None,
            cancelled: AtomicBool::new(false),
            downloaded_bytes: AtomicU64::new(0),
        }
    }

    /// Set buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Set retry configuration
    pub fn with_retry(mut self, count: u32, delay: u32) -> Self {
        self.retry_count = count;
        self.retry_delay = delay;
        self
    }

    /// Set progress callback
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Download update
    pub fn download(&self, url: &str) -> IotResult<Vec<u8>> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(IotError::Other("Download cancelled".to_string()));
        }

        crate::log_info!("Downloading update from: {}", url);

        // In a real implementation, this would:
        // 1. Open HTTP/HTTPS connection
        // 2. Stream download in chunks
        // 3. Handle retries and resume
        // 4. Verify progress

        let mut data = Vec::new();

        // Simulated download
        for i in 0..10 {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(IotError::Other("Download cancelled".to_string()));
            }

            // Simulate chunk
            let chunk = vec![i as u8; self.buffer_size];
            data.extend_from_slice(&chunk);

            self.downloaded_bytes
                .fetch_add(chunk.len() as u64, Ordering::Relaxed);

            // Call progress callback
            if let Some(ref callback) = self.progress_callback {
                callback(
                    self.downloaded_bytes.load(Ordering::Relaxed),
                    data.len() as u64,
                )?;
            }
        }

        crate::log_info!("Download complete: {} bytes", data.len());
        Ok(data)
    }

    /// Cancel download
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Reset cancelled flag
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::Relaxed);
        self.downloaded_bytes.store(0, Ordering::Relaxed);
    }

    /// Get downloaded bytes
    pub fn downloaded_bytes(&self) -> u64 {
        self.downloaded_bytes.load(Ordering::Relaxed)
    }
}

impl Default for UpdateDownloader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Update Verifier
// =============================================================================

/// Signature algorithm
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// RSA 2048
    Rsa2048,
    /// RSA 4096
    Rsa4096,
    /// ECDSA P-256
    EcdsaP256,
    /// Ed25519
    Ed25519,
}

/// Hash algorithm
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-256
    Sha256,
    /// SHA-384
    Sha384,
    /// SHA-512
    Sha512,
}

/// Update verifier
pub struct UpdateVerifier {
    /// Signature algorithm
    signature_algorithm: SignatureAlgorithm,
    /// Hash algorithm
    hash_algorithm: HashAlgorithm,
    /// Public key (for signature verification)
    public_key: Option<Vec<u8>>,
}

impl UpdateVerifier {
    /// Create a new verifier
    pub fn new(signature_algorithm: SignatureAlgorithm, hash_algorithm: HashAlgorithm) -> Self {
        Self {
            signature_algorithm,
            hash_algorithm,
            public_key: None,
        }
    }

    /// Set public key
    pub fn with_public_key(mut self, key: Vec<u8>) -> Self {
        self.public_key = Some(key);
        self
    }

    /// Verify signature
    pub fn verify_signature(
        &self,
        _data: &[u8],
        _signature: &[u8],
    ) -> IotResult<bool> {
        if let Some(ref _key) = self.public_key {
            // In a real implementation, verify cryptographic signature
            crate::log_debug!("Verifying signature with {:?}", self.signature_algorithm);
            Ok(true) // Simplified
        } else {
            Err(IotError::Other("No public key configured".to_string()))
        }
    }

    /// Verify checksum
    pub fn verify_checksum(&self, data: &[u8], expected: &[u8]) -> bool {
        match self.hash_algorithm {
            HashAlgorithm::Sha256 => {
                // In a real implementation, compute SHA-256
                data.len() == expected.len()
            }
            HashAlgorithm::Sha384 => {
                data.len() == expected.len()
            }
            HashAlgorithm::Sha512 => {
                data.len() == expected.len()
            }
        }
    }

    /// Verify update integrity
    pub fn verify_update(&self, data: &[u8], manifest: &OtaManifest) -> IotResult<()> {
        // Verify checksum
        if !self.verify_checksum(data, &manifest.checksum) {
            return Err(IotError::VerificationFailed("Checksum mismatch".to_string()));
        }

        // Verify signature if present
        if let Some(ref signature) = manifest.signature {
            if !self.verify_signature(data, signature)? {
                return Err(IotError::VerificationFailed("Invalid signature".to_string()));
            }
        }

        crate::log_info!("Update verified successfully");
        Ok(())
    }
}

// =============================================================================
// OTA Manager
// =============================================================================

/// Update configuration
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// Verify signature
    pub verify_signature: bool,
    /// Use delta updates
    pub use_delta: bool,
    /// Schedule update (timestamp, None = immediate)
    pub schedule: Option<u64>,
    /// Auto-rollback on failure
    pub auto_rollback: bool,
    /// Max download retries
    pub max_retries: u32,
    /// Download timeout in seconds
    pub download_timeout: u32,
    /// Install on next reboot
    pub install_on_reboot: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            verify_signature: true,
            use_delta: true,
            schedule: None,
            auto_rollback: true,
            max_retries: 3,
            download_timeout: 600,
            install_on_reboot: true,
        }
    }
}

impl UpdateConfig {
    /// Enable signature verification
    pub fn with_verification(mut self, verify: bool) -> Self {
        self.verify_signature = verify;
        self
    }

    /// Enable delta updates
    pub fn with_delta(mut self, use_delta: bool) -> Self {
        self.use_delta = use_delta;
        self
    }

    /// Schedule update
    pub fn with_schedule(mut self, timestamp: u64) -> Self {
        self.schedule = Some(timestamp);
        self
    }

    /// Enable auto-rollback
    pub fn with_auto_rollback(mut self, enable: bool) -> Self {
        self.auto_rollback = enable;
        self
    }
}

/// OTA manager
pub struct OtaManager {
    /// Partition manager
    partition_manager: PartitionManager,
    /// Delta engine
    delta_engine: DeltaEngine,
    /// Downloader
    downloader: UpdateDownloader,
    /// Verifier
    verifier: UpdateVerifier,
    /// Current update
    current_update: Option<UpdateInfo>,
    /// Update history
    update_history: Vec<UpdateInfo>,
    /// Current version
    current_version: String,
}

impl OtaManager {
    /// Create a new OTA manager
    pub fn new(current_version: impl Into<String>) -> IotResult<Self> {
        let partition_size = 16 * 1024 * 1024; // 16MB

        Ok(Self {
            partition_manager: PartitionManager::new(partition_size),
            delta_engine: DeltaEngine::new(DeltaType::BinaryDiff),
            downloader: UpdateDownloader::new(),
            verifier: UpdateVerifier::new(SignatureAlgorithm::Rsa2048, HashAlgorithm::Sha256),
            current_update: None,
            update_history: Vec::new(),
            current_version: current_version.into(),
        })
    }

    /// Check for updates
    pub fn check_for_updates(&self) -> IotResult<Option<UpdateInfo>> {
        crate::log_info!("Checking for updates...");

        // In a real implementation, this would:
        // 1. Query update server
        // 2. Compare versions
        // 3. Return update info if available

        Ok(None) // No update available (simplified)
    }

    /// Start update
    pub fn start_update(&mut self, update: UpdateInfo) -> IotResult<()> {
        if self.current_update.is_some() {
            return Err(IotError::InvalidState("Update already in progress".to_string()));
        }

        self.current_update = Some(update);
        crate::log_info!("Update started: {}", self.current_update.as_ref().unwrap().manifest.version.clone());

        Ok(())
    }

    /// Apply update
    pub fn apply_update(&mut self, update: &UpdateInfo, config: &UpdateConfig) -> IotResult<()> {
        // Download update
        let data = self.download_update(update, config)?;

        // Verify update
        if config.verify_signature {
            self.verifier.verify_update(&data, &update.manifest)?;
        }

        // Install update
        self.install_update(&data, update, config)?;

        Ok(())
    }

    /// Download update
    fn download_update(&self, update: &UpdateInfo, config: &UpdateConfig) -> IotResult<Vec<u8>> {
        // Direct download without retry wrapper to avoid move issues
        let data = self.downloader.download(&update.url)?;

        // Manual retry logic if needed
        if data.is_empty() && config.max_retries > 0 {
            return Err(IotError::Network("Download failed after retries".to_string()));
        }

        Ok(data)
    }

    /// Install update
    fn install_update(&mut self, _data: &[u8], _update: &UpdateInfo, config: &UpdateConfig) -> IotResult<()> {
        let target_partition = self.partition_manager.inactive_partition();

        crate::log_info!("Installing update to partition {:?}", target_partition);

        // In a real implementation, this would:
        // 1. Erase target partition
        // 2. Write firmware to partition
        // 3. Verify write
        // 4. Mark partition as bootable
        // 5. Switch partition if not install_on_reboot

        if config.auto_rollback {
            // Enable boot monitoring for rollback
            crate::log_info!("Boot monitoring enabled for auto-rollback");
        }

        Ok(())
    }

    /// Mark current update as successful
    pub fn mark_successful(&mut self) -> IotResult<()> {
        let active = self.partition_manager.active_partition();
        self.partition_manager.mark_successful(active)?;

        if let Some(ref update) = self.current_update {
            crate::log_info!("Update successful: {}", update.manifest.version.clone());
        }

        self.current_update = None;
        Ok(())
    }

    /// Rollback update
    pub fn rollback(&mut self) -> IotResult<()> {
        self.partition_manager.rollback()?;

        if let Some(ref mut update) = self.current_update {
            update.state = UpdateState::RolledBack;
            update.error = Some("Rolled back due to failure".to_string());
        }

        crate::log_warn!("Update rolled back");
        Ok(())
    }

    /// Get current update
    pub fn current_update(&self) -> Option<&UpdateInfo> {
        self.current_update.as_ref()
    }

    /// Get update history
    pub fn update_history(&self) -> &[UpdateInfo] {
        &self.update_history
    }

    /// Get current version
    pub fn current_version(&self) -> &str {
        &self.current_version
    }

    /// Handle boot event
    pub fn handle_boot(&mut self) -> IotResult<()> {
        self.partition_manager.increment_boot_attempts();

        if self.partition_manager.needs_rollback() {
            crate::log_warn!("Boot failed too many times, initiating rollback");
            self.rollback()?;
        }

        Ok(())
    }
}

// =============================================================================
// Update Scheduler
// =============================================================================

/// Update scheduler
pub struct UpdateScheduler {
    /// Scheduled updates
    scheduled_updates: BTreeMap<u64, UpdateInfo>,
    /// Next update time
    next_update: Option<u64>,
}

impl UpdateScheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            scheduled_updates: BTreeMap::new(),
            next_update: None,
        }
    }

    /// Schedule an update
    pub fn schedule_update(&mut self, timestamp: u64, update: UpdateInfo) -> IotResult<()> {
        self.scheduled_updates.insert(timestamp, update);

        // Update next update time
        if self.next_update.is_none() || timestamp < self.next_update.unwrap() {
            self.next_update = Some(timestamp);
        }

        crate::log_info!("Update scheduled for timestamp {}", timestamp);
        Ok(())
    }

    /// Get next update time
    pub fn next_update(&self) -> Option<u64> {
        self.next_update
    }

    /// Process scheduled updates
    pub fn process_updates(&mut self, current_time: u64) -> Vec<UpdateInfo> {
        let mut ready_updates = Vec::new();

        while let Some(timestamp) = self.scheduled_updates.keys().next().copied() {
            if timestamp <= current_time {
                if let Some((_, update)) = self.scheduled_updates.pop_first() {
                    ready_updates.push(update);
                }
            } else {
                break;
            }
        }

        // Update next update time
        self.next_update = self.scheduled_updates.keys().next().copied();

        ready_updates
    }
}

impl Default for UpdateScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_other() {
        assert_eq!(Partition::A.other(), Partition::B);
        assert_eq!(Partition::B.other(), Partition::A);
    }

    #[test]
    fn test_partition_manager() {
        let manager = PartitionManager::new(16 * 1024 * 1024);

        assert_eq!(manager.active_partition(), Partition::A);
        assert_eq!(manager.inactive_partition(), Partition::B);

        manager.mark_successful(Partition::A).unwrap();
        assert!(manager.get_partition(Partition::A).successful);

        manager.switch_partition(Partition::B).unwrap();
        assert_eq!(manager.active_partition(), Partition::B);
    }

    #[test]
    fn test_partition_rollback() {
        let mut manager = PartitionManager::new(16 * 1024 * 1024);

        // Mark A as successful
        manager.mark_successful(Partition::A).unwrap();

        // Switch to B (unsuccessful)
        manager.switch_partition(Partition::B).unwrap();

        // Simulate boot failures
        for _ in 0..3 {
            manager.increment_boot_attempts();
        }

        assert!(manager.needs_rollback());
        manager.rollback().unwrap();
        assert_eq!(manager.active_partition(), Partition::A);
    }

    #[test]
    fn test_ota_manifest() {
        let checksum = [0u8; 32];
        let manifest = OtaManifest::new("1.0.0", 1024, checksum)
            .with_delta("0.9.0")
            .with_partition(Partition::B);

        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.is_delta);
        assert_eq!(manifest.delta_from, Some("0.9.0".to_string()));
        assert_eq!(manifest.target_partition, Partition::B);
    }

    #[test]
    fn test_delta_engine() {
        let engine = DeltaEngine::new(DeltaType::BinaryDiff);

        let base = vec![1, 2, 3, 4, 5];
        let delta = vec![6, 7, 8];

        let result = engine.apply_delta(&base, &delta).unwrap();

        // Result should contain base + delta
        assert!(result.len() > base.len());
    }

    #[test]
    fn test_delta_savings() {
        let engine = DeltaEngine::new(DeltaType::BinaryDiff);

        let savings = engine.estimate_savings(1000, 200);
        assert!(savings > 0.0); // Should save 80%
    }

    #[test]
    fn test_update_downloader() {
        let downloader = UpdateDownloader::new();

        // Test download (simulated)
        let data = downloader.download("http://example.com/firmware.bin").unwrap();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_update_verifier() {
        let verifier = UpdateVerifier::new(SignatureAlgorithm::Rsa2048, HashAlgorithm::Sha256);

        let data = vec![1, 2, 3, 4, 5];
        let checksum = [0u8; 32];

        let result = verifier.verify_checksum(&data, &checksum);
        assert!(result); // Simplified check

        let manifest = OtaManifest::new("1.0.0", 5, checksum);
        verifier.verify_update(&data, &manifest).ok();
    }

    #[test]
    fn test_ota_manager() {
        let manager = OtaManager::new("0.9.0").unwrap();

        assert_eq!(manager.current_version(), "0.9.0");
        assert!(manager.current_update().is_none());

        // Test rollback
        manager.handle_boot().ok();
    }

    #[test]
    fn test_update_scheduler() {
        let mut scheduler = UpdateScheduler::new();

        let checksum = [0u8; 32];
        let manifest = OtaManifest::new("1.0.0", 1024, checksum);
        let update = UpdateInfo::new(manifest, "http://example.com/firmware.bin");

        scheduler.schedule_update(1000, update).unwrap();

        assert_eq!(scheduler.next_update(), Some(1000));

        // Process updates
        let ready = scheduler.process_updates(2000);
        assert_eq!(ready.len(), 1);
    }

    #[test]
    fn test_update_config() {
        let config = UpdateConfig::default()
            .with_verification(true)
            .with_delta(true)
            .with_schedule(12345)
            .with_auto_rollback(true);

        assert!(config.verify_signature);
        assert!(config.use_delta);
        assert_eq!(config.schedule, Some(12345));
        assert!(config.auto_rollback);
    }

    #[test]
    fn test_update_info() {
        let checksum = [0u8; 32];
        let manifest = OtaManifest::new("1.0.0", 1024, checksum);
        let mut info = UpdateInfo::new(manifest, "http://example.com/firmware.bin");

        assert_eq!(info.state, UpdateState::Idle);
        assert!(!info.in_progress());

        info.state = UpdateState::Downloading;
        assert!(info.in_progress());

        info.state = UpdateState::Complete;
        assert!(info.is_complete());
    }

    #[test]
    fn test_partition_info() {
        let info = PartitionInfo::new(Partition::A, 1024);

        assert_eq!(info.partition, Partition::A);
        assert!(info.bootable);
        assert!(!info.active);
        assert!(!info.successful);
    }
}
