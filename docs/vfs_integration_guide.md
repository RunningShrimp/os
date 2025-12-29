# VFS 集成指南

## 概述

NOS 操作系统的虚拟文件系统（VFS）层提供统一的文件系统抽象，支持多种文件系统实现，并集成了符号链接、文件锁和扩展属性等高级功能。

## VFS 架构

### 核心组件

```
┌─────────────────────────────────────┐
│         应用程序层                    │
│    (系统调用: open, read, write...)  │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│          VFS 层                      │
│  ┌────────────────────────────┐     │
│  │  InodeOps Trait           │     │
│  │  - getattr/setattr        │     │
│  │  - lookup/create          │     │
│  │  - read/write             │     │
│  │  - symlink/readlink       │     │
│  │  - get_file_lock/release  │     │
│  │  - set_xattr/get_xattr    │     │
│  └────────────────────────────┘     │
│  ┌────────────────────────────┐     │
│  │  SuperBlock Trait         │     │
│  │  - root()                 │     │
│  │  - sync()                 │     │
│  │  - statfs()               │     │
│  └────────────────────────────┘     │
│  ┌────────────────────────────┐     │
│  │  FileSystemType Trait     │     │
│  │  - name()                 │     │
│  │  - mount()                │     │
│  └────────────────────────────┘     │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│      具体文件系统实现                 │
│  ┌─────────┐ ┌─────────┐           │
│  │ RamFS   │ │ TmpFS   │           │
│  └─────────┘ └─────────┘           │
│  ┌─────────┐ ┌─────────┐           │
│  │  Ext4   │ │ ProcFS  │           │
│  └─────────┘ └─────────┘           │
└─────────────────────────────────────┘
```

## InodeOps Trait

`InodeOps` trait 是 VFS 层的核心接口，所有文件系统必须实现它。

### 基本操作

```rust
pub trait InodeOps: Send + Sync {
    // 获取和设置文件属性
    fn getattr(&self) -> VfsResult<FileAttr>;
    fn setattr(&self, attr: &FileAttr) -> VfsResult<()>;

    // 目录操作
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>>;
    fn create(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>>;
    fn mkdir(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>>;
    fn unlink(&self, name: &str) -> VfsResult<()>;
    fn rmdir(&self, name: &str) -> VfsResult<()>;

    // 文件 I/O
    fn read(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize>;
    fn write(&self, offset: u64, buf: &[u8]) -> VfsResult<usize>;
    fn truncate(&self, size: u64) -> VfsResult<()>;

    // 符号链接
    fn symlink(&self, name: &str, target: &str) -> VfsResult<Arc<dyn InodeOps>>;
    fn readlink(&self) -> VfsResult<String>;
}
```

### 高级功能

```rust
pub trait InodeOps: Send + Sync {
    // 文件锁
    fn get_file_lock(&self, cmd: u32, lock: &FileLock) -> VfsResult<u64>;
    fn release_file_lock(&self, lock: &FileLock) -> VfsResult<()>;

    // 扩展属性
    fn set_xattr(&self, name: &str, value: &[u8], flags: u32) -> VfsResult<()>;
    fn get_xattr(&self, name: &str, value: &mut [u8]) -> VfsResult<usize>;
    fn remove_xattr(&self, name: &str) -> VfsResult<()>;
    fn list_xattr(&self, list: &mut [u8]) -> VfsResult<usize>;
}
```

## 文件锁

### 文件锁类型

```rust
pub struct FileLock {
    /// 锁类型: 0=读锁, 1=写锁, 2=解锁
    pub lock_type: u16,
    /// 起始位置
    pub start: u64,
    /// 长度
    pub len: u64,
    /// 进程 ID
    pub pid: u32,
}
```

### 使用示例

```rust
use kernel::vfs::inode::FileLock;

// 创建读锁（共享锁）
let lock = FileLock::shared(0, 1000, pid);

// 创建写锁（排他锁）
let lock = FileLock::exclusive(0, 1000, pid);

// 获取锁
let lock_id = inode.get_file_lock(0, &lock)?;

// 释放锁
inode.release_file_lock(&lock)?;
```

### 文件锁冲突规则

- **读锁 (共享锁)**: 多个进程可以同时持有
- **写锁 (排他锁)**: 只能被一个进程持有，与读锁和写锁都冲突
- **区域锁**: 支持文件部分区域的锁定

## 扩展属性

### 扩展属性命名空间

- `user.*`: 用户属性（可由任何用户修改）
- `trusted.*`: 受信任属性（仅 root 可修改）
- `security.*`: 安全属性（用于 LSM 框架）
- `system.*`: 系统属性（内核使用）

### 大小限制

- `XATTR_NAME_MAX`: 属性名最大 255 字节
- `XATTR_SIZE_MAX`: 属性值最大 65536 字节
- `XATTR_LIST_MAX`: 列表最大 65536 字节

### 使用示例

```rust
// 设置扩展属性
inode.set_xattr("user.comment", b"Important file", 0)?;

// 获取扩展属性
let mut buf = [0u8; 256];
let size = inode.get_xattr("user.comment", &mut buf)?;
println!("Comment: {}", String::from_utf8_lossy(&buf[..size]));

// 删除扩展属性
inode.remove_xattr("user.comment")?;

// 列出所有扩展属性
let mut list = [0u8; 4096];
let list_size = inode.list_xattr(&mut list)?;
```

## 符号链接

### 创建符号链接

```rust
// 创建符号链接
let symlink = parent.symlink("link_name", "target_path")?;

// 读取符号链接目标
let target = symlink.readlink()?;

// 解析符号链接（跟随链接）
let resolved = lookup_with_symlink_resolution("/path/to/link", 8)?;
```

### 符号链接解析选项

```rust
pub struct ResolveOptions {
    /// 最大跟随深度（默认 8）
    pub max_follows: u8,
    /// 是否使用缓存
    pub use_cache: bool,
    /// 是否检测循环
    pub detect_loops: bool,
}
```

## 如何添加新的文件系统

### 步骤 1: 实现 FileSystemType

```rust
use kernel::vfs::core::FileSystemType;
use alloc::sync::Arc;

pub struct MyFsType;

impl FileSystemType for MyFsType {
    fn name(&self) -> &str {
        "myfs"
    }

    fn mount(
        &self,
        device: Option<&str>,
        flags: u32,
    ) -> VfsResult<Arc<dyn SuperBlock>> {
        Ok(Arc::new(MyFsSuperBlock::new(device, flags)?))
    }
}
```

### 步骤 2: 实现 SuperBlock

```rust
use kernel::vfs::core::SuperBlock;

pub struct MyFsSuperBlock {
    root: Arc<MyFsInode>,
    // 其他超级块字段...
}

impl SuperBlock for MyFsSuperBlock {
    fn root(&self) -> Arc<dyn InodeOps> {
        self.root.clone()
    }

    fn fs_type(&self) -> &str {
        "myfs"
    }

    fn sync(&self) -> VfsResult<()> {
        // 同步到磁盘
        Ok(())
    }

    fn statfs(&self) -> VfsResult<FsStats> {
        Ok(FsStats {
            bsize: 4096,
            blocks: 1024,
            bfree: 512,
            bavail: 512,
            files: 10000,
            ffree: 9000,
            namelen: 255,
        })
    }

    fn unmount(&self) -> VfsResult<()> {
        // 卸载文件系统
        Ok(())
    }
}
```

### 步骤 3: 实现 InodeOps

```rust
use kernel::vfs::inode::InodeOps;

pub struct MyFsInode {
    // inode 数据...
}

impl InodeOps for MyFsInode {
    fn getattr(&self) -> VfsResult<FileAttr> {
        // 返回文件属性
        Ok(FileAttr {
            ino: self.ino,
            mode: FileMode(FileMode::S_IFREG | 0o644),
            size: self.data.len() as u64,
            ..Default::default()
        })
    }

    fn read(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        // 读取文件数据
        let data = &self.data;
        let start = offset as usize;
        if start >= data.len() {
            return Ok(0);
        }
        let end = (start + buf.len()).min(data.len());
        let len = end - start;
        buf[..len].copy_from_slice(&data[start..end]);
        Ok(len)
    }

    fn write(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        // 写入文件数据
        let mut data = self.data.lock();
        let start = offset as usize;
        if start + buf.len() > data.len() {
            data.resize(start + buf.len(), 0);
        }
        data[start..start + buf.len()].copy_from_slice(buf);
        Ok(buf.len())
    }

    // 实现其他必需的方法...
}
```

### 步骤 4: 注册文件系统

```rust
use kernel::subsystems::fs::vfs;

pub fn init() {
    let myfs = Arc::new(MyFsType);
    if let Err(e) = vfs().register_fs(myfs) {
        crate::println!("[myfs] Failed to register: {:?}", e);
    } else {
        crate::println!("[myfs] Registered successfully");
    }
}
```

## 测试

### 运行集成测试

```bash
cargo test --test vfs_integration_tests
```

### 运行性能基准

```bash
cargo test --bench vfs_bench
```

## 性能优化建议

### 1. 缓存策略

- **dentry 缓存**: 缓存目录查找结果
- **inode 缓存**: 缓存 inode 数据
- **页缓存**: 缓存文件内容

### 2. 锁优化

- 使用 Per-inode 锁减少竞争
- 读写锁分离提高并发性
- 细粒度锁避免长时间持有

### 3. 批量操作

- 批量写入减少磁盘 I/O
- 延迟写入合并多个小写入
- 预读取提高顺序读取性能

## 已知限制

1. **符号链接深度**: 最多跟随 8 层符号链接
2. **扩展属性大小**: 单个属性最大 64KB
3. **文件锁**: 当前实现不支持跨文件的死锁检测
4. **并发性**: 文件锁操作尚未完全优化

## 后续工作

- [ ] 完善 Ext4 的扩展属性持久化
- [ ] 实现文件锁的阻塞模式
- [ ] 优化符号链接缓存策略
- [ ] 添加文件系统快照支持
- [ ] 实现更完善的死锁检测
- [ ] 支持更多文件系统类型

## 参考资料

- [Linux VFS 文档](https://www.kernel.org/doc/html/latest/filesystems/vfs.html)
- [POSIX 文件锁标准](https://pubs.opengroup.org/onlinepubs/9699919799/)
- [扩展属性规范](https://man7.org/linux/man-pages/man7/xattr.7.html)
