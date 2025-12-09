# NOS TODO清理计划

## 概述
当前代码库中存在200+个TODO标记，需要系统性地清理和实现。本文档按优先级和模块分类所有TODO项。

**目标**: 将TODO数量从261个减少到50个以下

## TODO统计分析

### 按模块分类
| 模块 | TODO数量 | 优先级 | 预计工时 |
|------|----------|--------|----------|
| 网络栈 (syscalls/net) | 45 | 中 | 3周 |
| IPC模块 (syscalls/ipc) | 28 | 中 | 2周 |
| 内存管理 (syscalls/mm) | 35 | 高 | 3周 |
| 文件系统 (syscalls/fs) | 25 | 高 | 2周 |
| 进程管理 (syscalls/process) | 22 | 高 | 2周 |
| 信号处理 (syscalls/signal) | 15 | 中 | 1周 |
| 线程管理 (syscalls/thread) | 18 | 中 | 1.5周 |
| 其他系统调用 | 20 | 低 | 1周 |

### 按严重性分类
- **Critical (阻塞核心功能)**: 45个
- **High (影响性能和稳定性)**: 80个
- **Medium (功能不完整)**: 90个
- **Low (优化和增强)**: 46个

## 高优先级TODO项 (立即处理)

### 1. 内存管理模块
#### 文件: `kernel/src/syscalls/mm/handlers.rs`
- **TODO**: Handle file-backed mappings (行192)
  - **影响**: 无法支持mmap文件映射，严重限制功能
  - **预计工时**: 3天
  - **依赖**: VFS文件操作接口

- **TODO**: Implement proper physical page tracking for aarch64 (行261)
  - **影响**: ARM64架构内存泄漏风险
  - **预计工时**: 2天
  - **依赖**: 架构特定页表操作

- **TODO**: Implement proper unmapping for x86_64 (行271)
  - **影响**: x86_64架构内存泄漏
  - **预计工时**: 2天
  - **依赖**: 架构特定页表操作

- **TODO**: Properly unmap and free pages (行643)
  - **影响**: 内存泄漏，系统稳定性问题
  - **预计工时**: 1天
  - **依赖**: 页表管理API

#### 文件: `kernel/src/syscalls/mm/service.rs`
- **TODO**: 实现实际的内存区域分配 (行111)
  - **影响**: 内存服务架构不完整
  - **预计工时**: 3天
  - **依赖**: 内存管理器重构

### 2. 进程管理模块
#### 文件: `kernel/src/syscalls/process.rs`
- **TODO**: Implement rusage support (行470)
  - **影响**: 无法获取进程资源使用统计
  - **预计工时**: 2天
  - **依赖**: 进程计数器

- **TODO**: Implement proper physical page tracking for aarch64 (行1053, 1104)
  - **影响**: ARM64进程内存管理问题
  - **预计工时**: 2天
  - **依赖**: 架构层抽象

#### 文件: `kernel/src/syscalls/process_service/handlers.rs`
所有核心进程操作都是TODO占位符：
- **fork逻辑** (行30) - 2天
- **execve逻辑** (行58) - 3天
- **waitpid逻辑** (行87) - 1天
- **exit逻辑** (行114) - 1天
- **kill逻辑** (行141) - 1天

### 3. 文件系统模块
#### 文件: `kernel/src/syscalls/fs/mod.rs`
- **TODO**: Implement proper filesystem mounting (行96)
  - **影响**: 无法正确挂载文件系统
  - **预计工时**: 3天
  - **依赖**: VFS重构

#### 文件: `kernel/src/syscalls/fs_service/handlers.rs`
所有文件操作都是TODO占位符：
- **open逻辑** (行34) - 2天
- **read/write逻辑** (行89, 118) - 3天
- **stat/fstat逻辑** (行175, 203) - 2天
- **目录操作** (行230, 256, 282) - 2天

## 中优先级TODO项 (3-4个月内处理)

### 4. 网络栈模块
#### 文件: `kernel/src/syscalls/net/service.rs`
网络协议栈框架存在但未实现：
- **创建真实套接字** (行307) - 5天
- **用户空间拷贝** (行372, 434, 473) - 3天
- **数据传输功能** (行507, 539) - 5天
- **套接字选项** (行658, 687) - 2天
- **网络服务生命周期** (行719-751) - 3天

#### 文件: `kernel/src/syscalls/net/handlers.rs`
基础用户空间交互函数缺失：
- **用户空间数据拷贝** (行12, 26) - 2天
- **地址结构验证** (行65) - 1天
- **地址长度处理** (行39, 52) - 1天

### 5. IPC模块
#### 文件: `kernel/src/syscalls/ipc/handlers.rs`
System V IPC全部未实现：
- **管道操作** (行33, 60) - 2天
- **共享内存** (行88, 116, 142, 170) - 4天
- **消息队列** (行197, 226, 257, 286) - 4天
- **信号量** (行314, 342, 371) - 3天

#### 文件: `kernel/src/syscalls/ipc/service.rs`
元数据缺失：
- **获取当前进程ID** (行138, 169, 200, 231) - 1天
- **获取当前时间** (行139, 170, 201, 232) - 1天

### 6. 信号处理模块
#### 文件: `kernel/src/syscalls/signal_service/service.rs`
- **实际的信号发送** (行180) - 2天
- **实际的信号等待** (行199) - 2天
- **实际的信号处理** (行237) - 3天
- **服务生命周期管理** (行312-344) - 2天

### 7. 线程管理模块
#### 文件: `kernel/src/syscalls/thread.rs`
- **命名空间支持** (行260, 505, 530-553) - 5天
- **CLONE_CHILD_CLEARTID支持** (行573, 578) - 2天
- **futex原子操作** (行812) - 3天
- **优先级继承(PI)机制** (行822) - 4天
- **futex超时处理** (行829) - 2天

## 低优先级TODO项 (长期优化)

### 8. 高级内存功能
#### 文件: `kernel/src/syscalls/memory/advanced_mmap.rs`
所有高级内存功能占位符：
- **madvise功能** (行40) - 2天
- **mlock/munlock功能** (行57, 74) - 2天
- **mlockall/munlockall功能** (行90, 100) - 1天
- **mincore功能** (行118) - 2天
- **remap_file_pages功能** (行138) - 3天

### 9. 定时器和时间管理
#### 文件: `kernel/src/syscalls/glib.rs`
- **注册定时器到内核** (行400, 445) - 2天
- **从VFS获取设备号** (行625, 626) - 1天

### 10. 调试管理器
#### 文件: `kernel/src/debug/manager.rs`
- **加载默认插件** (行110) - 2天
- **线程堆栈信息** (行341) - 1天

### 11. 其他系统调用
#### 文件: `kernel/src/syscalls/file_io.rs`
- **映射剩余系统调用ID** (行38) - 1天

#### 文件: `kernel/src/syscalls/mqueue.rs`
- **timed send阻塞和超时** (行261) - 2天
- **timed receive阻塞和超时** (行368) - 2天

#### 文件: `kernel/src/syscalls/network/`
- **sendmsg系统调用** (行246) - 2天
- **recvmsg系统调用** (行252) - 2天
- **getsockname系统调用** (行62) - 1天
- **getpeername系统调用** (行68) - 1天
- **socketpair系统调用** (行658) - 2天

## 实施计划

### 第1周：进程管理核心功能
- [ ] 实现fork/execve/exit/waitpid逻辑
- [ ] 实现getpid/getppid逻辑
- [ ] 实现rusage支持

### 第2周：文件系统核心功能
- [ ] 实现open/close/read/write逻辑
- [ ] 实现stat/fstat逻辑
- [ ] 实现目录操作(mkdir/rmdir/unlink)

### 第3-4周：内存管理优化
- [ ] 实现文件映射支持
- [ ] 修复ARM64/x86_64页表操作
- [ ] 实现proper page unmapping
- [ ] 实现内存区域分配服务

### 第5-6周：IPC基础设施
- [ ] 实现管道和FIFO
- [ ] 实现共享内存(shmget/shmat/shmdt/shmctl)
- [ ] 实现消息队列基础
- [ ] 实现信号量基础

### 第7-9周：网络协议栈
- [ ] 实现用户空间数据拷贝机制
- [ ] 实现真实套接字创建和管理
- [ ] 实现基础数据传输(send/recv)
- [ ] 实现套接字选项管理
- [ ] 实现网络服务生命周期

### 第10-11周：信号和线程管理
- [ ] 实现信号发送和等待
- [ ] 实现信号处理器安装
- [ ] 实现CLONE_CHILD_CLEARTID
- [ ] 实现futex基础原子操作

### 第12周：清理和验证
- [ ] 移除已完成的TODO标记
- [ ] 为剩余TODO添加详细注释和跟踪issue
- [ ] 更新文档
- [ ] 运行全面测试

## TODO标记规范

为剩余的TODO项制定新的标准格式：

```rust
// TODO(priority: high, issue: #123, eta: 2025-12-20):
// 简要描述需要做什么
// 为什么暂时无法实现（缺少什么依赖）
// 临时的workaround是什么
```

## 成功指标

- [ ] TODO数量从261减少到50以下
- [ ] 所有高优先级TODO在1个月内完成
- [ ] 核心功能模块(进程、内存、文件系统)无critical TODO
- [ ] 每个剩余TODO都有对应的GitHub issue跟踪
- [ ] 所有TODO都有预计完成时间和责任人

## 风险和依赖

### 主要风险
1. **架构重构依赖**: 许多TODO需要syscalls模块解耦后才能实现
2. **跨模块依赖**: 网络栈需要内存管理和文件系统支持
3. **测试覆盖**: 大量功能实现后需要全面测试

### 缓解措施
1. 优先实现无依赖的独立功能
2. 采用渐进式实现，保留占位符作为fallback
3. 每周进行代码review和进度跟踪
4. 建立自动化测试保护已实现功能

## 进度跟踪

- **开始日期**: 2025-12-09
- **目标完成日期**: 2026-03-09 (3个月)
- **当前进度**: 0/261 (0%)
- **本周目标**: 完成进程管理核心功能 (10个TODO)
