# NOS Kernel 目录结构重构计划

## 当前问题分析

1. **模块依赖复杂**：许多模块之间存在循环依赖或不必要的依赖
2. **功能分散**：相似功能分散在不同目录中
3. **层次不清**：核心模块和辅助模块混合在一起
4. **特性控制分散**：特性标志控制分散在多个文件中

## 重构目标

1. **清晰的层次结构**：建立明确的模块层次和依赖关系
2. **功能聚合**：将相关功能组织在一起
3. **减少循环依赖**：通过合理的架构设计避免循环依赖
4. **集中特性控制**：统一特性标志控制

## 新目录结构设计

```
kernel/src/
├── lib.rs                    # 主入口文件
├── main.rs                   # 内核主函数
├── boot/                    # 启动相关
│   ├── mod.rs
│   ├── boot_params.rs
│   └── init.rs
├── arch/                    # 架构相关
│   ├── mod.rs
│   ├── x86_64/
│   │   ├── mod.rs
│   │   ├── cpu.rs
│   │   ├── memory.rs
│   │   └── interrupt.rs
│   └── aarch64/
│       ├── mod.rs
│       ├── cpu.rs
│       ├── memory.rs
│       └── interrupt.rs
├── core/                    # 核心功能
│   ├── mod.rs
│   ├── sync/                # 同步原语
│   │   ├── mod.rs
│   │   ├── spinlock.rs
│   │   ├── mutex.rs
│   │   └── atomic.rs
│   ├── types/               # 基础类型
│   │   ├── mod.rs
│   │   ├── result.rs
│   │   └── traits.rs
│   └── macros/             # 核心宏
│       ├── mod.rs
│       └── logging.rs
├── mm/                     # 内存管理
│   ├── mod.rs
│   ├── phys/               # 物理内存管理
│   │   ├── mod.rs
│   │   ├── allocator.rs
│   │   └── page.rs
│   ├── virt/               # 虚拟内存管理
│   │   ├── mod.rs
│   │   ├── vm.rs
│   │   ├── mmap.rs
│   │   └── protection.rs
│   ├── alloc/              # 内存分配器
│   │   ├── mod.rs
│   │   ├── slab.rs
│   │   ├── buddy.rs
│   │   ├── percpu.rs
│   │   └── numa.rs
│   └── api/                # 内存管理API
│       ├── mod.rs
│       ├── syscalls.rs
│       └── error.rs
├── sched/                  # 调度器
│   ├── mod.rs
│   ├── core/               # 调度器核心
│   │   ├── mod.rs
│   │   ├── scheduler.rs
│   │   ├── o1.rs
│   │   └── realtime.rs
│   ├── process/            # 进程管理
│   │   ├── mod.rs
│   │   ├── thread.rs
│   │   ├── context.rs
│   │   └── exec.rs
│   └── api/               # 调度器API
│       ├── mod.rs
│       ├── syscalls.rs
│       └── error.rs
├── fs/                    # 文件系统
│   ├── mod.rs
│   ├── vfs/                # 虚拟文件系统
│   │   ├── mod.rs
│   │   ├── inode.rs
│   │   ├── dentry.rs
│   │   ├── file.rs
│   │   └── super.rs
│   ├── impls/              # 文件系统实现
│   │   ├── mod.rs
│   │   ├── ext4.rs
│   │   ├── fat32.rs
│   │   └── tmpfs.rs
│   └── api/               # 文件系统API
│       ├── mod.rs
│       ├── syscalls.rs
│       └── error.rs
├── net/                   # 网络栈
│   ├── mod.rs
│   ├── core/               # 网络核心
│   │   ├── mod.rs
│   │   ├── packet.rs
│   │   ├── buffer.rs
│   │   └── interface.rs
│   ├── protocols/           # 网络协议
│   │   ├── mod.rs
│   │   ├── ethernet.rs
│   │   ├── ipv4.rs
│   │   ├── tcp.rs
│   │   └── udp.rs
│   ├── drivers/             # 网络驱动
│   │   ├── mod.rs
│   │   ├── virtio_net.rs
│   │   └── e1000.rs
│   └── api/               # 网络API
│       ├── mod.rs
│       ├── socket.rs
│       ├── syscalls.rs
│       └── error.rs
├── ipc/                   # 进程间通信
│   ├── mod.rs
│   ├── pipe.rs
│   ├── mqueue.rs
│   ├── signal.rs
│   └── api/
│       ├── mod.rs
│       ├── syscalls.rs
│       └── error.rs
├── drivers/               # 设备驱动
│   ├── mod.rs
│   ├── bus/               # 总线驱动
│   │   ├── mod.rs
│   │   ├── pci.rs
│   │   └── usb.rs
│   ├── block/             # 块设备驱动
│   │   ├── mod.rs
│   │   ├── nvme.rs
│   │   └── virtio_blk.rs
│   ├── char/              # 字符设备驱动
│   │   ├── mod.rs
│   │   ├── console.rs
│   │   └── uart.rs
│   └── gpu/               # GPU驱动
│       ├── mod.rs
│       └── virtio_gpu.rs
├── syscall/               # 系统调用
│   ├── mod.rs
│   ├── dispatch/          # 系统调用分发
│   │   ├── mod.rs
│   │   ├── dispatcher.rs
│   │   └── fast_path.rs
│   ├── handlers/          # 系统调用处理
│   │   ├── mod.rs
│   │   ├── fs.rs
│   │   ├── mm.rs
│   │   ├── sched.rs
│   │   ├── net.rs
│   │   └── ipc.rs
│   └── api/              # 系统调用API
│       ├── mod.rs
│       ├── types.rs
│       └── error.rs
├── security/             # 安全机制
│   ├── mod.rs
│   ├── access/            # 访问控制
│   │   ├── mod.rs
│   │   ├── acl.rs
│   │   └── capabilities.rs
│   ├── memory/            # 内存安全
│   │   ├── mod.rs
│   │   ├── aslr.rs
│   │   ├── smap_smep.rs
│   │   └── stack_canaries.rs
│   ├── audit/             # 安全审计
│   │   ├── mod.rs
│   │   ├── logging.rs
│   │   └── analysis.rs
│   └── api/              # 安全API
│       ├── mod.rs
│       ├── syscalls.rs
│       └── error.rs
├── error/                # 错误处理
│   ├── mod.rs
│   ├── unified.rs         # 统一错误处理
│   ├── errno.rs          # POSIX错误代码
│   └── recovery.rs       # 错误恢复
├── cloud_native/         # 云原生特性
│   ├── mod.rs
│   ├── cgroups.rs
│   ├── namespaces.rs
│   ├── containers.rs
│   └── oci.rs
├── monitoring/          # 监控和性能
│   ├── mod.rs
│   ├── metrics.rs
│   ├── profiling.rs
│   └── tracing.rs
├── testing/             # 测试框架
│   ├── mod.rs
│   ├── unit.rs
│   ├── integration.rs
│   └── benchmarks.rs
└── features/            # 特性控制
    ├── mod.rs
    ├── kernel.rs
    ├── syscalls.rs
    ├── networking.rs
    ├── security.rs
    └── cloud_native.rs
```

## 重构步骤

1. **创建新的目录结构**：按照上述设计创建新的目录和文件
2. **移动现有代码**：将现有代码移动到新的目录结构中
3. **更新模块引用**：更新所有模块的引用路径
4. **解决依赖问题**：解决重构过程中出现的依赖问题
5. **更新构建配置**：更新Cargo.toml和构建脚本
6. **测试验证**：确保重构后的代码能够正常编译和运行

## 依赖关系优化

1. **单向依赖**：确保模块之间只有单向依赖，避免循环依赖
2. **层次化设计**：上层模块依赖下层模块，下层模块不依赖上层模块
3. **接口抽象**：通过接口抽象减少模块间的直接依赖
4. **事件驱动**：使用事件机制减少模块间的直接调用

## 特性控制优化

1. **集中控制**：将所有特性标志控制集中到features模块
2. **条件编译**：使用条件编译减少不必要的代码包含
3. **特性组合**：定义合理的特性组合，避免冲突
4. **默认特性**：设置合理的默认特性集

## 预期收益

1. **更好的可维护性**：清晰的目录结构使代码更易于维护
2. **更高的开发效率**：合理的模块组织减少开发时间
3. **更强的扩展性**：良好的架构设计使系统更易于扩展
4. **更好的性能**：优化的依赖关系减少编译时间和运行时开销