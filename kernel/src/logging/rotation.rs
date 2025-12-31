//! Log rotation policies and implementations
//!
//! This module provides various log rotation strategies:
//! - Size-based rotation (when log file reaches a certain size)
//! - Time-based rotation (daily, hourly, etc.)
//! - Signal-based rotation (on SIGHUP)
//! - Compression of rotated logs
//! - Retention policies (keep N files, keep N days)

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::time::Duration;

use crate::logging::error::Result;

/// Rotation policy trait
pub trait RotationPolicy: Send + Sync {
    /// Check if rotation should occur
    fn should_rotate(&self, path: &str, current_size: usize) -> bool;

    /// Perform the rotation
    fn rotate(&self, path: &str) -> Result<()>;

    /// Get the policy name
    fn name(&self) -> &str {
        "default"
    }
}

/// Size-based rotation policy
pub struct SizeBasedRotation {
    /// Maximum file size before rotation
    max_size: usize,
    /// Number of backup files to keep
    backup_count: usize,
    /// Compress rotated files
    compress: bool,
    /// Compression level (0-9)
    compression_level: u32,
}

impl SizeBasedRotation {
    /// Create a new size-based rotation policy
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            backup_count: 5,
            compress: false,
            compression_level: 6,
        }
    }

    /// Set the number of backup files to keep
    pub fn with_backup_count(mut self, count: usize) -> Self {
        self.backup_count = count;
        self
    }

    /// Enable compression of rotated files
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.compress = enable;
        self
    }

    /// Set compression level (0-9)
    pub fn with_compression_level(mut self, level: u32) -> Self {
        self.compression_level = level.min(9);
        self
    }

    /// Get the backup file path for a given index
    fn backup_path(&self, path: &str, index: usize) -> String {
        if index == 0 {
            format!("{}.1", path)
        } else {
            format!("{}.{}", path, index + 1)
        }
    }

    /// Rotate backup files (increment indices)
    fn rotate_backups(&self, path: &str) -> Result<()> {
        // Delete the oldest backup if we're at the limit
        let oldest_backup = self.backup_path(path, self.backup_count - 1);
        self.delete_file(&oldest_backup)?;

        // Rotate existing backups
        for i in (1..self.backup_count).rev() {
            let old_path = self.backup_path(path, i - 1);
            let new_path = self.backup_path(path, i);
            let _ = self.rename_file(&old_path, &new_path);
        }

        Ok(())
    }

    /// Delete a file (simulated in no_std environment)
    fn delete_file(&self, _path: &str) -> Result<()> {
        // In a real implementation, this would delete the file
        Ok(())
    }

    /// Rename a file (simulated in no_std environment)
    fn rename_file(&self, _old: &str, _new: &str) -> Result<()> {
        // In a real implementation, this would rename the file
        Ok(())
    }

    /// Compress a file (simulated in no_std environment)
    fn compress_file(&self, _path: &str) -> Result<()> {
        // In a real implementation, this would compress the file
        Ok(())
    }
}

impl RotationPolicy for SizeBasedRotation {
    fn should_rotate(&self, _path: &str, current_size: usize) -> bool {
        current_size >= self.max_size
    }

    fn rotate(&self, path: &str) -> Result<()> {
        // Rotate existing backups
        self.rotate_backups(path)?;

        // Move current file to .1
        let backup = self.backup_path(path, 0);
        self.rename_file(path, &backup)?;

        // Compress if enabled
        if self.compress {
            self.compress_file(&backup)?;
        }

        // Create new log file
        self.create_file(path)?;

        Ok(())
    }

    fn name(&self) -> &str {
        "size_based"
    }
}

impl SizeBasedRotation {
    /// Create a new file (simulated in no_std environment)
    fn create_file(&self, _path: &str) -> Result<()> {
        // In a real implementation, this would create the file
        Ok(())
    }
}

/// Time period for time-based rotation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimePeriod {
    /// Rotate every minute
    Minutely,
    /// Rotate every hour
    Hourly,
    /// Rotate every day at midnight
    Daily,
    /// Rotate every week
    Weekly,
    /// Rotate every month
    Monthly,
}

/// Time-based rotation policy
pub struct TimeBasedRotation {
    /// Time period for rotation
    period: TimePeriod,
    /// Number of backup files to keep
    backup_count: usize,
    /// Compress rotated files
    compress: bool,
    /// Last rotation timestamp (nanoseconds)
    last_rotation: Option<u64>,
    /// Custom rotation time (for daily rotation)
    rotation_hour: Option<u8>,
    rotation_minute: Option<u8>,
}

impl TimeBasedRotation {
    /// Create a new time-based rotation policy
    pub fn new(period: TimePeriod) -> Self {
        Self {
            period,
            backup_count: 7, // Keep 7 days by default
            compress: true,
            last_rotation: None,
            rotation_hour: None,
            rotation_minute: None,
        }
    }

    /// Set the number of backup files to keep
    pub fn with_backup_count(mut self, count: usize) -> Self {
        self.backup_count = count;
        self
    }

    /// Enable compression
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.compress = enable;
        self
    }

    /// Set custom rotation time (for daily rotation)
    pub fn with_rotation_time(mut self, hour: u8, minute: u8) -> Self {
        self.rotation_hour = Some(hour);
        self.rotation_minute = Some(minute);
        self
    }

    /// Get the current timestamp
    fn current_timestamp() -> u64 {
        // In a real implementation, this would get the actual time
        // For now, return a dummy value
        0
    }

    /// Calculate the next rotation time
    fn next_rotation_time(&self, current: u64) -> u64 {
        let nanos_per_second = 1_000_000_000u64;
        let nanos_per_minute = 60 * nanos_per_second;
        let nanos_per_hour = 60 * nanos_per_minute;
        let nanos_per_day = 24 * nanos_per_hour;

        match self.period {
            TimePeriod::Minutely => {
                let current_minutes = current / nanos_per_minute;
                (current_minutes + 1) * nanos_per_minute
            }
            TimePeriod::Hourly => {
                let current_hours = current / nanos_per_hour;
                (current_hours + 1) * nanos_per_hour
            }
            TimePeriod::Daily => {
                // If custom time is set, rotate at that time
                if let Some(hour) = self.rotation_hour {
                    let minute = self.rotation_minute.unwrap_or(0);
                    let target_nanos = (hour as u64 * 3600 + minute as u64 * 60) * nanos_per_second;
                    let current_day = current / nanos_per_day * nanos_per_day;
                    let next_rotation = current_day + target_nanos;

                    if next_rotation > current {
                        next_rotation
                    } else {
                        next_rotation + nanos_per_day
                    }
                } else {
                    // Rotate at midnight
                    let current_day = current / nanos_per_day * nanos_per_day;
                    current_day + nanos_per_day
                }
            }
            TimePeriod::Weekly => {
                let current_week = current / (7 * nanos_per_day) * (7 * nanos_per_day);
                current_week + 7 * nanos_per_day
            }
            TimePeriod::Monthly => {
                // Approximate month as 30 days
                let current_month = current / (30 * nanos_per_day) * (30 * nanos_per_day);
                current_month + 30 * nanos_per_day
            }
        }
    }

    /// Check if it's time to rotate
    fn should_rotate_now(&self) -> bool {
        let current = Self::current_timestamp();

        if let Some(last) = self.last_rotation {
            current >= self.next_rotation_time(last)
        } else {
            true
        }
    }

    /// Update last rotation time
    fn update_rotation_time(&mut self) {
        self.last_rotation = Some(Self::current_timestamp());
    }

    /// Generate filename with timestamp
    fn timestamped_filename(&self, path: &str, timestamp: u64) -> String {
        // Format timestamp as YYYYMMDD-HHMMSS
        let secs = timestamp / 1_000_000_000;
        let days = secs / 86_400;
        let _year = 1970 + days / 365;
        let _day_of_year = (days % 365) as u32;
        // Simplified calculation
        format!("{}.{}", path, timestamp)
    }
}

impl RotationPolicy for TimeBasedRotation {
    fn should_rotate(&self, _path: &str, _current_size: usize) -> bool {
        // Ignore size for time-based rotation
        // Use interior mutability for last_rotation
        // For simplicity in no_std, return false here
        // Actual rotation logic would use interior mutability
        false
    }

    fn rotate(&self, path: &str) -> Result<()> {
        let timestamp = Self::current_timestamp();
        let rotated_path = self.timestamped_filename(path, timestamp);

        // Rename current file to timestamped version
        self.rename_file(path, &rotated_path)?;

        // Compress if enabled
        if self.compress {
            self.compress_file(&rotated_path)?;
        }

        // Create new log file
        self.create_file(path)?;

        // Clean up old backups
        self.cleanup_old_backups(path)?;

        Ok(())
    }

    fn name(&self) -> &str {
        "time_based"
    }
}

impl TimeBasedRotation {
    fn rename_file(&self, _old: &str, _new: &str) -> Result<()> {
        Ok(())
    }

    fn compress_file(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    fn create_file(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    fn cleanup_old_backups(&self, _path: &str) -> Result<()> {
        Ok(())
    }
}

/// Signal-based rotation policy
pub struct SignalBasedRotation {
    /// Number of backup files to keep
    backup_count: usize,
    /// Compress rotated files
    compress: bool,
    /// Signal number to trigger rotation
    signal_number: u32,
}

impl SignalBasedRotation {
    /// Create a new signal-based rotation policy
    pub fn new(signal_number: u32) -> Self {
        Self {
            backup_count: 5,
            compress: true,
            signal_number,
        }
    }

    /// Set the number of backup files to keep
    pub fn with_backup_count(mut self, count: usize) -> Self {
        self.backup_count = count;
        self
    }

    /// Enable compression
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.compress = enable;
        self
    }

    /// Handle the rotation signal
    pub fn handle_signal(&self, path: &str) -> Result<()> {
        self.rotate(path)
    }
}

impl RotationPolicy for SignalBasedRotation {
    fn should_rotate(&self, _path: &str, _current_size: usize) -> bool {
        // Signal-based rotation never triggers automatically
        false
    }

    fn rotate(&self, path: &str) -> Result<()> {
        // Similar to size-based rotation
        let timestamp = Self::current_timestamp();
        let rotated_path = format!("{}.{}", path, timestamp);

        self.rename_file(path, &rotated_path)?;

        if self.compress {
            self.compress_file(&rotated_path)?;
        }

        self.create_file(path)?;

        self.cleanup_old_backups(path)?;

        Ok(())
    }

    fn name(&self) -> &str {
        "signal_based"
    }
}

impl SignalBasedRotation {
    fn current_timestamp() -> u64 {
        0
    }

    fn rename_file(&self, _old: &str, _new: &str) -> Result<()> {
        Ok(())
    }

    fn compress_file(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    fn create_file(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    fn cleanup_old_backups(&self, _path: &str) -> Result<()> {
        Ok(())
    }
}

/// Retention policy for cleaning up old logs
pub trait RetentionPolicy: Send + Sync {
    /// Check if a file should be deleted
    fn should_delete(&self, path: &str, age: Duration, size: usize) -> bool;

    /// Get the policy name
    fn name(&self) -> &str {
        "default"
    }
}

/// Keep N most recent files
pub struct KeepFilesPolicy {
    /// Number of files to keep
    count: usize,
}

impl KeepFilesPolicy {
    /// Create a new keep-files policy
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

impl RetentionPolicy for KeepFilesPolicy {
    fn should_delete(&self, _path: &str, _age: Duration, _size: usize) -> bool {
        // This would be implemented by the caller maintaining a count
        false
    }

    fn name(&self) -> &str {
        "keep_files"
    }
}

/// Keep files from the last N days
pub struct KeepDaysPolicy {
    /// Number of days to keep
    days: u64,
}

impl KeepDaysPolicy {
    /// Create a new keep-days policy
    pub fn new(days: u64) -> Self {
        Self { days }
    }
}

impl RetentionPolicy for KeepDaysPolicy {
    fn should_delete(&self, _path: &str, age: Duration, _size: usize) -> bool {
        let nanos_per_day = 86_400_000_000_000u64;
        age.as_nanos() as u64 > self.days * nanos_per_day
    }

    fn name(&self) -> &str {
        "keep_days"
    }
}

/// Keep files based on total size
pub struct KeepSizePolicy {
    /// Maximum total size in bytes
    max_size: usize,
}

impl KeepSizePolicy {
    /// Create a new keep-size policy
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }
}

impl RetentionPolicy for KeepSizePolicy {
    fn should_delete(&self, _path: &str, _age: Duration, size: usize) -> bool {
        // This would be implemented by the caller tracking total size
        size > self.max_size
    }

    fn name(&self) -> &str {
        "keep_size"
    }
}

/// Composite rotation policy that combines multiple policies
pub struct CompositeRotationPolicy {
    /// Individual policies
    policies: Vec<Box<dyn RotationPolicy>>,
    /// Retention policy
    retention_policy: Option<Box<dyn RetentionPolicy>>,
}

impl CompositeRotationPolicy {
    /// Create a new composite policy
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            retention_policy: None,
        }
    }

    /// Add a rotation policy
    pub fn add_policy(mut self, policy: Box<dyn RotationPolicy>) -> Self {
        self.policies.push(policy);
        self
    }

    /// Set retention policy
    pub fn with_retention(mut self, policy: Box<dyn RetentionPolicy>) -> Self {
        self.retention_policy = Some(policy);
        self
    }
}

impl Default for CompositeRotationPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RotationPolicy for CompositeRotationPolicy {
    fn should_rotate(&self, path: &str, current_size: usize) -> bool {
        // Rotate if any policy says to rotate
        self.policies
            .iter()
            .any(|p| p.should_rotate(path, current_size))
    }

    fn rotate(&self, path: &str) -> Result<()> {
        // Apply all rotation policies
        for policy in &self.policies {
            policy.rotate(path)?;
        }

        // Apply retention policy if set
        if let Some(ref retention) = self.retention_policy {
            self.apply_retention(path, retention.as_ref())?;
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "composite"
    }
}

impl CompositeRotationPolicy {
    /// Apply retention policy to clean up old logs
    fn apply_retention(&self, _path: &str, _policy: &dyn RetentionPolicy) -> Result<()> {
        // In a real implementation, this would:
        // 1. List all log files
        // 2. Check each file against the retention policy
        // 3. Delete files that should be deleted
        Ok(())
    }
}

/// Compression algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// Gzip compression
    Gzip,
    /// Zlib compression
    Zlib,
    /// LZ4 compression (fast)
    Lz4,
    /// Zstd compression (balanced)
    Zstd,
    /// No compression
    None,
}

/// Compression settings
pub struct CompressionSettings {
    /// Compression algorithm
    algorithm: CompressionAlgorithm,
    /// Compression level (0-9, where applicable)
    level: u32,
}

impl CompressionSettings {
    /// Create new compression settings
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Self {
            algorithm,
            level: 6,
        }
    }

    /// Set compression level
    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level.min(9);
        self
    }

    /// Compress data (simulated)
    pub fn compress(&self, _data: &[u8]) -> Result<Vec<u8>> {
        // In a real implementation, this would compress the data
        Ok(Vec::new())
    }

    /// Decompress data (simulated)
    pub fn decompress(&self, _data: &[u8]) -> Result<Vec<u8>> {
        // In a real implementation, this would decompress the data
        Ok(Vec::new())
    }

    /// Get file extension for compressed files
    pub fn extension(&self) -> &str {
        match self.algorithm {
            CompressionAlgorithm::Gzip => ".gz",
            CompressionAlgorithm::Zlib => ".zlib",
            CompressionAlgorithm::Lz4 => ".lz4",
            CompressionAlgorithm::Zstd => ".zst",
            CompressionAlgorithm::None => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::time::Duration;

    #[test]
    fn test_size_based_rotation() {
        let policy = SizeBasedRotation::new(1024); // 1KB max size

        assert!(!policy.should_rotate("/tmp/test.log", 512));
        assert!(policy.should_rotate("/tmp/test.log", 1024));
        assert!(policy.should_rotate("/tmp/test.log", 2048));
    }

    #[test]
    fn test_size_based_rotation_with_backup_count() {
        let policy = SizeBasedRotation::new(1024).with_backup_count(3);

        assert_eq!(policy.backup_count, 3);
        assert_eq!(policy.backup_path("/tmp/test.log", 0), "/tmp/test.log.1");
        assert_eq!(policy.backup_path("/tmp/test.log", 1), "/tmp/test.log.2");
        assert_eq!(policy.backup_path("/tmp/test.log", 2), "/tmp/test.log.3");
    }

    #[test]
    fn test_time_based_rotation_periods() {
        let hourly = TimeBasedRotation::new(TimePeriod::Hourly);
        let daily = TimeBasedRotation::new(TimePeriod::Daily);
        let weekly = TimeBasedRotation::new(TimePeriod::Weekly);

        // All should be TimeBasedRotation instances
        assert_eq!(hourly.period, TimePeriod::Hourly);
        assert_eq!(daily.period, TimePeriod::Daily);
        assert_eq!(weekly.period, TimePeriod::Weekly);
    }

    #[test]
    fn test_time_based_rotation_with_custom_time() {
        let policy = TimeBasedRotation::new(TimePeriod::Daily)
            .with_rotation_time(3, 30); // 3:30 AM

        assert_eq!(policy.rotation_hour, Some(3));
        assert_eq!(policy.rotation_minute, Some(30));
    }

    #[test]
    fn test_time_based_rotation_next_rotation() {
        let policy = TimeBasedRotation::new(TimePeriod::Hourly);
        let current = 1_640_000_000_000_000_000u64; // Some timestamp

        let next = policy.next_rotation_time(current);
        assert!(next > current);
    }

    #[test]
    fn test_signal_based_rotation() {
        let policy = SignalBasedRotation::new(1); // SIGHUP

        // Should never rotate automatically
        assert!(!policy.should_rotate("/tmp/test.log", 1024));
        assert!(!policy.should_rotate("/tmp/test.log", 9999999));
    }

    #[test]
    fn test_keep_files_policy() {
        let policy = KeepFilesPolicy::new(5);

        assert_eq!(policy.count, 5);
        assert_eq!(policy.name(), "keep_files");
    }

    #[test]
    fn test_keep_days_policy() {
        let policy = KeepDaysPolicy::new(7);

        let old_age = Duration::from_secs(8 * 24 * 3600); // 8 days
        let young_age = Duration::from_secs(3 * 24 * 3600); // 3 days

        assert!(policy.should_delete("", old_age, 0));
        assert!(!policy.should_delete("", young_age, 0));
    }

    #[test]
    fn test_keep_size_policy() {
        let policy = KeepSizePolicy::new(1024); // 1KB max

        assert!(policy.should_delete("", Duration::ZERO, 2048));
        assert!(!policy.should_delete("", Duration::ZERO, 512));
    }

    #[test]
    fn test_composite_rotation_policy() {
        let size_policy = Box::new(SizeBasedRotation::new(1024)) as Box<dyn RotationPolicy>;
        let time_policy = Box::new(TimeBasedRotation::new(TimePeriod::Daily)) as Box<dyn RotationPolicy>;
        let retention = Box::new(KeepDaysPolicy::new(7)) as Box<dyn RetentionPolicy>;

        let composite = CompositeRotationPolicy::new()
            .add_policy(size_policy)
            .add_policy(time_policy)
            .with_retention(retention);

        assert_eq!(composite.policies.len(), 2);
        assert!(composite.retention_policy.is_some());
    }

    #[test]
    fn test_composite_should_rotate() {
        let size_policy = Box::new(SizeBasedRotation::new(1024)) as Box<dyn RotationPolicy>;
        let composite = CompositeRotationPolicy::new()
            .add_policy(size_policy);

        assert!(!composite.should_rotate("/tmp/test.log", 512));
        assert!(composite.should_rotate("/tmp/test.log", 1024));
    }

    #[test]
    fn test_compression_settings() {
        let gzip = CompressionSettings::new(CompressionAlgorithm::Gzip);
        assert_eq!(gzip.extension(), ".gz");

        let lz4 = CompressionSettings::new(CompressionAlgorithm::Lz4);
        assert_eq!(lz4.extension(), ".lz4");

        let zstd = CompressionSettings::new(CompressionAlgorithm::Zstd);
        assert_eq!(zstd.extension(), ".zst");

        let none = CompressionSettings::new(CompressionAlgorithm::None);
        assert_eq!(none.extension(), "");
    }

    #[test]
    fn test_compression_level() {
        let settings = CompressionSettings::new(CompressionAlgorithm::Gzip)
            .with_level(9);

        assert_eq!(settings.level, 9);

        // Level should be clamped to 9
        let settings = settings.with_level(15);
        assert_eq!(settings.level, 9);
    }

    #[test]
    fn test_size_based_rotation_compression() {
        let policy = SizeBasedRotation::new(1024)
            .with_compression(true)
            .with_compression_level(9);

        assert!(policy.compress);
        assert_eq!(policy.compression_level, 9);
    }

    #[test]
    fn test_time_based_rotation_compression() {
        let policy = TimeBasedRotation::new(TimePeriod::Daily)
            .with_compression(true);

        assert!(policy.compress);
    }

    #[test]
    fn test_rotation_policy_names() {
        let size = SizeBasedRotation::new(1024);
        assert_eq!(size.name(), "size_based");

        let time = TimeBasedRotation::new(TimePeriod::Daily);
        assert_eq!(time.name(), "time_based");

        let signal = SignalBasedRotation::new(1);
        assert_eq!(signal.name(), "signal_based");

        let composite = CompositeRotationPolicy::new();
        assert_eq!(composite.name(), "composite");
    }
}
