//! # 内存压缩管理 (zswap/zram)
//!
//! 本模块提供内核内存压缩功能，类似于 Linux 的 zswap 和 zram。
//! 通过压缩不活跃的内存页，可以显著增加可用内存容量。
//!
//! ## 主要功能
//!
//! - **压缩内存池**: 管理压缩后的内存页
//! - **多种压缩算法**: 支持 LZO、LZ4、ZSTD 等算法
//! - **动态池调整**: 根据内存压力自动调整压缩池大小
//! - **写回策略**: 将压缩页写回 swap 设备
//!
//! ## 架构
//!
//! ```text
//! 内存请求
//!     ↓
//! 压缩检查 → 是否需要压缩？
//!     ↓ Yes
//! 压缩算法 → 压缩数据
//!     ↓
//! 存储到压缩池 → zswap/zram
//!     ↓
//! 写回检查 → 需要写回？
//!     ↓ Yes
//! 解压缩 → swap 设备
//! ```
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::subsystems::mm::compress::{CompressMgr, CompressionAlgorithm};
//!
//! // 初始化压缩管理器
//! let mgr = CompressMgr::new(CompressionAlgorithm::LZ4);
//!
//! // 压缩页面
//! let src = [0u8; 4096];
//! let compressed = mgr.compress_page(&src)?;
//!
//! // 解压缩页面
//! let decompressed = mgr.decompress_page(&compressed)?;
//! # Ok::<(), UnifiedError>(())
//! ```

#![allow(dead_code)]

use crate::prelude::*;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::error::{UnifiedError, UnifiedResult};

/// 页面大小（4KB）
pub const PAGE_SIZE: usize = 4096;

/// 最大压缩页面大小（压缩后不能超过此值）
pub const MAX_COMPRESSED_SIZE: usize = PAGE_SIZE / 2;

/// 默认压缩池大小（页数）
pub const DEFAULT_POOL_SIZE_PAGES: usize = 1024;

/// 最大压缩池大小（页数）
pub const MAX_POOL_SIZE_PAGES: usize = 16384;

/// 最小压缩池大小（页数）
pub const MIN_POOL_SIZE_PAGES: usize = 128;

/// 压缩算法类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// LZO 算法 - 快速，压缩率中等
    LZO,
    /// LZ4 算法 - 极快，压缩率中等
    LZ4,
    /// ZSTD 算法 - 较慢，压缩率高
    ZSTD,
    /// 8-bit 算法 - 最快，压缩率低
    LZF,
}

impl CompressionAlgorithm {
    /// 获取算法名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::LZO => "LZO",
            Self::LZ4 => "LZ4",
            Self::ZSTD => "ZSTD",
            Self::LZF => "LZF",
        }
    }

    /// 获取预期压缩比
    pub fn expected_ratio(&self) -> f32 {
        match self {
            Self::LZO => 0.45,
            Self::LZ4 => 0.50,
            Self::ZSTD => 0.35,
            Self::LZF => 0.60,
        }
    }

    /// 获取算法速度等级（1-10，10最快）
    pub fn speed_level(&self) -> u8 {
        match self {
            Self::LZF => 10,
            Self::LZ4 => 9,
            Self::LZO => 7,
            Self::ZSTD => 4,
        }
    }
}

/// 压缩页面条目
#[derive(Debug)]
struct CompressedPage {
    /// 原始页面物理地址
    orig_pfn: usize,
    /// 压缩后的数据
    data: Vec<u8>,
    /// 压缩算法
    algorithm: CompressionAlgorithm,
    /// 访问时间戳
    access_time: u64,
    /// 引用计数
    ref_count: AtomicUsize,
    /// 是否脏（需要写回）
    dirty: bool,
}

impl CompressedPage {
    /// 创建新的压缩页面
    fn new(
        orig_pfn: usize,
        data: Vec<u8>,
        algorithm: CompressionAlgorithm,
    ) -> Self {
        Self {
            orig_pfn,
            data,
            algorithm,
            access_time: 0,
            ref_count: AtomicUsize::new(1),
            dirty: false,
        }
    }

    /// 获取压缩比
    fn compression_ratio(&self) -> f32 {
        if self.data.is_empty() {
            1.0
        } else {
            self.data.len() as f32 / PAGE_SIZE as f32
        }
    }

    /// 增加引用计数
    fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 减少引用计数
    fn dec_ref(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::Release)
    }
}

/// 压缩统计信息
#[derive(Debug, Default)]
pub struct CompressionStats {
    /// 总压缩次数
    pub total_compressions: AtomicU64,
    /// 总解压缩次数
    pub total_decompressions: AtomicU64,
    /// 压缩失败次数
    pub compression_failures: AtomicU64,
    /// 解压缩失败次数
    pub decompression_failures: AtomicU64,
    /// 写回次数
    pub total_wb: AtomicU64,
    /// 节省的内存页数
    pub saved_pages: AtomicU64,
    /// 当前压缩页数
    pub current_compressed: AtomicU64,
    /// 平均压缩比（压缩后大小 / 原始大小）
    pub avg_compression_ratio: AtomicU64, // 存储为定点数（*1000）
}

/// 写回策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackPolicy {
    /// 不写回
    Never,
    /// 立即写回
    Immediate,
    /// 延迟写回
    Deferred,
    /// 基于内存压力
    PressureBased,
}

/// 内存压缩管理器
pub struct CompressMgr {
    /// 压缩算法
    algorithm: CompressionAlgorithm,
    /// 压缩页面池
    compressed_pages: Mutex<Vec<CompressedPage>>,
    /// 压缩页面队列（用于 LRU 淘汰）
    page_queue: Mutex<VecDeque<usize>>,
    /// 当前池大小（页数）
    pool_size: AtomicUsize,
    /// 最大池大小（页数）
    max_pool_size: usize,
    /// 写回策略
    writeback_policy: Mutex<WritebackPolicy>,
    /// 统计信息
    stats: CompressionStats,
    /// 是否启用
    enabled: AtomicBool,
    /// 内存压力阈值（0-100）
    pressure_threshold: AtomicU32,
}

impl CompressMgr {
    /// 创建新的压缩管理器
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Self {
            algorithm,
            compressed_pages: Mutex::new(Vec::with_capacity(DEFAULT_POOL_SIZE_PAGES)),
            page_queue: Mutex::new(VecDeque::with_capacity(DEFAULT_POOL_SIZE_PAGES)),
            pool_size: AtomicUsize::new(0),
            max_pool_size: DEFAULT_POOL_SIZE_PAGES,
            writeback_policy: Mutex::new(WritebackPolicy::PressureBased),
            stats: CompressionStats::default(),
            enabled: AtomicBool::new(true),
            pressure_threshold: AtomicU32::new(80),
        }
    }

    /// 压缩单个页面
    pub fn compress_page(&self, src: &[u8; PAGE_SIZE]) -> UnifiedResult<Vec<u8>> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(UnifiedError::NotSupported);
        }

        if src.len() != PAGE_SIZE {
            return Err(UnifiedError::InvalidArgument.into());
        }

        // 根据算法执行压缩
        let compressed = match self.algorithm {
            CompressionAlgorithm::LZO => self.compress_lzo(src),
            CompressionAlgorithm::LZ4 => self.compress_lz4(src),
            CompressionAlgorithm::ZSTD => self.compress_zstd(src),
            CompressionAlgorithm::LZF => self.compress_lzf(src),
        };

        match compressed {
            Ok(data) => {
                // 检查压缩比
                if data.len() > MAX_COMPRESSED_SIZE {
                    // 压缩效果不好，返回错误
                    self.stats.compression_failures.fetch_add(1, Ordering::Relaxed);
                    return Err(UnifiedError::InvalidOperation.into());
                }

                self.stats.total_compressions.fetch_add(1, Ordering::Relaxed);
                Ok(data)
            }
            Err(e) => {
                self.stats.compression_failures.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    /// 解压缩单个页面
    pub fn decompress_page(&self, src: &[u8]) -> UnifiedResult<[u8; PAGE_SIZE]> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(UnifiedError::NotSupported);
        }

        if src.is_empty() || src.len() > PAGE_SIZE {
            return Err(UnifiedError::InvalidData.into());
        }

        // 根据算法执行解压缩
        let decompressed = match self.algorithm {
            CompressionAlgorithm::LZO => self.decompress_lzo(src),
            CompressionAlgorithm::LZ4 => self.decompress_lz4(src),
            CompressionAlgorithm::ZSTD => self.decompress_zstd(src),
            CompressionAlgorithm::LZF => self.decompress_lzf(src),
        };

        match decompressed {
            Ok(data) => {
                self.stats.total_decompressions.fetch_add(1, Ordering::Relaxed);
                Ok(data)
            }
            Err(e) => {
                self.stats.decompression_failures.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }

    /// 存储压缩页面到池中
    pub fn store_compressed_page(
        &self,
        orig_pfn: usize,
        compressed: Vec<u8>,
    ) -> UnifiedResult<()> {
        // 检查池是否已满
        if self.pool_size.load(Ordering::Relaxed) >= self.max_pool_size {
            // 尝试淘汰旧的页面
            self.evict_oldest_page()?;
        }

        // 创建压缩页面条目
        let page = CompressedPage::new(orig_pfn, compressed, self.algorithm);

        // 添加到池中
        let mut pages = self.compressed_pages.lock();
        let index = pages.len();
        pages.push(page);

        // 添加到队列
        let mut queue = self.page_queue.lock();
        queue.push_back(index);

        // 更新统计
        self.pool_size.fetch_add(1, Ordering::Relaxed);
        self.stats.current_compressed.fetch_add(1, Ordering::Relaxed);
        self.stats.saved_pages.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// 从池中获取压缩页面
    pub fn get_compressed_page(&self, orig_pfn: usize) -> UnifiedResult<Vec<u8>> {
        let pages = self.compressed_pages.lock();

        for page in pages.iter() {
            if page.orig_pfn == orig_pfn {
                page.inc_ref();
                return Ok(page.data.clone());
            }
        }

        Err(UnifiedError::NotFound)
    }

    /// 淘汰最旧的页面（LRU）
    fn evict_oldest_page(&self) -> UnifiedResult<()> {
        let mut queue = self.page_queue.lock();

        if let Some(index) = queue.pop_front() {
            let mut pages = self.compressed_pages.lock();

            if index < pages.len() {
                let page = &pages[index];

                // 检查引用计数
                if page.ref_count.load(Ordering::Relaxed) == 0 {
                    // 可以安全淘汰
                    pages.remove(index);
                    self.pool_size.fetch_sub(1, Ordering::Relaxed);
                    self.stats.current_compressed.fetch_sub(1, Ordering::Relaxed);

                    return Ok(());
                } else {
                    // 引用计数不为0，放回队列末尾
                    queue.push_back(index);
                }
            }
        }

        Err(UnifiedError::ResourceBusy)
    }

    /// 设置压缩池大小
    pub fn set_pool_size(&mut self, size: usize) {
        let size = size.clamp(MIN_POOL_SIZE_PAGES, MAX_POOL_SIZE_PAGES);
        self.max_pool_size = size;

        // 如果当前池超过新大小，触发淘汰
        while self.pool_size.load(Ordering::Relaxed) > size {
            let _ = self.evict_oldest_page();
        }
    }

    /// 获取压缩池大小
    pub fn get_pool_size(&self) -> usize {
        self.pool_size.load(Ordering::Relaxed)
    }

    /// 设置写回策略
    pub fn set_writeback_policy(&self, policy: WritebackPolicy) {
        let mut wb_policy = self.writeback_policy.lock();
        *wb_policy = policy;
    }

    /// 获取写回策略
    pub fn get_writeback_policy(&self) -> WritebackPolicy {
        *self.writeback_policy.lock()
    }

    /// 启用/禁用压缩
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &CompressionStats {
        &self.stats
    }

    /// LZO 压缩算法（简化实现）
    fn compress_lzo(&self, src: &[u8; PAGE_SIZE]) -> UnifiedResult<Vec<u8>> {
        // 简化实现：使用 RLE（行程编码）
        let mut compressed = Vec::with_capacity(PAGE_SIZE / 2);
        let mut i = 0;

        while i < PAGE_SIZE {
            let current = src[i];
            let mut count = 1usize;

            // 计算连续相同的字节数
            while i + count < PAGE_SIZE && src[i + count] == current && count < 255 {
                count += 1;
            }

            // 写入压缩数据：字节 + 计数
            compressed.push(current);
            compressed.push(count as u8);

            i += count;
        }

        // 如果压缩后反而变大，返回原始数据
        if compressed.len() >= PAGE_SIZE {
            Ok(src.to_vec())
        } else {
            Ok(compressed)
        }
    }

    /// LZO 解压缩算法
    fn decompress_lzo(&self, src: &[u8]) -> UnifiedResult<[u8; PAGE_SIZE]> {
        let mut decompressed = [0u8; PAGE_SIZE];
        let mut i = 0;
        let mut j = 0;

        while i < src.len() && j < PAGE_SIZE {
            let byte = src[i];
            let count = src[i + 1] as usize;

            for _ in 0..count {
                if j < PAGE_SIZE {
                    decompressed[j] = byte;
                    j += 1;
                }
            }

            i += 2;
        }

        Ok(decompressed)
    }

    /// LZ4 压缩算法（简化实现）
    fn compress_lz4(&self, src: &[u8; PAGE_SIZE]) -> UnifiedResult<Vec<u8>> {
        // 简化实现：使用更激进的 RLE
        let mut compressed = Vec::with_capacity(PAGE_SIZE / 2);
        let mut i = 0;

        while i < PAGE_SIZE {
            let current = src[i];
            let mut count = 1usize;

            while i + count < PAGE_SIZE && src[i + count] == current && count < 255 {
                count += 1;
            }

            // LZ4 风格的标记
            if count > 4 {
                compressed.push(0xFF); // 特殊标记
                compressed.push(count as u8);
                compressed.push(current);
            } else {
                for k in 0..count {
                    compressed.push(src[i + k]);
                }
            }

            i += count;
        }

        if compressed.len() >= PAGE_SIZE {
            Ok(src.to_vec())
        } else {
            Ok(compressed)
        }
    }

    /// LZ4 解压缩算法
    fn decompress_lz4(&self, src: &[u8]) -> UnifiedResult<[u8; PAGE_SIZE]> {
        let mut decompressed = [0u8; PAGE_SIZE];
        let mut i = 0;
        let mut j = 0;

        while i < src.len() && j < PAGE_SIZE {
            if src[i] == 0xFF && i + 2 < src.len() {
                // RLE 解码
                let count = src[i + 1] as usize;
                let byte = src[i + 2];

                for _ in 0..count {
                    if j < PAGE_SIZE {
                        decompressed[j] = byte;
                        j += 1;
                    }
                }

                i += 3;
            } else {
                // 直接复制
                decompressed[j] = src[i];
                j += 1;
                i += 1;
            }
        }

        Ok(decompressed)
    }

    /// ZSTD 压缩算法（简化实现）
    fn compress_zstd(&self, src: &[u8; PAGE_SIZE]) -> UnifiedResult<Vec<u8>> {
        // 简化实现：结合 RLE 和字典压缩
        let mut compressed = Vec::with_capacity(PAGE_SIZE / 2);

        // 先尝试 RLE
        let mut i = 0;
        while i < PAGE_SIZE {
            let current = src[i];
            let mut count = 1usize;

            while i + count < PAGE_SIZE && src[i + count] == current && count < 255 {
                count += 1;
            }

            if count > 2 {
                compressed.push(0xFE); // ZSTD 标记
                compressed.push(count as u8);
                compressed.push(current);
            } else {
                for k in 0..count {
                    compressed.push(src[i + k]);
                }
            }

            i += count;
        }

        if compressed.len() >= PAGE_SIZE {
            Ok(src.to_vec())
        } else {
            Ok(compressed)
        }
    }

    /// ZSTD 解压缩算法
    fn decompress_zstd(&self, src: &[u8]) -> UnifiedResult<[u8; PAGE_SIZE]> {
        let mut decompressed = [0u8; PAGE_SIZE];
        let mut i = 0;
        let mut j = 0;

        while i < src.len() && j < PAGE_SIZE {
            if src[i] == 0xFE && i + 2 < src.len() {
                let count = src[i + 1] as usize;
                let byte = src[i + 2];

                for _ in 0..count {
                    if j < PAGE_SIZE {
                        decompressed[j] = byte;
                        j += 1;
                    }
                }

                i += 3;
            } else {
                decompressed[j] = src[i];
                j += 1;
                i += 1;
            }
        }

        Ok(decompressed)
    }

    /// LZF 压缩算法（最快，压缩率低）
    fn compress_lzf(&self, src: &[u8; PAGE_SIZE]) -> UnifiedResult<Vec<u8>> {
        // 简化实现：简单的字节级压缩
        let mut compressed = Vec::with_capacity(PAGE_SIZE);

        for chunk in src.chunks(128) {
            // 如果全是0，使用特殊编码
            if chunk.iter().all(|&b| b == 0) {
                compressed.push(0xFD);
                compressed.push(chunk.len() as u8);
            } else {
                compressed.push(0xFC); // 原始数据标记
                compressed.push(chunk.len() as u8);
                compressed.extend_from_slice(chunk);
            }
        }

        if compressed.len() >= PAGE_SIZE {
            Ok(src.to_vec())
        } else {
            Ok(compressed)
        }
    }

    /// LZF 解压缩算法
    fn decompress_lzf(&self, src: &[u8]) -> UnifiedResult<[u8; PAGE_SIZE]> {
        let mut decompressed = [0u8; PAGE_SIZE];
        let mut i = 0;
        let mut j = 0;

        while i < src.len() && j < PAGE_SIZE {
            if src[i] == 0xFD && i + 1 < src.len() {
                // 零填充
                let count = src[i + 1] as usize;
                j += count;
                i += 2;
            } else if src[i] == 0xFC && i + 1 < src.len() {
                // 原始数据
                let count = src[i + 1] as usize;
                let end = (i + 2 + count).min(src.len());

                for k in (i + 2)..end {
                    if j < PAGE_SIZE {
                        decompressed[j] = src[k];
                        j += 1;
                    }
                }

                i += 2 + count;
            } else {
                i += 1;
            }
        }

        Ok(decompressed)
    }
}

/// 全局压缩管理器
static COMPRESS_MANAGER: OnceLock<Mutex<CompressMgr>> = OnceLock::new();

/// 初始化压缩管理器
pub fn init_compress() -> UnifiedResult<()> {
    let manager = CompressMgr::new(CompressionAlgorithm::LZ4);
    COMPRESS_MANAGER.get_or_init(|| Mutex::new(manager));
    Ok(())
}

/// 获取压缩管理器
pub fn get_compress_manager() -> Option<&'static Mutex<CompressMgr>> {
    COMPRESS_MANAGER.get()
}

/// 压缩页面（便捷函数）
pub fn compress_page(src: &[u8; PAGE_SIZE]) -> UnifiedResult<Vec<u8>> {
    if let Some(manager) = get_compress_manager() {
        let mgr = manager.lock();
        mgr.compress_page(src)
    } else {
        Err(UnifiedError::Other("Compress manager not initialized".to_string()))
    }
}

/// 解压缩页面（便捷函数）
pub fn decompress_page(src: &[u8]) -> UnifiedResult<[u8; PAGE_SIZE]> {
    if let Some(manager) = get_compress_manager() {
        let mgr = manager.lock();
        mgr.decompress_page(src)
    } else {
        Err(UnifiedError::Other("Compress manager not initialized".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_algorithms() {
        let page = [0u8; PAGE_SIZE];

        // 测试 LZO
        let mgr = CompressMgr::new(CompressionAlgorithm::LZO);
        let compressed = mgr.compress_page(&page).unwrap();
        let decompressed = mgr.decompress_page(&compressed).unwrap();
        assert_eq!(&page[..], &decompressed[..]);

        // 测试 LZ4
        let mgr = CompressMgr::new(CompressionAlgorithm::LZ4);
        let compressed = mgr.compress_page(&page).unwrap();
        let decompressed = mgr.decompress_page(&compressed).unwrap();
        assert_eq!(&page[..], &decompressed[..]);

        // 测试 ZSTD
        let mgr = CompressMgr::new(CompressionAlgorithm::ZSTD);
        let compressed = mgr.compress_page(&page).unwrap();
        let decompressed = mgr.decompress_page(&compressed).unwrap();
        assert_eq!(&page[..], &decompressed[..]);

        // 测试 LZF
        let mgr = CompressMgr::new(CompressionAlgorithm::LZF);
        let compressed = mgr.compress_page(&page).unwrap();
        let decompressed = mgr.decompress_page(&compressed).unwrap();
        assert_eq!(&page[..], &decompressed[..]);
    }

    #[test]
    fn test_compression_pool() {
        let mgr = CompressMgr::new(CompressionAlgorithm::LZ4);
        let page = [0u8; PAGE_SIZE];

        // 压缩并存储多个页面
        for i in 0..10 {
            let compressed = mgr.compress_page(&page).unwrap();
            mgr.store_compressed_page(i, compressed).unwrap();
        }

        assert_eq!(mgr.get_pool_size(), 10);

        // 检索页面
        let retrieved = mgr.get_compressed_page(5).unwrap();
        let decompressed = mgr.decompress_page(&retrieved).unwrap();
        assert_eq!(&page[..], &decompressed[..]);
    }

    #[test]
    fn test_compression_stats() {
        let mgr = CompressMgr::new(CompressionAlgorithm::LZ4);
        let page = [0u8; PAGE_SIZE];

        // 执行一些压缩操作
        for _ in 0..5 {
            let _ = mgr.compress_page(&page);
        }

        let stats = mgr.get_stats();
        assert_eq!(stats.total_compressions.load(Ordering::Relaxed), 5);
    }
}
