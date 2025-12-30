//! # 自适应锁使用示例
//!
//! 本文件展示如何在实际场景中使用自适应锁。

use crate::sync::{
    AdaptiveConfig, AdaptiveRwLock, AdaptiveSpinlock, RawSpinLock, RwLock,
};

/// 示例1：简单计数器
///
/// 展示 AdaptiveSpinlock 的基本用法
pub fn example_counter() {
    let counter = AdaptiveSpinlock::new(0);

    // 多个线程可以安全地增加计数器
    {
        let mut data = counter.lock();
        *data += 1;
    }

    // 读取当前值
    let value = *counter.lock();
    assert_eq!(value, 1);
}

/// 示例2：低延迟配置
///
/// 对于短临界区，使用低延迟配置可以获得更好的性能
pub fn example_low_latency() {
    let config = AdaptiveConfig::low_latency();
    let lock = AdaptiveSpinlock::with_config(42, config);

    let data = lock.lock();
    assert_eq!(*data, 42);
}

/// 示例3：节能配置
///
/// 对于长临界区，使用节能配置可以降低CPU功耗
pub fn example_power_saving() {
    let config = AdaptiveConfig::power_saving();
    let lock = AdaptiveSpinlock::with_config(42, config);

    let data = lock.lock();
    assert_eq!(*data, 42);
}

/// 示例4：读写锁
///
/// 展示 AdaptiveRwLock 在读多写少场景的优势
pub fn example_rwlock() {
    let data = AdaptiveRwLock::new(Vec::new());

    // 多个读者可以同时访问
    {
        let r1 = data.read();
        let r2 = data.read();
        let r3 = data.read();
        // 所有读者都可以同时持有锁
        assert_eq!(r1.len(), 0);
        assert_eq!(r2.len(), 0);
        assert_eq!(r3.len(), 0);
    }

    // 写者需要独占访问
    {
        let mut w = data.write();
        w.push(1);
        w.push(2);
        w.push(3);
    }

    // 验证写入的数据
    let r = data.read();
    assert_eq!(r.len(), 3);
}

/// 示例5：获取统计信息
///
/// 展示如何监控锁的性能
pub fn example_stats() {
    let lock = AdaptiveSpinlock::new(0);

    // 模拟一些操作
    for _ in 0..10 {
        let _data = lock.lock();
    }

    // 获取统计信息
    let stats = lock.stats();
    println!("Is locked: {}", stats.is_locked);
    println!("Spin count: {}", stats.spin_count);
    println!("Contention count: {}", stats.contention_count);

    // 重置统计信息
    lock.reset_stats();
}

/// 示例6：读写锁统计
///
/// 展示如何监控读写锁的状态
pub fn example_rwlock_stats() {
    let lock = AdaptiveRwLock::new(42);

    // 获取初始统计
    let stats = lock.stats();
    assert_eq!(stats.active_readers, 0);
    assert_eq!(stats.active_writers, 0);

    // 添加读者
    let _r1 = lock.read();
    let _r2 = lock.read();
    let stats = lock.stats();
    assert_eq!(stats.active_readers, 2);

    // 释放读者
    drop(_r1);
    drop(_r2);

    // 添加写者
    let _w = lock.write();
    let stats = lock.stats();
    assert_eq!(stats.active_writers, 1);
}

/// 示例7：try_lock 非阻塞操作
///
/// 展示如何避免阻塞
pub fn example_try_lock() {
    let lock = AdaptiveSpinlock::new(42);

    // 尝试获取锁
    if let Some(_data) = lock.try_lock() {
        // 成功获取锁
        // assert_eq!(*data, 42);
    } else {
        // 锁已被其他线程持有
        // println!("Lock is busy");
    }

    // 在持有锁的情况下尝试
    let _guard = lock.lock();
    assert!(lock.try_lock().is_none());
}

/// 示例8：try_read / try_write
///
/// 展示读写锁的非阻塞操作
pub fn example_try_rw_lock() {
    let lock = AdaptiveRwLock::new(42);

    // 尝试读锁
    if let Some(_r) = lock.try_read() {
        // assert_eq!(*r, 42);
    } else {
        // println!("Cannot acquire read lock");
    }

    // 尝试写锁
    if let Some(mut _w) = lock.try_write() {
        // *w = 100;
    } else {
        // println!("Cannot acquire write lock");
    }

    // 在持有读锁时尝试写锁
    let _r = lock.read();
    assert!(lock.try_write().is_none());
}

/// 示例9：性能对比
///
/// 展示不同锁在不同场景下的性能特征
pub fn example_performance_comparison() {
    // 场景1：无竞争或低竞争
    // Spinlock 可能更快
    let spinlock = RawSpinLock::new();
    let adaptive = AdaptiveSpinlock::new(0);

    spinlock.lock();
    adaptive.lock();
    spinlock.unlock();
    drop(adaptive);

    // 场景2：高竞争
    // AdaptiveSpinlock 会表现更好
    // (需要多线程才能真正展示差异)
}

/// 示例10：复杂数据结构
///
/// 展示如何保护复杂数据结构
#[derive(Debug)]
struct SharedData {
    counter: usize,
    // values: Vec<usize>,
}

pub fn example_complex_data() {
    let data = AdaptiveSpinlock::new(SharedData {
        counter: 0,
        // values: Vec::new(),
    });

    // 修改数据
    {
        let mut d = data.lock();
        d.counter += 1;
        // d.values.push(d.counter);
    }

    // 读取数据
    {
        let d = data.lock();
        assert_eq!(d.counter, 1);
        // assert_eq!(d.values.len(), 1);
    }
}

/// 示例11：配置优化
///
/// 展示如何根据工作负载调整配置
pub fn example_configuration_tuning() {
    // 短临界区配置
    let short_critical_section = AdaptiveConfig {
        fast_spin_limit: 20,  // 更多快速自旋
        backoff_limit: 50,    // 更快进入退避
        max_backoff: 32,      // 较小的退避值
        long_wait_pause: 128, // 较短的长期等待
    };

    // 长临界区配置
    let long_critical_section = AdaptiveConfig {
        fast_spin_limit: 5,   // 快速放弃自旋
        backoff_limit: 50,    // 尽快退避
        max_backoff: 128,     // 较大的退避值
        long_wait_pause: 512, // 较长的长期等待
    };

    let lock1 = AdaptiveSpinlock::with_config(1, short_critical_section);
    let lock2 = AdaptiveSpinlock::with_config(2, long_critical_section);

    // 根据实际场景选择合适的配置
    let _data1 = lock1.lock();
    let _data2 = lock2.lock();
}

/// 示例12：读写锁的写者优先策略
///
/// 展示 AdaptiveRwLock 如何防止写者饥饿
pub fn example_writer_priority() {
    let lock = AdaptiveRwLock::new(0);

    // 创建多个读者
    let _r1 = lock.read();
    let _r2 = lock.read();

    // 写者会等待所有读者完成
    // 但写者会优先于新读者（通过waiting_writers计数）

    // 新的读者会检测到等待的写者并等待
    // 这确保了写者不会饿死

    let stats = lock.stats();
    println!("Active readers: {}", stats.active_readers);
    println!("Waiting writers: {}", stats.waiting_writers);
}

/// 示例13：与现有代码集成
///
/// 展示如何将现有代码迁移到自适应锁
pub fn example_migration() {
    // 旧代码：使用普通 SpinLock
    let _old_lock = RawSpinLock::new();

    // 新代码：直接替换为 AdaptiveSpinlock
    let new_lock = AdaptiveSpinlock::new(42);
    // 使用方式完全相同
    let _data = new_lock.lock();

    // 对于 RwLock 也是类似
    let _old_rwlock = RwLock::new(42);
    let new_rwlock = AdaptiveRwLock::new(42);

    // API 几乎完全相同
    let _r = new_rwlock.read();
    let _w = new_rwlock.write();
}

/// 示例14：错误处理
///
/// 展示如何处理锁操作中的边界情况
pub fn example_error_handling() {
    let lock = AdaptiveSpinlock::new(42);

    // 使用 try_lock 避免死锁
    if let Some(_data) = lock.try_lock() {
        // 成功获取锁
        // assert_eq!(*data, 42);
    } else {
        // 处理锁不可用的情况
        // println!("Lock is contended, retry later");
    }

    // 检查锁状态
    if lock.is_locked() {
        // println!("Lock is currently held");
    }
}

/// 示例15：自定义配置
///
/// 展示如何创建完全自定义的配置
pub fn example_custom_config() {
    let custom_config = AdaptiveConfig {
        fast_spin_limit: 15,
        backoff_limit: 75,
        max_backoff: 48,
        long_wait_pause: 200,
    };

    let lock = AdaptiveSpinlock::with_config(42, custom_config);

    // 使用自定义配置的锁
    let _data = lock.lock();

    // 查看统计信息以验证配置效果
    let stats = lock.stats();
    println!("Contention count: {}", stats.contention_count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_example() {
        example_counter();
    }

    #[test]
    fn test_low_latency() {
        example_low_latency();
    }

    #[test]
    fn test_power_saving() {
        example_power_saving();
    }

    #[test]
    fn test_rwlock_example() {
        example_rwlock();
    }

    #[test]
    fn test_stats_example() {
        example_stats();
    }

    #[test]
    fn test_rwlock_stats_example() {
        example_rwlock_stats();
    }

    #[test]
    fn test_try_lock_example() {
        example_try_lock();
    }

    #[test]
    fn test_try_rw_lock_example() {
        example_try_rw_lock();
    }

    #[test]
    fn test_complex_data() {
        example_complex_data();
    }

    #[test]
    fn test_configuration_tuning() {
        example_configuration_tuning();
    }

    #[test]
    fn test_writer_priority() {
        example_writer_priority();
    }

    #[test]
    fn test_migration() {
        example_migration();
    }

    #[test]
    fn test_error_handling() {
        example_error_handling();
    }

    #[test]
    fn test_custom_config() {
        example_custom_config();
    }
}
