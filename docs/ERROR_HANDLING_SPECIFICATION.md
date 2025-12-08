# NOS 错误处理规范

## 概述

本规范定义了NOS（New Operating System）项目的统一错误处理标准，旨在确保整个代码库中的错误处理一致性、可靠性和可维护性。本规范涵盖错误类型定义、POSIX兼容错误码、错误传播模式以及所有内核模块的一致性规则。

## 1. 错误处理原则

### 1.1 核心原则

- **显式错误处理**：所有可能失败的操作必须显式处理错误，不允许使用`unwrap()`或`expect()`进行隐式错误处理
- **类型安全**：使用强类型错误枚举，避免使用通用错误类型
- **POSIX兼容性**：系统调用层必须返回POSIX兼容的错误码
- **错误上下文**：错误应包含足够的上下文信息以便诊断和调试
- **错误聚合**：支持错误聚合和统计，便于监控和分析
- **恢复优先**：优先考虑错误恢复而非程序终止

### 1.2 错误处理层次

NOS采用分层错误处理架构：

```
用户空间应用
    ↓ (系统调用)
系统调用层 (POSIX错误码)
    ↓ (内部API)
内核服务层 (Result<T, E>)
    ↓ (组件间通信)
内核组件层 (Result<T, E>)
    ↓ (硬件抽象)
硬件驱动层 (Result<T, E>)
```

## 2. 统一错误类型 (KernelError)

NOS内核使用统一的错误类型 `KernelError` 进行错误处理，定义在 `kernel/src/error_handling/unified.rs`。

### 2.0.1 使用统一错误类型

所有新代码应使用 `KernelError` 和 `KernelResult<T>`：

```rust
use crate::error_handling::unified::{KernelError, KernelResult};

fn example_function() -> KernelResult<usize> {
    // 使用统一错误类型
    Ok(42)
}
```

### 2.0.2 错误类型转换

现有模块的错误类型可以通过 `From` trait 自动转换为 `KernelError`：

```rust
// SyscallError 自动转换
let err: KernelError = SyscallError::NotFound.into();

// ExecError 自动转换 (需要启用 feature)
let err: KernelError = ExecError::FileNotFound.into();

// ThreadError 自动转换 (需要启用 feature)
let err: KernelError = ThreadError::OutOfMemory.into();
```

## 2. 错误类型定义

### 2.1 错误严重级别

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Info = 0,      // 信息级别
    Warning = 1,   // 警告级别
    Error = 2,     // 错误级别
    Critical = 3,  // 严重错误
    Fatal = 4,     // 致命错误
}
```

### 2.2 错误类别

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    System,        // 系统错误
    Memory,        // 内存错误
    FileSystem,    // 文件系统错误
    Network,       // 网络错误
    Device,        // 设备错误
    Process,       // 进程错误
    Security,      // 安全错误
    Application,   // 应用错误
    Hardware,      // 硬件错误
    Configuration, // 配置错误
    User,          // 用户错误
    Resource,      // 资源错误
    Timeout,       // 超时错误
    Protocol,      // 协议错误
    Data,          // 数据错误
    Interface,     // 接口错误
}
```

### 2.3 错误类型

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    RuntimeError,      // 运行时错误
    LogicError,        // 逻辑错误
    CompileError,      // 编译时错误
    ConfigurationError,// 配置错误
    ResourceError,     // 资源错误
    PermissionError,   // 权限错误
    NetworkError,      // 网络错误
    IOError,           // I/O错误
    MemoryError,       // 内存错误
    SystemCallError,   // 系统调用错误
    ValidationError,   // 验证错误
    TimeoutError,      // 超时错误
    CancellationError, // 取消错误
}
```

### 2.4 错误状态

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStatus {
    New,         // 新错误
    Processing,  // 正在处理
    Recovered,   // 已恢复
    Handled,     // 已处理
    Ignored,     // 忽略
    Escalated,   // 升级中
    Closed,      // 已关闭
}
```

## 3. POSIX兼容错误码

### 3.1 标准错误码定义

NOS实现完整的POSIX错误码集合，位于`kernel/src/reliability/errno.rs`：

```rust
pub const EOK: i32 = 0;           // 成功
pub const EPERM: i32 = 1;         // 操作不被允许
pub const ENOENT: i32 = 2;        // 没有这样的文件或目录
pub const ESRCH: i32 = 3;         // 没有这样的进程
pub const EINTR: i32 = 4;         // 中断的系统调用
pub const EIO: i32 = 5;           // I/O错误
pub const ENXIO: i32 = 6;         // 没有这样的设备或地址
pub const E2BIG: i32 = 7;         // 参数列表过长
pub const ENOEXEC: i32 = 8;       // 可执行文件格式错误
pub const EBADF: i32 = 9;         // 文件描述符错误
pub const ECHILD: i32 = 10;       // 没有子进程
pub const EAGAIN: i32 = 11;       // 资源暂时不可用
pub const ENOMEM: i32 = 12;       // 内存不足
pub const EACCES: i32 = 13;       // 权限被拒绝
pub const EFAULT: i32 = 14;       // 错误的地址
pub const ENOTBLK: i32 = 15;      // 需要块设备
pub const EBUSY: i32 = 16;        // 设备或资源忙
pub const EEXIST: i32 = 17;       // 文件存在
pub const EXDEV: i32 = 18;        // 跨设备链接
pub const ENODEV: i32 = 19;       // 没有这样的设备
pub const ENOTDIR: i32 = 20;      // 不是目录
pub const EISDIR: i32 = 21;       // 是目录
pub const EINVAL: i32 = 22;       // 无效的参数
pub const ENFILE: i32 = 23;       // 文件表溢出
pub const EMFILE: i32 = 24;       // 打开的文件过多
pub const ENOTTY: i32 = 25;       // 不是打字机
pub const ETXTBSY: i32 = 26;      // 文本文件忙
pub const EFBIG: i32 = 27;        // 文件过大
pub const ENOSPC: i32 = 28;       // 设备上没有空间
pub const ESPIPE: i32 = 29;       // 非法查找
pub const EROFS: i32 = 30;        // 只读文件系统
pub const EMLINK: i32 = 31;       // 链接过多
pub const EPIPE: i32 = 32;        // 管道破裂
pub const EDOM: i32 = 33;         // 数学参数超出函数域
pub const ERANGE: i32 = 34;       // 数学结果不可表示
pub const EDEADLK: i32 = 35;      // 资源死锁会发生
pub const ENAMETOOLONG: i32 = 36;// 文件名过长
pub const ENOLCK: i32 = 37;       // 没有可用的记录锁
pub const ENOSYS: i32 = 38;       // 函数未实现
pub const ENOTEMPTY: i32 = 39;    // 目录不为空
pub const ELOOP: i32 = 40;        // 遇到太多符号链接
pub const ENOMSG: i32 = 42;       // 没有所需类型的消息
pub const EIDRM: i32 = 43;        // 标识符已删除
pub const ECHRNG: i32 = 44;       // 通道号超出范围
pub const EL2NSYNC: i32 = 45;     // 2级不同步
pub const EL3HLT: i32 = 46;       // 3级停止
pub const EL3RST: i32 = 47;       // 3级重置
pub const ELNRNG: i32 = 48;       // 链接号超出范围
pub const EUNATCH: i32 = 49;      // 协议驱动未连接
pub const ENOCSI: i32 = 50;       // 没有CSI结构可用
pub const EL2HLT: i32 = 51;       // 2级停止
pub const EBADE: i32 = 52;        // 无效的交换
pub const EBADR: i32 = 53;        // 无效的请求描述符
pub const EXFULL: i32 = 54;       // 交换满
pub const ENOANO: i32 = 55;       // 没有阳极
pub const EBADRQC: i32 = 56;      // 无效的请求代码
pub const EBADSLT: i32 = 57;      // 无效的插槽
pub const EBFONT: i32 = 59;       // 错误的字体文件格式
pub const ENOSTR: i32 = 60;       // 设备不是流
pub const ENODATA: i32 = 61;      // 没有数据可用
pub const ETIME: i32 = 62;        // 计时器到期
pub const ENOSR: i32 = 63;        // 流资源不足
pub const ENONET: i32 = 64;       // 机器不在网络上
pub const ENOPKG: i32 = 65;       // 包未安装
pub const EREMOTE: i32 = 66;      // 对象是远程的
pub const ENOLINK: i32 = 67;      // 链接已断开
pub const EADV: i32 = 68;         // 广告错误
pub const ESRMNT: i32 = 69;       // Srmount错误
pub const ECOMM: i32 = 70;        // 发送时的通信错误
pub const EPROTO: i32 = 71;       // 协议错误
pub const EMULTIHOP: i32 = 72;    // 多跳尝试
pub const EDOTDOT: i32 = 73;      // RFS特定错误
pub const EBADMSG: i32 = 74;      // 不是数据消息
pub const EOVERFLOW: i32 = 75;    // 值太大，无法用定义的数据类型表示
pub const ENOTUNIQ: i32 = 76;     // 名称在网络上不唯一
pub const EBADFD: i32 = 77;       // 文件描述符处于错误状态
pub const EREMCHG: i32 = 78;      // 远程地址已更改
pub const ELIBACC: i32 = 79;      // 无法访问需要的共享库
pub const ELIBBAD: i32 = 80;      // 损坏的共享库访问
pub const ELIBSCN: i32 = 81;      // a.out中的.lib部分损坏
pub const ELIBMAX: i32 = 82;      // 尝试链接太多共享库
pub const ELIBEXEC: i32 = 83;     // 无法直接执行共享库
pub const EILSEQ: i32 = 84;       // 非法字节序列
pub const ERESTART: i32 = 85;     // 中断的系统调用应重新启动
pub const ESTRPIPE: i32 = 86;     // 流管道错误
pub const EUSERS: i32 = 87;       // 用户过多
pub const ENOTSOCK: i32 = 88;     // 套接字操作在非套接字上
pub const EDESTADDRREQ: i32 = 89;// 需要目标地址
pub const EMSGSIZE: i32 = 90;     // 消息过长
pub const EPROTOTYPE: i32 = 91;   // 套接字的协议类型错误
pub const ENOPROTOOPT: i32 = 92;  // 协议不可用
pub const EPROTONOSUPPORT: i32 = 93; // 协议不支持
pub const ESOCKTNOSUPPORT: i32 = 94; // 套接字类型不支持
pub const EOPNOTSUPP: i32 = 95;   // 操作不支持
pub const EPFNOSUPPORT: i32 = 96; // 协议族不支持
pub const EAFNOSUPPORT: i32 = 97; // 地址族不支持协议
pub const EADDRINUSE: i32 = 98;   // 地址已在使用
pub const EADDRNOTAVAIL: i32 = 99;// 无法分配请求的地址
pub const ENETDOWN: i32 = 100;    // 网络已关闭
pub const ENETUNREACH: i32 = 101;// 网络不可达
pub const ENETRESET: i32 = 102;   // 由于重置，网络连接断开
pub const ECONNABORTED: i32 = 103;// 软件导致连接中止
pub const ECONNRESET: i32 = 104;  // 连接被对方重置
pub const ENOBUFS: i32 = 105;     // 没有可用的缓冲区空间
pub const EISCONN: i32 = 106;     // 传输端点已连接
pub const ENOTCONN: i32 = 107;    // 传输端点未连接
pub const ESHUTDOWN: i32 = 108;   // 传输端点关闭后无法发送
pub const ETOOMANYREFS: i32 = 109;// 引用过多，无法拼接
pub const ETIMEDOUT: i32 = 110;   // 连接超时
pub const ECONNREFUSED: i32 = 111;// 连接被拒绝
pub const EHOSTDOWN: i32 = 112;   // 主机已关闭
pub const EHOSTUNREACH: i32 = 113;// 没有到主机的路由
pub const EALREADY: i32 = 114;    // 操作已在进行中
pub const EINPROGRESS: i32 = 115; // 操作正在进行中
pub const ESTALE: i32 = 116;      // 过时的文件句柄
pub const EUCLEAN: i32 = 117;     // 结构需要清理
pub const ENOTNAM: i32 = 118;     // 不是XENIX命名类型文件
pub const ENAVAIL: i32 = 119;     // 没有可用的XENIX信号量
pub const EISNAM: i32 = 120;      // 是命名类型文件
pub const EREMOTEIO: i32 = 121;   // 远程I/O错误
pub const EDQUOT: i32 = 122;      // 超出磁盘配额
pub const ENOMEDIUM: i32 = 123;   // 没有找到介质
pub const EMEDIUMTYPE: i32 = 124; // 错误的介质类型
pub const ECANCELED: i32 = 125;   // 操作已取消
pub const ENOKEY: i32 = 126;      // 所需的密钥不可用
pub const EKEYEXPIRED: i32 = 127; // 密钥已过期
pub const EKEYREVOKED: i32 = 128; // 密钥已被撤销
pub const EKEYREJECTED: i32 = 129;// 密钥被服务拒绝
pub const EOWNERDEAD: i32 = 130;  // 所有者已死
pub const ENOTRECOVERABLE: i32 = 131; // 状态不可恢复
pub const ERFKILL: i32 = 132;     // 操作不可能由于RF-kill
pub const EHWPOISON: i32 = 133;   // 内存页有硬件错误
```

### 3.2 错误码转换

系统调用层必须将内部错误转换为POSIX错误码：

```rust
/// 将SyscallError转换为POSIX错误码
pub fn syscall_error_to_errno(error: SyscallError) -> i32 {
    match error {
        SyscallError::InvalidSyscall => ENOSYS,
        SyscallError::PermissionDenied => EPERM,
        SyscallError::InvalidArgument => EINVAL,
        SyscallError::NotFound => ENOENT,
        SyscallError::OutOfMemory => ENOMEM,
        SyscallError::Interrupted => EINTR,
        SyscallError::IoError => EIO,
        SyscallError::WouldBlock => EAGAIN,
        SyscallError::NotSupported => EOPNOTSUPP,
    }
}
```

## 4. 错误传播模式

### 4.1 Result类型使用

所有函数必须使用`Result<T, E>`类型进行错误处理：

```rust
// 正确的使用方式
pub fn allocate_memory(size: usize) -> Result<MemoryBlock, MemoryError> {
    // 实现
}

// 避免的错误方式
pub fn allocate_memory(size: usize) -> *mut u8 {
    // 可能panic或返回null
}
```

### 4.2 错误传播操作符

使用`?`操作符进行错误传播：

```rust
pub fn complex_operation() -> Result<Data, Error> {
    let resource = acquire_resource()?;
    let processed = process_data(resource)?;
    let result = validate_result(processed)?;
    Ok(result)
}
```

### 4.3 错误转换

使用`map_err`进行错误类型转换：

```rust
pub fn filesystem_operation() -> Result<(), FsError> {
    let file = open_file(path).map_err(|e| FsError::IoError(e))?;
    // 操作文件
    Ok(())
}
```

### 4.4 错误组合

使用错误组合模式处理多个错误源：

```rust
pub fn multi_step_operation() -> Result<(), CombinedError> {
    match step1() {
        Ok(data) => match step2(data) {
            Ok(result) => Ok(result),
            Err(e) => Err(CombinedError::Step2Failed(e)),
        },
        Err(e) => Err(CombinedError::Step1Failed(e)),
    }
}
```

## 5. 错误记录和报告

### 5.1 错误记录结构

```rust
#[derive(Debug, Clone)]
pub struct ErrorRecord {
    pub id: u64,                    // 错误ID
    pub code: u32,                  // 错误代码
    pub error_type: ErrorType,      // 错误类型
    pub category: ErrorCategory,    // 错误类别
    pub severity: ErrorSeverity,    // 严重级别
    pub status: ErrorStatus,        // 错误状态
    pub message: String,            // 错误消息
    pub description: String,        // 详细描述
    pub source: ErrorSource,        // 错误源
    pub timestamp: u64,             // 发生时间
    pub context: ErrorContext,      // 错误上下文
    pub stack_trace: Vec<StackFrame>, // 堆栈跟踪
    pub system_state: SystemStateSnapshot, // 系统状态快照
    pub recovery_actions: Vec<RecoveryAction>, // 恢复动作
    pub occurrence_count: u32,      // 重复次数
    pub last_occurrence: u64,       // 上次发生时间
    pub resolved: bool,             // 是否已解决
    pub resolution_time: Option<u64>, // 解决时间
    pub resolution_method: Option<String>, // 解决方法
    pub metadata: BTreeMap<String, String>, // 元数据
}
```

### 5.2 错误报告级别

- **DEBUG**: 详细的调试信息，包括堆栈跟踪和系统状态
- **INFO**: 一般信息，记录错误发生但不影响系统运行
- **WARNING**: 警告信息，可能需要注意但不严重
- **ERROR**: 错误信息，需要处理但系统可继续运行
- **CRITICAL**: 严重错误，可能影响系统稳定性
- **FATAL**: 致命错误，系统无法继续运行

## 6. 错误恢复策略

### 6.1 恢复策略类型

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    None,           // 无恢复
    Retry,          // 自动重试
    Degrade,        // 降级服务
    Restart,        // 重启组件
    Failover,       // 切换到备份
    Isolate,        // 隔离故障
    Manual,         // 用户干预
    Ignore,         // 忽略错误
}
```

### 6.2 自动恢复机制

```rust
pub struct ErrorRecoveryManager {
    pub max_retries: u32,
    pub retry_interval_ms: u64,
    pub escalation_threshold: u32,
    pub auto_recovery_strategies: Vec<RecoveryStrategy>,
}

impl ErrorRecoveryManager {
    pub fn attempt_recovery(&mut self, error: &ErrorRecord) -> Result<(), RecoveryError> {
        for strategy in &self.auto_recovery_strategies {
            match self.apply_strategy(strategy, error) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // 记录恢复失败，继续尝试下一个策略
                    log_recovery_failure(strategy, e);
                }
            }
        }
        Err(RecoveryError::AllStrategiesFailed)
    }
}
```

## 7. 模块特定错误处理规则

### 7.1 系统调用模块

系统调用必须：
- 返回POSIX兼容的错误码（负值）
- 使用`SyscallResult`类型
- 在`kernel/src/syscalls/mod.rs`中统一转换错误

```rust
// kernel/src/syscalls/mod.rs
pub fn dispatch(syscall_num: usize, args: &[usize]) -> isize {
    let result = match syscall_num {
        // ... syscall routing ...
    };

    match result {
        Ok(value) => value as isize,
        Err(error) => syscall_error_to_errno(error),
    }
}
```

### 7.2 内存管理模块

内存错误必须：
- 优先使用内存池避免分配失败
- 实现内存压力下的降级策略
- 提供详细的内存使用统计

```rust
pub enum MemoryError {
    OutOfMemory,
    InvalidAddress,
    PermissionDenied,
    AlignmentError,
    FragmentationError,
}

pub type MemoryResult<T> = Result<T, MemoryError>;
```

### 7.3 文件系统模块

文件系统错误必须：
- 区分临时错误和永久错误
- 实现文件系统一致性检查
- 支持事务回滚机制

```rust
pub enum FsError {
    NotFound,
    PermissionDenied,
    DiskFull,
    Corruption,
    LockConflict,
    Timeout,
}
```

### 7.4 网络模块

网络错误必须：
- 区分连接错误和数据错误
- 实现连接池和重试机制
- 提供网络诊断信息

```rust
pub enum NetworkError {
    ConnectionFailed,
    Timeout,
    HostUnreachable,
    ProtocolError,
    DataCorruption,
}
```

### 7.5 设备驱动模块

设备错误必须：
- 实现设备热插拔处理
- 提供设备状态监控
- 支持故障设备隔离

```rust
pub enum DeviceError {
    NotFound,
    Busy,
    Timeout,
    HardwareFailure,
    Unsupported,
}
```

## 8. 错误处理最佳实践

### 8.1 错误消息规范

- 使用英文编写错误消息
- 提供清晰、具体的错误描述
- 包含相关的上下文信息
- 避免暴露敏感信息

```rust
// 好的错误消息
pub const ERR_FILE_NOT_FOUND: &str = "File not found: {}";
pub const ERR_PERMISSION_DENIED: &str = "Permission denied for user {} on file {}";

// 不好的错误消息
pub const ERR_GENERIC: &str = "Something went wrong";
pub const ERR_SECRET: &str = "Access denied: password incorrect for user root";
```

### 8.2 错误处理模式

#### 8.2.1 早期返回模式

```rust
pub fn process_request(request: Request) -> Result<Response, Error> {
    // 验证输入
    if !request.is_valid() {
        return Err(Error::InvalidRequest);
    }

    // 检查权限
    if !has_permission(&request.user) {
        return Err(Error::PermissionDenied);
    }

    // 处理请求
    let response = perform_processing(request)?;
    Ok(response)
}
```

#### 8.2.2 错误转换模式

```rust
pub fn high_level_operation() -> Result<(), HighLevelError> {
    low_level_operation()
        .map_err(|e| match e {
            LowLevelError::IoError(io_err) => HighLevelError::StorageError(io_err),
            LowLevelError::NetworkError(net_err) => HighLevelError::CommunicationError(net_err),
            _ => HighLevelError::InternalError(e),
        })
}
```

#### 8.2.3 错误聚合模式

```rust
pub fn batch_operation(items: Vec<Item>) -> Result<Vec<Result>, BatchError> {
    let mut results = Vec::new();
    let mut errors = Vec::new();

    for item in items {
        match process_item(item) {
            Ok(result) => results.push(result),
            Err(error) => errors.push(error),
        }
    }

    if errors.is_empty() {
        Ok(results)
    } else {
        Err(BatchError {
            successful: results,
            failed: errors,
        })
    }
}
```

### 8.3 错误测试

每个错误处理路径都应该有对应的测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_handling() {
        // 测试正常情况
        assert!(normal_operation().is_ok());

        // 测试错误情况
        assert!(matches!(error_operation(), Err(Error::ExpectedError)));

        // 测试错误恢复
        assert!(recovery_operation().is_ok());
    }

    #[test]
    fn test_error_propagation() {
        let result = chained_operation();
        assert!(result.is_err());
        // 验证错误类型和消息
    }
}
```

## 9. 错误监控和分析

### 9.1 错误统计

```rust
pub struct ErrorStatistics {
    pub total_errors: u64,
    pub errors_by_category: HashMap<ErrorCategory, u64>,
    pub errors_by_severity: HashMap<ErrorSeverity, u64>,
    pub recovery_success_rate: f64,
    pub average_resolution_time: Duration,
}
```

### 9.2 错误趋势分析

- 错误发生频率分析
- 错误模式识别
- 预测性错误检测
- 性能影响评估

### 9.3 错误报告生成

系统应支持生成各种错误报告：
- 每日错误摘要
- 错误趋势报告
- 系统健康报告
- 调试诊断报告

## 10. 实施指南

### 10.1 迁移现有代码

1. **识别错误处理点**：查找所有使用`unwrap()`、`expect()`、`panic!`的地方
2. **定义错误类型**：为每个模块创建适当的错误枚举
3. **更新函数签名**：将返回类型改为`Result<T, E>`
4. **实现错误转换**：添加错误传播和转换逻辑
5. **更新调用者**：修改调用代码以处理Result类型

### 10.2 新代码开发

1. **设计错误类型**：在设计阶段就考虑错误处理
2. **使用类型驱动开发**：让错误类型驱动API设计
3. **编写测试先行**：先写错误情况的测试
4. **实现错误处理**：在实现过程中处理所有错误路径
5. **代码审查**：确保错误处理符合规范

### 10.3 工具支持

- **编译时检查**：使用Rust的类型系统强制错误处理
- **静态分析**：使用clippy检查错误处理问题
- **运行时监控**：实现错误收集和报告系统
- **测试覆盖**：确保错误路径有足够测试覆盖

## 11. 总结

本规范提供了NOS项目错误处理的完整框架：

1. **统一错误类型**：定义了标准化的错误类型、类别和严重级别
2. **POSIX兼容性**：确保系统调用层返回标准POSIX错误码
3. **错误传播模式**：定义了Result类型使用、错误转换和组合的规范
4. **模块特定规则**：为不同类型的模块提供专门的错误处理指南
5. **最佳实践**：提供了错误消息、处理模式和测试的指导原则
6. **监控和分析**：定义了错误统计、趋势分析和报告生成的框架

遵循本规范将确保NOS项目的错误处理一致性、可维护性和可靠性。

---

*版本：1.0*
*最后更新：2024年12月*
*维护者：NOS开发团队*