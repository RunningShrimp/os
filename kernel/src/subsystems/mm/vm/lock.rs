//! 内存锁定管理
//!
//! 提供内存锁定的详细实现，包括：
//! - 锁定内存页到RAM
//! - 跟踪锁定的内存页
//! - 处理内存限制

extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::string::ToString;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::prelude::*;
use crate::subsystems::sync::Mutex;

/// 内存锁定管理器
///
/// 跟踪所有被锁定的内存页
pub struct MemoryLockManager {
    /// 已锁定的页集合
    locked_pages: Mutex<BTreeSet<usize>>,
    /// 锁定的页数
    locked_count: AtomicUsize,
    /// 最大允许锁定的页数
    max_locked_pages: AtomicUsize,
}

impl MemoryLockManager {
    /// 创建新的内存锁定管理器
    pub fn new() -> Self {
        Self {
            locked_pages: Mutex::new(BTreeSet::new()),
            locked_count: AtomicUsize::new(0),
            max_locked_pages: AtomicUsize::new(usize::MAX), // 默认无限制
        }

        // 设置默认限制（可以根据实际资源调整）
        // 例如：最多锁定物理内存的50%
    }

    /// 锁定内存页
    ///
    /// # 参数
    /// - `pages`: 要锁定的页列表
    ///
    /// # 返回
    /// 成功时返回Ok(()),失败时返回错误
    pub fn lock_pages(&self, pages: &[usize]) -> Result<()> {
        let current_count = self.locked_count.load(Ordering::Acquire);
        let max_count = self.max_locked_pages.load(Ordering::Acquire);

        // 检查是否超过限制
        if current_count + pages.len() > max_count {
            return Err(UnifiedError::Other("Exceeded locked memory limit".to_string()));
        }

        // 检查页是否已锁定
        {
            let locked = self.locked_pages.lock();
            for &page in pages {
                if locked.contains(&page) {
                    return Err(UnifiedError::Other("Page already locked".to_string()));
                }
            }
        }

        // 锁定页
        {
            let mut locked = self.locked_pages.lock();
            for &page in pages {
                locked.insert(page);
            }
        }

        // 更新计数
        self.locked_count.fetch_add(pages.len(), Ordering::AcqRel);

        // TODO: 实际锁定页到RAM（防止被换出）
        // 这需要与页替换算法和内存管理器集成

        Ok(())
    }

    /// 解锁内存页
    ///
    /// # 参数
    /// - `pages`: 要解锁的页列表
    ///
    /// # 返回
    /// 成功时返回Ok(()),失败时返回错误
    pub fn unlock_pages(&self, pages: &[usize]) -> Result<()> {
        // 检查页是否已锁定
        {
            let locked = self.locked_pages.lock();
            for &page in pages {
                if !locked.contains(&page) {
                    return Err(UnifiedError::Other("Page not locked".to_string()));
                }
            }
        }

        // 解锁页
        {
            let mut locked = self.locked_pages.lock();
            for &page in pages {
                locked.remove(&page);
            }
        }

        // 更新计数
        self.locked_count.fetch_sub(pages.len(), Ordering::AcqRel);

        // TODO: 实际解锁页（允许被换出）

        Ok(())
    }

    /// 检查页是否已锁定
    ///
    /// # 参数
    /// - `page`: 页地址
    ///
    /// # 返回
    /// 如果页已锁定返回true，否则返回false
    pub fn is_page_locked(&self, page: usize) -> bool {
        let locked = self.locked_pages.lock();
        locked.contains(&page)
    }

    /// 获取当前锁定的页数
    pub fn locked_count(&self) -> usize {
        self.locked_count.load(Ordering::Acquire)
    }

    /// 设置最大锁定页数限制
    ///
    /// # 参数
    /// - `max`: 最大页数
    pub fn set_max_locked_pages(&self, max: usize) {
        self.max_locked_pages.store(max, Ordering::Release);
    }

    /// 获取最大锁定页数限制
    pub fn max_locked_pages(&self) -> usize {
        self.max_locked_pages.load(Ordering::Acquire)
    }

    /// 解锁所有页
    pub fn unlock_all(&self) {
        let mut locked = self.locked_pages.lock();
        let _count = locked.len();
        locked.clear();
        self.locked_count.store(0, Ordering::Release);

        // TODO: 实际解锁所有页
    }

    /// 获取锁定页的列表
    pub fn get_locked_pages(&self) -> Vec<usize> {
        let locked = self.locked_pages.lock();
        locked.iter().cloned().collect()
    }
}

impl Default for MemoryLockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 内存锁定错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLockError {
    /// 超过限制
    ExceededLimit,
    /// 页已锁定
    AlreadyLocked,
    /// 页未锁定
    NotLocked,
    /// 权限不足
    PermissionDenied,
    /// 内存不足
    OutOfMemory,
}

impl core::fmt::Display for MemoryLockError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MemoryLockError::ExceededLimit => write!(f, "Exceeded locked memory limit"),
            MemoryLockError::AlreadyLocked => write!(f, "Page already locked"),
            MemoryLockError::NotLocked => write!(f, "Page not locked"),
            MemoryLockError::PermissionDenied => write!(f, "Permission denied"),
            MemoryLockError::OutOfMemory => write!(f, "Out of memory"),
        }
    }
}

impl core::error::Error for MemoryLockError {}

/// 全局内存锁定管理器
static MEMORY_LOCK_MANAGER: Lazy<MemoryLockManager> = Lazy::new(|| {
    MemoryLockManager::new()
});

/// 获取全局内存锁定管理器
pub fn memory_lock_manager() -> &'static MemoryLockManager {
    &MEMORY_LOCK_MANAGER
}

/// 地址空间锁
///
/// 用于mlockall/munlockall，锁定整个地址空间
pub struct AddressSpaceLock {
    /// 是否已锁定
    locked: AtomicUsize,
    /// 锁定标志
    flags: Mutex<AddressSpaceLockFlags>,
}

/// 地址空间锁定标志
#[derive(Debug, Clone, Copy)]
pub struct AddressSpaceLockFlags {
    /// 锁定当前映射
    pub current: bool,
    /// 锁定未来映射
    pub future: bool,
}

impl AddressSpaceLock {
    /// 创建新的地址空间锁
    pub fn new() -> Self {
        Self {
            locked: AtomicUsize::new(0),
            flags: Mutex::new(AddressSpaceLockFlags {
                current: false,
                future: false,
            }),
        }
    }

    /// 锁定地址空间
    ///
    /// # 参数
    /// - `flags`: 锁定标志
    ///
    /// # 返回
    /// 成功时返回Ok(()),失败时返回错误
    pub fn lock(&self, flags: AddressSpaceLockFlags) -> Result<()> {
        // TODO: 实现地址空间锁定
        // 1. 如果current=true，锁定所有当前映射的页
        // 2. 如果future=true，标记所有未来的映射都应该被锁定

        let mut current_flags = self.flags.lock();
        current_flags.current = flags.current;
        current_flags.future = flags.future;

        self.locked.store(1, Ordering::Release);

        Ok(())
    }

    /// 解锁地址空间
    ///
    /// # 返回
    /// 成功时返回Ok(()),失败时返回错误
    pub fn unlock(&self) -> Result<()> {
        // TODO: 实现地址空间解锁
        // 解锁所有被锁定的页

        let mut flags = self.flags.lock();
        flags.current = false;
        flags.future = false;

        self.locked.store(0, Ordering::Release);

        Ok(())
    }

    /// 检查地址空间是否已锁定
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Acquire) != 0
    }

    /// 检查是否应该锁定未来的映射
    pub fn should_lock_future(&self) -> bool {
        let flags = self.flags.lock();
        flags.future
    }

    /// 获取锁定标志
    pub fn flags(&self) -> AddressSpaceLockFlags {
        let flags = self.flags.lock();
        *flags
    }
}

impl Default for AddressSpaceLock {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_unlock_pages() {
        let manager = MemoryLockManager::new();
        let pages = vec![0x1000, 0x2000, 0x3000];

        // 锁定页
        let result = manager.lock_pages(&pages);
        assert!(result.is_ok());
        assert_eq!(manager.locked_count(), 3);

        // 检查页是否已锁定
        assert!(manager.is_page_locked(0x1000));
        assert!(manager.is_page_locked(0x2000));
        assert!(manager.is_page_locked(0x3000));

        // 解锁页
        let result = manager.unlock_pages(&pages);
        assert!(result.is_ok());
        assert_eq!(manager.locked_count(), 0);

        // 检查页是否已解锁
        assert!(!manager.is_page_locked(0x1000));
    }

    #[test]
    fn test_lock_already_locked() {
        let manager = MemoryLockManager::new();
        let pages = vec![0x1000];

        // 第一次锁定
        assert!(manager.lock_pages(&pages).is_ok());

        // 第二次锁定应该失败
        let result = manager.lock_pages(&pages);
        assert!(matches!(result, Err(UnifiedError::Other(_))));
    }

    #[test]
    fn test_unlock_not_locked() {
        let manager = MemoryLockManager::new();
        let pages = vec![0x1000];

        // 解锁未锁定的页应该失败
        let result = manager.unlock_pages(&pages);
        assert!(matches!(result, Err(UnifiedError::Other(_))));
    }

    #[test]
    fn test_address_space_lock() {
        let lock = AddressSpaceLock::new();
        let flags = AddressSpaceLockFlags {
            current: true,
            future: false,
        };

        // 锁定地址空间
        assert!(lock.lock(flags).is_ok());
        assert!(lock.is_locked());

        // 解锁地址空间
        assert!(lock.unlock().is_ok());
        assert!(!lock.is_locked());
    }
}
