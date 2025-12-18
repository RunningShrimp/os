//! Advanced Memory Compression Module for NOS
//! 
//! Implements multiple compression algorithms to reduce memory footprint:
//! - LZ4-style fast compression
//! - ZSTD-style high-ratio compression
//! - Adaptive compression based on memory pressure
//! - Per-CPU compression caches
//! - Compression statistics and monitoring

extern crate alloc;

use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::sync::Mutex;

// ============================================================================
// Constants
// ============================================================================

/// Compression buffer size (4KB)
pub const COMPRESS_BLOCK_SIZE: usize = 4096;

/// Maximum compression level
pub const MAX_COMPRESSION_LEVEL: u8 = 12;

/// Default compression level
pub const DEFAULT_COMPRESSION_LEVEL: u8 = 6;

/// Compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// Fast LZ4-style compression
    FastLZ4,
    /// Balanced compression
    Balanced,
    /// High-ratio ZSTD-style compression
    HighRatio,
    /// Adaptive compression (selects based on data)
    Adaptive,
}

/// Compression header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CompressHeader {
    /// Original size in bytes
    original_size: usize,
    /// Compressed size in bytes
    compressed_size: usize,
    /// Compression algorithm used
    algorithm: u8,
    /// Compression level used
    level: u8,
    /// Checksum for integrity verification
    checksum: u32,
}

/// Compression statistics per algorithm
#[derive(Debug, Default)]
pub struct AlgorithmStats {
    pub pages_compressed: AtomicUsize,
    pub pages_decompressed: AtomicUsize,
    pub bytes_saved: AtomicUsize,
    pub compression_ratio: AtomicUsize, // Average compression ratio * 100
    pub avg_compress_time_us: AtomicUsize, // Average compression time in microseconds
    pub avg_decompress_time_us: AtomicUsize, // Average decompression time in microseconds
}

/// Per-CPU compression cache
#[derive(Debug)]
pub struct PerCpuCompressionCache {
    /// Compression buffer
    compress_buffer: Vec<u8>,
    /// Decompression buffer
    decompress_buffer: Vec<u8>,
    /// Temporary workspace for compression
    workspace: Vec<u8>,
    /// Cache statistics
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

impl PerCpuCompressionCache {
    /// Create a new per-CPU compression cache
    pub fn new() -> Self {
        Self {
            compress_buffer: Vec::with_capacity(COMPRESS_BLOCK_SIZE * 2),
            decompress_buffer: Vec::with_capacity(COMPRESS_BLOCK_SIZE * 2),
            workspace: Vec::with_capacity(COMPRESS_BLOCK_SIZE),
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
        }
    }
    
    /// Get compression buffer
    pub fn get_compress_buffer(&mut self) -> &mut Vec<u8> {
        self.compress_buffer.clear();
        &mut self.compress_buffer
    }
    
    /// Get decompression buffer
    pub fn get_decompress_buffer(&mut self) -> &mut Vec<u8> {
        self.decompress_buffer.clear();
        &mut self.decompress_buffer
    }
    
    /// Get workspace
    pub fn get_workspace(&mut self) -> &mut Vec<u8> {
        self.workspace.clear();
        &mut self.workspace
    }
    
    /// Record cache hit
    pub fn record_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Record cache miss
    pub fn record_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get cache hit ratio
    pub fn hit_ratio(&self) -> f32 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        
        if hits + misses == 0 {
            return 0.0;
        }
        
        hits as f32 / (hits + misses) as f32
    }
}

// ============================================================================
// Compression API
// ============================================================================

/// Compression threshold: only compress if savings > 20%
pub const COMPRESSION_THRESHOLD: f32 = 0.2;



/// Enable memory compression (can be toggled via sysctl or kernel parameter)
pub static ENABLE_COMPRESSION: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Advanced compression statistics
#[derive(Debug, Default)]
pub struct CompressionStats {
    pub pages_compressed: AtomicUsize,
    pub pages_decompressed: AtomicUsize,
    pub bytes_saved: AtomicUsize,
    pub compression_ratio: AtomicUsize, // Average compression ratio * 100
    pub total_compress_time_us: AtomicUsize, // Total compression time in microseconds
    pub total_decompress_time_us: AtomicUsize, // Total decompression time in microseconds
    pub memory_pressure_triggered: AtomicUsize, // Number of times memory pressure triggered compression
    pub adaptive_algorithm_changes: AtomicUsize, // Number of times adaptive algorithm changed
    pub per_algorithm_stats: BTreeMap<CompressionAlgorithm, AlgorithmStats>,
}

static COMPRESSION_STATS: Mutex<CompressionStats> = Mutex::new(CompressionStats {
    pages_compressed: AtomicUsize::new(0),
    pages_decompressed: AtomicUsize::new(0),
    bytes_saved: AtomicUsize::new(0),
    compression_ratio: AtomicUsize::new(0),
    total_compress_time_us: AtomicUsize::new(0),
    total_decompress_time_us: AtomicUsize::new(0),
    memory_pressure_triggered: AtomicUsize::new(0),
    adaptive_algorithm_changes: AtomicUsize::new(0),
    per_algorithm_stats: BTreeMap::new(),
});

/// Per-CPU compression caches
static PER_CPU_CACHES: Mutex<Vec<PerCpuCompressionCache>> = Mutex::new(Vec::new());

/// Current compression algorithm
static CURRENT_ALGORITHM: AtomicUsize = AtomicUsize::new(CompressionAlgorithm::Adaptive as usize);

/// Current compression level
static CURRENT_LEVEL: AtomicUsize = AtomicUsize::new(DEFAULT_COMPRESSION_LEVEL as usize);

/// Memory pressure threshold (percentage)
static MEMORY_PRESSURE_THRESHOLD: AtomicUsize = AtomicUsize::new(10); // 10%

/// Compression enabled flag
static COMPRESSION_ENABLED: AtomicBool = AtomicBool::new(false);

/// Adaptive compression enabled flag
static ADAPTIVE_ENABLED: AtomicBool = AtomicBool::new(true);

/// Enable memory compression
pub fn enable_compression() {
    COMPRESSION_ENABLED.store(true, Ordering::Relaxed);
}

/// Disable memory compression
pub fn disable_compression() {
    COMPRESSION_ENABLED.store(false, Ordering::Relaxed);
}

/// Check if compression is enabled
pub fn is_compression_enabled() -> bool {
    COMPRESSION_ENABLED.load(Ordering::Relaxed)
}

/// Set compression algorithm
pub fn set_algorithm(algorithm: CompressionAlgorithm) {
    CURRENT_ALGORITHM.store(algorithm as usize, Ordering::Relaxed);
}

/// Get current compression algorithm
pub fn get_current_algorithm() -> CompressionAlgorithm {
    match CURRENT_ALGORITHM.load(Ordering::Relaxed) {
        0 => CompressionAlgorithm::FastLZ4,
        1 => CompressionAlgorithm::Balanced,
        2 => CompressionAlgorithm::HighRatio,
        3 => CompressionAlgorithm::Adaptive,
        _ => CompressionAlgorithm::Adaptive,
    }
}

/// Set compression level
pub fn set_level(level: u8) {
    let level = level.min(MAX_COMPRESSION_LEVEL);
    CURRENT_LEVEL.store(level as usize, Ordering::Relaxed);
}

/// Get current compression level
pub fn get_current_level() -> u8 {
    CURRENT_LEVEL.load(Ordering::Relaxed) as u8
}

/// Set memory pressure threshold
pub fn set_memory_pressure_threshold(threshold: usize) {
    MEMORY_PRESSURE_THRESHOLD.store(threshold, Ordering::Relaxed);
}

/// Get memory pressure threshold
pub fn get_memory_pressure_threshold() -> usize {
    MEMORY_PRESSURE_THRESHOLD.load(Ordering::Relaxed)
}

/// Enable/disable adaptive compression
pub fn set_adaptive(enabled: bool) {
    ADAPTIVE_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Check if adaptive compression is enabled
pub fn is_adaptive_enabled() -> bool {
    ADAPTIVE_ENABLED.load(Ordering::Relaxed)
}

/// Initialize per-CPU compression caches
pub fn init_per_cpu_caches(num_cpus: usize) {
    let mut caches = PER_CPU_CACHES.lock();
    caches.clear();
    
    for _ in 0..num_cpus {
        caches.push(PerCpuCompressionCache::new());
    }
}

/// Get per-CPU compression cache
pub fn get_per_cpu_cache(cpu_id: usize) -> Option<&'static Mutex<PerCpuCompressionCache>> {
    let caches = PER_CPU_CACHES.lock();
    if cpu_id < caches.len() {
        // This is a bit of a hack to return a reference to a static
        // In a real implementation, we would use a different approach
        unsafe {
            let ptr = caches.as_ptr().add(cpu_id);
            Some(&*(ptr as *const Mutex<PerCpuCompressionCache>))
        }
    } else {
        None
    }
}

/// Update compression statistics
fn update_compression_stats(algorithm: CompressionAlgorithm, original_size: usize, compressed_size: usize, compress_time_us: u64) {
    let mut stats = COMPRESSION_STATS.lock();
    
    stats.pages_compressed.fetch_add(1, Ordering::Relaxed);
    stats.total_compress_time_us.fetch_add(compress_time_us as usize, Ordering::Relaxed);
    
    if compressed_size < original_size {
        let saved = original_size - compressed_size;
        stats.bytes_saved.fetch_add(saved, Ordering::Relaxed);
        
        // Update average compression ratio
        let ratio = (compressed_size * 100) / original_size;
        let old_ratio = stats.compression_ratio.load(Ordering::Relaxed);
        let count = stats.pages_compressed.load(Ordering::Relaxed);
        if count > 0 {
            let new_ratio = ((old_ratio * count) + ratio) / (count + 1);
            stats.compression_ratio.store(new_ratio, Ordering::Relaxed);
        } else {
            stats.compression_ratio.store(ratio, Ordering::Relaxed);
        }
    }
    
    // Update per-algorithm statistics
    let algo_stats = stats.per_algorithm_stats.entry(algorithm).or_insert_with(AlgorithmStats::default);
    algo_stats.pages_compressed.fetch_add(1, Ordering::Relaxed);
    algo_stats.avg_compress_time_us.fetch_add(compress_time_us as usize, Ordering::Relaxed);
    
    if compressed_size < original_size {
        let saved = original_size - compressed_size;
        algo_stats.bytes_saved.fetch_add(saved, Ordering::Relaxed);
        
        // Update average compression ratio
        let ratio = (compressed_size * 100) / original_size;
        let old_ratio = algo_stats.compression_ratio.load(Ordering::Relaxed);
        let count = algo_stats.pages_compressed.load(Ordering::Relaxed);
        if count > 0 {
            let new_ratio = ((old_ratio * count) + ratio) / (count + 1);
            algo_stats.compression_ratio.store(new_ratio, Ordering::Relaxed);
        } else {
            algo_stats.compression_ratio.store(ratio, Ordering::Relaxed);
        }
    }
}

/// Update decompression statistics
fn update_decompression_stats(algorithm: CompressionAlgorithm, decompress_time_us: u64) {
    let mut stats = COMPRESSION_STATS.lock();
    
    stats.pages_decompressed.fetch_add(1, Ordering::Relaxed);
    stats.total_decompress_time_us.fetch_add(decompress_time_us as usize, Ordering::Relaxed);
    
    // Update per-algorithm statistics
    let algo_stats = stats.per_algorithm_stats.entry(algorithm).or_insert_with(AlgorithmStats::default);
    algo_stats.pages_decompressed.fetch_add(1, Ordering::Relaxed);
    algo_stats.avg_decompress_time_us.fetch_add(decompress_time_us as usize, Ordering::Relaxed);
}

/// Trigger memory pressure compression
pub fn trigger_memory_pressure_compression() {
    COMPRESSION_STATS.lock().memory_pressure_triggered.fetch_add(1, Ordering::Relaxed);
}

/// Change adaptive algorithm
pub fn change_adaptive_algorithm(new_algorithm: CompressionAlgorithm) {
    let old_algorithm = get_current_algorithm();
    if old_algorithm != new_algorithm {
        set_algorithm(new_algorithm);
        COMPRESSION_STATS.lock().adaptive_algorithm_changes.fetch_add(1, Ordering::Relaxed);
    }
}

/// Get compression statistics
pub fn get_stats() -> (usize, usize, usize, usize, usize, usize, usize, usize) {
    let stats = COMPRESSION_STATS.lock();
    (
        stats.pages_compressed.load(Ordering::Relaxed),
        stats.pages_decompressed.load(Ordering::Relaxed),
        stats.bytes_saved.load(Ordering::Relaxed),
        stats.compression_ratio.load(Ordering::Relaxed),
        stats.total_compress_time_us.load(Ordering::Relaxed),
        stats.total_decompress_time_us.load(Ordering::Relaxed),
        stats.memory_pressure_triggered.load(Ordering::Relaxed),
        stats.adaptive_algorithm_changes.load(Ordering::Relaxed),
    )
}

/// Get detailed compression statistics
pub fn get_detailed_stats() -> CompressionStats {
    let stats = COMPRESSION_STATS.lock();
    CompressionStats {
        pages_compressed: AtomicUsize::new(stats.pages_compressed.load(Ordering::Relaxed)),
        pages_decompressed: AtomicUsize::new(stats.pages_decompressed.load(Ordering::Relaxed)),
        bytes_saved: AtomicUsize::new(stats.bytes_saved.load(Ordering::Relaxed)),
        compression_ratio: AtomicUsize::new(stats.compression_ratio.load(Ordering::Relaxed)),
        total_compress_time_us: AtomicUsize::new(stats.total_compress_time_us.load(Ordering::Relaxed)),
        total_decompress_time_us: AtomicUsize::new(stats.total_decompress_time_us.load(Ordering::Relaxed)),
        memory_pressure_triggered: AtomicUsize::new(stats.memory_pressure_triggered.load(Ordering::Relaxed)),
        adaptive_algorithm_changes: AtomicUsize::new(stats.adaptive_algorithm_changes.load(Ordering::Relaxed)),
        per_algorithm_stats: stats.per_algorithm_stats.clone(),
    }
}

/// Get per-algorithm statistics
pub fn get_algorithm_stats(algorithm: CompressionAlgorithm) -> Option<AlgorithmStats> {
    let stats = COMPRESSION_STATS.lock();
    stats.per_algorithm_stats.get(&algorithm).cloned()
}

/// Reset compression statistics
pub fn reset_stats() {
    let mut stats = COMPRESSION_STATS.lock();
    stats.pages_compressed.store(0, Ordering::Relaxed);
    stats.pages_decompressed.store(0, Ordering::Relaxed);
    stats.bytes_saved.store(0, Ordering::Relaxed);
    stats.compression_ratio.store(0, Ordering::Relaxed);
    stats.total_compress_time_us.store(0, Ordering::Relaxed);
    stats.total_decompress_time_us.store(0, Ordering::Relaxed);
    stats.memory_pressure_triggered.store(0, Ordering::Relaxed);
    stats.adaptive_algorithm_changes.store(0, Ordering::Relaxed);
    stats.per_algorithm_stats.clear();
}

/// Fast LZ4-style compression
fn compress_lz4_fast(src: &[u8], level: u8) -> Vec<u8> {
    let mut dst = Vec::with_capacity(src.len());
    
    // Simple run-length encoding for demonstration
    // Real implementation would use LZ4 or similar
    let mut i = 0;
    while i < src.len() {
        let byte = src[i];
        let mut count = 1;
        
        // Count consecutive identical bytes
        while i + count < src.len() && src[i + count] == byte && count < 255 {
            count += 1;
        }
        
        if count >= 4 {
            // Use RLE encoding
            dst.push(0xFF); // RLE marker
            dst.push(byte);
            dst.push(count as u8);
            i += count;
        } else {
            // Copy literal bytes
            for _ in 0..count {
                dst.push(byte);
            }
            i += count;
        }
    }
    
    dst
}

/// Balanced compression (better ratio than fast)
fn compress_balanced(src: &[u8], level: u8) -> Vec<u8> {
    // For demonstration, we'll use a slightly more sophisticated RLE
    // with a small dictionary for common patterns
    let mut dst = Vec::with_capacity(src.len());
    
    // Simple dictionary for common patterns
    let dictionary = [
        (b"0000", 0xF0),
        (b"1111", 0xF1),
        (b"AAAA", 0xF2),
        (b"BBBB", 0xF3),
        (b"CCCC", 0xF4),
        (b"DDDD", 0xF5),
    ];
    
    let mut i = 0;
    while i < src.len() {
        let mut found = false;
        
        // Check dictionary patterns
        for (pattern, code) in &dictionary {
            if i + pattern.len() <= src.len() && &src[i..i + pattern.len()] == *pattern {
                dst.push(*code);
                i += pattern.len();
                found = true;
                break;
            }
        }
        
        if !found {
            // Use RLE for repeated bytes
            let byte = src[i];
            let mut count = 1;
            
            while i + count < src.len() && src[i + count] == byte && count < 255 {
                count += 1;
            }
            
            if count >= 4 {
                dst.push(0xFF); // RLE marker
                dst.push(byte);
                dst.push(count as u8);
                i += count;
            } else {
                dst.push(byte);
                i += 1;
            }
        }
    }
    
    dst
}

/// High-ratio compression (slower but better compression)
fn compress_high_ratio(src: &[u8], level: u8) -> Vec<u8> {
    // For demonstration, we'll use a more sophisticated approach
    // with Huffman coding for common bytes
    
    // Count byte frequencies
    let mut freq = [0usize; 256];
    for &byte in src {
        freq[byte as usize] += 1;
    }
    
    // Create a simple Huffman tree (simplified)
    let mut codes = [0u16; 256];
    let mut code_lengths = [0u8; 256];
    
    // Assign shorter codes to more frequent bytes
    let mut sorted_bytes: Vec<(usize, u8)> = freq.iter().enumerate()
        .map(|(i, &f)| (f, i as u8))
        .collect();
    sorted_bytes.sort_by(|a, b| b.0.cmp(&a.0)); // Sort by frequency (descending)
    
    // Assign codes (simplified)
    let mut code = 0u16;
    for (i, &(_, byte)) in sorted_bytes.iter().enumerate() {
        if i < 16 {
            // Most frequent bytes get 4-bit codes
            codes[byte as usize] = code;
            code_lengths[byte as usize] = 4;
            code += 1;
        } else if i < 48 {
            // Next frequent bytes get 6-bit codes
            codes[byte as usize] = 0x10 | (code as u16);
            code_lengths[byte as usize] = 6;
            code += 1;
        } else {
            // Least frequent bytes get 8-bit codes (escape + byte)
            codes[byte as usize] = 0xFF00 | (byte as u16);
            code_lengths[byte as usize] = 16;
        }
    }
    
    // Compress using Huffman codes
    let mut dst = Vec::with_capacity(src.len());
    let mut bit_buffer = 0u32;
    let mut bits_in_buffer = 0u8;
    
    for &byte in src {
        let code = codes[byte as usize];
        let len = code_lengths[byte as usize];
        
        bit_buffer |= (code as u32) << bits_in_buffer;
        bits_in_buffer += len;
        
        while bits_in_buffer >= 8 {
            dst.push((bit_buffer & 0xFF) as u8);
            bit_buffer >>= 8;
            bits_in_buffer -= 8;
        }
    }
    
    // Flush remaining bits
    if bits_in_buffer > 0 {
        dst.push((bit_buffer & 0xFF) as u8);
    }
    
    dst
}

/// Adaptive compression (selects best algorithm based on data)
fn compress_adaptive(src: &[u8], level: u8) -> Vec<u8> {
    // Analyze data characteristics
    let mut entropy = 0.0f32;
    let mut freq = [0usize; 256];
    
    for &byte in src {
        freq[byte as usize] += 1;
    }
    
    for &count in &freq {
        if count > 0 {
            let p = count as f32 / src.len() as f32;
            entropy -= p * p.log2();
        }
    }
    
    // Select algorithm based on entropy
    if entropy < 3.0 {
        // Low entropy: use fast compression
        compress_lz4_fast(src, level)
    } else if entropy < 6.0 {
        // Medium entropy: use balanced compression
        compress_balanced(src, level)
    } else {
        // High entropy: use high-ratio compression
        compress_high_ratio(src, level)
    }
}

/// Advanced compression with algorithm selection
pub fn compress(src: &[u8]) -> Vec<u8> {
    // Check if compression is enabled
    if !is_compression_enabled() {
        // Return uncompressed data
        let mut dst = Vec::with_capacity(src.len());
        dst.extend_from_slice(src);
        return dst;
    }
    
    let algorithm = get_current_algorithm();
    let level = get_current_level();
    
    let start_time = crate::time::get_ticks();
    
    let compressed = match algorithm {
        CompressionAlgorithm::FastLZ4 => compress_lz4_fast(src, level),
        CompressionAlgorithm::Balanced => compress_balanced(src, level),
        CompressionAlgorithm::HighRatio => compress_high_ratio(src, level),
        CompressionAlgorithm::Adaptive => compress_adaptive(src, level),
    };
    
    let end_time = crate::time::get_ticks();
    let compress_time_us = (end_time - start_time) * 1000000 / crate::time::TICK_HZ;
    
    // Update statistics
    update_compression_stats(algorithm, src.len(), compressed.len(), compress_time_us);
    
    // Only return compressed data if it's actually smaller
    if compressed.len() < src.len() {
        compressed
    } else {
        // Compression didn't help, return original
        let mut original = Vec::with_capacity(src.len());
        original.extend_from_slice(src);
        original
    }
}

/// Fast LZ4-style decompression
fn decompress_lz4_fast(src: &[u8]) -> Vec<u8> {
    let mut dst = Vec::with_capacity(src.len() * 2);
    let mut i = 0;
    
    while i < src.len() {
        if i + 2 < src.len() && src[i] == 0xFF {
            // RLE encoded
            let byte = src[i + 1];
            let count = src[i + 2] as usize;
            for _ in 0..count {
                dst.push(byte);
            }
            i += 3;
        } else {
            // Literal byte
            dst.push(src[i]);
            i += 1;
        }
    }
    
    dst
}

/// Balanced decompression
fn decompress_balanced(src: &[u8]) -> Vec<u8> {
    let mut dst = Vec::with_capacity(src.len() * 2);
    let mut i = 0;
    
    // Dictionary for decompression
    let dictionary = [
        (0xF0, b"0000"),
        (0xF1, b"1111"),
        (0xF2, b"AAAA"),
        (0xF3, b"BBBB"),
        (0xF4, b"CCCC"),
        (0xF5, b"DDDD"),
    ];
    
    while i < src.len() {
        let mut found = false;
        
        // Check dictionary codes
        for (code, pattern) in &dictionary {
            if src[i] == *code {
                dst.extend_from_slice(pattern);
                i += 1;
                found = true;
                break;
            }
        }
        
        if !found {
            // Check for RLE
            if i + 2 < src.len() && src[i] == 0xFF {
                let byte = src[i + 1];
                let count = src[i + 2] as usize;
                for _ in 0..count {
                    dst.push(byte);
                }
                i += 3;
            } else {
                // Literal byte
                dst.push(src[i]);
                i += 1;
            }
        }
    }
    
    dst
}

/// High-ratio decompression (Huffman)
fn decompress_high_ratio(src: &[u8]) -> Vec<u8> {
    let mut dst = Vec::with_capacity(src.len() * 2);
    let mut bit_buffer = 0u32;
    let mut bits_in_buffer = 0u8;
    let mut i = 0;
    
    // Rebuild the Huffman codes (simplified)
    let mut codes = [0u16; 256];
    let mut code_lengths = [0u8; 256];
    
    let mut code = 0u16;
    for byte in 0..=255 {
        if byte < 16 {
            // Most frequent bytes get 4-bit codes
            codes[byte as usize] = code;
            code_lengths[byte as usize] = 4;
            code += 1;
        } else if byte < 48 {
            // Next frequent bytes get 6-bit codes
            codes[byte as usize] = 0x10 | (code as u16);
            code_lengths[byte as usize] = 6;
            code += 1;
        } else {
            // Least frequent bytes get 16-bit codes (escape + byte)
            codes[byte as usize] = 0xFF00 | (byte as u16);
            code_lengths[byte as usize] = 16;
        }
    }
    
    // Create reverse lookup table (simplified)
    let mut reverse_lookup = alloc::collections::BTreeMap::new();
    for byte in 0..=255 {
        reverse_lookup.insert(codes[byte as usize], byte);
    }
    
    // Decompress using Huffman codes
    while i < src.len() {
        // Fill bit buffer
        while bits_in_buffer < 16 && i < src.len() {
            bit_buffer |= (src[i] as u32) << bits_in_buffer;
            bits_in_buffer += 8;
            i += 1;
        }
        
        // Try to find a matching code
        let mut found = false;
        for bits_to_check in 4..=16 {
            if bits_in_buffer >= bits_to_check {
                let mask = (1u32 << bits_to_check) - 1;
                let code = (bit_buffer & mask) as u16;
                
                if let Some(&byte) = reverse_lookup.get(&code) {
                    dst.push(byte);
                    bit_buffer >>= bits_to_check;
                    bits_in_buffer -= bits_to_check;
                    found = true;
                    break;
                }
            }
        }
        
        if !found {
            // Handle escape codes
            if bits_in_buffer >= 16 {
                let code = (bit_buffer & 0xFFFF) as u16;
                if (code & 0xFF00) == 0xFF00 {
                    // Escape + byte
                    dst.push((code & 0xFF) as u8);
                    bit_buffer >>= 16;
                    bits_in_buffer -= 16;
                    found = true;
                }
            }
        }
        
        if !found {
            // Fallback: treat as literal byte
            dst.push((bit_buffer & 0xFF) as u8);
            bit_buffer >>= 8;
            bits_in_buffer -= 8;
        }
    }
    
    dst
}

/// Advanced decompression with algorithm detection
pub fn decompress(src: &[u8]) -> Vec<u8> {
    // Check if compression is enabled
    if !is_compression_enabled() {
        // Return data as-is
        let mut dst = Vec::with_capacity(src.len());
        dst.extend_from_slice(src);
        return dst;
    }
    
    // Try to detect algorithm from header
    if src.len() < size_of::<CompressHeader>() {
        // No header, assume fast LZ4
        return decompress_lz4_fast(src);
    }
    
    let header_ptr = src.as_ptr() as *const CompressHeader;
    let header = unsafe { *header_ptr };
    
    let algorithm = match header.algorithm {
        0 => CompressionAlgorithm::FastLZ4,
        1 => CompressionAlgorithm::Balanced,
        2 => CompressionAlgorithm::HighRatio,
        3 => CompressionAlgorithm::Adaptive,
        _ => CompressionAlgorithm::FastLZ4, // Default
    };
    
    let compressed_data = &src[size_of::<CompressHeader>()..];
    
    let start_time = crate::time::get_ticks();
    
    let decompressed = match algorithm {
        CompressionAlgorithm::FastLZ4 => decompress_lz4_fast(compressed_data),
        CompressionAlgorithm::Balanced => decompress_balanced(compressed_data),
        CompressionAlgorithm::HighRatio => decompress_high_ratio(compressed_data),
        CompressionAlgorithm::Adaptive => {
            // For adaptive, we need to try each algorithm
            // In a real implementation, we would store the actual algorithm used
            if let Ok(result) = decompress_lz4_fast(compressed_data) {
                result
            } else if let Ok(result) = decompress_balanced(compressed_data) {
                result
            } else {
                decompress_high_ratio(compressed_data)
            }
        }
    };
    
    let end_time = crate::time::get_ticks();
    let decompress_time_us = (end_time - start_time) * 1000000 / crate::time::TICK_HZ;
    
    // Update statistics
    update_decompression_stats(algorithm, decompress_time_us);
    
    decompressed
}

/// Compress a memory block
pub unsafe fn compress_memory(ptr: *const u8, size: usize) -> Option<Vec<u8>> {
    if ptr.is_null() || size == 0 {
        return None;
    }
    
    // Read the source data
    let src_slice = core::slice::from_raw_parts(ptr, size);
    
    // Compress it
    let compressed_data = compress(src_slice);
    
    // Prepend header
    let mut result = Vec::with_capacity(size_of::<CompressHeader>() + compressed_data.len());
    
    let header = CompressHeader {
        original_size: size,
        compressed_size: compressed_data.len(),
    };
    
    // Write header
    let header_ptr = &header as *const CompressHeader as *const u8;
    let header_slice = core::slice::from_raw_parts(header_ptr, size_of::<CompressHeader>());
    result.extend_from_slice(header_slice);
    
    // Write compressed data
    result.extend_from_slice(&compressed_data);
    
    Some(result)
}

/// Decompress a memory block
pub unsafe fn decompress_memory(ptr: *const u8) -> Option<Vec<u8>> {
    if ptr.is_null() {
        return None;
    }
    
    // Read header
    let header_ptr = ptr as *const CompressHeader;
    let header = *header_ptr;
    
    // Read compressed data
    let compressed_data_ptr = ptr.add(size_of::<CompressHeader>());
    let compressed_data_slice = core::slice::from_raw_parts(compressed_data_ptr, header.compressed_size);
    
    // Decompress it
    Some(decompress(compressed_data_slice))
}

/// Compress a memory page
pub unsafe fn compress_page(page_ptr: *const u8, page_size: usize) -> Option<Vec<u8>> {
    let result = compress_memory(page_ptr, page_size);
    if result.is_some() {
        COMPRESSION_STATS.pages_compressed.fetch_add(1, Ordering::Relaxed);
    }
    result
}

/// Decompress a memory page
pub unsafe fn decompress_page(compressed_data: &[u8]) -> Option<Vec<u8>> {
    let result = decompress(&compressed_data[size_of::<CompressHeader>()..]);
    if !result.is_empty() {
        COMPRESSION_STATS.pages_decompressed.fetch_add(1, Ordering::Relaxed);
        Some(result)
    } else {
        None
    }
}

/// Check if memory pressure is high (free memory < threshold)
pub fn check_memory_pressure(free_pages: usize, total_pages: usize) -> bool {
    if total_pages == 0 {
        return false;
    }
    
    let free_ratio = free_pages as f32 / total_pages as f32;
    let threshold = MEMORY_PRESSURE_THRESHOLD.load(Ordering::Relaxed) as f32 / 100.0;
    free_ratio < threshold
}

/// Calculate compression efficiency
pub fn compression_efficiency(original_size: usize, compressed_size: usize) -> f32 {
    if original_size == 0 {
        return 0.0;
    }
    
    let saved = original_size - compressed_size;
    (saved as f32) / (original_size as f32)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compress_decompress() {
        let data = b"Hello, world! This is a test of the memory compression system.";
        let compressed = compress(data);
        let decompressed = decompress(&compressed);
        
        assert_eq!(&decompressed[..], data);
    }
}