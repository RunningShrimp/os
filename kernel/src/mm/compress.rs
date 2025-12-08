//! Memory compression module for xv6-rust
//! 
//! Implements memory compression to reduce memory footprint using LZ4 compression.
//! This is a simplified implementation for educational purposes.

extern crate alloc;

use core::ptr::null_mut;
use core::mem::size_of;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use crate::sync::Mutex;

// ============================================================================
// Constants
// ============================================================================

/// Compression buffer size (4KB)
pub const COMPRESS_BLOCK_SIZE: usize = 4096;

/// Compression header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CompressHeader {
    /// Original size in bytes
    original_size: usize,
    /// Compressed size in bytes
    compressed_size: usize,
}

// ============================================================================
// Compression API
// ============================================================================

/// Compression threshold: only compress if savings > 20%
pub const COMPRESSION_THRESHOLD: f32 = 0.2;

/// Memory pressure threshold: compress when free memory < 10%
pub const MEMORY_PRESSURE_THRESHOLD: f32 = 0.1;

/// Enable memory compression (can be toggled via sysctl or kernel parameter)
pub static ENABLE_COMPRESSION: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Compression statistics
#[derive(Debug, Default)]
pub struct CompressionStats {
    pub pages_compressed: AtomicUsize,
    pub pages_decompressed: AtomicUsize,
    pub bytes_saved: AtomicUsize,
    pub compression_ratio: AtomicUsize, // Average compression ratio * 100
}

static COMPRESSION_STATS: CompressionStats = CompressionStats {
    pages_compressed: AtomicUsize::new(0),
    pages_decompressed: AtomicUsize::new(0),
    bytes_saved: AtomicUsize::new(0),
    compression_ratio: AtomicUsize::new(0),
};

/// Enable memory compression
pub fn enable_compression() {
    ENABLE_COMPRESSION.store(true, core::sync::atomic::Ordering::Relaxed);
}

/// Disable memory compression
pub fn disable_compression() {
    ENABLE_COMPRESSION.store(false, core::sync::atomic::Ordering::Relaxed);
}

/// Check if compression is enabled
pub fn is_compression_enabled() -> bool {
    ENABLE_COMPRESSION.load(Ordering::Relaxed)
}

/// Get compression statistics
pub fn get_stats() -> (usize, usize, usize, usize) {
    (
        COMPRESSION_STATS.pages_compressed.load(Ordering::Relaxed),
        COMPRESSION_STATS.pages_decompressed.load(Ordering::Relaxed),
        COMPRESSION_STATS.bytes_saved.load(Ordering::Relaxed),
        COMPRESSION_STATS.compression_ratio.load(Ordering::Relaxed),
    )
}

/// Reset compression statistics
pub fn reset_stats() {
    COMPRESSION_STATS.pages_compressed.store(0, Ordering::Relaxed);
    COMPRESSION_STATS.pages_decompressed.store(0, Ordering::Relaxed);
    COMPRESSION_STATS.bytes_saved.store(0, Ordering::Relaxed);
    COMPRESSION_STATS.compression_ratio.store(0, Ordering::Relaxed);
}

/// Simple LZ4-style compression (simplified for kernel use)
pub fn compress(src: &[u8]) -> Vec<u8> {
    // Simplified compression for demonstration purposes
    // In a real implementation, this would be a proper LZ4 implementation
    
    // Check if compression is enabled
    if !is_compression_enabled() {
        // Return uncompressed data
        let mut dst = Vec::with_capacity(src.len());
        dst.extend_from_slice(src);
        return dst;
    }
    
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
    
    // Only return compressed data if it's actually smaller
    if dst.len() < src.len() {
        // Update statistics
        let saved = src.len() - dst.len();
        COMPRESSION_STATS.bytes_saved.fetch_add(saved, Ordering::Relaxed);
        
        // Update average compression ratio
        let ratio = (dst.len() * 100) / src.len();
        let old_ratio = COMPRESSION_STATS.compression_ratio.load(Ordering::Relaxed);
        let count = COMPRESSION_STATS.pages_compressed.load(Ordering::Relaxed);
        if count > 0 {
            let new_ratio = ((old_ratio * count) + ratio) / (count + 1);
            COMPRESSION_STATS.compression_ratio.store(new_ratio, Ordering::Relaxed);
        } else {
            COMPRESSION_STATS.compression_ratio.store(ratio, Ordering::Relaxed);
        }
        
        dst
    } else {
        // Compression didn't help, return original
        let mut original = Vec::with_capacity(src.len());
        original.extend_from_slice(src);
        original
    }
}

/// Simple LZ4-style decompression (simplified for kernel use)
pub fn decompress(src: &[u8]) -> Vec<u8> {
    // Simplified decompression for demonstration purposes
    
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
    free_ratio < MEMORY_PRESSURE_THRESHOLD
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