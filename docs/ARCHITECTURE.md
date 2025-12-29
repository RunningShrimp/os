# NOS 架构设计文档

## 文档信息

- **内核名称**: NOS (New Operating System)
- **版本**: 0.1.0
- **开发语言**: Rust (Edition 2024)
- **架构类型**: 微内核 + 模块化设计
- **目标架构**: x86_64, AArch64, RISC-V 64

---

## 1. 总体架构

### 1.1 设计理念

NOS 采用 **微内核 + 模块化子系统** 的混合架构设计，结合了以下特性：

- **微内核设计**: 核心功能最小化，大部分服务运行在用户空间
- **模块化子系统**: 清晰的层次结构，每个子系统独立开发和测试
- **架构抽象层**: 统一的多架构支持接口
- **POSIX 兼容**: 提供 POSIX 兼容层，支持现有应用程序
- **安全优先**: 从底层设计开始就注重安全性

### 1.2 架构层次

```
┌─────────────────────────────────────────────────────────┐
│                     用户空间应用                          │
│              (POSIX Applications / Services)             │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│                     POSIX 兼容层                          │
│   (System Calls, libc Interface, Signal Handling)       │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│                   微内核核心 (Core)                       │
│         (Process, Memory, IPC, Scheduler)                │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│                    模块化子系统层                         │
│  ┌────────┬────────┬────────┬────────┬────────┬────────┐ │
│  │   FS   │  Net  │  IPC  │ Driver│Security│ MM    │ │
│  └────────┴────────┴────────┴────────┴────────┴────────┘ │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│                  平台抽象层 (Platform)                    │
│      (Arch, Drivers, Boot, Trap Handling)                │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│                  硬件抽象层 (HAL)                         │
│         (CPU, Memory, Devices, Interrupts)               │
└─────────────────────────────────────────────────────────┘
```

---

## 2. 核心组件

### 2.1 进程管理 (Process Management)

**位置**: `kernel/src/subsystems/process/`

**主要功能**:
- 进程生命周期管理（创建、销毁、等待）
- 线程管理和调度
- 上下文切换
- 进程间通信（IPC）
- 文件描述符管理
- 信号处理

**关键模块**:
- `manager.rs`: 进程管理器
- `thread.rs`: 线程管理
- `exec.rs`: ELF 加载和执行
- `context_switch.rs`: 上下文切换
- `fd_cache.rs`: 文件描述符缓存
- `rcu_table.rs`: RCU 进程表

**数据结构**:
```rust
pub struct Process {
    pid: Pid,
    parent: Option<Pid>,
    state: ProcessState,
    memory: MemorySpace,
    fds: FdTable,
    signals: SignalState,
}
```

### 2.2 内存管理 (Memory Management)

**位置**: `kernel/src/subsystems/mm/`

**主要功能**:
- 物理内存分配器（Buddy System）
- 虚拟内存管理（页表）
- 内存映射（mmap）
- 内存隔离（ASLR, SMEP, SMAP）
- 大页支持（2MB, 1GB）
- NUMA 支持
- 内存压缩

**关键模块**:
- `phys.rs`: 物理内存管理
- `vm.rs`: 虚拟内存管理
- `buddy.rs`: Buddy 分配器
- `slab.rs`: Slab 分配器
- `allocator.rs`: 通用分配器
- `hugepage.rs`: 大页支持
- `numa.rs`: NUMA 支持
- `api/`: 内存管理 API

**内存布局**:
```
Kernel Space (High Memory)
├── Kernel Code & Data
├── Kernel Heap
├── Page Tables
└── Kernel Stack

User Space (Low Memory)
├── Text Segment
├── Data Segment
├── Heap
├── Memory Mapping Segment
└── Stack
```

### 2.3 文件系统 (File System)

**位置**: `kernel/src/subsystems/fs/` 和 `kernel/src/vfs/`

**主要功能**:
- 虚拟文件系统（VFS）抽象
- 多文件系统支持（ext4, ramfs, tmpfs）
- 文件锁和权限管理
- 日志和恢复
- VFS 缓存优化

**关键模块**:
- `vfs/`: VFS 核心实现
- `ext4.rs`: ext4 文件系统
- `ramfs.rs`: 内存文件系统
- `tmpfs.rs`: 临时文件系统
- `journaling_*.rs`: 日志系统
- `file_locking.rs`: 文件锁
- `file_permissions.rs`: 权限管理

**VFS 架构**:
```
Application
    ↓
System Call Interface
    ↓
VFS Layer (Virtual File System)
    ↓
File System Implementations
    ├── ext4
    ├── ramfs
    └── tmpfs
```

### 2.4 网络栈 (Network Stack)

**位置**: `kernel/src/subsystems/net/`

**主要功能**:
- TCP/IP 协议栈
- UDP 协议
- Socket 接口
- 零拷贝 I/O
- 网络设备驱动接口
- 路由和转发

**关键模块**:
- `tcp/`: TCP 实现
- `udp.rs`: UDP 实现
- `ipv4.rs`: IPv4 协议
- `socket.rs`: Socket 接口
- `device.rs`: 网络设备
- `zero_copy.rs`: 零拷贝优化
- `buffer_management.rs`: 缓冲区管理

### 2.5 系统调用 (System Calls)

**位置**: `kernel/src/subsystems/syscalls/`

**主要功能**:
- 系统调用分发
- 快速路径优化
- 批量系统调用
- 参数验证
- 错误处理

**关键模块**:
- `core/`: 核心分发逻辑
- `dispatch/`: 系统调用分发器
- `fs/`: 文件系统系统调用
- `process/`: 进程管理系统调用
- `network/`: 网络系统调用
- `ipc/`: IPC 系统调用
- `signal/`: 信号系统调用
- `memory/`: 内存管理系统调用
- `fast_path/`: 快速路径优化

**系统调用流程**:
```
User Space
    ↓ (syscall instruction)
Trap Handler
    ↓
System Call Dispatcher
    ↓ (validation & routing)
Handler Functions
    ↓
Kernel Subsystems
```

### 2.6 安全机制 (Security)

**位置**: `kernel/src/security/`

**主要功能**:
- 访问控制（ACL, Capabilities）
- 地址空间布局随机化（ASLR）
- 内存安全（SMEP, SMAP）
- 栈保护（Stack Canaries）
- 安全审计
- SELinux 支持
- Seccomp 支持

**关键模块**:
- `aslr.rs`: ASLR 实现
- `smap_smep.rs`: SMEP/SMAP 保护
- `stack_canaries.rs`: 栈保护
- `capabilities.rs`: 能力安全
- `acl.rs`: 访问控制列表
- `memory_security.rs`: 内存安全
- `audit.rs`: 安全审计（独立模块）

### 2.7 设备驱动 (Device Drivers)

**位置**: `kernel/src/platform/drivers/` 和 `kernel/src/subsystems/drivers/`

**主要功能**:
- 设备管理
- 驱动程序注册
- 设备发现
- 中断处理
- 设备抽象

**关键模块**:
- `device_manager.rs`: 设备管理器
- `driver_manager.rs`: 驱动管理
- `driver_registration.rs`: 驱动注册
- `uart.rs`: UART 驱动
- `nvme.rs`: NVMe 驱动
- `virtio_gpu.rs`: VirtIO GPU
- `usb.rs`: USB 驱动
- `gic.rs`: 中断控制器

---

## 3. 模块依赖关系

### 3.1 目录结构

```
kernel/src/
├── api/                    # 公共 API 接口
├── arch/                   # 架构相关代码
│   ├── x86_64/            # x86_64 特定实现
│   ├── memory_layout.rs   # 内存布局
│   ├── kpti.rs           # 页表隔离
│   └── retpoline.rs      # Retpoline 缓解
├── core/                   # 核心功能模块
│   ├── init.rs           # 内核初始化
│   ├── interrupt.rs      # 中断处理
│   ├── lib.rs            # 核心库
│   ├── syscall/          # 系统调用核心
│   ├── sync.rs           # 同步原语
│   └── types.rs          # 核心类型
├── subsystems/             # 主要子系统
│   ├── mm/               # 内存管理
│   ├── process/          # 进程管理
│   ├── fs/               # 文件系统
│   ├── net/              # 网络栈
│   ├── ipc/              # 进程间通信
│   ├── syscalls/         # 系统调用
│   ├── sync/             # 同步原语
│   ├── scheduler/        # 调度器
│   ├── drivers/          # 设备驱动
│   ├── cloud_native/     # 云原生支持
│   └── formal_verification/ # 形式化验证
├── platform/               # 平台相关代码
│   ├── arch/             # 架构抽象
│   ├── boot/             # 启动代码
│   ├── drivers/          # 平台驱动
│   └── trap/             # 陷阱处理
├── posix/                  # POSIX 兼容层
│   ├── types.rs          # POSIX 类型
│   ├── thread.rs         # 线程支持
│   ├── signal.rs         # 信号处理
│   ├── semaphore.rs      # 信号量
│   ├── shm.rs            # 共享内存
│   └── mqueue.rs         # 消息队列
├── security/               # 安全机制
│   ├── aslr.rs           # ASLR
│   ├── capabilities.rs   # 能力
│   ├── acl.rs            # 访问控制
│   └── smap_smep.rs      # 内存保护
├── vfs/                    # 虚拟文件系统
├── compat/                 # 兼容层
│   ├── linux.rs          # Linux 兼容
│   ├── android.rs        # Android 兼容
│   └── windows.rs        # Windows 兼容
├── error/                  # 错误处理
├── monitoring/             # 监控和性能
├── debug/                  # 调试支持
└── main.rs                # 内核入口点
```

### 3.2 依赖图

```
main.rs
  ↓
core::init
  ↓
┌─────────────┬─────────────┬─────────────┬─────────────┐
│             │             │             │             │
platform    subsystems    security     posix         error
  ↓             ↓             ↓             ↓             ↓
arch         mm/process    aslr/acl     types         panic
drivers      fs/net        capabilities  signal        recovery
trap         ipc/syscalls  smap_smep    shm           unified
boot         scheduler     canaries     mqueue
```

---

## 4. 关键设计决策

### 4.1 为什么选择微内核？

**优势**:
1. **安全性**: 最小特权原则，减少攻击面
2. **可靠性**: 服务隔离，单个服务崩溃不影响整个系统
3. **可扩展性**: 易于添加新服务和功能
4. **可维护性**: 模块化设计便于开发和调试

**权衡**:
- 性能开销（通过快速路径和优化缓解）
- 复杂的消息传递（使用高效的 IPC 机制）

### 4.2 为什么使用 Rust？

**优势**:
1. **内存安全**: 编译时检查，避免内存泄漏、悬垂指针
2. **线程安全**: 所有权系统防止数据竞争
3. **零成本抽象**: 高级特性不影响性能
4. **模式匹配**: 强大的错误处理
5. **无运行时**: 适合操作系统开发

**实践**:
```rust
// 所有权确保内存安全
fn allocate_memory() -> Box<[u8]> {
    Box::new([0u8; 4096])
} // 自动释放

// 借用检查防止数据竞争
fn shared_access(data: &Mutex<Data>) {
    let guard = data.lock();
    // 安全地访问共享数据
}
```

### 4.3 如何保证安全性？

**多层防护**:

1. **编译时安全**:
   - Rust 类型系统
   - 所有权和借用检查
   - 生命周期检查

2. **运行时保护**:
   - ASLR（地址空间布局随机化）
   - SMEP/SMAP（模式保护）
   - Stack Canaries（栈保护）
   - KPTI（页表隔离）

3. **访问控制**:
   - POSIX 权限
   - ACL（访问控制列表）
   - Capabilities（能力安全）
   - SELinux 策略

4. **审计和监控**:
   - 系统调用审计
   - 安全事件日志
   - 异常检测
   - 入侵检测系统（IDS）

---

## 5. 内存布局

### 5.1 虚拟内存布局

```
高地址
┌────────────────────────────────────┐
│     内核空间 (Kernel Space)         │
│  ┌──────────────────────────────┐  │
│  │  Kernel Code & Data          │  │
│  │  (Read-only, Executable)     │  │
│  ├──────────────────────────────┤  │
│  │  Kernel Heap                 │  │
│  │  (Read-write, No execute)    │  │
│  ├──────────────────────────────┤  │
│  │  Page Tables                 │  │
│  │  (Managed by MM)             │  │
│  ├──────────────────────────────┤  │
│  │  Kernel Stack per CPU        │  │
│  │  (Guarded, No execute)       │  │
│  └──────────────────────────────┘  │
├────────────────────────────────────┤
│   用户空间 (User Space)             │
│  ┌──────────────────────────────┐  │
│  │  Stack (growing down)        │  │
│  │  (Guarded page)              │  │
│  ├──────────────────────────────┤  │
│  │  Memory Mapping Segment      │  │
│  │  (mmap, shared libraries)    │  │
│  ├──────────────────────────────┤  │
│  │  Heap (growing up)           │  │
│  │  (Dynamic allocations)       │  │
│  ├──────────────────────────────┤  │
│  │  BSS (uninitialized data)    │  │
│  ├──────────────────────────────┤  │
│  │  Data (initialized data)     │  │
│  ├──────────────────────────────┤  │
│  │  Text (code, read-only)      │  │
│  └──────────────────────────────┘  │
└────────────────────────────────────┘
低地址
```

### 5.2 物理内存布局

```
┌──────────────────────────────┐
│   Reserved Memory            │
│   (Firmware, ACPI)           │
├──────────────────────────────┤
│   Kernel Image               │
│   (Code, Data, BSS)          │
├──────────────────────────────┤
│   Kernel Heap                │
│   (Dynamic allocations)      │
├──────────────────────────────┤
│   Page Tables                │
│   (Multi-level structures)   │
├──────────────────────────────┤
│   Free Memory                │
│   (Managed by allocator)     │
├──────────────────────────────┤
│   MMIO Regions               │
│   (Device registers)         │
└──────────────────────────────┘
```

### 5.3 页表结构

**x86_64 (4-level paging)**:
```
PML4 (Page Map Level 4)
  ↓
PDPT (Page Directory Pointer Table)
  ↓
PD (Page Directory)
  ↓
PT (Page Table)
  ↓
Physical Page (4KB)
```

**AArch64 (4-level or 5-level paging)**:
```
TTBR0_EL1 (User space)
  ↓
L0 Table (optional, 48-bit or 52-bit VA)
  ↓
L1 Table
  ↓
L2 Table
  ↓
L3 Table
  ↓
Physical Page (4KB/16KB/64KB)
```

---

## 6. 中断处理

### 6.1 中断流程

```
Hardware Interrupt
    ↓
CPU Context Save
    ↓
Trap Handler (platform/trap/)
    ↓
Interrupt Dispatcher
    ↓
Device-Specific Handler
    ↓
Platform Drivers (platform/drivers/)
    ↓
Kernel Subsystems
    ↓
Context Restore
    ↓
Return to User Space
```

### 6.2 中断类型

1. **硬件中断**:
   - 时钟中断
   - 设备中断（键盘、网络、磁盘）
   - IPI（处理器间中断）

2. **异常**:
   - 页错误
   - 一般保护错误
   - 浮点异常
   - 系统调用

3. **软件中断**:
   - 软中断（bottom half）
   - 任务队列

### 6.3 中断控制器

**x86_64**:
- APIC (Local APIC + I/O APIC)
- 支持 MSI/MSI-X

**AArch64**:
- GIC (Generic Interrupt Controller)
- GICv2, GICv3, GICv4 支持

**RISC-V**:
- PLIC (Platform-Level Interrupt Controller)
- CLINT (Core-Local Interrupt Controller)

---

## 7. 调度策略

### 7.1 调度器架构

**位置**: `kernel/src/subsystems/scheduler/`

**调度器类型**:
1. **实时调度器** (`realtime.rs`):
   - SCHED_FIFO: 先进先出
   - SCHED_RR: 时间片轮转
   - 优先级调度

2. **完全公平调度器** (`unified.rs`):
   - CFS (Completely Fair Scheduler)
   - 红黑树实现
   - O(log n) 复杂度

3. **实时简化调度器** (`realtime_simple.rs`):
   - 轻量级实时调度
   - 优先级队列

### 7.2 调度流程

```
Timer Interrupt
    ↓
Scheduler Tick
    ↓
Update Task Statistics
    ↓
Check if Preemption Needed
    ↓ (if yes)
Choose Next Task
    ↓
Context Switch
    ↓
Run Next Task
```

### 7.3 调度优先级

**优先级范围**:
- 实时任务: 0-99 (数值越大，优先级越高)
- 普通任务: 100-139 (数值越大，优先级越低)

**Nice 值**:
- -20 到 19
- 映射到普通优先级 100-139

---

## 8. 启动流程

### 8.1 启动顺序

```
1. Bootloader (bootloader crate)
   ↓ Load kernel and boot info
2. rust_main() (main.rs)
   ↓ Parse boot parameters
3. core::init::init_kernel_core()
   ↓ Initialize core subsystems
4. platform::init_platform()
   ↓ Initialize platform and drivers
5. subsystems initialization
   ↓ Initialize MM, Process, FS, etc.
6. scheduler start
   ↓ Run init process
7. System running
```

### 8.2 初始化步骤

**核心初始化** (`core/init.rs`):
```rust
pub fn init_kernel_core(boot_params: Option<&BootParameters>) {
    // 1. Early console
    early_console_init();

    // 2. CPU initialization
    cpu::init();

    // 3. Memory initialization
    mm::init();

    // 4. Interrupt handling
    trap::init();

    // 5. Scheduler
    sched::init();

    // 6. File system
    fs::init();

    // 7. Network (optional)
    #[cfg(feature = "net_stack")]
    net::init();
}
```

---

## 9. 特性标志 (Feature Flags)

### 9.1 核心特性

```toml
[features]
default = ["posix_layer", "net_stack", "syscalls", "services", "error_handling"]
baremetal = []           # 裸机启动
kernel_tests = []        # 内核测试
syscalls = []            # 系统调用支持
services = []            # 服务管理
error_handling = []      # 错误处理
net_stack = []           # 网络栈
posix_layer = []         # POSIX 层
```

### 9.2 性能优化特性

```toml
fast_syscall = []        # 快速系统调用
zero_copy = []           # 零拷贝 I/O
batch_syscalls = []      # 批量系统调用
net_opt = []             # 网络优化
sched_opt = []           # 调度器优化
lazy_init = []           # 延迟初始化
```

### 9.3 高级特性

```toml
security_audit = []      # 安全审计
formal_verification = [] # 形式化验证
cloud_native = []        # 云原生支持
graphics_subsystem = []  # 图形子系统
web_engine = []          # Web 引擎
observability = []       # 可观测性
```

### 9.4 内存特性

```toml
hpage_2mb = []           # 2MB 大页
hpage_1gb = []           # 1GB 大页
```

---

## 10. 性能优化

### 10.1 系统调用优化

**快速路径** (`subsystems/syscalls/fast_path/`):
- 热点系统调用优化
- 减少参数验证开销
- 快速上下文切换

**批量系统调用**:
- 一次系统调用处理多个请求
- 减少用户/内核切换次数

**零拷贝 I/O**:
- 减少内存拷贝
- 使用页映射和 DMA

### 10.2 内存管理优化

**Slab 分配器**:
- 快速的小对象分配
- 减少碎片

**大页支持**:
- 减少 TLB 缺失
- 提高内存访问速度

**Per-CPU 缓存**:
- 减少锁竞争
- 提高并发性能

### 10.3 网络优化

**零拷贝网络**:
- 直接从用户空间发送数据
- 减少 CPU 开销

**批量处理**:
- GSO (Generic Segmentation Offload)
- LRO (Large Receive Offload)

---

## 11. 可观测性

### 11.1 监控子系统

**位置**: `kernel/src/monitoring/`

**功能**:
- 性能指标收集
- 健康检查
- 告警系统
- 时间线追踪

### 11.2 调试支持

**位置**: `kernel/src/debug/`

**功能**:
- 符号表管理
- 性能分析
- 断点管理
- 错误诊断
- 日志系统

### 11.3 性能分析

**位置**: `kernel/src/perf/` 和 `kernel/src/benchmark/`

**功能**:
- CPU 性能计数器
- 内存访问分析
- 系统调用追踪
- 基准测试框架

---

## 12. 兼容性

### 12.1 POSIX 兼容

**位置**: `kernel/src/posix/`

**支持的 POSIX 标准**:
- POSIX.1-2008
- SUSv4
- Linux 扩展

**兼容层** (`kernel/src/compat/`):
- Linux 系统调用兼容
- Android 兼容
- macOS/iOS 兼容
- Windows WSL 兼容

### 12.2 libc 支持

**位置**: `kernel/src/libc/`

**实现**:
- newlib 接口
- 标准 C 库函数
- 数学库
- 字符串处理
- I/O 管理

---

## 13. 安全架构

### 13.1 纵深防御

```
应用层安全
    ↓
POSIX 权限 & Capabilities
    ↓
访问控制 (ACL)
    ↓
强制访问控制 (SELinux)
    ↓
内存保护 (ASLR, SMEP, SMAP)
    ↓
运行时保护 (Stack Canaries, KPTI)
    ↓
硬件安全特性
```

### 13.2 安全审计

**位置**: `kernel/src/security_audit/`

**功能**:
- 安全事件记录
- 合规性检查
- 取证分析
- 威胁检测
- 报告生成

---

## 14. 扩展性

### 14.1 云原生支持

**位置**: `kernel/src/subsystems/cloud_native/`

**功能**:
- 容器支持（Container）
- 编排（Orchestration）
- 服务发现
- 命名空间（Namespaces）
- Cgroups
- OCI 兼容

### 14.2 形式化验证

**位置**: `kernel/src/subsystems/formal_verification/`

**工具**:
- 模型检查器
- 定理证明器
- 类型检查器
- 静态分析器
- 安全证明器

---

## 15. 开发指南

### 15.1 编译内核

```bash
# 标准编译
cargo build --release

# 带特定特性
cargo build --features "baremetal kernel_tests"

# 交叉编译
cargo build --target aarch64-unknown-none
```

### 15.2 运行测试

```bash
# 运行所有测试
cargo test --features kernel_tests

# 运行特定测试
cargo test --test integration_tests

# 运行基准测试
cargo bench --features kernel_tests
```

### 15.3 添加新功能

1. 在相应的子系统目录下创建模块
2. 更新 `mod.rs` 导出新模块
3. 在 `Cargo.toml` 中添加依赖（如需要）
4. 编写测试和文档
5. 运行 `cargo fmt` 和 `cargo clippy`

---

## 16. 参考资料

### 16.1 外部依赖

- **nos-api**: 核心 API 和类型定义
- **nos-syscalls**: 系统调用接口
- **nos-services**: 服务管理框架
- **nos-error-handling**: 错误处理框架
- **nos-memory-management**: 内存管理基础

### 16.2 相关文档

- [OSDev Wiki](https://wiki.osdev.org/)
- [Rust Embedded Book](https://rust-embedded.github.io/book/)
- [POSIX Standard](https://pubs.opengroup.org/onlinepubs/9699919799/)
- [Linux Kernel Documentation](https://www.kernel.org/doc/html/latest/)

### 16.3 内部文档

- `docs/asm_to_rust_migration.md`: 汇编到 Rust 迁移指南
- `docs/promote.md`: 项目推广文档

---

## 附录 A: 系统调用表

### 文件系统系统调用
- `open`, `close`, `read`, `write`, `lseek`
- `stat`, `fstat`, `lstat`
- `mkdir`, `rmdir`, `unlink`, `link`, `rename`
- `chmod`, `fchmod`, `chown`, `fchown`

### 进程管理系统调用
- `fork`, `execve`, `exit`, `wait`, `waitpid`
- `getpid`, `getppid`, `getuid`, `getgid`
- `clone`, `set_tid_address`

### 内存管理系统调用
- `mmap`, `munmap`, `mprotect`, `msync`
- `brk`, `sbrk`
- `madvise`, `mincore`

### IPC 系统调用
- `pipe`, `socket`, `bind`, `listen`, `accept`
- `connect`, `send`, `recv`, `shutdown`
- `shmget`, `shmat`, `shmdt`
- `semget`, `semop`, `semctl`

### 信号系统调用
- `kill`, `sigaction`, `sigprocmask`
- `sigpending`, `sigsuspend`
- `sigaltstack`

### 文件描述符系统调用
- `dup`, `dup2`, `fcntl`, `ioctl`
- `select`, `poll`, `epoll_create`, `epoll_ctl`, `epoll_wait`

---

## 附录 B: 错误码

### POSIX errno 值
- `EPERM`: Operation not permitted
- `ENOENT`: No such file or directory
- `ESRCH`: No such process
- `EINTR`: Interrupted system call
- `EIO`: I/O error
- `ENXIO`: No such device or address
- `E2BIG`: Argument list too long
- `ENOEXEC`: Exec format error
- `EBADF`: Bad file number
- `ECHILD`: No child processes

### NOS 特定错误码
- `NOS_ERROR_OUT_OF_MEMORY`: 内存不足
- `NOS_ERROR_INVALID_ARGUMENT`: 无效参数
- `NOS_ERROR_PERMISSION_DENIED`: 权限拒绝
- `NOS_ERROR_RESOURCE_BUSY`: 资源忙碌
- `NOS_ERROR_NOT_SUPPORTED`: 不支持的操作

---

## 附录 C: 配置选项

### 内核配置宏
- `CONFIG_MAX_CPUS`: 最大 CPU 数量
- `CONFIG_MAX_PROCESSES`: 最大进程数
- `CONFIG_MAX_FILES`: 最大打开文件数
- `CONFIG_PAGE_SIZE`: 页面大小（默认 4096）
- `CONFIG_HEAP_SIZE`: 堆大小
- `CONFIG_STACK_SIZE`: 栈大小

### 运行时配置
- `/proc/sys/kernel/`: 内核参数
- `/sys/`: sysfs 接口
- `/proc/sys/vm/`: 虚拟内存参数

---

**文档版本**: 1.0
**最后更新**: 2025-12-28
**维护者**: NOS Kernel Team
