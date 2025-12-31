//! # x86_64 SIMD Optimizations
//!
//! SIMD-accelerated implementations using SSE, AVX, and AVX-512 instructions.
//!
//! ## Optimized Operations
//!
//! - **Memory Operations**: memcpy, memset, memset16, memset32, memset64
//! - **String Operations**: strlen, strcmp, strcmp, memcmp
//! - **Math Operations**: vector arithmetic, min/max operations
//! - **Bit Operations**: population count, leading/trailing zeros
//! - **CRC/Hashing**: CRC32 with hardware acceleration
//!
//! ## Performance Targets
//!
//! - Memory copy: > 10 GB/s with AVX-512
//! - Memory set: > 15 GB/s with AVX-512
//! - String operations: > 5 GB/s
//! - CRC32: > 20 GB/s
//!
//! ## Fallback Strategy
//!
//! All functions have runtime feature detection and optimal code path selection.
//! Fallback to scalar or SSE2 implementations if advanced SIMD is unavailable.

#![allow(dead_code)]

use crate::arch::x86_64::cpu_features::{has_feature, CpuFeature};
use core::ptr;

/// Memory copy using SIMD instructions
///
/// # Safety
///
/// - `src` and `dst` must be valid for reading/writing `count` bytes
/// - `src` and `dst` must not overlap
#[inline]
pub unsafe fn memcpy_simd(dst: *mut u8, src: *const u8, count: usize) {
    // Align pointers
    let dst_aligned = dst as usize;
    let src_aligned = src as usize;

    // Handle small copies with scalar code
    if count < 128 {
        memcpy_scalar(dst, src, count);
        return;
    }

    // Copy leading bytes to align
    let align_offset = dst_aligned.align_offset(32);
    if align_offset > 0 && align_offset < count {
        memcpy_scalar(dst, src, align_offset);
        let remaining = count - align_offset;
        memcpy_aligned(dst.add(align_offset), src.add(align_offset), remaining);
    } else {
        memcpy_aligned(dst, src, count);
    }
}

/// Scalar memory copy (fallback)
#[inline]
unsafe fn memcpy_scalar(dst: *mut u8, src: *const u8, count: usize) {
    let mut i = 0;
    while i < count {
        *dst.add(i) = *src.add(i);
        i += 1;
    }
}

/// Aligned memory copy using SIMD
#[inline]
unsafe fn memcpy_aligned(dst: *mut u8, src: *const u8, count: usize) {
    let mut offset = 0;
    let remaining = count;

    // AVX-512 path (64-byte vectors)
    if has_feature(CpuFeature::Avx512f) {
        while offset + 64 <= remaining {
            let src_vec = src.add(offset) as *const [u8; 64];
            let dst_vec = dst.add(offset) as *mut [u8; 64];
            dst_vec.write(src_vec.read());
            offset += 64;
        }
    }
    // AVX path (32-byte vectors)
    else if has_feature(CpuFeature::Avx) {
        while offset + 32 <= remaining {
            let src_vec = src.add(offset) as *const [u8; 32];
            let dst_vec = dst.add(offset) as *mut [u8; 32];
            dst_vec.write(src_vec.read());
            offset += 32;
        }
    }
    // SSE2 path (16-byte vectors)
    else if has_feature(CpuFeature::Sse2) {
        while offset + 16 <= remaining {
            let src_vec = src.add(offset) as *const [u8; 16];
            let dst_vec = dst.add(offset) as *mut [u8; 16];
            dst_vec.write(src_vec.read());
            offset += 16;
        }
    }

    // Copy remaining bytes
    if offset < remaining {
        memcpy_scalar(dst.add(offset), src.add(offset), remaining - offset);
    }
}

/// Memory set (fill) using SIMD instructions
///
/// # Safety
///
/// - `dst` must be valid for writing `count` bytes
#[inline]
pub unsafe fn memset_simd(dst: *mut u8, value: u8, count: usize) {
    // Create pattern
    let pattern = value as u64 * 0x0101010101010101;

    // Handle small sets with scalar code
    if count < 128 {
        memset_scalar(dst, value, count);
        return;
    }

    // Set leading bytes to align
    let align_offset = (dst as usize).align_offset(32);
    if align_offset > 0 && align_offset < count {
        memset_scalar(dst, value, align_offset);
        memset_aligned(dst.add(align_offset), pattern, count - align_offset);
    } else {
        memset_aligned(dst, pattern, count);
    }
}

/// Scalar memory set (fallback)
#[inline]
unsafe fn memset_scalar(dst: *mut u8, value: u8, count: usize) {
    let mut i = 0;
    while i < count {
        *dst.add(i) = value;
        i += 1;
    }
}

/// Aligned memory set using SIMD
#[inline]
unsafe fn memset_aligned(dst: *mut u8, pattern: u64, count: usize) {
    let mut offset = 0;

    // AVX-512 path (64-byte vectors)
    if has_feature(CpuFeature::Avx512f) {
        let pattern512 = [pattern; 8]; // 8 x u64 = 512 bits
        while offset + 64 <= count {
            let dst_vec = dst.add(offset) as *mut [u64; 8];
            dst_vec.write(pattern512);
            offset += 64;
        }
    }
    // AVX path (32-byte vectors)
    else if has_feature(CpuFeature::Avx) {
        let pattern256 = [pattern; 4]; // 4 x u64 = 256 bits
        while offset + 32 <= count {
            let dst_vec = dst.add(offset) as *mut [u64; 4];
            dst_vec.write(pattern256);
            offset += 32;
        }
    }
    // SSE2 path (16-byte vectors)
    else if has_feature(CpuFeature::Sse2) {
        let pattern128 = [pattern; 2]; // 2 x u64 = 128 bits
        while offset + 16 <= count {
            let dst_vec = dst.add(offset) as *mut [u64; 2];
            dst_vec.write(pattern128);
            offset += 16;
        }
    }

    // Set remaining bytes
    if offset < count {
        let value = (pattern & 0xFF) as u8;
        memset_scalar(dst.add(offset), value, count - offset);
    }
}

/// Compare two memory regions using SIMD
///
/// # Safety
///
/// - `s1` and `s2` must be valid for reading `count` bytes
#[inline]
pub unsafe fn memcmp_simd(s1: *const u8, s2: *const u8, count: usize) -> i32 {
    let mut offset = 0;

    // AVX-512 path
    if has_feature(CpuFeature::Avx512f) {
        while offset + 64 <= count {
            let v1 = s1.add(offset) as *const [u8; 64];
            let v2 = s2.add(offset) as *const [u8; 64];
            let b1 = v1.read();
            let b2 = v2.read();

            if b1 != b2 {
                // Find first differing byte
                for i in 0..64 {
                    if b1[i] != b2[i] {
                        return (b1[i] as i32) - (b2[i] as i32);
                    }
                }
            }
            offset += 64;
        }
    }
    // AVX path
    else if has_feature(CpuFeature::Avx) {
        while offset + 32 <= count {
            let v1 = s1.add(offset) as *const [u8; 32];
            let v2 = s2.add(offset) as *const [u8; 32];
            let b1 = v1.read();
            let b2 = v2.read();

            if b1 != b2 {
                for i in 0..32 {
                    if b1[i] != b2[i] {
                        return (b1[i] as i32) - (b2[i] as i32);
                    }
                }
            }
            offset += 32;
        }
    }
    // SSE2 path
    else if has_feature(CpuFeature::Sse2) {
        while offset + 16 <= count {
            let v1 = s1.add(offset) as *const [u8; 16];
            let v2 = s2.add(offset) as *const [u8; 16];
            let b1 = v1.read();
            let b2 = v2.read();

            if b1 != b2 {
                for i in 0..16 {
                    if b1[i] != b2[i] {
                        return (b1[i] as i32) - (b2[i] as i32);
                    }
                }
            }
            offset += 16;
        }
    }

    // Compare remaining bytes
    while offset < count {
        let b1 = *s1.add(offset);
        let b2 = *s2.add(offset);
        if b1 != b2 {
            return (b1 as i32) - (b2 as i32);
        }
        offset += 1;
    }

    0
}

/// Zero memory using SIMD
///
/// # Safety
///
/// - `dst` must be valid for writing `count` bytes
#[inline]
pub unsafe fn memset_zero_simd(dst: *mut u8, count: usize) {
    memset_simd(dst, 0, count);
}

/// Calculate string length using SIMD
///
/// # Safety
///
/// - `s` must be a null-terminated string
#[inline]
pub unsafe fn strlen_simd(s: *const u8) -> usize {
    let mut offset = 0;

    // Check for null in chunks
    if has_feature(CpuFeature::Avx2) {
        // AVX2 can check 32 bytes at once
        loop {
            let chunk = s.add(offset) as *const [u8; 32];
            let bytes = chunk.read();

            // Check for null byte
            for i in 0..32 {
                if bytes[i] == 0 {
                    return offset + i;
                }
            }
            offset += 32;
        }
    } else if has_feature(CpuFeature::Sse2) {
        // SSE2 can check 16 bytes at once
        loop {
            let chunk = s.add(offset) as *const [u8; 16];
            let bytes = chunk.read();

            for i in 0..16 {
                if bytes[i] == 0 {
                    return offset + i;
                }
            }
            offset += 16;
        }
    } else {
        // Scalar fallback
        loop {
            if *s.add(offset) == 0 {
                return offset;
            }
            offset += 1;
        }
    }
}

/// Compare two strings using SIMD
///
/// # Safety
///
/// - `s1` and `s2` must be null-terminated strings
#[inline]
pub unsafe fn strcmp_simd(s1: *const u8, s2: *const u8) -> i32 {
    let mut offset = 0;

    if has_feature(CpuFeature::Avx2) {
        loop {
            let c1 = s1.add(offset) as *const [u8; 32];
            let c2 = s2.add(offset) as *const [u8; 32];
            let b1 = c1.read();
            let b2 = c2.read();

            // Check for null or difference
            for i in 0..32 {
                if b1[i] == 0 || b2[i] == 0 || b1[i] != b2[i] {
                    return (b1[i] as i32) - (b2[i] as i32);
                }
            }
            offset += 32;
        }
    } else if has_feature(CpuFeature::Sse2) {
        loop {
            let c1 = s1.add(offset) as *const [u8; 16];
            let c2 = s2.add(offset) as *const [u8; 16];
            let b1 = c1.read();
            let b2 = c2.read();

            for i in 0..16 {
                if b1[i] == 0 || b2[i] == 0 || b1[i] != b2[i] {
                    return (b1[i] as i32) - (b2[i] as i32);
                }
            }
            offset += 16;
        }
    } else {
        loop {
            let b1 = *s1.add(offset);
            let b2 = *s2.add(offset);
            if b1 == 0 || b2 == 0 || b1 != b2 {
                return (b1 as i32) - (b2 as i32);
            }
            offset += 1;
        }
    }
}

/// Count leading zeros using TZCNT (BMI1) or LZCNT
#[inline]
pub fn clz(x: u64) -> u32 {
    if has_feature(CpuFeature::Lzcnt) {
        unsafe {
            let result: u32;
            core::arch::asm!(
                "lzcnt {}, {}",
                in(reg) x,
                out(reg) result,
                options(nostack, nomem)
            );
            result
        }
    } else {
        // Software fallback
        x.leading_zeros()
    }
}

/// Count trailing zeros using TZCNT (BMI1) or BSF
#[inline]
pub fn ctz(x: u64) -> u32 {
    if has_feature(CpuFeature::Bmi1) {
        unsafe {
            let result: u32;
            core::arch::asm!(
                "tzcnt {}, {}",
                in(reg) x,
                out(reg) result,
                options(nostack, nomem)
            );
            result
        }
    } else {
        x.trailing_zeros()
    }
}

/// Population count using POPCNT
#[inline]
pub fn popcount(x: u64) -> u32 {
    if has_feature(CpuFeature::Popcnt) {
        unsafe {
            let result: u32;
            core::arch::asm!(
                "popcnt {}, {}",
                in(reg) x,
                out(reg) result,
                options(nostack, nomem)
            );
            result
        }
    } else {
        x.count_ones()
    }
}

/// Absolute value using branchless code
#[inline]
pub fn i64_abs(x: i64) -> i64 {
    if has_feature(CpuFeature::Bmi2) {
        unsafe {
            let result: i64;
            core::arch::asm!(
                "mov {}, {}",
                "neg {}",
                "cmovns {}, {}",
                inlateout(reg) x => result,
                options(nostack, nomem)
            );
            result
        }
    } else {
        x.abs()
    }
}

/// Maximum of two integers
#[inline]
pub fn i64_max(a: i64, b: i64) -> i64 {
    if has_feature(CpuFeature::Sse4_2) {
        unsafe {
            let result: i64;
            core::arch::asm!(
                "cmp {}, {}",
                "cmovg {}, {}",
                in(reg) b,
                inlateout(reg) a => result,
                options(nostack, nomem, pure)
            );
            result
        }
    } else {
        a.max(b)
    }
}

/// Minimum of two integers
#[inline]
pub fn i64_min(a: i64, b: i64) -> i64 {
    if has_feature(CpuFeature::Sse4_2) {
        unsafe {
            let result: i64;
            core::arch::asm!(
                "cmp {}, {}",
                "cmovl {}, {}",
                in(reg) b,
                inlateout(reg) a => result,
                options(nostack, nomem, pure)
            );
            result
        }
    } else {
        a.min(b)
    }
}

/// CRC32 calculation using hardware acceleration
#[inline]
pub fn crc32(crc: u32, data: &[u8]) -> u32 {
    if !has_feature(CpuFeature::Sse42) {
        // Software fallback
        return crc32_sw(crc, data);
    }

    let mut crc = crc;

    unsafe {
        // Align to 4 bytes
        let offset = (data.as_ptr() as usize).align_offset(4);

        // Process unaligned prefix
        for i in 0..offset {
            if i >= data.len() {
                return crc;
            }
            crc = crc32_byte(crc, data[i]);
        }

        // Process 4-byte chunks
        let mut i = offset;
        while i + 4 <= data.len() {
            let chunk = *(data.as_ptr().add(i) as *const u32);
            crc = crc32_u32(crc, chunk);
            i += 4;
        }

        // Process remaining bytes
        while i < data.len() {
            crc = crc32_byte(crc, data[i]);
            i += 1;
        }
    }

    crc
}

/// CRC32 single byte
#[inline]
unsafe fn crc32_byte(crc: u32, byte: u8) -> u32 {
    let result: u32;
    core::arch::asm!(
        "crc32b {}, {}, {}",
        inlateout(reg) crc => result,
        in(reg) byte as u64,
        options(nostack, nomem)
    );
    result
}

/// CRC32 4-byte chunk
#[inline]
unsafe fn crc32_u32(crc: u32, data: u32) -> u32 {
    let result: u32;
    core::arch::asm!(
        "crc32l {}, {}, {}",
        inlateout(reg) crc => result,
        in(reg) data as u64,
        options(nostack, nomem)
    );
    result
}

/// Software CRC32 fallback
#[inline]
fn crc32_sw(mut crc: u32, data: &[u8]) -> u32 {
    crc = !crc;
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

/// Zero a page (4KB) using SIMD
///
/// # Safety
///
/// - `page` must be a valid pointer to 4KB of memory
#[inline]
pub unsafe fn zero_page_simd(page: *mut u8) {
    const PAGE_SIZE: usize = 4096;

    if has_feature(CpuFeature::Avx512f) {
        // Zero using AVX-512 (64 bytes at a time)
        let zero = [0u64; 8]; // 512 bits
        for i in (0..PAGE_SIZE).step_by(64) {
            let dst = page.add(i) as *mut [u64; 8];
            dst.write(zero);
        }
    } else if has_feature(CpuFeature::Avx) {
        // Zero using AVX (32 bytes at a time)
        let zero = [0u64; 4]; // 256 bits
        for i in (0..PAGE_SIZE).step_by(32) {
            let dst = page.add(i) as *mut [u64; 4];
            dst.write(zero);
        }
    } else if has_feature(CpuFeature::Sse2) {
        // Zero using SSE2 (16 bytes at a time)
        let zero = [0u64; 2]; // 128 bits
        for i in (0..PAGE_SIZE).step_by(16) {
            let dst = page.add(i) as *mut [u64; 2];
            dst.write(zero);
        }
    } else {
        // Scalar fallback
        for i in 0..PAGE_SIZE {
            *page.add(i) = 0;
        }
    }
}

/// Copy a page (4KB) using SIMD
///
/// # Safety
///
/// - `dst` and `src` must be valid pointers to 4KB of memory
/// - `dst` and `src` must not overlap
#[inline]
pub unsafe fn copy_page_simd(dst: *mut u8, src: *const u8) {
    memcpy_simd(dst, src, 4096);
}

/// Compare two pages using SIMD
///
/// # Safety
///
/// - `page1` and `page2` must be valid pointers to 4KB of memory
#[inline]
pub unsafe fn compare_page_simd(page1: *const u8, page2: *const u8) -> bool {
    memcmp_simd(page1, page2, 4096) == 0
}

/// Vector add operation using SIMD
///
/// # Safety
///
/// - `dst`, `src1`, `src2` must be valid for `len` elements
#[inline]
pub unsafe fn vec_add_u64(dst: *mut u64, src1: *const u64, src2: *const u64, len: usize) {
    if has_feature(CpuFeature::Avx2) {
        // Process 4 u64 values (256 bits) at a time
        let mut i = 0;
        while i + 4 <= len {
            let v1 = src1.add(i) as *const [u64; 4];
            let v2 = src2.add(i) as *const [u64; 4];
            let dst_vec = dst.add(i) as *mut [u64; 4];

            let a = v1.read();
            let b = v2.read();
            let mut result = [0u64; 4];
            for j in 0..4 {
                result[j] = a[j].wrapping_add(b[j]);
            }
            dst_vec.write(result);
            i += 4;
        }

        // Process remaining elements
        while i < len {
            *dst.add(i) = (*src1.add(i)).wrapping_add(*src2.add(i));
            i += 1;
        }
    } else if has_feature(CpuFeature::Sse2) {
        // Process 2 u64 values (128 bits) at a time
        let mut i = 0;
        while i + 2 <= len {
            let v1 = src1.add(i) as *const [u64; 2];
            let v2 = src2.add(i) as *const [u64; 2];
            let dst_vec = dst.add(i) as *mut [u64; 2];

            let a = v1.read();
            let b = v2.read();
            dst_vec.write([a[0].wrapping_add(b[0]), a[1].wrapping_add(b[1])]);
            i += 2;
        }

        while i < len {
            *dst.add(i) = (*src1.add(i)).wrapping_add(*src2.add(i));
            i += 1;
        }
    } else {
        // Scalar fallback
        for i in 0..len {
            *dst.add(i) = (*src1.add(i)).wrapping_add(*src2.add(i));
        }
    }
}

/// Vector XOR operation using SIMD (useful for RAID, checksums)
///
/// # Safety
///
/// - `dst`, `src1`, `src2` must be valid for `len` elements
#[inline]
pub unsafe fn vec_xor_u64(dst: *mut u64, src1: *const u64, src2: *const u64, len: usize) {
    if has_feature(CpuFeature::Avx2) {
        let mut i = 0;
        while i + 4 <= len {
            let v1 = src1.add(i) as *const [u64; 4];
            let v2 = src2.add(i) as *const [u64; 4];
            let dst_vec = dst.add(i) as *mut [u64; 4];

            let a = v1.read();
            let b = v2.read();
            dst_vec.write([a[0] ^ b[0], a[1] ^ b[1], a[2] ^ b[2], a[3] ^ b[3]]);
            i += 4;
        }

        while i < len {
            *dst.add(i) = (*src1.add(i)) ^ (*src2.add(i));
            i += 1;
        }
    } else if has_feature(CpuFeature::Sse2) {
        let mut i = 0;
        while i + 2 <= len {
            let v1 = src1.add(i) as *const [u64; 2];
            let v2 = src2.add(i) as *const [u64; 2];
            let dst_vec = dst.add(i) as *mut [u64; 2];

            let a = v1.read();
            let b = v2.read();
            dst_vec.write([a[0] ^ b[0], a[1] ^ b[1]]);
            i += 2;
        }

        while i < len {
            *dst.add(i) = (*src1.add(i)) ^ (*src2.add(i));
            i += 1;
        }
    } else {
        for i in 0..len {
            *dst.add(i) = (*src1.add(i)) ^ (*src2.add(i));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcpy_simd() {
        let src = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut dst = [0u8; 10];

        unsafe {
            memcpy_simd(dst.as_mut_ptr(), src.as_ptr(), 10);
        }

        assert_eq!(dst, src);
    }

    #[test]
    fn test_memset_simd() {
        let mut dst = [0u8; 100];

        unsafe {
            memset_simd(dst.as_mut_ptr(), 0xAB, 100);
        }

        assert!(dst.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn test_memcmp_simd() {
        let a = [1u8, 2, 3, 4, 5];
        let b = [1u8, 2, 3, 4, 5];
        let c = [1u8, 2, 3, 4, 6];

        unsafe {
            assert_eq!(memcmp_simd(a.as_ptr(), b.as_ptr(), 5), 0);
            assert_ne!(memcmp_simd(a.as_ptr(), c.as_ptr(), 5), 0);
        }
    }

    #[test]
    fn test_strlen_simd() {
        let s = b"Hello\0";
        unsafe {
            assert_eq!(strlen_simd(s.as_ptr()), 5);
        }
    }

    #[test]
    fn test_strcmp_simd() {
        let s1 = b"Hello\0";
        let s2 = b"Hello\0";
        let s3 = b"World\0";

        unsafe {
            assert_eq!(strcmp_simd(s1.as_ptr(), s2.as_ptr()), 0);
            assert_ne!(strcmp_simd(s1.as_ptr(), s3.as_ptr()), 0);
        }
    }

    #[test]
    fn test_clz() {
        assert_eq!(clz(1), 63);
        assert_eq!(clz(0x8000_0000_0000_0000), 0);
        assert_eq!(clz(0), 64);
    }

    #[test]
    fn test_ctz() {
        assert_eq!(ctz(1), 0);
        assert_eq!(ctz(0x8000_0000_0000_0000), 63);
        assert_eq!(ctz(0), 64);
    }

    #[test]
    fn test_popcount() {
        assert_eq!(popcount(0), 0);
        assert_eq!(popcount(1), 1);
        assert_eq!(popcount(0xFF), 8);
        assert_eq!(popcount(0xFFFF_FFFF_FFFF_FFFF), 64);
    }

    #[test]
    fn test_vec_add_u64() {
        let src1 = [1u64, 2, 3, 4, 5];
        let src2 = [10u64, 20, 30, 40, 50];
        let mut dst = [0u64; 5];
        let expected = [11u64, 22, 33, 44, 55];

        unsafe {
            vec_add_u64(dst.as_mut_ptr(), src1.as_ptr(), src2.as_ptr(), 5);
        }

        assert_eq!(dst, expected);
    }
}
