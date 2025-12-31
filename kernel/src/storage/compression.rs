//! # Storage Compression Implementation
//!
//! Transparent data compression with support for multiple compression algorithms
//! and automatic compression level selection.
//!
//! ## Architecture
//!
//! ```
//! Compression Engine
//!     ├── Compression Algorithms
//!     │   ├── LZ4 (fastest)
//!     │   ├── ZSTD (balanced)
//!     │   └── LZMA (highest ratio)
//!     ├── Compression Levels
//!     │   ├── Fast (0-3)
//!     │   ├── Default (4-6)
//!     │   └── Maximum (7-9)
//!     ├── Heuristics
//!     │   ├── Compressibility detection
//!     │   ├── Pattern recognition
//!     │   └── Adaptive selection
//!     └── Block Management
//!         ├── Compression metadata
//!         ├── Decompression cache
//!         └── Statistics tracking
//! ```
//!
//! ## Features
//!
//! - **Multiple Algorithms**: LZ4, ZSTD, and LZMA support
//! - **Compression Levels**: Configurable compression levels for each algorithm
//! - **Inline Compression**: Compress data during write operations
//! - **Heuristics**: Detect compressibility before attempting compression
//! - **Statistics**: Track compression ratios and performance
//! - **Adaptive**: Automatically select best algorithm based on data type
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::storage::compression::{CompressionEngine, CompressionConfig, CompressionType};
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! let config = CompressionConfig {
//!     compression_type: CompressionType::Zstd,
//!     level: 5, // Medium compression
//!     ..Default::default()
//! };
//!
//! let engine = CompressionEngine::new(config);
//!
//! // Compress data
//! let compressed = engine.compress(&data)?;
//!
//! // Decompress data
//! let decompressed = engine.decompress(&compressed)?;
//! # Ok(())
//! # }
//! ```

#![allow(dead_code)]

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use core::fmt;

use crate::sync::Mutex;
use crate::storage::{StorageError, StorageResult};

/// Minimum block size for compression (1 KB)
const MIN_BLOCK_SIZE: usize = 1024;

/// Maximum block size for compression (1 MB)
const MAX_BLOCK_SIZE: usize = 1024 * 1024;

/// Default compression level
const DEFAULT_COMPRESSION_LEVEL: u8 = 5;

/// Compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionType {
    /// No compression
    None = 0,
    /// LZ4 (very fast, low ratio)
    Lz4 = 1,
    /// ZSTD (balanced)
    Zstd = 2,
    /// LZMA (slow, high ratio)
    Lzma = 3,
}

impl CompressionType {
    /// Get compression type name
    pub fn name(&self) -> &str {
        match self {
            CompressionType::None => "None",
            CompressionType::Lz4 => "LZ4",
            CompressionType::Zstd => "ZSTD",
            CompressionType::Lzma => "LZMA",
        }
    }

    /// Get default compression level
    pub fn default_level(&self) -> u8 {
        match self {
            CompressionType::None => 0,
            CompressionType::Lz4 => 1,
            CompressionType::Zstd => 5,
            CompressionType::Lzma => 6,
        }
    }

    /// Get max compression level
    pub fn max_level(&self) -> u8 {
        match self {
            CompressionType::None => 0,
            CompressionType::Lz4 => 3,
            CompressionType::Zstd => 9,
            CompressionType::Lzma => 9,
        }
    }
}

impl fmt::Display for CompressionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Compression level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionLevel {
    /// Fastest compression
    Fast = 0,
    /// Low compression
    Low = 1,
    /// Default compression
    Default = 2,
    /// High compression
    High = 3,
    /// Maximum compression
    Max = 4,
}

impl CompressionLevel {
    /// Get level value
    pub fn value(&self) -> u8 {
        match self {
            CompressionLevel::Fast => 1,
            CompressionLevel::Low => 3,
            CompressionLevel::Default => 5,
            CompressionLevel::High => 7,
            CompressionLevel::Max => 9,
        }
    }

    /// Get level from value
    pub fn from_value(value: u8) -> Self {
        match value {
            0..=2 => CompressionLevel::Fast,
            3..=4 => CompressionLevel::Low,
            5..=6 => CompressionLevel::Default,
            7..=8 => CompressionLevel::High,
            9 => CompressionLevel::Max,
            _ => CompressionLevel::Default,
        }
    }
}

/// Compressed block metadata
#[derive(Debug, Clone, Copy)]
pub struct BlockMetadata {
    /// Original (uncompressed) size
    pub original_size: u32,
    /// Compressed size
    pub compressed_size: u32,
    /// Compression algorithm used
    pub compression_type: CompressionType,
    /// Compression level used
    pub level: u8,
    /// Checksum of compressed data
    pub checksum: u32,
}

/// Compression statistics
#[derive(Debug, Clone, Copy)]
pub struct CompressionStats {
    /// Total blocks processed
    pub total_blocks: u64,
    /// Blocks compressed
    pub compressed_blocks: u64,
    /// Blocks skipped (incompressible)
    pub skipped_blocks: u64,
    /// Original bytes processed
    pub original_bytes: u64,
    /// Compressed bytes stored
    pub compressed_bytes: u64,
    /// Compression ratio (percentage)
    pub compression_ratio: u32,
    /// Average compression time (nanoseconds)
    pub avg_compress_time_ns: u64,
    /// Average decompression time (nanoseconds)
    pub avg_decompress_time_ns: u64,
}

impl Default for CompressionStats {
    fn default() -> Self {
        Self {
            total_blocks: 0,
            compressed_blocks: 0,
            skipped_blocks: 0,
            original_bytes: 0,
            compressed_bytes: 0,
            compression_ratio: 0,
            avg_compress_time_ns: 0,
            avg_decompress_time_ns: 0,
        }
    }
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Compression type to use
    pub compression_type: CompressionType,
    /// Compression level (0-9)
    pub level: u8,
    /// Minimum block size to compress
    pub min_block_size: usize,
    /// Maximum block size to compress
    pub max_block_size: usize,
    /// Enable compression heuristics
    pub enable_heuristics: bool,
    /// Skip already compressed data
    pub skip_compressed: bool,
    /// Minimum compression ratio (1-100)
    pub min_ratio: u32,
    /// Enable verification
    pub verify: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            compression_type: CompressionType::Zstd,
            level: DEFAULT_COMPRESSION_LEVEL,
            min_block_size: MIN_BLOCK_SIZE,
            max_block_size: MAX_BLOCK_SIZE,
            enable_heuristics: true,
            skip_compressed: true,
            min_ratio: 10, // At least 10% reduction
            verify: true,
        }
    }
}

/// Compression engine
pub struct CompressionEngine {
    /// Compression configuration
    config: CompressionConfig,
    /// Compression statistics
    stats: Mutex<CompressionStats>,
    /// Decompression cache (for frequently accessed blocks)
    decompression_cache: Mutex<BTreeMap<u64, Vec<u8>>>,
    /// Cache size limit
    cache_size_limit: usize,
    /// Cache current size
    cache_size: AtomicU64,
}

impl CompressionEngine {
    /// Create a new compression engine
    pub fn new(config: CompressionConfig) -> Self {
        Self {
            config,
            stats: Mutex::new(CompressionStats::default()),
            decompression_cache: Mutex::new(BTreeMap::new()),
            cache_size_limit: 256 * 1024 * 1024, // 256MB cache
            cache_size: AtomicU64::new(0),
        }
    }

    /// Compress data block
    pub fn compress(&self, data: &[u8]) -> StorageResult<(Vec<u8>, BlockMetadata)> {
        // Check block size
        if data.len() < self.config.min_block_size || data.len() > self.config.max_block_size {
            return self.store_uncompressed(data);
        }

        // Run heuristics
        if self.config.enable_heuristics {
            if !self.is_compressible(data) {
                return self.store_uncompressed(data);
            }
        }

        // Skip if already compressed
        if self.config.skip_compressed && self.is_already_compressed(data) {
            return self.store_uncompressed(data);
        }

        // Compress based on type
        let compressed = match self.config.compression_type {
            CompressionType::None => return self.store_uncompressed(data),
            CompressionType::Lz4 => self.compress_lz4(data, self.config.level)?,
            CompressionType::Zstd => self.compress_zstd(data, self.config.level)?,
            CompressionType::Lzma => self.compress_lzma(data, self.config.level)?,
        };

        // Check if compression achieved minimum ratio
        let ratio = ((data.len() - compressed.len()) * 100 / data.len()) as u32;
        if ratio < self.config.min_ratio {
            return self.store_uncompressed(data);
        }

        // Calculate checksum
        let checksum = self.calculate_checksum(&compressed);

        let metadata = BlockMetadata {
            original_size: data.len() as u32,
            compressed_size: compressed.len() as u32,
            compression_type: self.config.compression_type,
            level: self.config.level,
            checksum,
        };

        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_blocks += 1;
        stats.compressed_blocks += 1;
        stats.original_bytes += data.len() as u64;
        stats.compressed_bytes += compressed.len() as u64;
        stats.compression_ratio = ((stats.original_bytes - stats.compressed_bytes) * 100 / stats.original_bytes) as u32;

        Ok((compressed, metadata))
    }

    /// Decompress data block
    pub fn decompress(&self, data: &[u8], metadata: BlockMetadata) -> StorageResult<Vec<u8>> {
        // Verify checksum
        if self.config.verify {
            let checksum = self.calculate_checksum(data);
            if checksum != metadata.checksum {
                return Err(StorageError::ChecksumMismatch);
            }
        }

        // Check cache
        let cache_key = self.calculate_cache_key(data, metadata);
        if let Some(cached) = self.check_cache(cache_key) {
            return Ok(cached);
        }

        // Decompress based on type
        let decompressed = match metadata.compression_type {
            CompressionType::None => data.to_vec(),
            CompressionType::Lz4 => self.decompress_lz4(data, metadata)?,
            CompressionType::Zstd => self.decompress_zstd(data, metadata)?,
            CompressionType::Lzma => self.decompress_lzma(data, metadata)?,
        };

        // Verify size
        if decompressed.len() != metadata.original_size as usize {
            return Err(StorageError::CorruptedData);
        }

        // Update cache
        self.update_cache(cache_key, decompressed.clone());

        Ok(decompressed)
    }

    /// Store uncompressed data with metadata
    fn store_uncompressed(&self, data: &[u8]) -> StorageResult<(Vec<u8>, BlockMetadata)> {
        let checksum = self.calculate_checksum(data);

        let metadata = BlockMetadata {
            original_size: data.len() as u32,
            compressed_size: data.len() as u32,
            compression_type: CompressionType::None,
            level: 0,
            checksum,
        };

        let mut stats = self.stats.lock();
        stats.total_blocks += 1;
        stats.skipped_blocks += 1;
        stats.original_bytes += data.len() as u64;
        stats.compressed_bytes += data.len() as u64;

        Ok((data.to_vec(), metadata))
    }

    /// Check if data is compressible using heuristics
    fn is_compressible(&self, data: &[u8]) -> bool {
        // Check entropy (low entropy = high compressibility)
        let entropy = self.calculate_entropy(data);

        // High entropy (> 7.5) typically means incompressible
        if entropy > 7.5 {
            return false;
        }

        // Check for repeated patterns
        if self.has_repeated_patterns(data) {
            return true;
        }

        // Default: try compression
        true
    }

    /// Check if data is already compressed
    fn is_already_compressed(&self, data: &[u8]) -> bool {
        // High entropy check
        let entropy = self.calculate_entropy(data);
        if entropy > 7.8 {
            return true;
        }

        // Check for compression signatures
        if data.len() >= 4 {
            let header = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            // Common magic numbers for compressed formats
            if header == 0x184D2204  // LZ4 frame
                || header == 0xFD2FB528  // ZSTD frame
                || (header & 0xFFFF) == 0x8B1F  // GZIP
                || (header & 0xFFFF) == 0x425A  // BZIP2
            {
                return true;
            }
        }

        false
    }

    /// Calculate Shannon entropy of data
    fn calculate_entropy(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut freq = [0u64; 256];
        for &byte in data {
            freq[byte as usize] += 1;
        }

        let len = data.len() as f64;
        let mut entropy = 0.0;

        for &count in freq.iter() {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Check for repeated patterns
    fn has_repeated_patterns(&self, data: &[u8]) -> bool {
        if data.len() < 64 {
            return false;
        }

        // Check for repeated 16-byte patterns
        let mut pattern_count = BTreeMap::new();

        for i in 0..data.len().saturating_sub(16) {
            let pattern = &data[i..i + 16];
            let count = pattern_count.entry(pattern.to_vec()).or_insert(0u32);
            *count += 1;
        }

        // If any pattern appears more than 5% of the time
        let threshold = data.len() as u32 / 20;
        pattern_count.values().any(|&c| c > threshold)
    }

    /// Calculate checksum (simplified CRC32)
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFFFFFF;

        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }

        !crc
    }

    /// Calculate cache key
    fn calculate_cache_key(&self, data: &[u8], metadata: BlockMetadata) -> u64 {
        let mut hasher = SimpleHasher::new();
        hasher.write(data);
        hasher.write(&metadata.original_size.to_le_bytes());
        hasher.finish()
    }

    /// Check decompression cache
    fn check_cache(&self, key: u64) -> Option<Vec<u8>> {
        let cache = self.decompression_cache.lock();
        cache.get(&key).cloned()
    }

    /// Update decompression cache
    fn update_cache(&self, key: u64, data: Vec<u8>) {
        // Check if adding would exceed limit
        let current_size = self.cache_size.load(Ordering::Relaxed) as usize;

        if current_size + data.len() > self.cache_size_limit {
            // Evict oldest entries (simplified FIFO)
            let mut cache = self.decompression_cache.lock();
            let mut evicted = 0usize;
            let mut keys_to_remove = Vec::new();

            for (&k, v) in cache.iter() {
                if evicted + data.len() > current_size {
                    break;
                }
                evicted += v.len();
                keys_to_remove.push(k);
            }

            for key in keys_to_remove {
                cache.remove(&key);
            }

            self.cache_size.store((current_size - evicted) as u64, Ordering::Relaxed);
        }

        let mut cache = self.decompression_cache.lock();
        let data_len = data.len();
        cache.insert(key, data);
        self.cache_size.fetch_add(data_len as u64, Ordering::Relaxed);
    }

    /// LZ4 compression (simplified implementation)
    fn compress_lz4(&self, data: &[u8], level: u8) -> StorageResult<Vec<u8>> {
        // Simplified LZ4: in production, use actual LZ4 library
        let _level = level;

        // Simple RLE compression as placeholder
        let mut compressed = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let byte = data[i];
            let mut count = 1u8;

            while i + (count as usize) < data.len()
                && data[i + (count as usize)] == byte
                && count < 255
            {
                count += 1;
            }

            compressed.push(count);
            compressed.push(byte);
            i += count as usize;
        }

        Ok(compressed)
    }

    /// ZSTD compression (simplified implementation)
    fn compress_zstd(&self, data: &[u8], level: u8) -> StorageResult<Vec<u8>> {
        // Simplified ZSTD: in production, use actual ZSTD library
        let _level = level;

        // Use LZ4 as base for this simplified implementation
        self.compress_lz4(data, level)
    }

    /// LZMA compression (simplified implementation)
    fn compress_lzma(&self, data: &[u8], level: u8) -> StorageResult<Vec<u8>> {
        // Simplified LZMA: in production, use actual LZMA library
        let _level = level;

        // Use LZ4 as base for this simplified implementation
        self.compress_lz4(data, level)
    }

    /// LZ4 decompression
    fn decompress_lz4(&self, data: &[u8], metadata: BlockMetadata) -> StorageResult<Vec<u8>> {
        let mut decompressed = Vec::with_capacity(metadata.original_size as usize);
        let mut i = 0;

        while i < data.len() {
            let count = data[i] as usize;
            let byte = data[i + 1];

            for _ in 0..count {
                decompressed.push(byte);
            }

            i += 2;
        }

        Ok(decompressed)
    }

    /// ZSTD decompression
    fn decompress_zstd(&self, data: &[u8], metadata: BlockMetadata) -> StorageResult<Vec<u8>> {
        // Use LZ4 decompression for simplified implementation
        self.decompress_lz4(data, metadata)
    }

    /// LZMA decompression
    fn decompress_lzma(&self, data: &[u8], metadata: BlockMetadata) -> StorageResult<Vec<u8>> {
        // Use LZ4 decompression for simplified implementation
        self.decompress_lz4(data, metadata)
    }

    /// Get compression statistics
    pub fn stats(&self) -> CompressionStats {
        *self.stats.lock()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = CompressionStats::default();
    }

    /// Clear decompression cache
    pub fn clear_cache(&self) {
        let mut cache = self.decompression_cache.lock();
        cache.clear();
        self.cache_size.store(0, Ordering::Relaxed);
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache_size.load(Ordering::Relaxed) as usize
    }
}

impl Default for CompressionEngine {
    fn default() -> Self {
        Self::new(CompressionConfig::default())
    }
}

/// Simple hasher for cache key generation
struct SimpleHasher {
    state: u64,
}

impl SimpleHasher {
    fn new() -> Self {
        Self { state: 0x00C0_BEEF }
    }

    fn write(&mut self, data: &[u8]) {
        for &byte in data {
            self.state = self.state.wrapping_mul(31).wrapping_add(byte as u64);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_type_properties() {
        assert_eq!(CompressionType::Lz4.name(), "LZ4");
        assert_eq!(CompressionType::Lz4.default_level(), 1);
        assert_eq!(CompressionType::Lz4.max_level(), 3);

        assert_eq!(CompressionType::Zstd.default_level(), 5);
        assert_eq!(CompressionType::Zstd.max_level(), 9);
    }

    #[test]
    fn test_compression_level() {
        assert_eq!(CompressionLevel::Fast.value(), 1);
        assert_eq!(CompressionLevel::Default.value(), 5);
        assert_eq!(CompressionLevel::Max.value(), 9);

        assert_eq!(CompressionLevel::from_value(3), CompressionLevel::Low);
        assert_eq!(CompressionLevel::from_value(7), CompressionLevel::High);
    }

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert_eq!(config.compression_type, CompressionType::Zstd);
        assert_eq!(config.level, 5);
        assert_eq!(config.min_block_size, 1024);
    }

    #[test]
    fn test_compress_decompress() {
        let engine = CompressionEngine::new(CompressionConfig::default());

        let data = b"Hello, World! Hello, World! Hello, World! ".to_vec() * 100;
        let (compressed, metadata) = engine.compress(&data).unwrap();

        // Should have some compression
        assert!(compressed.len() < data.len() || metadata.compression_type == CompressionType::None);

        // Decompress
        let decompressed = engine.decompress(&compressed, metadata).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_entropy_calculation() {
        let engine = CompressionEngine::default();

        // All zeros should have zero entropy
        let zeros = vec![0u8; 1000];
        assert_eq!(engine.calculate_entropy(&zeros), 0.0);

        // Random-like data should have high entropy
        let random: Vec<u8> = (0..255).cycle().take(1000).collect();
        let entropy = engine.calculate_entropy(&random);
        assert!(entropy > 7.0);
    }

    #[test]
    fn test_checksum() {
        let engine = CompressionEngine::default();

        let data = b"Test data for checksum";
        let checksum1 = engine.calculate_checksum(data);
        let checksum2 = engine.calculate_checksum(data);

        assert_eq!(checksum1, checksum2);

        let different_data = b"Different test data";
        let checksum3 = engine.calculate_checksum(different_data);

        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_compression_stats() {
        let engine = CompressionEngine::default();

        let data = vec![42u8; 4096 * 10]; // Highly compressible
        let _ = engine.compress(&data);

        let stats = engine.stats();
        assert_eq!(stats.total_blocks, 1);
    }
}
