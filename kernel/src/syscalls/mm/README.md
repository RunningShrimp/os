# Memory Management System Calls Module

## 概述

此模块包含所有与内存管理相关的系统调用实现，包括虚拟内存分配、映射、保护和控制等功能。

## 系统调用范围

### 内存映射 (0x3000-0x3FFF)
- `mmap()` - 映射文件或设备到内存
- `munmap()` - 取消内存映射
- `mprotect()` - 修改内存保护属性
- `msync()` - 同步内存映射到存储设备
- `mremap()` - 重新映射虚拟内存地址
- `mlock()` - 锁定内存页防止换出
- `munlock()` - 解锁内存页

### 内存分配
- `brk()` - 改变程序的堆段大小
- `sbrk()` - 旧式堆扩展函数
- `memfd_create()` - 创建匿名内存文件

### 高级内存特性
- `madvise()` - 建议内存使用模式
- `mincore()` - 检查页是否在内存中
- `move_pages()` - 移动页到其他节点（NUMA）

### 进程地址空间
- `membarrier()` - 进程间内存屏障
- `process_vm_readv()` - 从其他进程读取内存
- `process_vm_writev()` - 向其他进程写入内存

## 相关文件

- `memory.rs` - 核心内存管理系统调用
- `advanced_mmap.rs` - 高级mmap功能和HugePage支持
- `mod.rs` - 模块声明和导出

## 架构依赖

依赖以下内核子系统：
- `crate::mm::vm` - 虚拟内存管理器
- `crate::mm::allocator` - 物理内存分配器
- `crate::mm::paging` - 页表管理
- `crate::process::manager` - 进程虚拟地址空间

## 重构目标

根据分析报告，此模块应该独立成为一个服务，提供内存管理的统一接口。当前代码中存在大量直接访问全局状态的问题，计划通过依赖注入彻底解决内存锁竞争性能瓶颈，为系统带来100-300%的性能提升空间。