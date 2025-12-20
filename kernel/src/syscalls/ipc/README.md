# Inter-Process Communication System Calls Module

## 概述

此模块包含所有与进程间通信（IPC）相关的系统调用实现，包括管道、消息队列、信号量、共享内存等机制。

## 系统调用范围

### 管道通信 (Pipe-based IPC)
- `pipe()` - 创建匿名管道
- `pipe2()` - 创建管道（带选项）
- `mkfifo()` - 创建命名管道

### 消息队列
- `msgget()` - 获取消息队列标识符
- `msgsnd()` - 发送消息到队列
- `msgrcv()` - 从队列接收消息
- `msgctl()` - 控制消息队列

### 信号量
- `semget()` - 获取信号量集
- `semop()` - 信号量操作
- `semctl()` - 控制信号量

### 共享内存
- `shmget()` - 获取共享内存段
- `shmat()` - 附加共享内存段
- `shmdt()` - 分离共享内存段
- `shmctl()` - 控制共享内存

### 其他IPC机制
- `mq_open()` - 打开消息队列
- `mq_send()` - 发送消息
- `mq_receive()` - 接收消息

## 相关文件

- `pipe.rs` - 管道相关系统调用
- `mqueue.rs` - 消息队列系统调用
- `mod.rs` - 模块声明和导出

## 架构依赖

依赖以下内核子系统：
- `crate::ipc::pipe` - 管道实现
- `crate::ipc::message_queue` - 消息队列实现
- `crate::ipc::semaphore` - 信号量实现
- `crate::ipc::shared_memory` - 共享内存实现

## 重构目标

根据分析报告，IPC模块具有中等复杂度，通过模块化拆分可以有效降低系统耦合度。当前代码分布在多个文件中，计划进行统一重组，实现更好的抽象和接口隔离。