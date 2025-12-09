# 第一周实施指南 - 详细步骤

> **时间**: 2025-12-09 至 2025-12-15  
> **目标**: 完成根目录清理 + 实现核心进程和文件系统功能  
> **TODO减少**: 261 → 251 (-10个)

---

## Day 1: 根目录清理 (2025-12-09)

### 上午任务: 执行清理脚本 (2小时)

#### 步骤1: 备份当前状态
```bash
cd /Users/didi/Desktop/nos

# 创建备份分支
git checkout -b backup/pre-cleanup
git add .
git commit -m "backup: 清理前的项目状态备份"

# 切换到工作分支
git checkout master
git checkout -b feature/week1-core-implementations
```

#### 步骤2: 执行清理
```bash
# 运行清理脚本
./scripts/cleanup_root.sh

# 查看结果
ls -la
tree -L 1 temp/
tree -L 1 docs/
```

#### 步骤3: 验证和提交
```bash
# 确认根目录只剩核心文件
ls -1 | wc -l  # 应该 <10

# 提交更改
git add .
git commit -m "chore: 清理根目录，建立项目结构规范

- 移动构建日志到 temp/build_logs/
- 移动错误分析文件到 temp/analysis/
- 移动报告文档到 docs/reports/
- 移动计划文档到 docs/plans/
- 更新 .gitignore 排除临时文件
- 创建文档导航 docs/README.md"
```

### 下午任务: 代码结构分析 (3小时)

#### 分析进程管理代码
```bash
# 查看进程管理相关文件
ls -la kernel/src/process/
ls -la kernel/src/syscalls/process_service/

# 阅读核心文件
cat kernel/src/process/mod.rs
cat kernel/src/syscalls/process_service/handlers.rs
cat kernel/src/syscalls/process.rs
```

**关键发现记录** (创建 `notes/day1-analysis.md`):
```markdown
# 进程管理代码分析

## 核心结构
- Process结构定义位置: kernel/src/process/process.rs
- 进程表管理: kernel/src/process/table.rs (或类似)
- 当前进程获取: 通过per-CPU变量

## 系统调用路由
- 入口: kernel/src/syscalls/mod.rs::syscall_handler()
- 进程服务: kernel/src/syscalls/process_service/
- 旧实现: kernel/src/syscalls/process.rs (作为参考)

## 需要实现的函数
1. sys_getpid() - 获取当前进程ID
2. sys_getppid() - 获取父进程ID
3. sys_exit() - 进程退出
4. sys_fork() - 创建子进程
5. sys_execve() - 执行新程序
```

---

## Day 2: 实现简单进程操作 (2025-12-10)

### 任务1: 实现 sys_getpid() (1小时)

**文件**: `kernel/src/syscalls/process_service/handlers.rs`

#### 当前代码 (第30行):
```rust
pub fn sys_getpid() -> Result<usize> {
    // TODO: 实现getpid逻辑
    Err(KernelError::NotImplemented)
}
```

#### 新实现:
```rust
pub fn sys_getpid() -> Result<usize> {
    use crate::process::current_process;
    
    // 获取当前进程
    let process = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    // 返回进程ID
    Ok(process.pid().as_usize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_getpid_returns_valid_pid() {
        // 在测试上下文中应该有一个当前进程
        let result = sys_getpid();
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }
}
```

#### 验证:
```bash
# 编译测试
cargo test --package kernel --lib syscalls::process_service::handlers::tests::test_getpid

# 如果失败，检查错误并调整
```

### 任务2: 实现 sys_getppid() (1小时)

```rust
pub fn sys_getppid() -> Result<usize> {
    use crate::process::current_process;
    
    let process = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    // 获取父进程ID
    let ppid = process.parent_pid()
        .ok_or(KernelError::NoParentProcess)?;
    
    Ok(ppid.as_usize())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_getppid_returns_parent_pid() {
        // Init进程(PID 1)没有父进程，应该返回错误或0
        // 普通进程应该返回父进程ID
        let result = sys_getppid();
        // 测试逻辑根据当前进程上下文调整
    }
}
```

### 任务3: 实现 sys_exit() (2小时)

```rust
pub fn sys_exit(status: i32) -> Result<usize> {
    use crate::process::{current_process, ProcessState};
    use crate::scheduler::schedule;
    
    let process = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    // 设置退出状态
    process.set_exit_status(status);
    
    // 标记为僵尸进程
    process.set_state(ProcessState::Zombie);
    
    // 唤醒等待的父进程
    if let Some(parent) = process.parent() {
        parent.wake_up_if_waiting();
    }
    
    // 清理资源（但保留基本信息供父进程waitpid）
    process.cleanup_resources();
    
    // 触发调度，切换到其他进程
    schedule();
    
    // 不应该返回
    unreachable!("Process should not return from exit")
}
```

### 下午: 测试和调试 (3小时)

```bash
# 编译检查
cargo build --package kernel

# 运行单元测试
cargo test --package kernel syscalls::process_service

# 如果有集成测试
cargo test --test process_tests
```

**问题记录**: 在 `notes/day2-issues.md` 中记录遇到的问题和解决方案

---

## Day 3: 实现文件系统操作 (2025-12-11)

### 任务1: 实现 sys_open() (2小时)

**文件**: `kernel/src/syscalls/fs_service/handlers.rs`

```rust
use crate::fs::vfs::{VFS, OpenFlags};
use crate::process::current_process;

pub fn sys_open(path: &str, flags: i32, mode: u32) -> Result<usize> {
    let process = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    // 解析打开标志
    let open_flags = OpenFlags::from_bits(flags as u32)
        .ok_or(KernelError::InvalidArgument)?;
    
    // 通过VFS打开文件
    let vfs = VFS::global();
    let inode = vfs.open(path, open_flags, mode)?;
    
    // 分配文件描述符
    let fd = process.fd_table()
        .allocate_fd(inode)?;
    
    Ok(fd)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_open_existing_file() {
        // 假设/dev/null始终存在
        let fd = sys_open("/dev/null", O_RDONLY, 0);
        assert!(fd.is_ok());
        assert!(fd.unwrap() >= 0);
    }
    
    #[test]
    fn test_open_nonexistent_file() {
        let fd = sys_open("/nonexistent", O_RDONLY, 0);
        assert!(fd.is_err());
    }
}
```

### 任务2: 实现 sys_close() (1小时)

```rust
pub fn sys_close(fd: usize) -> Result<usize> {
    let process = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    // 从文件描述符表中移除
    process.fd_table()
        .close_fd(fd)?;
    
    Ok(0)
}
```

### 任务3: 实现 sys_read() (2小时)

```rust
pub fn sys_read(fd: usize, buf: *mut u8, count: usize) -> Result<usize> {
    let process = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    // 验证用户空间缓冲区
    if !process.memory_manager().is_valid_user_buffer(buf, count) {
        return Err(KernelError::InvalidAddress);
    }
    
    // 获取文件对象
    let file = process.fd_table()
        .get_file(fd)?;
    
    // 创建内核缓冲区
    let mut kernel_buf = vec![0u8; count];
    
    // 从文件读取
    let bytes_read = file.read(&mut kernel_buf)?;
    
    // 拷贝到用户空间
    unsafe {
        core::ptr::copy_nonoverlapping(
            kernel_buf.as_ptr(),
            buf,
            bytes_read
        );
    }
    
    Ok(bytes_read)
}
```

### 任务4: 实现 sys_write() (2小时)

```rust
pub fn sys_write(fd: usize, buf: *const u8, count: usize) -> Result<usize> {
    let process = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    // 验证用户空间缓冲区
    if !process.memory_manager().is_valid_user_buffer_const(buf, count) {
        return Err(KernelError::InvalidAddress);
    }
    
    // 获取文件对象
    let file = process.fd_table()
        .get_file(fd)?;
    
    // 从用户空间拷贝数据
    let mut kernel_buf = vec![0u8; count];
    unsafe {
        core::ptr::copy_nonoverlapping(
            buf,
            kernel_buf.as_mut_ptr(),
            count
        );
    }
    
    // 写入文件
    let bytes_written = file.write(&kernel_buf)?;
    
    Ok(bytes_written)
}
```

---

## Day 4: 完善和测试 (2025-12-12)

### 上午: 实现剩余文件操作 (3小时)

#### sys_lseek()
```rust
pub fn sys_lseek(fd: usize, offset: i64, whence: i32) -> Result<usize> {
    let process = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    let file = process.fd_table().get_file(fd)?;
    
    let new_offset = match whence {
        SEEK_SET => offset,
        SEEK_CUR => file.offset() + offset,
        SEEK_END => file.size() as i64 + offset,
        _ => return Err(KernelError::InvalidArgument),
    };
    
    if new_offset < 0 {
        return Err(KernelError::InvalidArgument);
    }
    
    file.set_offset(new_offset as u64)?;
    Ok(new_offset as usize)
}
```

#### sys_stat() 和 sys_fstat()
```rust
pub fn sys_stat(path: &str, statbuf: *mut Stat) -> Result<usize> {
    // 验证用户空间指针
    // 通过VFS获取inode
    // 填充stat结构
    // 拷贝到用户空间
    todo!("实现sys_stat")
}

pub fn sys_fstat(fd: usize, statbuf: *mut Stat) -> Result<usize> {
    // 类似sys_stat，但通过fd获取文件
    todo!("实现sys_fstat")
}
```

### 下午: 集成测试 (4小时)

创建 `kernel/tests/integration/process_and_fs_tests.rs`:

```rust
use kernel::syscalls::process_service::*;
use kernel::syscalls::fs_service::*;

#[test]
fn test_process_lifecycle() {
    // 测试进程创建、执行、退出的完整生命周期
    let pid = sys_getpid().unwrap();
    assert!(pid > 0);
    
    // 创建子进程
    let child_pid = sys_fork().unwrap();
    if child_pid == 0 {
        // 子进程
        sys_exit(0).unwrap();
    } else {
        // 父进程等待子进程
        let mut status = 0;
        sys_waitpid(child_pid, &mut status, 0).unwrap();
    }
}

#[test]
fn test_file_operations() {
    // 测试文件打开、读写、关闭
    let fd = sys_open("/tmp/test.txt", O_RDWR | O_CREAT, 0o644).unwrap();
    
    let data = b"Hello, NOS!";
    let written = sys_write(fd, data.as_ptr(), data.len()).unwrap();
    assert_eq!(written, data.len());
    
    sys_lseek(fd, 0, SEEK_SET).unwrap();
    
    let mut buf = [0u8; 32];
    let read = sys_read(fd, buf.as_mut_ptr(), buf.len()).unwrap();
    assert_eq!(read, data.len());
    assert_eq!(&buf[..read], data);
    
    sys_close(fd).unwrap();
}
```

运行测试:
```bash
cargo test --test integration
```

---

## Day 5: fork和execve实现 (2025-12-13)

### 任务1: 实现 sys_fork() (4小时)

这是最复杂的实现，需要：
1. 复制进程结构
2. 复制内存空间
3. 复制文件描述符表
4. 设置父子关系

```rust
pub fn sys_fork() -> Result<usize> {
    use crate::process::{Process, ProcessState};
    use crate::scheduler::add_process;
    
    let parent = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    // 创建子进程
    let mut child = Process::new_forked_from(&parent)?;
    
    // 复制内存空间
    child.memory_manager().copy_from(&parent.memory_manager())?;
    
    // 复制文件描述符表
    child.fd_table().copy_from(&parent.fd_table())?;
    
    // 设置父子关系
    child.set_parent(parent.pid());
    parent.add_child(child.pid());
    
    // 设置子进程返回值为0
    child.set_fork_return_value(0);
    
    let child_pid = child.pid();
    
    // 将子进程加入调度队列
    add_process(child);
    
    // 父进程返回子进程PID
    Ok(child_pid.as_usize())
}
```

### 任务2: 实现 sys_execve() (4小时)

```rust
pub fn sys_execve(
    path: &str,
    argv: &[&str],
    envp: &[&str]
) -> Result<usize> {
    use crate::process::current_process;
    use crate::loader::ElfLoader;
    
    let process = current_process()
        .ok_or(KernelError::ProcessNotFound)?;
    
    // 加载新程序
    let loader = ElfLoader::new(path)?;
    let entry_point = loader.load_into_process(&process)?;
    
    // 清理旧的内存空间（除了内核空间）
    process.memory_manager().clear_user_space()?;
    
    // 设置新的栈和参数
    process.setup_initial_stack(argv, envp)?;
    
    // 设置程序入口点
    process.set_entry_point(entry_point);
    
    // execve成功不返回，直接开始执行新程序
    process.start_execution();
    
    unreachable!("Should not return from execve")
}
```

---

## Day 6: 测试和调试 (2025-12-14)

### 全天: 综合测试 (8小时)

#### 1. 单元测试
```bash
cargo test --package kernel --lib
```

#### 2. 集成测试
```bash
cargo test --test integration
```

#### 3. 手动测试
创建测试程序 `user/test_basic.rs`:
```rust
fn main() {
    // 测试getpid
    let pid = unsafe { syscall!(SYS_GETPID) };
    println!("My PID: {}", pid);
    
    // 测试fork
    let child = unsafe { syscall!(SYS_FORK) };
    if child == 0 {
        println!("I'm child");
    } else {
        println!("I'm parent, child PID: {}", child);
    }
    
    // 测试文件I/O
    let fd = unsafe { 
        syscall!(SYS_OPEN, "/tmp/test.txt", O_RDWR | O_CREAT, 0o644) 
    };
    unsafe { syscall!(SYS_WRITE, fd, "Hello\n", 6) };
    unsafe { syscall!(SYS_CLOSE, fd) };
}
```

#### 4. 问题修复
记录和修复测试中发现的所有bug

---

## Day 7: 文档和总结 (2025-12-15)

### 上午: 更新文档 (3小时)

#### 1. 更新TODO列表
```bash
# 标记已完成的TODO
# 更新 docs/plans/TODO_CLEANUP_PLAN.md
```

#### 2. 撰写周报
使用模板 `docs/templates/WEEKLY_REPORT_TEMPLATE.md` 创建第一周周报

#### 3. 更新路线图
更新 `NOS_IMPROVEMENT_ROADMAP.md` 中的进度指标

### 下午: 代码整理和提交 (2小时)

```bash
# 运行代码格式化
cargo fmt

# 运行代码检查
cargo clippy -- -D warnings

# 最终测试
cargo test --all

# 提交所有更改
git add .
git commit -m "feat: 实现核心进程和文件系统功能

实现的功能:
- 进程管理: getpid, getppid, exit, fork, execve
- 文件系统: open, close, read, write, lseek, stat, fstat
- 完整的单元测试和集成测试
- TODO数量从261减少到251

性能: 所有基础操作正常工作
测试: 100% 通过率"

# 推送到远程
git push origin feature/week1-core-implementations
```

### 晚上: 准备下周计划 (1小时)

查看 `NOS_IMPROVEMENT_ROADMAP.md` 第2周任务并做准备

---

## 检查清单

### Day 1
- [ ] 执行清理脚本
- [ ] 提交根目录清理
- [ ] 分析代码结构
- [ ] 创建分析笔记

### Day 2-3
- [ ] 实现sys_getpid
- [ ] 实现sys_getppid
- [ ] 实现sys_exit
- [ ] 实现sys_open
- [ ] 实现sys_close
- [ ] 实现sys_read
- [ ] 实现sys_write

### Day 4
- [ ] 实现sys_lseek
- [ ] 实现sys_stat/fstat
- [ ] 编写集成测试
- [ ] 所有测试通过

### Day 5
- [ ] 实现sys_fork (复杂)
- [ ] 实现sys_execve (复杂)
- [ ] 测试进程生命周期

### Day 6
- [ ] 运行全面测试
- [ ] 修复所有发现的bug
- [ ] 性能验证

### Day 7
- [ ] 更新所有文档
- [ ] 撰写周报
- [ ] 代码格式化和检查
- [ ] 提交所有更改

---

## 成功标准

✅ **功能完整性**:
- 5+个进程管理函数可用
- 5+个文件系统函数可用
- 所有实现有单元测试

✅ **代码质量**:
- 无编译警告
- Clippy检查通过
- 测试覆盖率>60%

✅ **进度**:
- TODO: 261 → 251 (-10)
- 根目录文件: 25+ → <10
- 提交次数: 10+次

---

**祝第一周工作顺利！** 🚀
