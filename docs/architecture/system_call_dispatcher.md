# 系统调用分发器架构设计文档

## 概述

NOS操作系统使用统一的系统调用分发器（UnifiedSyscallDispatcher）来处理所有系统调用请求。该分发器整合了多个旧实现的最佳特性，提供了高性能、可扩展的系统调用处理机制。

## 架构设计

### 核心组件

1. **UnifiedSyscallDispatcher**
   - 位置：`kernel/src/subsystems/syscalls/dispatch/unified.rs`
   - 功能：统一的系统调用分发器，整合了快速路径、per-CPU缓存、处理器注册等特性

2. **快速路径优化**
   - 256个常用系统调用的直接跳转表
   - O(1)时间复杂度的快速路径处理
   - 支持自适应优化

3. **Per-CPU缓存**
   - 每个CPU独立的缓存，减少锁竞争
   - 频率统计和自适应快速路径更新

4. **处理器注册机制**
   - 动态注册系统调用处理器
   - 支持处理器优先级和范围映射

## 性能特性

- **快速路径**：常用系统调用（getpid、gettid等）直接跳转，延迟极低
- **Per-CPU缓存**：减少跨CPU锁竞争，提高并发性能
- **自适应优化**：根据调用频率动态调整快速路径
- **批量处理**：支持批量系统调用处理

## 使用示例

```rust
use kernel::subsystems::syscalls::dispatch::unified::{
    UnifiedSyscallDispatcher, UnifiedDispatcherConfig, init_unified_dispatcher
};

// 初始化统一分发器
let config = UnifiedDispatcherConfig::default();
init_unified_dispatcher(config);

// 分发系统调用
use kernel::subsystems::syscalls::dispatch::unified::unified_dispatch;
let result = unified_dispatch(syscall_num, args);
```

## 迁移指南

旧的系统调用分发器实现已被标记为deprecated：
- `kernel/src/syscall/fast_path_dispatcher.rs` - 已删除
- `kernel/src/subsystems/syscalls/fast_dispatcher.rs` - 已删除
- `kernel/src/subsystems/syscalls/unified_dispatcher.rs` - 已删除

请使用新的 `UnifiedSyscallDispatcher` 替代。


