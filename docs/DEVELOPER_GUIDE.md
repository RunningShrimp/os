# NOS 开发者指南

欢迎使用 NOS (New Operating System) 开发者指南。本文档旨在帮助开发者快速上手 NOS 操作系统的开发、编译、测试和调试。

## 目录

1. [开发环境设置](#1-开发环境设置)
2. [编译和测试](#2-编译和测试)
3. [代码风格规范](#3-代码风格规范)
4. [提交指南](#4-提交指南)
5. [调试技巧](#5-调试技巧)
6. [性能分析](#6-性能分析)
7. [故障排查](#7-故障排查)

---

## 1. 开发环境设置

### 1.1 前置要求

在开始开发 NOS 之前，请确保您的系统已安装以下工具：

- **Rust 工具链**: Rust 2024 edition
- **Nightly 版本**: 用于内核开发
- **QEMU**: 用于虚拟化测试
- **GDB**: 用于内核调试
- **make**: 构建工具（可选）

#### 系统要求

- **操作系统**: Linux, macOS, 或 WSL (Windows)
- **内存**: 至少 8GB RAM（推荐 16GB）
- **磁盘空间**: 至少 5GB 可用空间

### 1.2 安装步骤

#### 1.2.1 安装 Rust

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 nightly 工具链
rustup install nightly
rustup default nightly

# 添加必要的组件
rustup component add rust-src
rustup component add clippy
rustup component add rustfmt
```

#### 1.2.2 安装 QEMU

**macOS:**
```bash
brew install qemu
```

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install qemu-system-x86 qemu-utils
```

**Arch Linux:**
```bash
sudo pacman -S qemu
```

#### 1.2.3 克隆仓库

```bash
# 克隆 NOS 仓库
git clone <repository-url>
cd nos

# 验证环境
cargo --version
rustc --version
qemu-system-x86_64 --version
```

#### 1.2.4 安装开发工具

```bash
# 安装 tarpaulin（代码覆盖率工具）
cargo install cargo-tarpaulin

# 安装 rust-analyzer（用于 IDE 支持）
rustup component add rust-analyzer
```

---

## 2. 编译和测试

### 2.1 编译选项

NOS 支持多种编译配置，根据不同需求选择合适的编译方式。

#### 2.1.1 开发版本

```bash
# 编译整个工作区
cargo build --workspace

# 仅编译内核
cargo build --package kernel

# 仅编译 API 层
cargo build --package nos-api
```

#### 2.1.2 发布版本

```bash
# 发布版本编译（优化性能）
cargo build --workspace --release

# 查看编译后的二进制大小
ls -lh target/x86_64-nos/release/
```

#### 2.1.3 裸机版本

```bash
# 编译裸机内核版本
cargo build --kernel --features baremetal

# 指定目标架构
cargo build --target x86_64-nos.json
```

#### 2.1.4 特性标志

```bash
# 启用内核测试
cargo build --kernel --features kernel_tests

# 启用性能分析
cargo build --kernel --features profiling

# 启用调试符号
cargo build --kernel --features debug_symbols
```

### 2.2 运行测试

#### 2.2.1 全部测试

```bash
# 运行所有工作区测试
cargo test --workspace

# 显示测试输出
cargo test --workspace -- --nocapture

# 并行运行测试
cargo test --workspace --jobs 8
```

#### 2.2.2 特定包测试

```bash
# 内核测试
cargo test --package kernel

# API 测试
cargo test --package nos-api

# 系统调用测试
cargo test --package nos-syscalls
```

#### 2.2.3 集成测试

```bash
# 运行集成测试
cargo test --test integration_tests

# 运行 POSIX 兼容性测试
cargo test --test posix_tests
```

#### 2.2.4 覆盖率测试

```bash
# 使用 tarpaulin 生成覆盖率报告
cargo tarpaulin --workspace --out Html

# 查看覆盖率报告
open tarpaulin-report.html
```

#### 2.2.5 性能基准测试

```bash
# 运行性能测试
cargo test --release --benches

# 运行特定基准测试
cargo test --bench syscall_bench -- --nocapture
```

### 2.3 运行 NOS

#### 2.3.1 在 QEMU 中运行

```bash
# 使用默认配置运行
cargo run --kernel

# 指定内存大小
qemu-system-x86_64 -m 2G -kernel target/x86_64-nos/debug/kernel

# 启用 KVM 加速
qemu-system-x86_64 -enable-kvm -kernel target/x86_64-nos/debug/kernel

# 串口输出
qemu-system-x86_64 -serial stdio -kernel target/x86_64-nos/debug/kernel
```

#### 2.3.2 调试模式运行

```bash
# 等待 GDB 连接
qemu-system-x86_64 -s -S -kernel target/x86_64-nos/debug/kernel
```

---

## 3. 代码风格规范

### 3.1 Rust 规范

NOS 遵循 Rust 2024 edition 规范和社区最佳实践。

#### 3.1.1 代码格式化

```bash
# 格式化所有代码
cargo fmt

# 检查格式是否正确
cargo fmt --check
```

#### 3.1.2 Lint 检查

```bash
# 运行 clippy 检查
cargo clippy --workspace

# 修复可自动修复的问题
cargo clippy --workspace --fix

# 使用严格的 clippy 规则
cargo clippy --workspace -- -D warnings
```

#### 3.1.3 常用 Clippy Lints

- `#![deny(missing_docs)]`: 强制文档注释
- `#![warn(unused_extern_crates)]`: 警告未使用的外部 crate
- `#![forbid(unsafe_code)]`: 禁止 unsafe 代码（在某些模块中）
- `#![deny(clippy::all)]`: 启用所有 clippy 检查

### 3.2 命名约定

遵循 Rust 标准命名约定：

| 类别 | 约定 | 示例 |
|------|------|------|
| 模块 | `snake_case` | `mod memory_manager` |
| 类型 | `PascalCase` | `struct ProcessManager` |
| 函数 | `snake_case` | `fn allocate_page()` |
| 变量 | `snake_case` | `let page_count` |
| 常量 | `SCREAMING_SNAKE_CASE` | `const MAX_PAGES: usize` |
| 生命周期参数 | 短小写字母 | `<'a, 'b>` |
| 类型参数 | 简短大写 | `<T, E>` |

### 3.3 文档规范

#### 3.3.1 模块文档

```rust
//! # 内存管理模块
//!
//! 此模块提供 NOS 内核的内存管理功能，包括：
//! - 物理页面分配
//! - 虚拟内存映射
//! - 内存回收策略
//!
//! ## 示例
//!
//! ```rust
//! use kernel::memory::allocate_page;
//! let page = allocate_page().expect("Failed to allocate page");
//! ```

use crate::memory::types::Page;
```

#### 3.3.2 函数文档

```rust
/// 分配一个新的物理页面
///
/// 此函数会从系统中分配一个 4KB 大小的物理页面。
/// 如果没有可用页面，返回 `Err`。
///
/// # 参数
///
/// 无
///
/// # 返回值
///
/// - `Ok(Page)` - 成功分配的页面
/// - `Err(AllocError)` - 分配失败
///
/// # 错误
///
/// - `AllocError::OutOfMemory` - 系统内存不足
///
/// # 示例
///
/// ```rust
/// let page = allocate_page()?;
/// ```
///
/// # 安全性
///
/// 此函数是线程安全的，可以在多个上下文中同时调用。
pub fn allocate_page() -> Result<Page, AllocError> {
    // 实现...
}
```

### 3.4 错误处理

使用 `Result` 类型进行错误处理：

```rust
use kernel::error::KernelError;

pub fn do_something() -> Result<(), KernelError> {
    // 使用 ? 运算符传播错误
    let result = risky_operation()?;
    Ok(())
}
```

### 3.5 Unsafe 代码规范

使用 `unsafe` 时必须添加注释说明原因：

```rust
/// # Safety
///
/// 调用者必须确保：
/// 1. 指针 `ptr` 已正确对齐
/// 2. 指针 `ptr` 指向的内存已初始化
/// 3. 在此函数执行期间，没有其他代码访问该内存
unsafe unsafe_function(ptr: *mut u8) {
    // unsafe 代码...
}
```

---

## 4. 提交指南

### 4.1 Commit 消息格式

使用 Conventional Commits 规范：

```
<type>(<scope>): <subject>

<body>

<footer>
```

#### Type 类型

- **feat**: 新功能
- **fix**: Bug 修复
- **docs**: 文档更新
- **style**: 代码格式（不影响代码运行的变动）
- **refactor**: 重构（既不是新增功能，也不是修改 bug）
- **perf**: 性能优化
- **test**: 增加测试
- **chore**: 构建过程或辅助工具的变动

#### 示例

```
feat(memory): 实现物理页面分配器

- 添加基于位图的页面分配器
- 支持多 CPU 并发分配
- 实现页面回收机制

Closes #123
```

```
fix(syscall): 修复 write 系统调用参数验证问题

在某些情况下，write 系统调用没有正确验证用户空间指针，
可能导致内核崩溃。现在添加了完整的指针验证。

Fixes #456
```

### 4.2 提交前检查

```bash
# 格式化代码
cargo fmt

# 运行 clippy
cargo clippy --workspace

# 运行测试
cargo test --workspace

# 检查未跟踪的文件
git status
```

### 4.3 代码审查流程

1. **创建功能分支**: 从 `master` 分支创建新分支
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **开发并测试**: 在功能分支上进行开发

3. **提交代码**: 使用规范的提交消息

4. **推送分支**: 推送到远程仓库
   ```bash
   git push origin feature/your-feature-name
   ```

5. **创建 Pull Request**: 在 GitHub 上创建 PR，填写模板

6. **代码审查**: 等待维护者审查并处理反馈

7. **合并**: 审查通过后合并到主分支

### 4.4 Pull Request 模板

创建 PR 时应包含以下信息：

```markdown
## 变更类型
- [ ] 新功能
- [ ] Bug 修复
- [ ] 重构
- [ ] 文档更新
- [ ] 性能优化

## 变更说明
简要描述此 PR 的内容和目的。

## 测试
- [ ] 已添加单元测试
- [ ] 已添加集成测试
- [ ] 已手动测试
- [ ] 所有测试通过

## 检查清单
- [ ] 代码遵循项目规范
- [ ] 已通过 `cargo fmt`
- [ ] 已通过 `cargo clippy`
- [ ] 已添加必要的文档
- [ ] 已更新相关文档

## 相关 Issue
Closes #issue_number
```

---

## 5. 调试技巧

### 5.1 日志输出

NOS 提供了丰富的日志功能。

#### 5.1.1 使用宏输出日志

```rust
use kernel::debug::log::{info, warn, error, debug};

pub fn some_function() {
    debug!("进入 some_function");
    info!("正常信息: value = {}", 42);
    warn!("警告信息: resource low");
    error!("错误信息: operation failed");
}
```

#### 5.1.2 日志级别

```bash
# 设置日志级别（通过 QEMU 参数）
cargo run --kernel -- --log-level debug

# 或在环境变量中设置
export RUST_LOG=debug
```

### 5.2 GDB 调试

#### 5.2.1 启动调试会话

**终端 1 - 启动 QEMU:**
```bash
qemu-system-x86_64 -s -S -kernel target/x86_64-nos/debug/kernel
```

**终端 2 - 启动 GDB:**
```bash
gdb target/x86_64-nos/debug/kernel

# 在 GDB 中
(gdb) target remote :1234
(gdb) break kernel_main
(gdb) continue
```

#### 5.2.2 常用 GDB 命令

```bash
# 设置断点
(gdb) break kernel::memory::allocate_page

# 查看调用栈
(gdb) backtrace
(gdb) bt  # 简写

# 查看变量
(gdb) print variable_name
(gdb) p *pointer  # 解引用

# 单步执行
(gdb) step      # 进入函数
(gdb) next      # 跳过函数
(gdb) continue  # 继续执行

# 查看寄存器
(gdb) info registers

# 查看内存
(gdb) x/10x 0x1000  # 以十六进制查看内存
```

#### 5.2.3 GDB 脚本自动化

创建 `.gdbinit` 文件：

```gdb
# .gdbinit
target remote :1234
break kernel_main
break kernel::memory::allocate_page
continue
```

### 5.3 QEMU 调试

#### 5.3.1 QEMU 监控器

```bash
# 启动 QEMU 时启用监控器
qemu-system-x86_64 -monitor stdio -kernel target/x86_64-nos/debug/kernel
```

在 QEMU 监控器中：
```
(qemu) info registers    # 查看寄存器
(qemu) info mem          # 查看内存映射
(qemu) xp /10x 0x1000    # 查看物理内存
(qemu) stop              # 停止虚拟机
(qemu) cont              # 继续执行
```

#### 5.3.2 GDB 与 QEMU 集成

```bash
# 启用 GDB 服务器
qemu-system-x86_64 -s -S -kernel target/x86_64-nos/debug/kernel

# 在另一个终端连接 GDB
gdb -ex "target remote :1234" target/x86_64-nos/debug/kernel
```

### 5.4 性能分析

#### 5.4.1 使用 flamegraph

```bash
# 安装 flamegraph
cargo install flamegraph

# 生成火焰图
cargo flamegraph --kernel --features profiling

# 查看火焰图
open flamegraph.svg
```

#### 5.4.2 使用 perf (Linux)

```bash
# 记录性能数据
perf record -g ./target/x86_64-nos/debug/kernel

# 分析数据
perf report

# 生成火焰图
perf script | FlameGraph/stackcollapse-perf.pl | FlameGraph/flamegraph.pl > kernel.svg
```

---

## 6. 性能分析

### 6.1 基准测试

NOS 提供了完整的性能测试套件。

#### 6.1.1 运行基准测试

```bash
# 运行所有基准测试
cargo bench --workspace

# 运行特定基准测试
cargo bench --bench syscall_bench

# 比较性能变化
cargo bench --bench syscall_bench -- --baseline main
```

#### 6.1.2 编写基准测试

```rust
#[bench]
fn bench_syscall_dispatch(b: &mut Bencher) {
    b.iter(|| {
        // 被测试的代码
        syscall_handler(SYS_READ, fd, buf, count);
    });
}
```

### 6.2 性能分析工具

#### 6.2.1 cargo-flamegraph

```bash
# 安装
cargo install flamegraph

# 使用
cargo flamegraph --bin kernel
```

#### 6.2.2 heaptrack

```bash
# 跟踪内存分配
heaptrack ./target/x86_64-nos/debug/kernel

# 分析结果
heaptrack_print kernel.heaptrack
```

### 6.3 性能优化建议

1. **减少系统调用开销**: 使用批处理和缓存
2. **零拷贝技术**: 减少不必要的内存复制
3. **SIMD 优化**: 使用向量化指令
4. **锁优化**: 使用无锁数据结构或读写锁
5. **内存池**: 预分配内存减少分配开销

---

## 7. 故障排查

### 7.1 编译错误

#### 7.1.1 常见编译错误

**错误**: `error[E0432]: unresolved import`
```
解决方案：
1. 检查模块路径是否正确
2. 确认模块是否在 mod.rs 中声明
3. 检查 Cargo.toml 中的依赖
```

**错误**: `error[E0277]: trait bound not satisfied`
```
解决方案：
1. 添加必要的 trait 约束
2. 检查类型是否实现了所需的 trait
3. 使用 where 子句明确约束
```

#### 7.1.2 清理构建缓存

```bash
# 清理所有构建产物
cargo clean

# 清理特定包
cargo clean -p kernel

# 重新编译
cargo build --workspace
```

### 7.2 测试失败

#### 7.2.1 查看详细输出

```bash
# 显示测试输出
cargo test -- --nocapture

# 显示详细的错误信息
RUST_BACKTRACE=1 cargo test

# 只运行失败的测试
cargo test -- --ignored
```

#### 7.2.2 运行单个测试

```bash
# 运行特定测试
cargo test test_name

# 运行特定模块的所有测试
cargo test module_name::
```

### 7.3 运行时问题

#### 7.3.1 内核崩溃

1. **查看崩溃信息**: 检查串口输出
2. **使用 GDB**: 定位崩溃位置
3. **检查日志**: 查看崩溃前的日志

```bash
# 启用详细日志
qemu-system-x86_64 -kernel target/x86_64-nos/debug/kernel -- -d int,cpu_reset
```

#### 7.3.2 性能问题

1. **使用性能分析工具**: 火焰图、perf
2. **检查热点函数**: 优化高频调用的函数
3. **内存泄漏**: 使用 valgrind 或类似工具

### 7.4 获取帮助

#### 7.4.1 社区资源

- **GitHub Issues**: 报告 bug 和功能请求
- **Discord/Slack**: 实时交流
- **邮件列表**: 技术讨论

#### 7.4.2 调试信息收集

提交问题时，请提供以下信息：

```bash
# 系统信息
uname -a
rustc --version
cargo --version
qemu-system-x86_64 --version

# 编译日志
cargo build --workspace 2> build.log

# 测试结果
cargo test --workspace 2> test.log

# GDB 回溯
gdb -batch -ex "bt" target/x86_64-nos/debug/kernel core
```

---

## 附录

### A. 常用命令速查

```bash
# 构建
cargo build --workspace
cargo build --release
cargo clean

# 测试
cargo test --workspace
cargo test --package kernel
cargo clippy --workspace
cargo fmt --check

# 运行
cargo run --kernel
qemu-system-x86_64 -m 2G -kernel target/x86_64-nos/debug/kernel

# 调试
cargo flamegraph --kernel
RUST_BACKTRACE=1 cargo test
```

### B. 项目结构

```
nos/
├── bootloader/       # 引导加载程序
├── kernel/           # 内核代码
│   ├── src/
│   │   ├── arch/     # 架构相关代码
│   │   ├── mm/       # 内存管理
│   │   ├── process/  # 进程管理
│   │   ├── fs/       # 文件系统
│   │   └── ...
│   └── Cargo.toml
├── nos-api/          # 用户空间 API
├── nos-syscalls/     # 系统调用接口
├── nos-services/     # 系统服务
├── docs/             # 文档
└── Cargo.toml        # 工作空间配置
```

### C. 相关资源

- [Rust 官方文档](https://doc.rust-lang.org/)
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/)
- [OSDev Wiki](https://wiki.osdev.org/)
- [Writing an OS in Rust](https://os.phil-opp.com/)

---

**版本**: 1.0.0
**最后更新**: 2025-12-28
**维护者**: NOS 开发团队
