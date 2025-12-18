# Signal Processing System Calls Module

## 概述

此模块包含所有与信号处理相关的系统调用实现，包括信号发送、接收、阻塞和处理等功能。

## 系统调用范围

### 信号管理
- `kill()` - 向进程发送信号
- `tkill()` - 向特定线程发送信号
- `tgkill()` - 向线程组发送信号

### 信号配置
- `sigaction()` - 检查或更改信号行为
- `signal()` - 简化版信号处理设置
- `sigsuspend()` - 等待信号
- `sigaltstack()` - 设置信号堆栈

### 信号掩码操作
- `sigprocmask()` - 检查或更改阻塞信号掩码
- `sigpending()` - 检查等待中的信号
- `sigwaitinfo()` - 同步等待信号

### 实 时信号
- `rt_sigaction()` - 实时信号行为设置
- `rt_sigprocmask()` - 实时信号掩码操作
- `rt_sigpending()` - 实时等待信号检查
- `rt_sigtimedwait()` - 带超时的实时信号等待
- `rt_sigqueueinfo()` - 队列实时信号

### 信号队列
- `rt_tgsigqueueinfo()` - 向线程组队列信号

## 相关文件

- `signal.rs` - 核心信号处理系统调用
- `advanced_signal.rs` - 高级信号功能
- `mod.rs` - 模块声明和导出

## 架构依赖

依赖以下内核子系统：
- `crate::signal::manager` - 信号管理器
- `crate::signal::handler` - 信号处理程序
- `crate::signal::queue` - 信号队列机制

## 重构目标

根据分析报告，信号处理模块复杂度适中，拆分潜力为★★★☆☆。通过模块重组可以提高代码组织性，并为未来的异步信号处理架构奠定基础。