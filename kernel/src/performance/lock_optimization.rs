//! 锁优化指南和示例
//!
//! 本模块展示如何优化锁的使用模式，减少临界区和锁竞争。

use crate::subsystems::sync::Mutex;

/// 锁优化示例：缩小临界区
///
/// 反模式：持有锁时间过长
pub fn anti_pattern_example(data: &Mutex<Vec<u8>>) {
    let guard = data.lock();
    // 在持有锁的情况下执行慢速操作
    process_slowly(&guard);
    drop(guard);
}

/// 优化模式：复制数据，缩小临界区
pub fn optimized_pattern_example(data: &Mutex<Vec<u8>>) {
    // 快速复制数据，缩小临界区
    let snapshot = {
        let data = data.lock();
        data.clone() // 快速操作
    };

    // 在锁外处理数据
    process_slowly(&snapshot);
}

/// 批处理优化：批量操作减少锁获取次数
pub fn batch_processing_example(data: &Mutex<Vec<u8>>, items: &[u8]) {
    let mut data = data.lock();

    // 批量添加，只获取一次锁
    for &item in items {
        data.push(item);
    }

    // 一次性处理
    process_batch(&data);
}

/// 读-复制-更新(RCU)模式
///
/// 适用于读多写少的场景
pub struct RcuOptimizedData<T> {
    data: Mutex<Box<T>>,
}

impl<T: Clone> RcuOptimizedData<T> {
    /// 快速读取（无锁）
    pub fn read_fast(&self) -> T {
        let data = self.data.lock();
        // 克隆数据并立即释放锁
        (*data).clone()
    }

    /// 写入（需要锁）
    pub fn write(&self, new_data: T) {
        let mut data = self.data.lock();
        *data = Box::new(new_data);
    }
}

/// 分段锁：减少锁竞争
///
/// 将大锁拆分为多个小锁
pub struct ShardedLock<T> {
    shards: [Mutex<T>; 16],
}

impl<T: Default> Default for ShardedLock<T> {
    fn default() -> Self {
        Self {
            shards: [
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
                Mutex::new(T::default()),
            ],
        }
    }
}

impl<T> ShardedLock<T> {
    /// 根据key选择shard，减少竞争
    fn get_shard(&self, key: usize) -> &Mutex<T> {
        &self.shards[key % 16]
    }

    /// 操作指定shard
    pub fn operate_on_shard<F>(&self, key: usize, f: F)
    where
        F: FnOnce(&T),
    {
        let shard = self.get_shard(key);
        let data = shard.lock();
        f(&data);
    }
}

/// 原子操作优化：避免使用锁
///
/// 对于简单计数器，使用原子操作替代锁
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct AtomicCounter {
    count: AtomicUsize,
}

impl AtomicCounter {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    /// 无锁递增
    #[inline(always)]
    pub fn increment(&self) -> usize {
        self.count.fetch_add(1, Ordering::Relaxed)
    }

    /// 无锁读取
    #[inline(always)]
    pub fn get(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
}

// 辅助函数（仅用于示例）
fn process_slowly<T>(_data: &[T]) {
    // 慢速处理
}

fn process_batch<T>(_data: &[T]) {
    // 批量处理
}

/// 性能优化技巧总结：
///
/// 1. **缩小临界区**：
///    - 只在必要时持有锁
///    - 复制数据，在锁外处理
///
/// 2. **减少锁持有时间**：
///    - 避免在锁内进行I/O操作
///    - 避免在锁内进行复杂计算
///
/// 3. **使用合适的锁类型**：
///    - 读多写少：使用RwLock
///    - 短临界区：使用SpinLock
///    - 长临界区：使用Mutex
///
/// 4. **避免嵌套锁**：
///    - 重构代码避免嵌套
///    - 使用更大的锁替代多个小锁
///
/// 5. **使用无锁数据结构**：
///    - 简单计数器：原子操作
///    - 队列：无锁队列
///    - 环形缓冲：无锁实现
///
/// 6. **分段锁**：
///    - 将大锁拆分为多个小锁
///    - 根据key路由到不同锁
///
/// 7. **批处理**：
///    - 合并多个操作
///    - 减少锁获取次数
