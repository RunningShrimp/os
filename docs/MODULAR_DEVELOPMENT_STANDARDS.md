# NOS 模块化开发规范

## 概述

本文档定义了NOS（New Operating System）项目的模块化开发标准，旨在提高代码质量、可维护性和开发效率。这些标准适用于所有参与NOS项目的开发者。

## 1. 架构原则

### 1.1 微内核 + 服务架构

NOS采用混合架构设计，结合了微内核的稳定性和服务的灵活性：

- **微内核核心**：提供最基本的系统服务（调度、内存管理、IPC）
- **服务层**：独立的系统服务（内存管理、进程管理、文件系统、网络、设备驱动等）
- **IPC通信**：服务间通过高性能IPC机制通信

### 1.2 模块化设计原则

- **单一职责**：每个模块只负责一个明确的功能
- **松耦合**：模块间依赖最小化，通过定义良好的接口交互
- **高内聚**：相关功能集中在同一模块内
- **可测试性**：每个模块都应该可以独立测试

## 2. 代码组织结构

### 2.1 目录结构

```
kernel/src/
├── main.rs                 # 内核入口点
├── microkernel/            # 微内核核心
│   ├── mod.rs
│   ├── scheduler.rs        # 调度器
│   ├── memory.rs           # 内存管理
│   ├── ipc.rs              # IPC机制
│   ├── interrupt.rs        # 中断处理
│   ├── timer.rs            # 定时器
│   └── service_registry.rs # 服务注册表
├── services/               # 服务层
│   ├── mod.rs
│   ├── memory.rs           # 内存管理服务
│   ├── process.rs          # 进程管理服务
│   ├── fs.rs               # 文件系统服务
│   ├── network.rs          # 网络服务
│   ├── driver.rs           # 设备驱动服务
│   ├── syscall.rs          # 系统调用服务
│   └── ipc.rs              # IPC服务
├── posix/                  # POSIX兼容层
│   ├── mod.rs
│   ├── types.rs            # POSIX类型定义
│   ├── errno.rs            # 错误码定义
│   └── [other posix modules]
├── syscalls/               # 系统调用实现
├── drivers/                # 设备驱动
├── arch/                   # 架构相关代码
│   ├── x86_64/
│   ├── aarch64/
│   └── riscv64/
├── mm/                     # 内存管理
├── vm/                     # 虚拟内存
├── fs/                     # 文件系统
├── net/                    # 网络协议栈
└── [other kernel modules]
```

### 2.2 模块命名规范

- 使用小写字母和下划线
- 模块名应具有描述性
- 同一功能领域使用一致的命名前缀
- 避免缩写，除非是广泛接受的缩写

```rust
// 好的命名示例
mod memory_service;
mod process_manager;
mod file_system;
mod network_stack;

// 不好的命名示例
mod mem;           // 过于简短
mod proc;          // 缩写不明确
mod fs_v2;         // 版本号不合适
mod net_stuff;     # 不具体
```

## 3. 编码规范

### 3.1 Rust编码规范

#### 3.1.1 命名约定

- **模块名**：snake_case
- **类型名**：PascalCase
- **函数名**：snake_case
- **常量名**：SCREAMING_SNAKE_CASE
- **静态变量**：snake_case或SCREAMING_SNAKE_CASE（根据用途）

```rust
// 类型定义
pub struct MemoryManager {
    // 私有字段使用snake_case
    page_allocator: PageAllocator,
    total_memory: usize,
}

// 公共常量
pub const PAGE_SIZE: usize = 4096;
pub const MAX_PROCESSES: usize = 1024;

// 函数名
pub fn allocate_pages(count: usize) -> Result<*mut u8, AllocationError> {
    // 实现
}
```

#### 3.1.2 错误处理

- 使用`Result<T, E>`类型进行错误处理
- 定义具体的错误类型，而不是使用通用错误
- 使用`?`操作符进行错误传播
- 避免使用`panic!`，除非是不可恢复的错误

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryError {
    OutOfMemory,
    InvalidAddress,
    PermissionDenied,
    AlignmentError,
}

pub type MemoryResult<T> = Result<T, MemoryError>;

pub fn allocate_memory(size: usize) -> MemoryResult<*mut u8> {
    if size == 0 {
        return Err(MemoryError::InvalidAddress);
    }

    // 分配逻辑
    Ok(ptr)
}
```

#### 3.1.3 文档注释

- 使用`///`编写文档注释
- 为公共API提供完整的文档
- 包含使用示例
- 说明参数、返回值和可能的错误

```rust
/// 分配指定大小的物理内存页
///
/// # 参数
///
/// * `page_count` - 要分配的页面数量
/// * `flags` - 分配标志
///
/// # 返回值
///
/// 成功时返回分配的内存起始地址，失败时返回`MemoryError`
///
/// # 示例
///
/// ```
/// let pages = allocate_physical_pages(10, AllocationFlags::NORMAL)?;
/// println!("Allocated pages at {:?}", pages);
/// ```
///
/// # 错误
///
/// - `MemoryError::OutOfMemory` - 内存不足
/// - `MemoryError::InvalidAddress` - 无效的地址
pub fn allocate_physical_pages(
    page_count: usize,
    flags: AllocationFlags
) -> MemoryResult<PhysAddr> {
    // 实现
}
```

### 3.2 模块接口设计

#### 3.2.1 公共API设计原则

- **最小化公共API**：只暴露必要的接口
- **一致性**：相似功能使用相似的接口设计
- **向后兼容**：API变更时保持向后兼容性
- **类型安全**：使用强类型避免运行时错误

```rust
// 好的API设计
pub trait FileSystem {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError>;
    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), FsError>;
    fn delete_file(&self, path: &str) -> Result<(), FsError>;
    fn list_directory(&self, path: &str) -> Result<Vec<DirEntry>, FsError>;
}

// 避免这样的设计
pub fn file_operation(op: FileOp, path: &str, data: Option<&[u8]>) -> Result<(), Error> {
    // 类型不安全，容易出错
}
```

#### 3.2.2 服务接口标准

所有服务都应实现以下标准接口：

```rust
pub trait Service {
    /// 服务类型
    fn service_type() -> ServiceType;

    /// 服务名称
    fn service_name() -> &'static str;

    /// 服务版本
    fn service_version() -> &'static str;

    /// 初始化服务
    fn init() -> Result<(), ServiceError>;

    /// 启动服务
    fn start() -> Result<(), ServiceError>;

    /// 停止服务
    fn stop() -> Result<(), ServiceError>;

    /// 获取服务状态
    fn status() -> ServiceStatus;

    /// 处理请求
    fn handle_request(&mut self, request: ServiceRequest) -> ServiceResponse;
}
```

## 4. 服务开发指南

### 4.1 服务创建流程

1. **定义服务接口**
   ```rust
   // services/memory.rs
   pub trait MemoryService {
       fn allocate(&mut self, size: usize) -> Result<MemoryBlock, MemoryError>;
       fn deallocate(&mut self, block: MemoryBlock) -> Result<(), MemoryError>;
       fn get_stats(&self) -> MemoryStats;
   }
   ```

2. **实现服务结构**
   ```rust
   pub struct MemoryManager {
       allocator: PageAllocator,
       stats: MemoryStats,
   }

   impl MemoryService for MemoryManager {
       fn allocate(&mut self, size: usize) -> Result<MemoryBlock, MemoryError> {
           // 实现
       }
       // ...
   }
   ```

3. **注册到服务管理器**
   ```rust
   // 在services/mod.rs的init函数中
   memory::init()?;
   ```

### 4.2 服务间通信

#### 4.2.1 IPC消息标准

```rust
#[derive(Debug, Clone)]
pub struct ServiceMessage {
    pub id: u64,
    pub sender: ServiceId,
    pub receiver: ServiceId,
    pub message_type: MessageType,
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Request(RequestType),
    Response(ResponseType),
    Event(EventType),
    Notification(NotificationType),
}
```

#### 4.2.2 服务发现

```rust
// 使用服务注册表查找服务
pub fn find_service(service_type: ServiceType) -> Option<ServiceId> {
    let registry = get_service_registry()?;
    registry.find_by_type(service_type)
}

// 发送消息到服务
pub fn send_to_service(
    service_id: ServiceId,
    message: ServiceMessage
) -> Result<(), IpcError> {
    let ipc_service = get_ipc_service()?;
    ipc_service.send_message(service_id, message)
}
```

### 4.3 服务生命周期管理

```rust
pub struct ServiceLifecycle {
    state: ServiceState,
    dependencies: Vec<ServiceType>,
}

impl ServiceLifecycle {
    pub fn initialize(&mut self) -> Result<(), ServiceError> {
        // 1. 检查依赖
        self.check_dependencies()?;

        // 2. 初始化资源
        self.init_resources()?;

        // 3. 注册服务
        self.register_service()?;

        // 4. 更新状态
        self.state = ServiceState::Running;

        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), ServiceError> {
        // 关闭流程
        self.state = ServiceState::Stopping;
        self.cleanup_resources()?;
        self.unregister_service()?;
        self.state = ServiceState::Stopped;
        Ok(())
    }
}
```

## 5. 测试规范

### 5.1 单元测试

- 每个模块都应有对应的单元测试
- 测试文件命名：`module_name.rs` -> `module_name_tests.rs`
- 测试覆盖率应达到80%以上

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_allocation() {
        let mut manager = MemoryManager::new();
        let block = manager.allocate(1024).unwrap();
        assert_eq!(block.size, 1024);

        manager.deallocate(block).unwrap();
    }

    #[test]
    fn test_memory_allocation_failure() {
        let mut manager = MemoryManager::new();
        // 测试分配过大内存的情况
        let result = manager.allocate(usize::MAX);
        assert!(result.is_err());
    }
}
```

### 5.2 集成测试

- 测试服务间的交互
- 测试完整的业务流程
- 使用模拟对象隔离外部依赖

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_service_communication() {
        // 设置测试环境
        let mut memory_service = MemoryService::new();
        let mut process_service = ProcessService::new();

        // 测试服务间通信
        let request = ServiceRequest::AllocateMemory { size: 4096 };
        let response = memory_service.handle_request(request);

        assert!(matches!(response, ServiceResponse::MemoryAllocated { .. }));
    }
}
```

### 5.3 性能测试

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_ipc_performance() {
        let ipc_service = IpcService::new().unwrap();
        let channel = ipc_service.create_channel("test", IpcChannelType::PointToPoint, 1000).unwrap();

        let start = Instant::now();

        // 发送1000个消息
        for i in 0..1000 {
            let message = IpcMessage::new(1, 2, IpcMessageType::Data)
                .with_data(vec![i as u8; 64]);
            ipc_service.send_message(channel, message).unwrap();
        }

        let duration = start.elapsed();
        println!("Sent 1000 messages in {:?}", duration);

        // 性能断言
        assert!(duration.as_millis() < 100); // 应该在100ms内完成
    }
}
```

## 6. 文档规范

### 6.1 代码文档

- 每个模块都应有模块级文档
- 公共API必须有文档注释
- 复杂算法需要有实现说明

```rust
//! 内存管理服务
//!
//! 提供物理内存和虚拟内存的分配、释放和管理功能。
//! 支持多种分配策略和内存优化技术。
//!
//! # 功能特性
//!
//! - 物理内存页面分配
//! - 虚拟内存映射
//! - 内存压缩
//! - 大页内存支持
//! - 内存统计和监控
//!
//! # 使用示例
//!
//! ```
//! use nos::services::memory::MemoryService;
//!
//! let mut memory_service = MemoryService::new().unwrap();
//!
//! // 分配1MB内存
//! let block = memory_service.allocate(1024 * 1024).unwrap();
//!
//! // 使用内存
//! unsafe {
//!     let ptr = block.as_mut_ptr();
//!     ptr.write_bytes(0, 1024 * 1024);
//! }
//!
//! // 释放内存
//! memory_service.deallocate(block).unwrap();
//! ```
```

### 6.2 API文档

使用`rustdoc`生成API文档，遵循以下规范：

- 提供完整的模块、结构体、函数文档
- 包含使用示例
- 说明性能特征和限制
- 提供相关概念的链接

### 6.3 架构文档

为复杂的系统组件编写架构文档：

```
docs/
├── architecture/
│   ├── overview.md              # 整体架构概述
│   ├── microkernel.md            # 微内核设计
│   ├── services.md               # 服务层设计
│   ├── ipc.md                    # IPC机制
│   └── memory-management.md      # 内存管理架构
├── api/
│   ├── services.md               # 服务API
│   ├── syscalls.md               # 系统调用API
│   └── drivers.md                # 驱动API
├── development/
│   ├── coding-standards.md       # 编码规范
│   ├── testing-guide.md          # 测试指南
│   ├── performance.md            # 性能优化指南
│   └── debugging.md              # 调试指南
└── examples/
    ├── hello-world.md            # 示例程序
    ├── device-driver.md          # 设备驱动示例
    └── filesystem.md             # 文件系统示例
```

## 7. 版本控制规范

### 7.1 分支策略

- `main`：主开发分支，包含最新的稳定代码
- `develop`：开发分支，集成新功能
- `feature/*`：功能分支，开发单个功能
- `hotfix/*`：热修复分支，紧急修复
- `release/*`：发布分支，发布准备

### 7.2 提交信息规范

使用Conventional Commits格式：

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

类型（type）：
- `feat`：新功能
- `fix`：bug修复
- `docs`：文档更新
- `style`：代码格式化
- `refactor`：重构
- `test`：测试相关
- `chore`：构建过程或辅助工具的变动

示例：
```
feat(memory): add huge page support

Implement 2MB and 1GB huge page allocation to improve TLB hit rate
for large memory allocations. This change reduces page table overhead
by ~90% for allocations larger than 2MB.

Closes #123
```

### 7.3 代码审查要求

所有代码变更都必须经过代码审查：

- **功能正确性**：代码实现符合需求
- **代码质量**：遵循编码规范
- **性能影响**：评估性能影响
- **安全性**：检查安全漏洞
- **测试覆盖**：确保有足够的测试

## 8. 性能优化指南

### 8.1 内存优化

- 避免不必要的内存分配
- 使用对象池重用对象
- 优先使用栈分配
- 减少内存碎片

```rust
// 好的实践：使用对象池
pub struct BufferPool {
    buffers: Vec<Vec<u8>>,
    available: Vec<usize>,
}

impl BufferPool {
    pub fn get_buffer(&mut self) -> Vec<u8> {
        if let Some(index) = self.available.pop() {
            let mut buffer = self.buffers.swap_remove(index);
            buffer.clear();
            buffer
        } else {
            Vec::with_capacity(4096)
        }
    }

    pub fn return_buffer(&mut self, mut buffer: Vec<u8>) {
        buffer.clear();
        if buffer.capacity() <= 4096 {
            self.buffers.push(buffer);
            self.available.push(self.buffers.len() - 1);
        }
    }
}
```

### 8.2 并发优化

- 使用无锁数据结构
- 减少锁的粒度和持有时间
- 避免false sharing
- 使用适当的并发原语

```rust
// 使用原子操作避免锁
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct LockFreeCounter {
    count: AtomicUsize,
}

impl LockFreeCounter {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
        }
    }

    pub fn increment(&self) -> usize {
        self.count.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn get(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}
```

### 8.3 I/O优化

- 使用异步I/O
- 批量操作减少系统调用
- 缓存频繁访问的数据
- 零拷贝技术

## 9. 调试指南

### 9.1 日志系统

使用统一的日志系统：

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

pub fn log(level: LogLevel, module: &str, message: &str) {
    match level {
        LogLevel::Error => eprintln!("[ERROR][{}] {}", module, message),
        LogLevel::Warn => println!("[WARN][{}] {}", module, message),
        LogLevel::Info => println!("[INFO][{}] {}", module, message),
        LogLevel::Debug => println!("[DEBUG][{}] {}", module, message),
        LogLevel::Trace => println!("[TRACE][{}] {}", module, message),
    }
}

// 使用宏简化日志记录
macro_rules! log_info {
    ($($arg:tt)*) => (log!(LogLevel::Info, module_path!(), &format!($($arg)*)))
}
```

### 9.2 断点调试

提供调试接口：

```rust
#[cfg(debug_assertions)]
pub fn debug_assert_enabled() -> bool {
    true
}

#[cfg(not(debug_assertions))]
pub fn debug_assert_enabled() -> bool {
    false
}

// 条件断点
#[macro_export]
macro_rules! debug_break {
    ($($arg:tt)*) => {
        if debug_assert_enabled() {
            println!("[DEBUG] {}: {}", module_path!(), format!($($arg)*));
        }
    };
}
```

### 9.3 性能分析

集成性能分析工具：

```rust
pub struct PerformanceProfiler {
    start_time: u64,
    measurements: Vec<(&'static str, u64)>,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            start_time: rdtsc(),
            measurements: Vec::new(),
        }
    }

    pub fn measure(&mut self, name: &'static str) {
        let current_time = rdtsc();
        self.measurements.push((name, current_time - self.start_time));
        self.start_time = current_time;
    }

    pub fn print_results(&self) {
        println!("Performance Profile:");
        for (name, time) in &self.measurements {
            println!("  {}: {} cycles", name, time);
        }
    }
}
```

## 10. 安全性规范

### 10.1 内存安全

- 避免缓冲区溢出
- 检查边界条件
- 使用安全的字符串操作
- 防止UAF（Use After Free）

```rust
// 安全的字符串复制
pub fn safe_strcpy(dest: &mut [u8], src: &str) -> Result<(), ()> {
    if src.len() > dest.len() {
        return Err(());
    }

    dest[..src.len()].copy_from_slice(src.as_bytes());
    Ok(())
}

// 边界检查的数组访问
pub fn safe_array_access<T>(arr: &[T], index: usize) -> Option<&T> {
    arr.get(index)
}
```

### 10.2 权限检查

- 验证所有输入参数
- 实施最小权限原则
- 检查系统调用权限
- 防止权限提升

```rust
pub fn check_file_permissions(path: &str, uid: uid_t, mode: u32) -> Result<(), i32> {
    let metadata = get_file_metadata(path)?;

    // 检查文件是否存在
    if !metadata.exists {
        return Err(ENOENT);
    }

    // 检查读权限
    if (mode & O_RDONLY) != 0 && !metadata.is_readable_by(uid) {
        return Err(EACCES);
    }

    // 检查写权限
    if (mode & O_WRONLY) != 0 && !metadata.is_writable_by(uid) {
        return Err(EACCES);
    }

    Ok(())
}
```

## 11. 总结

本规范定义了NOS项目的模块化开发标准，包括：

1. **架构原则**：微内核+服务架构的设计理念
2. **代码组织**：标准化的目录结构和命名规范
3. **编码规范**：Rust语言的最佳实践
4. **服务开发**：标准化的服务开发流程
5. **测试规范**：全面的测试策略
6. **文档规范**：完整的文档要求
7. **版本控制**：标准化的工作流程
8. **性能优化**：性能优化指南
9. **调试指南**：调试工具和技术
10. **安全性规范**：安全编程实践

遵循这些规范将确保NOS项目的代码质量、可维护性和可持续发展。

---

*版本：1.0*
*最后更新：2024年*
*维护者：NOS开发团队*