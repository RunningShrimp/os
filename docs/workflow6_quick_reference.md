# 工作流 6 快速参考

## 符号链接 (Symbolic Links)

### 系统调用
```rust
// 创建符号链接
sys_symlink(oldpath: &str, newpath: &str) -> isize

// 读取符号链接
sys_readlink(path: &str, buf: &mut [u8]) -> isize
```

### 高级 API
```rust
use kernel::vfs::{Path, symlink};

// 解析符号链接
let resolved = symlink::resolve_symlink(&path, 8)?;

// 读取链接目标
let target = symlink::readlink(&path)?;

// 创建链接
symlink::symlink(&target, &link_path)?;

// 检查是否是符号链接
let is_link = symlink::is_symlink(&path);

// 获取链接信息
let info = symlink::get_symlink_info(&path)?;
```

### 路径操作
```rust
use kernel::vfs::Path;

let path = Path::new("/usr/bin/bash");

// 检查路径类型
path.is_absolute();   // true
path.is_root();       // false

// 路径组件
path.parent();        // Some(Path("/usr/bin"))
path.file_name();     // Some("bash")

// 路径操作
path.join(&Path::new("python"));  // /usr/bin/bash/python
path.canonicalize();              // 规范化路径
```

## 文件锁 (File Locking)

### flock 系统调用
```rust
const LOCK_SH: u32 = 1;  // 共享锁
const LOCK_EX: u32 = 2;  // 独占锁
const LOCK_UN: u32 = 4;  // 解锁
const LOCK_NB: u32 = 8;  // 非阻塞

// 获取独占锁（非阻塞）
sys_flock(fd, LOCK_EX | LOCK_NB);

// 释放锁
sys_flock(fd, LOCK_UN);
```

### fcntl 文件锁
```rust
const F_RDLCK: i16 = 0;  // 读锁
const F_WRLCK: i16 = 1;  // 写锁
const F_UNLCK: i16 = 2;  // 解锁

const F_GETLK: u32 = 5;   // 测试锁
const F_SETLK: u32 = 6;   // 设置锁（非阻塞）
const F_SETLKW: u32 = 7;  // 设置锁（阻塞）

// 创建锁结构
let mut flock = FlockStruct {
    l_type: F_WRLCK,       // 独占锁
    l_start: 0,            // 文件开始
    l_len: 0,              // 整个文件（0 = 到 EOF）
    l_whence: SEEK_SET,    // 相对位置
    l_pid: 0,              // 填充
};

// 设置锁（阻塞）
sys_fcntl_lock(fd, F_SETLKW, &mut flock);

// 测试锁
sys_fcntl_lock(fd, F_GETLK, &mut flock);
```

### 锁类型
```rust
use kernel::subsystems::fs::file_locking::{
    LockType, LockRange, LockManager,
};

// 锁类型
LockType::Shared     // 共享锁（读锁）
LockType::Exclusive  // 独占锁（写锁）
LockType::None       // 无锁

// 锁范围
let range = LockRange::entire_file();        // 整个文件
let range = LockRange::new(0, 999);          // 字节 0-999

// 锁冲突检测
range1.overlaps(&range2);
range1.contains(&range2);
```

## 扩展属性 (Extended Attributes)

### 系统调用
```rust
// 设置属性
sys_setxattr(
    "/etc/file",           // 路径
    "user.comment",        // 属性名
    b"important",          // 值
    9,                     // 大小
    0                      // 标志
);

// 获取属性
let mut buf = [0u8; 256];
let size = sys_getxattr("/etc/file", "user.comment", &mut buf, 256);

// 列出所有属性
let size = sys_listxattr("/etc/file", &mut buf, 1024);

// 删除属性
sys_removexattr("/etc/file", "user.comment");
```

### 命名空间
```rust
use kernel::subsystems::fs::xattr::XattrNamespace;

XattrNamespace::User      // user.* - 任何用户可读写
XattrNamespace::Trusted   // trusted.* - 仅 root 可读写
XattrNamespace::Security  // security.* - LSM 框架
XattrNamespace::System    // system.* - 内核使用

// 解析命名空间
let ns = XattrNamespace::from_name("user.comment")?;
let prefix = ns.prefix();  // "user"
```

### 操作标志
```rust
const XATTR_CREATE: u32 = 0x1;   // 仅创建
const XATTR_REPLACE: u32 = 0x2;  // 仅替换

// 仅创建新属性（失败如果已存在）
sys_setxattr(path, name, value, size, XATTR_CREATE);

// 仅替换现有属性（失败如果不存在）
sys_setxattr(path, name, value, size, XATTR_REPLACE);
```

### 大小限制
```rust
XATTR_NAME_MAX = 255    // 属性名最大长度
XATTR_SIZE_MAX = 65536  // 属性值最大大小
XATTR_LIST_MAX = 65536  // 列表最大大小
```

## 错误码映射

### 符号链接错误
```
ENOENT (2)      - 路径不存在
EACCES (13)     - 权限不足
EEXIST (17)     - 文件已存在
ENOSPC (28)     - 空间不足
EINVAL (22)     - 无效参数
ENAMETOOLONG (36) - 路径名过长
ELOOP (40)      - 符号链接循环
EOPNOTSUPP (95) - 不支持
```

### 文件锁错误
```
EBADF (9)       - 无效文件描述符
EAGAIN (11)     - 操作会阻塞
EDEADLK (35)    - 死锁检测
ENOSYS (38)     - 功能未实现
```

### 扩展属性错误
```
ENOENT (2)      - 文件或属性不存在
EACCES (13)     - 权限不足
ENOSPC (28)     - 空间不足
E2BIG (34)      - 值太大
EINVAL (22)     - 无效参数
EEXIST (17)     - 属性已存在（XATTR_CREATE）
ERANGE (34)     - 缓冲区太小
```

## 使用示例

### 示例 1: 创建和使用符号链接
```rust
use kernel::vfs::{Path, symlink};

// 1. 创建符号链接
let target = Path::new("/etc/hostname");
let link = Path::new("/tmp/mylink");
symlink::symlink(&target, &link)?;

// 2. 读取链接目标
let target = symlink::readlink(&link)?;
println!("Link points to: {}", target.as_str());

// 3. 解析链接（跟随所有链接）
let resolved = symlink::resolve_symlink(&link, 8)?;
println!("Resolved path: {}", resolved.as_str());
```

### 示例 2: 使用文件锁保护文件访问
```rust
// 1. 打开文件
let fd = sys_open("/tmp/data.txt", O_RDWR);

// 2. 获取独占锁（非阻塞）
match sys_flock(fd, LOCK_EX | LOCK_NB) {
    0 => {
        // 成功获取锁
        // ... 读写文件 ...

        // 释放锁
        sys_flock(fd, LOCK_UN);
    }
    _ => {
        // 文件已被锁定
        println!("File is locked");
    }
}
```

### 示例 3: 使用 fcntl 区域锁
```rust
// 锁定文件的前 1000 字节
let mut flock = FlockStruct {
    l_type: F_WRLCK,
    l_start: 0,
    l_len: 1000,
    l_whence: SEEK_SET,
    l_pid: 0,
};

// 设置锁（阻塞等待）
sys_fcntl_lock(fd, F_SETLKW, &mut flock);

// ... 读写文件 ...

// 解锁
flock.l_type = F_UNLCK;
sys_fcntl_lock(fd, F_SETLK, &mut flock);
```

### 示例 4: 使用扩展属性存储元数据
```rust
// 1. 设置文件注释
sys_setxattr(
    "/etc/config",
    "user.comment",
    b"Main configuration file",
    25,
    0
)?;

// 2. 设置安全上下文
sys_setxattr(
    "/etc/sensitive",
    "security.selinux",
    b"system_u:object_r:etc_t:s0",
    29,
    0
)?;

// 3. 读取注释
let mut buf = [0u8; 256];
let size = sys_getxattr("/etc/config", "user.comment", &mut buf, 256);
println!("Comment: {}", core::str::from_utf8(&buf[..size]).unwrap());

// 4. 列出所有属性
let size = sys_listxattr("/etc/config", &mut buf, 256);
let list = core::str::from_utf8(&buf[..size]).unwrap();
for name in list.split('\0') {
    if !name.is_empty() {
        println!("  {}", name);
    }
}

// 5. 删除注释
sys_removexattr("/etc/config", "user.comment")?;
```

## 性能提示

### 符号链接
- 使用缓存减少解析开销
- 限制链接深度（最多 8 层）
- 避免创建循环链接

### 文件锁
- 使用最小的锁定范围
- 及时释放锁
- 优先使用非阻塞模式

### 扩展属性
- 避免存储大量数据
- 使用合适的命名空间
- 批量操作优于单个操作

## 调试

### 符号链接调试
```rust
// 获取统计信息
let stats = symlink::get_stats();
println!("Total: {}", stats.total_resolutions);
println!("Success: {}", stats.successful_resolutions);
println!("Loops: {}", stats.loop_detections);
```

### 文件锁调试
```rust
use kernel::subsystems::fs::file_locking::get_lock_manager;

if let Some(manager) = get_lock_manager() {
    // 获取文件的所有锁
    let locks = manager.get_file_locks(inode);
    for lock in locks {
        println!("PID: {:?}, Type: {:?}", lock.pid, lock.lock_type);
    }

    // 获取进程的所有锁
    let locks = manager.get_process_locks(pid);
    println!("Process {} holds {} locks", pid, locks.len());

    // 检测死锁
    if let Some(pids) = manager.detect_deadlock() {
        println!("Deadlock detected: {:?}", pids);
    }

    // 获取统计
    let stats = manager.get_stats();
    println!("Total requests: {}", stats.total_requests);
}
```

### 扩展属性调试
```rust
use kernel::subsystems::fs::xattr::get_manager;

if let Some(manager) = get_manager()) {
    // 获取统计
    let stats = manager.get_stats();
    println!("Sets: {}", stats.total_sets);
    println!("Gets: {}", stats.total_gets);
    println!("Removes: {}", stats.total_removes);
    println!("Lists: {}", stats.total_lists);
}
```

## 常见问题

### Q: 如何检测符号链接循环？
A: 使用 `resolve_symlink()`，它会自动检测循环并返回 `VfsError::Loop`。

### Q: flock 和 fcntl 锁有什么区别？
A:
- `flock`: 整个文件锁，进程级，不同文件描述符不独立
- `fcntl`: POSIX 记录锁，文件描述符级，支持区域锁

### Q: 扩展属性会持久化吗？
A: 当前实现是内存中的，持久化支持正在开发中。

### Q: 如何选择扩展属性命名空间？
A:
- `user.*`: 应用数据，任何用户可访问
- `trusted.*`: 系统数据，仅 root 可访问
- `security.*`: LSM 框架，安全标签
- `system.*`: 内核使用

---

**更多信息**: 参见 `docs/workflow6_filesystem_completion.md`
