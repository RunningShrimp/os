# 错误处理架构设计文档

## 概述

NOS操作系统使用统一的错误处理机制，提供一致的错误码映射和错误恢复策略。所有错误类型都被映射到POSIX errno值，确保与标准兼容。

## 架构设计

### 核心组件

1. **UnifiedErrorMapper**
   - 位置：`kernel/src/error/unified_mapping.rs`
   - 功能：统一的错误码映射器

2. **错误类型映射**
   - UnifiedError → Errno
   - ApiSyscallError → Errno
   - InterfaceSyscallError → Errno
   - NosErrorType → Errno

3. **错误处理层次**
   - 内核层：`kernel/src/error/`
   - 系统调用层：`kernel/src/subsystems/syscalls/api/syscall_result.rs`
   - 接口层：`kernel/src/subsystems/syscalls/interface/mod.rs`
   - 独立crate：`nos-error-handling/`

## 错误码映射表

| 内核错误 | POSIX errno | 说明 |
|---------|-------------|------|
| InvalidArgument | EINVAL | 无效参数 |
| InvalidAddress | EFAULT | 无效地址 |
| PermissionDenied | EACCES | 权限拒绝 |
| NotFound | ENOENT | 资源未找到 |
| AlreadyExists | EEXIST | 资源已存在 |
| ResourceBusy | EBUSY | 资源忙碌 |
| ResourceUnavailable | EAGAIN | 资源不可用 |
| OutOfMemory | ENOMEM | 内存不足 |

## 使用示例

```rust
use kernel::error::unified_mapping::{unified_error_to_errno, Errno};

let kernel_error = UnifiedError::InvalidArgument;
let errno = unified_error_to_errno(&kernel_error);
assert_eq!(errno, Errno::EINVAL);
```

## 错误恢复策略

- **Retry**：自动重试操作
- **Fallback**：使用备用策略
- **PartialRecovery**：部分恢复，继续运行
- **AutomaticRecovery**：自动恢复



