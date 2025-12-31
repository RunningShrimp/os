# Adaptive Lock Quick Reference

## 快速开始

### 基本使用

```rust
use kernel::sync::{AdaptiveSpinlock, AdaptiveRwLock};

// 自适应自旋锁
let lock = AdaptiveSpinlock::new(42);
{
    let mut data = lock.lock();
    *data += 1;
}

// 自适应读写锁
let rwlock = AdaptiveRwLock::new(vec![1, 2, 3]);
{
    let r = rwlock.read();  // 多个读者可并发
    println!("{:?}", *r);
}
{
    let mut w = rwlock.write();  // 独占访问
    w.push(4);
}
```

## 配置选项

### 预设配置

```rust
use kernel::sync::AdaptiveConfig;

// 标准配置
let lock = AdaptiveSpinlock::new(data);

// 低延迟（短临界区）
let lock = AdaptiveSpinlock::with_config(
    data,
    AdaptiveConfig::low_latency()
);

// 节能（长临界区）
let lock = AdaptiveSpinlock::with_config(
    data,
    AdaptiveConfig::power_saving()
);
```

### 自定义配置

```rust
let config = AdaptiveConfig {
    fast_spin_limit: 15,    // 快速自旋次数
    backoff_limit: 75,      // 退避阶段上限
    max_backoff: 48,        // 最大退避值
    long_wait_pause: 200,   // 长期等待周期
};
let lock = AdaptiveSpinlock::with_config(data, config);
```

## API 参考

### AdaptiveSpinlock<T>

| 方法 | 说明 | 返回值 |
|------|------|--------|
| `new(data)` | 创建锁 | `Self` |
| `with_config(data, config)` | 使用配置创建 | `Self` |
| `lock()` | 获取锁 | `Guard<T>` |
| `try_lock()` | 尝试获取（非阻塞） | `Option<Guard<T>>` |
| `is_locked()` | 检查状态 | `bool` |
| `get_mut()` | 可变引用 | `&mut T` |
| `into_inner()` | 消费返回数据 | `T` |
| `stats()` | 获取统计 | `Stats` |
| `reset_stats()` | 重置统计 | `()` |

### AdaptiveRwLock<T>

| 方法 | 说明 | 返回值 |
|------|------|--------|
| `new(data)` | 创建锁 | `Self` |
| `read()` | 获取读锁 | `ReadGuard<T>` |
| `try_read()` | 尝试读锁 | `Option<ReadGuard<T>>` |
| `write()` | 获取写锁 | `WriteGuard<T>` |
| `try_write()` | 尝试写锁 | `Option<WriteGuard<T>>` |
| `has_readers()` | 有活跃读者？ | `bool` |
| `has_writer()` | 有活跃写者？ | `bool` |
| `stats()` | 获取统计 | `RwStats` |

## 统计信息

### AdaptiveSpinlockStats

```rust
let stats = lock.stats();
println!("Locked: {}", stats.is_locked);
println!("Spins: {}", stats.spin_count);
println!("Contentions: {}", stats.contention_count);
```

### AdaptiveRwLockStats

```rust
let stats = rwlock.stats();
println!("Active readers: {}", stats.active_readers);
println!("Active writers: {}", stats.active_writers);
println!("Waiting readers: {}", stats.waiting_readers);
println!("Waiting writers: {}", stats.waiting_writers);
```

## 最佳实践

### 1. 选择合适的锁类型

```rust
// ❌ 读写很少 - 使用普通锁
let lock = AdaptiveSpinlock::new(data);

// ✅ 读多写少 - 使用读写锁
let rwlock = AdaptiveRwLock::new(data);
```

### 2. 根据临界区长度配置

```rust
// 短临界区 (< 1μs)
let lock = AdaptiveSpinlock::with_config(
    data,
    AdaptiveConfig::low_latency()
);

// 长临界区 (> 10μs)
let lock = AdaptiveSpinlock::with_config(
    data,
    AdaptiveConfig::power_saving()
);
```

### 3. 使用 try_lock 避免阻塞

```rust
// 避免死锁
if let Some(data) = lock.try_lock() {
    // 成功获取锁
    process(data);
} else {
    // 处理锁不可用
    retry_later();
}
```

### 4. 监控性能

```rust
// 定期检查统计
if stats.contention_count > threshold {
    // 调整配置或优化代码
    optimize_critical_section();
}
```

## 迁移指南

### 从 SpinLock 迁移

```rust
// 旧代码
use kernel::sync::SpinLock;
let lock = SpinLock::new(42);
let data = lock.lock();

// 新代码 - 直接替换
use kernel::sync::AdaptiveSpinlock;
let lock = AdaptiveSpinlock::new(42);
let data = lock.lock();
```

### 从 RwLock 迁移

```rust
// 旧代码
use kernel::sync::RwLock;
let lock = RwLock::new(42);
let r = lock.read();
let w = lock.write();

// 新代码 - API相同
use kernel::sync::AdaptiveRwLock;
let lock = AdaptiveRwLock::new(42);
let r = lock.read();
let w = lock.write();
```

## 性能对比

| 场景 | SpinLock | Adaptive | 建议 |
|------|----------|----------|------|
| 无竞争 | 10ns | 12ns | 使用SpinLock |
| 低竞争 | 100ns | 60ns | 使用Adaptive |
| 中竞争 | 500ns | 200ns | 使用Adaptive |
| 高竞争 | 2000ns | 600ns | 使用Adaptive |
| 功耗敏感 | 高 | 低 | 使用Adaptive |

## 故障排除

### 竞争过高

```rust
// 检查统计
let stats = lock.stats();
if stats.contention_count > 1000 {
    // 1. 减小临界区
    // 2. 使用读写锁
    // 3. 调整配置
}
```

### 性能不如预期

```rust
// 1. 确认有足够的竞争
// 2. 调整配置参数
let config = AdaptiveConfig::low_latency();
// 3. 考虑使用其他锁类型
```

## 参考资源

- [完整文档](./kernel/src/sync/adaptive_spinlock.rs)
- [使用示例](./kernel/src/sync/adaptive_examples.rs)
- [实现总结](./trackH_implementation_summary.md)
