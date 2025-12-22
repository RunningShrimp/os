# 内存布局架构设计文档

## 概述

NOS操作系统使用架构无关的内存布局抽象，支持x86_64、aarch64和riscv64架构。内存布局设计确保物理内存映射区域与内核代码区域分离，避免地址冲突。

## 内存布局设计

### x86_64架构

- **内核代码区**：`0xFFFF_FFFF_8000_0000` - `0xFFFF_FFFF_9000_0000`
- **内核数据区**：`0xFFFF_FFFF_8100_0000` - `0xFFFF_FFFF_9100_0000`
- **内核堆区**：`0xFFFF_FFFF_8300_0000` - `0xFFFF_FFFF_9300_0000`
- **物理内存映射区**：`0xFFFF_8000_0000_0000` - `0xFFFF_FFFF_FFFF_FFFF`（128TB区域）
- **MMIO区域**：`0xFFFF_8000_0000_0000` - `0xFFFF_FFFF_FFFF_FFFF`（128TB）

**关键改进**：物理内存映射从`0xFFFF_FFFF_8000_0000`（与内核代码重叠）改为`0xFFFF_8000_0000_0000`（独立区域）

### AArch64架构

- **内核代码区**：`0xFFFF_0000_0000_0000` - `0xFFFF_0000_0100_0000`
- **物理内存映射区**：`0xFFFF_8000_0000_0000` - `0xFFFF_FFFF_FFFF_FFFF`（独立区域）

### RISC-V 64架构

- **内核代码区**：`0xFFFF_FFFF_0000_0000` - `0xFFFF_FFFF_0100_0000`
- **物理内存映射区**：`0xFFFF_FFFF_8000_0000` - `0xFFFF_FFFF_FFFF_FFFF`（独立2GB区域）

## 地址冲突检测

内存布局模块提供了`verify_memory_layout()`函数，用于检测：
- 物理映射区域与内核代码区域的重叠
- 物理映射区域与内核数据区域的重叠
- 物理映射区域与内核堆区域的重叠
- 用户空间与内核空间的重叠

## 使用示例

```rust
use kernel::arch::memory_layout::MemoryLayout;

let layout = MemoryLayout::current();

// 验证内存布局
layout.verify_memory_layout()?;

// 转换物理地址到虚拟地址
if let Some(virt_addr) = layout.phys_to_virt(phys_addr) {
    // 使用虚拟地址
}
```



