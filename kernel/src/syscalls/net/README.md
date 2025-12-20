# Network System Calls Module

## 概述

此模块包含所有与网络通信相关的系统调用实现，包括套接字接口、网络连接、高级网络特性等功能。

## 系统调用范围

### 基础套接字操作 (0x4000-0x4FFF)
- `socket()` - 创建新的套接字
- `bind()` - 绑定套接字到地址
- `listen()` - 监听连接请求
- `accept()` - 接受客户端连接
- `connect()` - 连接到远程套接字
- `shutdown()` - 关闭套接字的双向通信
- `close()` - 关闭套接字

### 数据传输
- `send()` - 发送数据到套接字
- `recv()` - 从套接字接收数据
- `sendto()` - 发送数据到指定地址
- `recvfrom()` - 从指定地址接收数据
- `sendmsg()` - 发送分散-聚集数据
- `recvmsg()` - 接收分散-聚集数据

### 套接字选项和控制
- `getsockopt()` - 获取套接字选项
- `setsockopt()` - 设置套接字选项
- `getsockname()` - 获取套接字本地地址
- `getpeername()` - 获取套接字对端地址

### 高级网络特性
- `epoll_create()` - 创建epoll实例
- `epoll_ctl()` - 控制epoll实例
- `epoll_wait()` - 等待事件发生

## 相关文件

- `network.rs` - 核心网络系统调用（应迁移到此目录）
- `epoll.rs` - epoll事件机制（位于glib_epoll/目录）
- `mod.rs` - 模块声明和导出

## 架构依赖

依赖以下内核子系统：
- `crate::net::socket` - 套接字抽象层
- `crate::net::tcp` - TCP协议实现
- `crate::net::udp` - UDP协议实现
- `crate::net::epoll` - 事件轮询机制

## 重构目标

根据分析报告，此模块当前复杂度较低，拆分潜力为★★★☆☆。主要目标是将网络相关功能独立封装，提高代码组织性和可维护性，与其他核心模块保持一致的抽象层次。