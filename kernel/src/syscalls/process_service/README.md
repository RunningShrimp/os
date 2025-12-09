# Process Management System Calls Module

## 概述

此模块包含所有与进程管理相关的系统调用实现，包括进程创建、销毁、状态管理等核心功能。

## 系统调用范围

### 进程生命周期管理 (0x1000-0x1FFF)
- `fork()` - 创建子进程
- `execve()` - 执行新程序
- `wait4()` - 等待子进程结束
- `exit()` - 进程退出
- `kill()` - 发送信号给进程

### 进程信息查询
- `getpid()` - 获取当前进程ID
- `getppid()` - 获取父进程ID
- `getpgid()` - 获取进程组ID
- `setpgid()` - 设置进程组ID

### 进程调度和控制
- `sched_setscheduler()` - 设置调度策略
- `sched_getscheduler()` - 获取调度策略
- `sched_yield()` - 让出CPU时间片

## 相关文件

- `process.rs` - 核心进程管理系统调用
- `thread.rs` - 线程相关系统调用（未来迁移）
- `advanced_thread.rs` - 高级线程功能
- `mod.rs` - 模块声明和导出

## 架构依赖

依赖以下内核子系统：
- `crate::process::manager` - 进程管理器
- `crate::scheduler` - 处理器调度器
- `crate::memory::vm` - 虚拟内存管理

## 重构目标

根据分析报告，此模块应该独立成为一个服务，提供进程管理的统一接口，避免与其他模块的紧耦合，从而提高性能和可维护性。