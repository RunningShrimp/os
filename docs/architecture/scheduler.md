# 调度器架构设计文档

## 概述

NOS操作系统使用统一的调度器（UnifiedScheduler）实现O(log n)时间复杂度的调度决策，替代了原有的O(n)线性搜索。调度器支持per-CPU运行队列和工作窃取机制。

## 架构设计

### 核心组件

1. **UnifiedScheduler**
   - 位置：`kernel/src/subsystems/scheduler/unified.rs`
   - 功能：统一的调度器实现，使用优先级队列

2. **优先级队列**
   - 使用BTreeMap实现O(log n)操作
   - 支持多级优先级（0-255）
   - FIFO排序用于相同优先级的线程

3. **Per-CPU运行队列**
   - 每个CPU独立的运行队列
   - 减少锁竞争
   - 支持CPU亲和性

4. **工作窃取**
   - 当本地队列为空时，从其他CPU窃取工作
   - 实现负载均衡

## 性能改进

### 旧实现（O(n)）
- 线性搜索所有线程（最多1024个）
- 每次调度需要遍历整个线程表
- 时间复杂度：O(n)

### 新实现（O(log n)）
- 优先级队列快速查找
- 只搜索就绪线程
- 时间复杂度：O(log n)

## 调度策略

- **FIFO**：实时FIFO调度，无限时间片
- **RoundRobin**：实时轮询调度，10ms时间片
- **Normal**：普通时间分片调度，10ms时间片
- **Batch**：批处理调度，50ms时间片
- **Idle**：空闲调度，100ms时间片

## 使用示例

```rust
use kernel::subsystems::scheduler::unified::{
    UnifiedScheduler, init_unified_scheduler, unified_schedule
};

// 初始化统一调度器
init_unified_scheduler(num_cpus);

// 调度下一个线程
if let Some(next_tid) = unified_schedule() {
    // 切换到next_tid
}
```



