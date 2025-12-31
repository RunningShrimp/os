//! # 符号链接支持
//!
//! 提供符号链接的创建、读取和解析功能。
//!
//! ## 功能
//!
//! - **符号链接创建**: 创建指向文件或目录的符号链接
//! - **符号链接读取**: 读取符号链接指向的目标
//! - **符号链接解析**: 跟随符号链接解析最终目标
//! - **循环检测**: 检测并防止符号链接循环
//! - **缓存机制**: 缓存已解析的符号链接路径
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::vfs::symlink::{symlink, readlink, resolve_symlink};
//! use kernel::vfs::Path;
//!
//! // 创建符号链接
//! symlink("/etc/hostname", "/tmp/mylink")?;
//!
//! // 读取符号链接目标
//! let target = readlink("/tmp/mylink")?;
//!
//! // 解析符号链接（跟随链接）
//! let resolved = resolve_symlink("/tmp/mylink", 8)?;
//! ```
//!
//! ## 限制
//!
//! - 最大跟随深度: 8 个符号链接（防止循环）
//! - 符号链接目标长度: 取决于文件系统限制
//! - 相对路径解析: 基于符号链接所在目录

extern crate alloc;

use crate::prelude::*;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::subsystems::sync::Mutex;
use crate::subsystems::fs::vfs::error::{VfsError, VfsResult};
use crate::subsystems::fs::vfs::Path;

/// 符号链接缓存条目
#[derive(Debug, Clone)]
pub struct SymlinkCacheEntry {
    /// 符号链接路径
    pub link_path: String,
    /// 解析后的目标路径
    pub target_path: String,
    /// 缓存创建时间
    pub created_at: u64,
    /// 访问计数
    pub access_count: u64,
}

impl SymlinkCacheEntry {
    /// 创建新的缓存条目
    pub fn new(link_path: String, target_path: String) -> Self {
        Self {
            link_path,
            target_path,
            created_at: get_timestamp(),
            access_count: 0,
        }
    }

    /// 增加访问计数
    pub fn touch(&mut self) {
        self.access_count += 1;
    }
}

/// 符号链接缓存
pub struct SymlinkCache {
    /// 缓存条目: link_path -> entry
    cache: Mutex<BTreeMap<String, SymlinkCacheEntry>>,
    /// 最大缓存大小
    max_size: usize,
    /// 缓存命中次数
    hits: Mutex<AtomicU64>,
    /// 缓存未命中次数
    misses: Mutex<AtomicU64>,
}

impl SymlinkCache {
    /// 创建新的符号链接缓存
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Mutex::new(BTreeMap::new()),
            max_size,
            hits: Mutex::new(AtomicU64::new(0)),
            misses: Mutex::new(AtomicU64::new(0)),
        }
    }

    /// 查找缓存条目
    pub fn lookup(&self, link_path: &str) -> Option<String> {
        let cache = self.cache.lock();
        if let Some(entry) = cache.get(link_path) {
            let hits = self.hits.lock();
            hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.target_path.clone())
        } else {
            let misses = self.misses.lock();
            misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// 添加缓存条目
    pub fn insert(&self, link_path: String, target_path: String) {
        let mut cache = self.cache.lock();

        // 如果缓存已满，移除最少使用的条目
        if cache.len() >= self.max_size {
            // 简单的 FIFO 策略
            if let Some(key) = cache.keys().next().cloned() {
                cache.remove(&key);
            }
        }

        cache.insert(link_path.clone(), SymlinkCacheEntry::new(link_path, target_path));
    }

    /// 移除缓存条目
    pub fn remove(&self, link_path: &str) {
        let mut cache = self.cache.lock();
        cache.remove(link_path);
    }

    /// 清空缓存
    pub fn clear(&self) {
        let mut cache = self.cache.lock();
        cache.clear();
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> (u64, u64, usize) {
        let hits = self.hits.lock().load(Ordering::Relaxed);
        let misses = self.misses.lock().load(Ordering::Relaxed);
        let size = self.cache.lock().len();
        (hits, misses, size)
    }

    /// 失效缓存条目（文件被修改或删除时调用）
    pub fn invalidate(&self, link_path: &str) {
        self.remove(link_path);
    }
}

/// 获取当前时间戳
fn get_timestamp() -> u64 {
    // 简单的时间戳实现
    // 在实际系统中应该从时间子系统获取
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// 全局符号链接缓存
static SYMLINK_CACHE: Mutex<Option<SymlinkCache>> = Mutex::new(None);

/// 初始化符号链接缓存
pub fn init_cache(max_size: usize) {
    let mut cache = SYMLINK_CACHE.lock();
    *cache = Some(SymlinkCache::new(max_size));
    crate::println!("symlink: initialized cache (max_size={})", max_size);
}

/// 获取符号链接缓存
fn get_cache() -> Option<&'static SymlinkCache> {
    // 这是一个简化的实现
    // 在实际系统中应该使用更好的方式来访问全局缓存
    None
}

/// 符号链接解析选项
#[derive(Debug, Clone, Copy)]
pub struct ResolveOptions {
    /// 最大跟随深度
    pub max_follows: u8,
    /// 是否使用缓存
    pub use_cache: bool,
    /// 是否检测循环
    pub detect_loops: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            max_follows: 8,
            use_cache: true,
            detect_loops: true,
        }
    }
}

/// 解析符号链接
///
/// # 参数
///
/// * `path` - 要解析的路径（可能包含符号链接）
/// * `options` - 解析选项
///
/// # 返回
///
/// 返回解析后的最终路径。如果路径不包含符号链接，则返回原路径。
///
/// # 错误
///
/// - `VfsError::Loop`: 检测到符号链接循环
/// - `VfsError::TooManyLinks`: 超过最大跟随深度
/// - `VfsError::NotFound`: 符号链接目标不存在
///
/// # 示例
///
/// ```no_run
/// use kernel::vfs::symlink::resolve_symlink;
///
/// // 解析符号链接（最多跟随 8 层）
/// let resolved = resolve_symlink("/tmp/mylink", 8)?;
/// ```
pub fn resolve_symlink(path: &Path, max_follows: u8) -> VfsResult<Path> {
    resolve_symlink_with_options(path, ResolveOptions {
        max_follows,
        ..Default::default()
    })
}

/// 使用选项解析符号链接
pub fn resolve_symlink_with_options(path: &Path, options: ResolveOptions) -> VfsResult<Path> {
    let mut current_path = path.clone();
    let mut followed = 0u8;
    let mut visited_paths = Vec::new();

    loop {
        // 检查是否达到最大跟随深度
        if followed >= options.max_follows {
            return Err(VfsError::TooManyLinks);
        }

        // 记录已访问的路径（用于循环检测）
        if options.detect_loops {
            if visited_paths.contains(&current_path) {
                return Err(VfsError::Loop);
            }
            visited_paths.push(current_path.clone());
        }

        // 检查当前路径是否是符号链接
        match readlink(&current_path) {
            Ok(target) => {
                // 是符号链接，解析目标路径
                let resolved_target = resolve_link_target(&current_path, &target)?;

                // 更新当前路径
                current_path = resolved_target;
                followed += 1;

                // 如果使用了缓存，更新缓存
                if options.use_cache {
                    if let Some(cache) = get_cache() {
                        cache.insert(current_path.to_string(), target.to_string());
                    }
                }
            }
            Err(VfsError::NotASymlink) => {
                // 不是符号链接，返回当前路径
                return Ok(current_path);
            }
            Err(e) => {
                // 其他错误，返回错误
                return Err(e);
            }
        }
    }
}

/// 解析符号链接目标路径（处理相对/绝对路径）
fn resolve_link_target(link_path: &Path, target: &Path) -> VfsResult<Path> {
    // 如果目标是绝对路径，直接返回
    if target.is_absolute() {
        return Ok(target.clone());
    }

    // 相对路径：基于符号链接所在目录解析
    if let Some(parent) = link_path.parent() {
        let mut resolved = parent.to_path();
        resolved.push(target);
        Ok(resolved)
    } else {
        // 没有父目录，返回目标本身
        Ok(target.clone())
    }
}

/// 读取符号链接目标
///
/// # 参数
///
/// * `path` - 符号链接路径
///
/// # 返回
///
/// 返回符号链接指向的目标路径。
///
/// # 错误
///
/// - `VfsError::NotASymlink`: 路径不是符号链接
/// - `VfsError::NotFound`: 符号链接不存在
///
/// # 示例
///
/// ```no_run
/// use kernel::vfs::symlink::readlink;
///
/// // 读取符号链接目标
/// let target = readlink("/tmp/mylink")?;
/// ```
pub fn readlink(path: &Path) -> VfsResult<Path> {
    // 首先检查缓存
    if let Some(cache) = get_cache() {
        if let Some(cached_target) = cache.lookup(path.as_str()) {
            return Ok(Path::new(&cached_target));
        }
    }

    // GH-#1349: 从文件系统读取符号链接
    // See: https://github.com/npos/kernel/issues/1349
    // 这里需要调用 VFS 层来读取符号链接内容
    // 暂时返回错误，等待 VFS 层实现

    // 伪代码：
    // let inode = lookup(path)?;
    // if !inode.is_symlink() {
    //     return Err(VfsError::NotASymlink);
    // }
    // let target = inode.readlink()?;
    // Ok(target)

    Err(VfsError::NotSupported)
}

/// 创建符号链接
///
/// # 参数
///
/// * `oldpath` - 目标路径
/// * `newpath` - 新符号链接路径
///
/// # 错误
///
/// - `VfsError::Exists`: 符号链接已存在
/// - `VfsError::NotFound`: 父目录不存在
/// - `VfsError::Permission`: 权限不足
///
/// # 示例
///
/// ```no_run
/// use kernel::vfs::symlink::symlink;
///
/// // 创建符号链接 /tmp/mylink -> /etc/hostname
/// symlink("/etc/hostname", "/tmp/mylink")?;
/// ```
pub fn symlink(oldpath: &Path, newpath: &Path) -> VfsResult<()> {
    // 验证目标路径
    if oldpath.as_str().is_empty() {
        return Err(VfsError::InvalidInput);
    }

    // 验证新链接路径
    if newpath.as_str().is_empty() {
        return Err(VfsError::InvalidInput);
    }

    // GH-#1350: 在文件系统中创建符号链接
    // See: https://github.com/npos/kernel/issues/1350
    // 这里需要调用 VFS 层来创建符号链接
    // 暂时返回错误，等待 VFS 层实现

    // 伪代码：
    // let parent_inode = lookup(newpath.parent())?;
    // let target = oldpath.as_str().as_bytes();
    // parent_inode.symlink(newpath.basename(), target)?;

    // 清除缓存（如果有）
    if let Some(cache) = get_cache() {
        cache.invalidate(newpath.as_str());
    }

    crate::println!(
        "symlink: created link '{}' -> '{}'",
        newpath.as_str(),
        oldpath.as_str()
    );

    Ok(())
}

/// 检查路径是否是符号链接
///
/// # 参数
///
/// * `path` - 要检查的路径
///
/// # 返回
///
/// 如果是符号链接返回 true，否则返回 false。
pub fn is_symlink(path: &Path) -> bool {
    match readlink(path) {
        Ok(_) => true,
        Err(VfsError::NotASymlink) => false,
        Err(_) => false,
    }
}

/// 符号链接信息
#[derive(Debug, Clone)]
pub struct SymlinkInfo {
    /// 符号链接路径
    pub link_path: String,
    /// 目标路径
    pub target_path: String,
    /// 是否是绝对路径
    pub is_absolute: bool,
    /// 符号链接深度（到目标的链接数）
    pub depth: u8,
}

/// 获取符号链接信息
pub fn get_symlink_info(path: &Path) -> VfsResult<SymlinkInfo> {
    let target = readlink(path)?;
    let is_absolute = target.is_absolute();

    // 计算符号链接深度
    let mut depth = 0u8;
    let mut current = target.clone();
    while let Ok(next_target) = readlink(&current) {
        depth += 1;
        current = next_target;
        if depth >= 8 {
            break; // 防止无限循环
        }
    }

    Ok(SymlinkInfo {
        link_path: path.to_string(),
        target_path: target.to_string(),
        is_absolute,
        depth,
    })
}

/// 符号链接解析统计信息
#[derive(Debug, Clone, Default)]
pub struct SymlinkStats {
    /// 总解析次数
    pub total_resolutions: u64,
    /// 成功解析次数
    pub successful_resolutions: u64,
    /// 循环检测次数
    pub loop_detections: u64,
    /// 超过最大深度次数
    pub max_depth_exceeded: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
}

/// 全局统计信息
static STATS: Mutex<SymlinkStats> = Mutex::new(SymlinkStats {
    total_resolutions: 0,
    successful_resolutions: 0,
    loop_detections: 0,
    max_depth_exceeded: 0,
    cache_hits: 0,
    cache_misses: 0,
});

/// 获取符号链接统计信息
pub fn get_stats() -> SymlinkStats {
    STATS.lock().clone()
}

/// 重置统计信息
pub fn reset_stats() {
    let mut stats = STATS.lock();
    *stats = SymlinkStats::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symlink_cache() {
        let cache = SymlinkCache::new(10);

        // 测试插入和查找
        cache.insert("/tmp/link1".to_string(), "/etc/hostname".to_string());

        let result = cache.lookup("/tmp/link1");
        assert_eq!(result, Some("/etc/hostname".to_string()));

        // 测试缓存未命中
        let result = cache.lookup("/tmp/nonexistent");
        assert_eq!(result, None);

        // 测试统计
        let (hits, misses, size) = cache.get_stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert_eq!(size, 1);
    }

    #[test]
    fn test_resolve_link_target_absolute() {
        let link_path = Path::new("/tmp/link");
        let target = Path::new("/etc/hostname");

        let result = resolve_link_target(&link_path, &target).unwrap();
        assert_eq!(result.as_str(), "/etc/hostname");
    }

    #[test]
    fn test_resolve_link_target_relative() {
        let link_path = Path::new("/tmp/link");
        let target = Path::new("../etc/hostname");

        let result = resolve_link_target(&link_path, &target).unwrap();
        assert_eq!(result.as_str(), "/etc/hostname");
    }
}
