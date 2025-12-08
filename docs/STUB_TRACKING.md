# 存根跟踪系统

本文档跟踪项目中剩余的存根（stub）实现，记录它们的位置、用途和替换计划。

## 已替换的存根

### IPC相关类型 ✅
- **ServiceId**: 已替换为 `crate::microkernel::service_registry::ServiceId` (u64类型别名)
- **Message**: 已替换为 `crate::microkernel::ipc::IpcMessage`
- **MessageType**: 已实现为包装u32的结构体，映射到IpcMessage的message_type字段
- **send_message/receive_message**: 已实现使用真实的IPC管理器

### 服务注册表相关 ✅
- **ServiceInfo**: 已替换为 `crate::microkernel::service_registry::ServiceInfo`
- **ServiceCategory**: 已替换为 `crate::microkernel::service_registry::ServiceCategory` (枚举)
- **InterfaceVersion**: 已替换为 `crate::microkernel::service_registry::InterfaceVersion`
- **ServiceRegistry**: 已替换为 `crate::microkernel::service_registry::ServiceRegistry`
- **get_service_registry**: 已替换为 `crate::microkernel::service_registry::get_service_registry`

## 剩余的存根

### POSIX类型存根
位置: `kernel/src/types/stubs.rs`

- **pid_t**: `pub type pid_t = i32;` - 基本类型别名，可能需要移动到posix模块
- **uid_t**: `pub type uid_t = u32;` - 基本类型别名，可能需要移动到posix模块
- **gid_t**: `pub type gid_t = u32;` - 基本类型别名，可能需要移动到posix模块
- **AF_UNIX**: `pub type AF_UNIX = i32;` - 套接字地址族常量，可能需要移动到posix模块

### 进程相关存根
位置: `kernel/src/types/stubs.rs`

- **Process**: 简化的进程结构体
  - 当前实现: 只有pid和name字段
  - 真实实现: 应使用 `crate::process::Process` 或类似的结构
  - 状态: 待替换

### 内存地址类型
位置: `kernel/src/types/stubs.rs`

- **VirtAddr**: 虚拟地址包装类型
  - 当前实现: 简单的usize包装
  - 状态: 可能需要保留，但应移动到mm模块

### RNG存根
位置: `kernel/src/types/stubs.rs`

- **RNG**: 随机数生成器存根
  - 当前实现: 简单的伪随机生成器
  - 真实实现: 应使用真实的随机数生成器实现
  - 状态: 待替换

### 错误处理存根
位置: `kernel/src/types/stubs.rs`

- **errno模块**: 只包含部分错误码
  - 当前实现: EPERM, EACCES, ENOENT
  - 真实实现: 应使用 `crate::reliability::errno` 或 `crate::posix::errno`
  - 状态: 待替换

### VFS相关存根
位置: `kernel/src/types/stubs.rs`

- **VfsNode**: 空的存根结构体
  - 状态: 待替换为真实的VFS节点实现
- **FileMode**: 空的存根结构体
  - 状态: 待替换为真实的文件模式实现

### IPC管理器存根
位置: `kernel/src/types/stubs.rs`

- **IpcManager**: 空的存根结构体
  - 状态: 已替换为 `crate::microkernel::ipc::IpcManager`
- **IpcMessage**: 空的存根结构体
  - 状态: 已替换为 `crate::microkernel::ipc::IpcMessage`

### 内存管理器存根
位置: `kernel/src/types/stubs.rs`

- **MicroMemoryManager**: 空的存根结构体
  - 状态: 待替换为真实的内存管理器实现

### 设备驱动存根
位置: `kernel/src/types/stubs.rs`

- **BlockDevice**: Trait存根
  - 当前实现: 只有read和write方法
  - 状态: 可能需要保留，但应移动到drivers模块

## 替换优先级

### P0 - 高优先级（影响核心功能）
1. ✅ ServiceId, Message, MessageType - **已完成**
2. ✅ ServiceInfo, ServiceCategory, InterfaceVersion - **已完成**
3. Process存根 - 影响进程管理功能
4. errno存根 - 影响错误处理

### P1 - 中优先级（影响功能完整性）
5. VfsNode, FileMode - 影响文件系统功能
6. MicroMemoryManager - 影响内存管理功能
7. RNG - 影响安全性功能

### P2 - 低优先级（代码组织）
8. pid_t, uid_t, gid_t类型别名 - 移动到posix模块
9. VirtAddr - 移动到mm模块
10. BlockDevice trait - 移动到drivers模块

## 替换计划

### 阶段1: 核心类型替换（已完成）✅
- [x] IPC类型替换
- [x] 服务注册表类型替换

### 阶段2: 进程和错误处理（计划中）
- [ ] Process存根替换
- [ ] errno存根替换

### 阶段3: 文件系统和内存管理（计划中）
- [ ] VfsNode, FileMode替换
- [ ] MicroMemoryManager替换

### 阶段4: 代码组织（计划中）
- [ ] 类型别名移动到正确模块
- [ ] Trait移动到正确模块

## 使用统计

### 当前存根使用情况
- `kernel/src/services/ipc.rs`: 使用Message, MessageType, ServiceId ✅
- `kernel/src/services/syscall.rs`: 使用ServiceId, Message, MessageType ✅
- `kernel/src/services/driver.rs`: 使用Message, MessageType ✅
- `kernel/src/services/network.rs`: 使用Message, MessageType ✅
- `kernel/src/security/*.rs`: 使用通配符导入（需要检查具体使用）
- `kernel/src/process/mod.rs`: 使用uid_t, gid_t

## 注意事项

1. **向后兼容**: 替换存根时，需要确保API兼容性
2. **测试**: 替换后需要运行测试确保功能正常
3. **文档**: 更新相关文档说明新的实现
4. **性能**: 确保新实现不会引入性能问题

## 完整存根清单（2024-12扫描）

**总计**: 318处TODO/FIXME/STUB标记

### 按模块分布（前20个文件）

| 文件 | 存根数量 | 优先级 |
|------|---------|--------|
| `syscalls/process.rs` | 31 | P0 |
| `ids/host_ids/host_ids.rs` | 20 | P1 |
| `syscalls/time.rs` | 16 | P1 |
| `syscalls/signal.rs` | 16 | P1 |
| `syscalls/memory.rs` | 16 | P0 |
| `syscalls/thread.rs` | 15 | P0 |
| `syscalls/glib.rs` | 13 | P2 |
| `syscalls/file_io.rs` | 13 | P0 |
| `formal_verification/static_analyzer.rs` | 13 | P2 |
| `types/stubs.rs` | 12 | P1 |
| `posix/timer.rs` | 10 | P1 |
| `syscalls/network/interface.rs` | 9 | P1 |
| `syscalls/fs.rs` | 9 | P0 |
| `services/network.rs` | 9 | P1 |
| `ipc/mod.rs` | 8 | P0 |
| `fs/fs_impl.rs` | 7 | P0 |
| `drivers/mod.rs` | 6 | P1 |
| `posix/mqueue.rs` | 5 | P1 |
| `vfs/ext4.rs` | 4 | P0 |
| `syscalls/zero_copy.rs` | 4 | P1 |

### 按功能分类

#### P0 - 高优先级（核心功能，影响系统稳定性）

**系统调用模块** (约100处)
- `syscalls/process.rs`: 31处 - 进程管理相关
- `syscalls/memory.rs`: 16处 - 内存管理相关
- `syscalls/thread.rs`: 15处 - 线程管理相关
- `syscalls/file_io.rs`: 13处 - 文件I/O相关
- `syscalls/fs.rs`: 9处 - 文件系统相关

**IPC和进程管理** (约40处)
- `ipc/mod.rs`: 8处 - IPC机制相关
- `process/`: 进程管理相关存根

**文件系统** (约20处)
- `vfs/ext4.rs`: 4处 - ext4文件系统
- `fs/fs_impl.rs`: 7处 - 文件系统实现

#### P1 - 中优先级（功能完整性）

**POSIX接口** (约30处)
- `posix/timer.rs`: 10处 - 定时器相关
- `posix/mqueue.rs`: 5处 - 消息队列相关
- `posix/shm.rs`: 共享内存相关

**网络和安全** (约30处)
- `syscalls/network/`: 网络系统调用
- `ids/host_ids/host_ids.rs`: 20处 - 主机IDS相关
- `services/network.rs`: 9处 - 网络服务

**时间管理** (约16处)
- `syscalls/time.rs`: 16处 - 时间相关系统调用

#### P2 - 低优先级（代码组织、工具）

**形式化验证** (约30处)
- `formal_verification/static_analyzer.rs`: 13处
- `formal_verification/model_checker.rs`: 时间戳相关
- `formal_verification/theorem_prover.rs`: 时间戳相关

**类型定义** (约12处)
- `types/stubs.rs`: 12处 - 类型存根

**其他** (约40处)
- 各种模块中的TODO标记

### 详细存根列表（按模块）

#### 系统调用模块

**syscalls/process.rs** (31处)
- 进程创建、销毁相关存根
- 进程状态管理存根
- 进程间通信存根

**syscalls/memory.rs** (16处)
- 内存映射相关存根
- 内存保护相关存根
- 共享内存相关存根

**syscalls/thread.rs** (15处)
- 线程创建相关存根
- 线程同步原语存根
- TLS相关存根

**syscalls/file_io.rs** (13处)
- 文件读写相关存根
- 文件描述符管理存根

**syscalls/fs.rs** (9处)
- 文件系统操作存根
- 目录操作存根

**syscalls/time.rs** (16处)
- 时间获取相关存根
- 定时器相关存根

**syscalls/signal.rs** (16处)
- 信号处理相关存根
- 信号发送/接收存根

#### POSIX模块

**posix/timer.rs** (10处)
- 定时器创建/删除存根
- 定时器通知存根
- 时钟相关存根

**posix/mqueue.rs** (5处)
- 消息队列操作存根
- 超时处理存根

**posix/shm.rs**
- 共享内存操作存根
- 时间戳相关存根

#### 文件系统模块

**vfs/ext4.rs** (4处)
- ext4文件系统挂载存根
- 同步操作存根
- inode操作存根

**fs/fs_impl.rs** (7处)
- 文件系统实现存根
- 文件操作存根

#### 网络模块

**syscalls/network/interface.rs** (9处)
- 网络接口操作存根
- 套接字操作存根

**services/network.rs** (9处)
- 网络服务相关存根

#### 安全模块

**ids/host_ids/host_ids.rs** (20处)
- 主机IDS相关存根
- 检测规则相关存根

#### 类型定义

**types/stubs.rs** (12处)
- POSIX类型存根
- 进程相关类型存根
- 内存地址类型存根
- RNG存根
- 错误处理存根
- VFS相关存根
- IPC管理器存根
- 内存管理器存根
- 设备驱动存根

## 更新日志

- 2024-12-XX: 完成IPC类型和服务注册表类型的替换
- 2024-12-XX: 建立存根跟踪文档
- 2024-12-XX: 完成libc模块重复实现清理
- 2024-12-XX: 生成完整存根清单（318处）

