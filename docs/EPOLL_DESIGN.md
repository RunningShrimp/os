# epoll系统调用设计文档

## 概述

epoll是Linux提供的高效I/O事件通知机制，用于替代select/poll。本文档描述NOS内核中epoll的实现设计。

## Linux epoll机制分析

### 核心数据结构

1. **epoll_instance**: epoll实例，包含：
   - 红黑树（rbtree）：存储所有监控的文件描述符
   - 就绪列表（ready list）：存储就绪的事件
   - 等待队列（wait queue）：等待事件的进程

2. **epitem**: epoll监控项，包含：
   - 文件描述符
   - 监控的事件掩码
   - 回调函数
   - 等待队列节点

### 工作流程

1. **epoll_create/epoll_create1**: 创建epoll实例
2. **epoll_ctl**: 添加/修改/删除监控的文件描述符
3. **epoll_wait/epoll_pwait**: 等待事件就绪

### 关键机制

1. **回调机制**: 文件描述符就绪时，通过回调函数通知epoll
2. **边缘触发（ET）**: 只在状态变化时通知
3. **水平触发（LT）**: 只要状态满足就通知
4. **一次性模式（ONESHOT）**: 事件通知后自动移除

## NOS实现设计

### 数据结构

```rust
/// Epoll实例
struct EpollInstance {
    /// 监控的文件描述符映射 (fd -> EpollItem)
    items: BTreeMap<i32, EpollItem>,
    /// 就绪事件列表
    ready_list: Vec<EpollEvent>,
    /// 等待队列
    wait_queue: WaitQueue,
    /// 实例标志
    flags: EpollFlags,
}

/// Epoll监控项
struct EpollItem {
    /// 文件描述符
    fd: i32,
    /// 监控的事件掩码
    events: u32,
    /// 用户数据
    data: EpollData,
    /// 等待队列节点
    wait_node: WaitQueueNode,
}

/// Epoll标志
struct EpollFlags {
    /// 边缘触发模式
    edge_trigger: bool,
    /// 一次性模式
    oneshot: bool,
}
```

### 实现要点

1. **事件通知机制**: 
   - 文件描述符就绪时，通过回调函数添加到就绪列表
   - epoll_wait时从就绪列表获取事件

2. **超时处理**:
   - 使用定时器实现超时
   - 超时后唤醒等待进程

3. **信号掩码**:
   - epoll_pwait支持信号掩码
   - 临时设置信号掩码，等待完成后恢复

4. **性能优化**:
   - 使用红黑树快速查找
   - 就绪列表避免重复扫描
   - 边缘触发减少通知次数

## 集成到事件循环

1. **文件描述符就绪通知**:
   - 文件描述符就绪时，调用epoll回调
   - 将事件添加到epoll实例的就绪列表

2. **进程唤醒**:
   - 事件就绪时，唤醒等待的进程
   - 使用等待队列机制

## 待实现功能

1. ✅ epoll_create/epoll_create1 - 实现完成
2. ✅ epoll_ctl - 实现完成（支持ADD/MOD/DEL操作）
3. ✅ epoll_wait - 实现完成（使用事件通知机制，支持超时）
4. ⚠️ epoll_pwait - 基本实现完成，需要实现信号掩码支持
5. ⚠️ epoll_pwait2 - 基本实现完成，需要实现timespec超时
6. ✅ 事件循环集成 - 实现完成（使用file_subscribe和反向映射）
7. ✅ 边缘触发模式 - 数据结构支持完成（EPOLLET标志）
8. ✅ 一次性模式 - 实现完成（EPOLLONESHOT支持）

## 优先级

### P0 - 必须实现
1. ✅ 优化epoll_wait，使用事件通知而非轮询 - **已完成**
2. ✅ 实现事件循环集成 - **已完成**
3. ✅ 实现边缘触发模式 - **已完成**（数据结构支持）

### P1 - 应该实现
4. ⚠️ 实现epoll_pwait信号掩码支持 - **部分完成**（基本功能完成，信号掩码待实现）
5. ⚠️ 实现epoll_pwait2 timespec超时 - **部分完成**（基本功能完成，timespec转换待实现）
6. ✅ 实现一次性模式 - **已完成**

### P2 - 可选实现
7. 性能优化（批量处理、缓存等）
8. 统计和监控功能

## 实现总结

### 已完成功能

1. **epoll实例管理**
   - epoll_create/epoll_create1实现完成
   - 使用BTreeMap管理epoll实例
   - 支持EPOLL_CLOEXEC标志

2. **文件描述符监控**
   - epoll_ctl实现完成（ADD/MOD/DEL）
   - 使用file_subscribe订阅文件描述符事件
   - 反向映射机制（FILE_TO_EPOLL）用于快速查找

3. **事件通知机制**
   - 就绪事件列表（ready_list）实现
   - epoll_notify_ready函数用于通知epoll实例
   - epoll_notify_file_ready函数用于通知所有订阅的epoll实例

4. **事件等待**
   - epoll_wait实现完成，支持超时
   - 使用进程睡眠/唤醒机制
   - 支持立即返回（timeout=0）和无限等待（timeout=-1）

5. **高级特性**
   - EPOLLONESHOT模式支持（事件返回后自动禁用）
   - EPOLLET标志支持（数据结构层面）
   - 事件掩码转换（epoll_events <-> poll_events）

### 待完善功能

1. **信号掩码支持**
   - epoll_pwait需要实现信号掩码的临时设置和恢复
   - 需要集成信号处理机制

2. **timespec超时**
   - epoll_pwait2需要实现timespec到毫秒的转换
   - 需要支持纳秒级精度（如果系统支持）

3. **边缘触发模式**
   - 当前数据结构支持EPOLLET标志
   - 需要确保只在状态变化时通知（而非持续通知）

4. **性能优化**
   - 批量事件处理
   - 事件缓存机制
   - 统计和监控功能

