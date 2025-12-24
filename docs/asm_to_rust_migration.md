# 汇编到 Rust 迁移指南

本文档提供了将 NOS 内核中遗留的汇编代码迁移到 Rust 的指南和最佳实践。

## 目录

1. [迁移概述](#迁移概述)
2. [架构特定迁移](#架构特定迁移)
3. [通用模式转换](#通用模式转换)
4. [内联汇编替代方案](#内联汇编替代方案)
5. [测试和验证](#测试和验证)
6. [性能考虑](#性能考虑)

## 迁移概述

### 当前汇编代码位置

```
bootloader/src/arch/
├── x86_64/
│   ├── boot.s           # x86_64 启动代码
│   ├── multiboot_boot.s # Multiboot 入口点
│   └── bios.S           # BIOS 相关代码
├── aarch64/
│   └── boot.s           # AArch64 启动代码
└── riscv64/
    └── boot.s           # RISC-V 64 启动代码
```

### 迁移优先级

1. **高优先级**：核心启动代码、BSS 初始化、栈设置
2. **中优先级**：上下文切换、中断处理
3. **低优先级**：硬件加速、专用指令

## 架构特定迁移

### x86_64 迁移

#### 当前汇编代码分析

```assembly
_start:
    cli                     ; 禁用中断
    mov rsp, 0x200000      ; 设置栈指针
    and rsp, -16           ; 16字节对齐
    
    ; 清零 BSS 段
    lea rbx, [rel __bss_start]
    lea rcx, [rel __bss_end]
    cmp rbx, rcx
    jge bss_done
    
    xor eax, eax
.bss_loop:
    mov [rbx], eax
    add rbx, 4
    cmp rbx, rcx
    jl .bss_loop
bss_done:
    
    lea rax, [rel boot_main]
    call rax
    hlt
    jmp .
```

#### Rust 等价实现

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        // 禁用中断
        core::arch::asm!("cli");
        
        // 设置栈指针
        const STACK_TOP: u64 = 0x200000;
        let stack_ptr = STACK_TOP & !0xf; // 16字节对齐
        core::arch::asm!("mov rsp, {}", in(reg) stack_ptr);
        
        // 清零 BSS 段
        zero_bss();
        
        // 调用 Rust 入口点
        extern "C" {
            fn boot_main() -> !;
        }
        boot_main();
    }
}

unsafe fn zero_bss() {
    extern "C" {
        static mut __bss_start: u8;
        static mut __bss_end: u8;
    }
    
    let start = &raw mut __bss_start as *mut u8;
    let end = &raw mut __bss_end as *mut u8;
    let size = end.offset_from(start) as usize;
    
    // 使用 core::ptr::write_bytes 进行高效清零
    core::ptr::write_bytes(start, 0, size);
}
```

### AArch64 迁移

#### 当前汇编代码分析

```assembly
_start:
    msr daifset, #0xf      ; 禁用所有异常级别的中断
    mov sp, 0x200000       ; 设置栈指针
    
    ; 清零 BSS 段
    adrp x0, __bss_start
    adrp x1, __bss_end
    add x0, x0, :lo12:__bss_start
    add x1, x1, :lo12:__bss_end
    cmp x0, x1
    bge .bss_done
    
    mov x2, xzr
.bss_loop:
    str x2, [x0], #8
    cmp x0, x1
    blt .bss_loop
bss_done:
    
    ; 启用 FPU（可选）
    mrs x0, cpacr_el1
    orr x0, x0, #(3 << 20)
    msr cpacr_el1, x0
    
    dsb sy                 ; 数据同步屏障
    isb                    ; 指令同步屏障
    
    bl boot_main
    b .
```

#### Rust 等价实现

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        // 禁用所有中断
        core::arch::asm!("msr daifset, #0xf");
        
        // 设置栈指针
        const STACK_TOP: u64 = 0x200000;
        core::arch::asm!("mov sp, {}", in(reg) STACK_TOP);
        
        // 清零 BSS 段
        zero_bss();
        
        // 启用 FPU
        let mut cpacr: u64;
        core::arch::asm!("mrs {}, cpacr_el1", out(reg) cpacr);
        cpacr |= 3 << 20;
        core::arch::asm!("msr cpacr_el1, {}", in(reg) cpacr);
        
        // 内存屏障
        core::arch::asm!("dsb sy");
        core::arch::asm!("isb");
        
        // 调用 Rust 入口点
        extern "C" {
            fn boot_main() -> !;
        }
        boot_main();
    }
}

unsafe fn zero_bss() {
    extern "C" {
        static mut __bss_start: u8;
        static mut __bss_end: u8;
    }
    
    let start = &raw mut __bss_start as *mut u8;
    let end = &raw mut __bss_end as *mut u8;
    let size = end.offset_from(start) as usize;
    
    core::ptr::write_bytes(start, 0, size);
}
```

### RISC-V 迁移

#### 当前汇编代码分析

```assembly
_start:
    csrci mstatus, 0x8     ; 禁用中断（清除 MIE 位）
    li sp, 0x200000        ; 设置栈指针
    
    ; 清零 BSS 段
    la x1, __bss_start
    la x2, __bss_end
    bge x1, x2, .bss_done
    
    xor x3, x3, x3
.bss_loop:
    sd x3, (x1)
    addi x1, x1, 8
    blt x1, x2, .bss_loop
bss_done:
    
    fence                  ; 内存屏障
    fence.i                ; 指令屏障
    
    call boot_main
    wfi
    j .
```

#### Rust 等价实现

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        // 禁用中断
        core::arch::asm!("csrci mstatus, 0x8");
        
        // 设置栈指针
        const STACK_TOP: u64 = 0x200000;
        core::arch::asm!("li sp, {}", in(reg) STACK_TOP);
        
        // 清零 BSS 段
        zero_bss();
        
        // 内存屏障
        core::arch::asm!("fence");
        core::arch::asm!("fence.i");
        
        // 调用 Rust 入口点
        extern "C" {
            fn boot_main() -> !;
        }
        boot_main();
    }
}

unsafe fn zero_bss() {
    extern "C" {
        static mut __bss_start: u8;
        static mut __bss_end: u8;
    }
    
    let start = &raw mut __bss_start as *mut u8;
    let end = &raw mut __bss_end as *mut u8;
    let size = end.offset_from(start) as usize;
    
    core::ptr::write_bytes(start, 0, size);
}
```

## 通用模式转换

### 1. 栈设置和对齐

**汇编模式：**
```assembly
mov sp, 0x200000
and sp, -16
```

**Rust 实现：**
```rust
pub fn setup_stack() -> u64 {
    const STACK_BASE: u64 = 0x200000;
    const STACK_ALIGN: u64 = 16;
    STACK_BASE & !(STACK_ALIGN - 1)
}
```

### 2. BSS 清零

**汇编模式：**
```assembly
lea rdi, [rel __bss_start]
lea rcx, [rel __bss_end]
sub rdi, rcx
xor eax, eax
rep stosb
```

**Rust 实现：**
```rust
pub unsafe fn zero_bss() {
    extern "C" {
        static mut __bss_start: u8;
        static mut __bss_end: u8;
    }
    
    let start = &raw mut __bss_start as *mut u8;
    let end = &raw mut __bss_end as *mut u8;
    let size = end.offset_from(start) as usize;
    
    core::ptr::write_bytes(start, 0, size);
}
```

### 3. 中断控制

**汇编模式：**
```assembly
cli     ; x86_64
msr daifset, #0xf  ; AArch64
csrci mstatus, 0x8 ; RISC-V
```

**Rust 实现（使用内联汇编）：**
```rust
pub unsafe fn disable_interrupts() {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!("cli");
    
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("msr daifset, #0xf");
    
    #[cfg(target_arch = "riscv64")]
    core::arch::asm!("csrci mstatus, 0x8");
}
```

### 4. 内存屏障

**汇编模式：**
```assembly
mfence          ; x86_64
dsb sy; isb     ; AArch64
fence; fence.i  ; RISC-V
```

**Rust 实现：**
```rust
pub unsafe fn memory_barrier() {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!("mfence", options(nostack));
    
    #[cfg(target_arch = "aarch64")]
    {
        core::arch::asm!("dsb sy", options(nostack));
        core::arch::asm!("isb", options(nostack));
    }
    
    #[cfg(target_arch = "riscv64")]
    {
        core::arch::asm!("fence", options(nostack));
        core::arch::asm!("fence.i", options(nostack));
    }
}
```

## 内联汇编替代方案

### 1. 读取控制寄存器

**汇编方式：**
```assembly
mrs x0, cpacr_el1
```

**Rust 内联汇编：**
```rust
#[inline(always)]
pub unsafe fn read_cpacr_el1() -> u64 {
    let value: u64;
    core::arch::asm!("mrs {}, cpacr_el1", out(reg) value);
    value
}
```

### 2. 写入控制寄存器

**汇编方式：**
```assembly
msr cpacr_el1, x0
```

**Rust 内联汇编：**
```rust
#[inline(always)]
pub unsafe fn write_cpacr_el1(value: u64) {
    core::arch::asm!("msr cpacr_el1, {}", in(reg) value);
}
```

### 3. 使用 `volatile` 内存访问

```rust
// 模拟汇编中的 volatile 读写
pub unsafe fn read_volatile_u32(addr: usize) -> u32 {
    (addr as *const u32).read_volatile()
}

pub unsafe fn write_volatile_u32(addr: usize, value: u32) {
    (addr as *mut u32).write_volatile(value);
}
```

## 测试和验证

### 1. 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stack_alignment() {
        let stack = setup_stack();
        assert_eq!(stack % 16, 0);
    }
    
    #[test]
    fn test_zero_bss() {
        // 在测试环境中模拟 BSS 区域
        let mut bss_region: [u8; 1024] = [0xFF; 1024];
        
        unsafe {
            let ptr = bss_region.as_mut_ptr();
            core::ptr::write_bytes(ptr, 0, 1024);
        }
        
        assert!(bss_region.iter().all(|&x| x == 0));
    }
}
```

### 2. 集成测试

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_boot_sequence() {
        unsafe {
            disable_interrupts();
            zero_bss();
            
            // 验证 BSS 已清零
            extern "C" {
                static __bss_start: u8;
            }
            assert_eq!(&__bss_start as *const u8 as u8, 0);
        }
    }
}
```

### 3. 性能基准测试

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    
    #[bench]
    fn bench_zero_bss(b: &mut test::Bencher) {
        b.iter(|| unsafe {
            zero_bss();
        });
    }
}
```

## 性能考虑

### 1. 内联函数

对于性能关键的代码，使用 `#[inline(always)]`：

```rust
#[inline(always)]
pub unsafe fn enable_interrupts() {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!("sti", options(nostack));
}
```

### 2. 避免 `volatile` 除非必要

`volatile` 会阻止编译器优化：

```rust
// 避免不必要的 volatile
pub fn read_status_reg() -> u32 {
    unsafe { (STATUS_REGISTER as *const u32).read_volatile() }
}

// 对于非易失性寄存器，使用普通读取
pub fn read_config_reg() -> u32 {
    unsafe { *(CONFIG_REGISTER as *const u32) }
}
```

### 3. 使用 `naked` 函数

对于不使用栈的函数，使用 `naked` 属性：

```rust
#[naked]
pub unsafe extern "C" fn context_switch() {
    core::arch::naked_asm!(
        "save_context",
        "load_context",
        "ret",
    );
}
```

## 迁移检查清单

- [ ] 识别所有汇编文件
- [ ] 为每个架构创建对应的 Rust 模块
- [ ] 将汇编代码转换为 Rust 内联汇编
- [ ] 添加类型安全和文档
- [ ] 编写单元测试
- [ ] 编写集成测试
- [ ] 运行性能基准测试
- [ ] 验证功能正确性
- [ ] 更新构建系统
- [ ] 删除旧的汇编文件

## 迁移后的代码结构

```
kernel/src/arch/
├── x86_64/
│   ├── boot.rs         # 启动代码（Rust 实现）
│   ├── context.rs      # 上下文切换
│   ├── interrupt.rs    # 中断处理
│   └── mod.rs
├── aarch64/
│   ├── boot.rs
│   ├── context.rs
│   ├── interrupt.rs
│   └── mod.rs
├── riscv64/
│   ├── boot.rs
│   ├── context.rs
│   ├── interrupt.rs
│   └── mod.rs
└── mod.rs
```

## 参考资源

- [Rust 内联汇编文档](https://doc.rust-lang.org/reference/inline-assembly.html)
- [The Rustonomicon - Unsafe Rust](https://doc.rust-lang.org/nomicon/unsafe.html)
- [Rust 内核开发指南](https://rust-osdev.com/)
