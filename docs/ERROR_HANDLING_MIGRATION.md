# 错误处理迁移指南

## 概述

本文档记录了将NOS内核从`Option<T>`和`unwrap()`模式迁移到统一`Result<T, E>`错误处理模式的进度和计划。

## 迁移原则

1. **系统调用层**：保持`Option<T>`返回类型，但内部使用`Result<T, E>`
2. **内核服务层**：逐步迁移到`Result<T, E>`和`KernelError`
3. **组件层**：使用`Result<T, E>`，错误类型可以是模块特定的
4. **避免panic**：所有`unwrap()`和`expect()`调用都应被替换为适当的错误处理

## 已完成迁移的模块

### ✅ 网络模块 (`kernel/src/syscalls/network/`)
- `socket.rs`: 修复了`unwrap()`调用，使用`ok_or()`和`Result`
- `options.rs`: 修复了`unwrap()`调用
- `mod.rs`: 修复了`unwrap()`调用

### ✅ VFS模块 (`kernel/src/vfs/`)
- `tmpfs.rs`: 将`expect()`替换为错误日志记录
- `ramfs.rs`: 将`expect()`替换为错误日志记录

## 待迁移的模块

### ⏳ 进程管理模块 (`kernel/src/process/`)
**当前状态**: 使用`Option<T>`返回类型
**迁移计划**:
- `myproc()`: 保持`Option<Pid>`（系统调用层需要）
- `fork()`: 保持`Option<Pid>`（系统调用层需要）
- `wait()`: 保持`Option<Pid>`（系统调用层需要）
- `fdalloc()`: 保持`Option<i32>`（系统调用层需要）
- `fdlookup()`: 保持`Option<usize>`（系统调用层需要）

**注意**: 这些函数在系统调用层被使用，返回`Option`是合理的。内部实现应使用`Result`进行错误处理。

### ⏳ 文件系统模块 (`kernel/src/fs/`)
**当前状态**: 部分使用`Option<T>`，部分使用`Result<T, E>`
**迁移计划**: 
- 统一所有文件操作返回`Result<T, FsError>`
- 移除所有`unwrap()`调用

### ⏳ 内存管理模块 (`kernel/src/mm/`)
**当前状态**: 需要检查
**迁移计划**: 
- 内存分配函数应返回`Result<T, MemoryError>`
- 移除所有`unwrap()`调用

## 错误处理辅助函数

### `error_handling/unified.rs` 提供的辅助函数

```rust
// 从Option转换为Result
pub fn option_to_result<T>(opt: Option<T>, error: KernelError) -> KernelResult<T>
pub fn option_to_result_not_found<T>(opt: Option<T>) -> KernelResult<T>
pub fn option_to_result_oom<T>(opt: Option<T>) -> KernelResult<T>
```

## 迁移检查清单

- [ ] 扫描所有`unwrap()`调用
- [ ] 扫描所有`expect()`调用
- [ ] 识别返回`Option<T>`的函数
- [ ] 决定是否需要迁移到`Result<T, E>`
- [ ] 更新函数签名
- [ ] 更新调用者代码
- [ ] 添加错误处理测试

## 注意事项

1. **系统调用层兼容性**: 系统调用层函数可以继续使用`Option<T>`，因为POSIX错误码映射在系统调用入口处处理
2. **测试代码**: 测试代码中的`unwrap()`可以保留，但应添加错误路径测试
3. **初始化代码**: 初始化代码中的`expect()`应替换为日志记录或panic（如果确实是致命错误）

## 进度跟踪

- **2024-12-03**: 创建统一错误处理模块
- **2024-12-03**: 修复网络模块错误处理
- **2024-12-03**: 修复VFS模块错误处理
- **下一步**: 继续扫描和修复其他模块

