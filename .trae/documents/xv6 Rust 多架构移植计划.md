## 项目目标

* 将教学版 Unix xv6 的内核与基本用户态工具以功能等价的方式完整翻译为 Rust

* 在 RISC-V（riscv64）、AMD64（x86\_64）、ARM64（aarch64）三种架构上可编译、可引导、能在 QEMU/真实硬件上运行并通过基础用例

* 保持内存安全，尽量最小化 `unsafe`，将必要的底层操作封装在清晰的边界内

## 模块映射与优先级

* 核心内核：启动引导、异常/中断、内存管理（物理页分配、页表、虚拟内存）、进程与调度、系统调用层

* I/O 与驱动：控制台（UART/串口）、时钟定时器（时基/中断源）、块设备（先 RAMDisk，再可插 SATA/virtio）

* 文件系统：xv6 简化版 FS（超级块、inode、目录、日志/缓冲层）

* 用户态：最小 libc 适配（syscall 封装）、基本命令（sh、cat、ls、echo、mkdir 等）

* 先实现独立/可单测的工具与内存/同步原语，再逐步接入内核主路径，最后接入驱动与架构特定代码

## Rust 总体架构

* 工作区结构：顶层 `Cargo.toml`（workspace）管理多 crate

* crate 划分：

  * `boot-x86_64`、`boot-aarch64`、`boot-riscv64`（纯自研引导，`#![no_std]`、`#![no_main]`，少量汇编/内联汇编）

  * `kernel`（通用内核逻辑，`no_std`，按 `arch` 特性插桩）

  * `arch`（`arch-x86_64`、`arch-aarch64`、`arch-riscv64` 子模块，提供统一接口：中断、页表、时钟、UART）

  * `drivers`（UART、时钟、块设备抽象与实现，面向 `arch` 能力）

  * `fs`（xv6 FS 的数据结构与操作）

  * `sync`（自研 `SpinLock`、`Mutex`、`Once` 等原语，避免外部依赖）

  * `user`（最小 libc、系统调用封装、基础命令）

  * `xtask`（构建/打包/运行 QEMU 的工具任务）

* 条件编译：`cfg(target_arch = "x86_64" | "aarch64" | "riscv64")` 与 feature 控制平台差异

* 依赖策略：核心保持 `no_std`，可用 `alloc`，尽量不引入第三方；用户态可使用 `libc` 进行最小封装（内核不使用）

* `unsafe` 边界：集中在 `arch` 层（寄存器读写、页表、屏障、特权指令）、引导阶段与少量同步原语内部

## 引导与启动流程（自研 bootloader）

* x86\_64：

  * 阶段划分：`stage0`（实模式进入保护模式）、`stage1`（启用长模式，建立临时页表与栈）、`stage2`（跳入 Rust 内核入口）

  * 准备 `GDT/IDT/TSS` 基础，启用分页与高地址映射策略

* aarch64：

  * 早期设置 `SCTLR/TTBR0/TTBR1`、页表建立与 MMU 使能，异常向量表初始化

* riscv64：

  * 设置 `sstatus/satp/stvec`，建立 Sv39 页表与陷阱向量，切入 S 态内核

* 通用：

  * 统一入口 `kernel_main()`；早期 UART 初始化作为 console；时钟源初始化；内存检测与物理页管理初始化

## 内存管理与分页

* 物理内存管理：位图/伙伴分配器，支持页分配/释放，接口统一

* 虚拟内存：

  * x86\_64：四级页表，内核高半区映射，用户态低地址空间，支持 COW（后续迭代）

  * aarch64：三级/四级页表（取决于实现），分离 TTBR0/1 空间

  * riscv64：Sv39 页表布局，内核/用户空间划分

* 内核堆：基于 `alloc` 的自研分配器入口，早期通过 `boot heap` 过渡到页分配器支持

## 进程与调度

* 进程控制块（PCB）：寄存器上下文、地址空间、文件描述符表、状态与优先级

* 调度器：时间片轮转（与 xv6 等价），后续可扩充优先级/负载均衡

* 上下文切换：依赖 `arch` 提供的保存/恢复原语（`switch()`）

* 线程模型：先以进程为主，后续可增加轻量级线程（可选）

## 中断与异常

* 统一接口：`arch::interrupts::{init, enable, disable, register_handler}`

* 时钟中断：

  * x86\_64：APIC/HPET 或 PIT，优先选 APIC

  * aarch64：Generic Timer + GIC

  * riscv64：CLINT（mtime）+ PLIC

* 串口：

  * x86\_64：16550A

  * aarch64：PL011（或 virtio-console）

  * riscv64：UART0/8250 类

## 系统调用层

* 与 xv6 等价的 syscall 集：`fork/exec/exit/wait/sleep/wake/read/write/open/close/link/unlink/mkdir/chdir/sbrk/kill/getpid` 等

* 内核接口：统一 `sys_*` 入口 + `arch` 异常分发到 syscall 号表

* 用户态最小 libc：以 `asm`/`syscall` 指令桥接到内核，保持 API 与 xv6 命令兼容

## 文件系统与块设备

* xv6 FS：超级块、inode、目录项、缓冲区缓存、简单日志/事务（可选）

* 块设备抽象：`BlockDevice` trait；初期 RAMDisk；后续 virtio-blk/SATA 可插

* VFS 层：最小路径解析、文件描述符、管道与设备文件（`/dev/tty`）

## 多架构适配与抽象层

* `arch` 模块暴露统一能力：

  * 页表操作、TLB 刷新、屏障指令

  * 中断控制、异常向量、时钟/时间源

  * UART/console 初始化与读写

* 通过 `cfg(target_arch)` 选择具体实现；公共接口由 `kernel` 使用

## 构建与交叉编译

* 顶层 `Cargo.toml` workspace + 每 crate 自身 `Cargo.toml`

* `build.rs`：生成/选择架构特定链接脚本（`linker.ld`）、内核镜像布局、符号导出

* `target.json`：必要时定义自研 target（`no_std` 环境、禁用默认 CRT）

* 交叉编译：

  * x86\_64/aarch64/riscv64 三目标 `cargo build --target <target>`

  * `xtask` 封装：`cargo xtask build/run --arch <x86_64|aarch64|riscv64> --qemu`

* 产物：`kernel.bin`、各架构 `boot` 镜像（raw/ELF），QEMU 可直接加载

## 测试与验证

* 单元测试：在可 `alloc` 的纯逻辑模块（fs、sync、算法）使用自研 `no_std` 测试宏

* 集成测试：QEMU 启动脚本，自动执行基础命令序列并验证输出（`sh`, `ls`, `echo`, `fork/exec` 等）

* 架构矩阵：三架构均跑启动、shell、文件读写、进程/系统调用用例

* 安全验证：`miri` 在用户态库层验证；内核以审计 + 运行时断言

* 风格：开启 `clippy` lint；CI 运行 `fmt/clippy/build/test`（本地先提供 `xtask`）

## `unsafe` 边界与封装

* 归档 `unsafe` 使用点：页表操作、特权寄存器、内联汇编、设备 MMIO 访问

* 外部暴露安全接口，内部通过精确不变式与断言保护

## 性能与优化

* 页分配器与缓冲区缓存命中率优化；减少锁竞争（细粒度 `SpinLock`）

* UART/块设备采用环形缓冲与中断驱动；必要时增加批量 I/O

## 目录结构（交付物）

* `/Cargo.toml`（workspace）

* `/xtask/`（构建与运行辅助）

* `/boot-{arch}/`（自研 bootloader：汇编 + Rust）

* `/kernel/`（平台无关核心）

* `/arch/{x86_64,aarch64,riscv64}/`（架构层实现）

* `/drivers/`（UART/时钟/块设备）

* `/fs/`（xv6 文件系统）

* `/sync/`（同步原语）

* `/user/`（最小 libc 与命令）

* `/build.rs`、`/linker/{x86_64,aarch64,riscv64}.ld`、`/targets/*.json`

* `/README.md`（构建与运行说明，含 QEMU 参数）

## 里程碑与实施顺序

1. `sync`/基础工具与内核接口草拟（可测试）
2. 三架构 `boot` 进入内核、UART 输出、时钟 tick（"Hello kernel" + 计时）
3. 物理/虚拟内存管理、内核堆
4. 进程/调度、上下文切换、系统调用入口
5. 文件系统与 RAMDisk、基础命令
6. 拓展驱动与 VFS、更多命令
7. 完整测试矩阵与性能调优、文档完善

## 成功判定标准

* 三架构在 QEMU 上可引导到 shell，能运行基础命令并通过脚本化集成测试

* 代码通过 `clippy` 与基本静态检查；`unsafe` 仅在标注边界内

* 仓库结构清晰，支持 `cargo build --target` 与 `xtask run` 一键运行

