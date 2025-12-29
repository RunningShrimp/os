# NOS 现代化升级迁移指南

## 概述

本文档说明了从 v0.1.0 升级到 v0.2.0 的破坏性变更及迁移步骤。本次升级主要聚焦于代码现代化、模块重构和特性整合，旨在提升代码可维护性和性能。

**主要变更**：
- 特性标志整合（30+ → 15个）
- 模块路径优化重构
- 大文件拆分与模块化
- 包结构优化
- 死代码清理
- 依赖版本更新

**升级影响等级**：中等（需要代码调整但改动可控）

---

## 破坏性变更清单

### 1. 特性标志变更

**变更说明**：30+个特性标志整合为15个核心特性，提升配置管理效率。

#### 1.1 网络特性整合

**旧特性** → **新特性**：
- `net_stack` → `networking`

**影响**：
- 使用 `net_stack` 特性的项目需要改用 `networking`
- 所有网络相关功能现在统一在 `networking` 特性下

**迁移步骤**：
```toml
# 旧版 .toml 配置
[dependencies]
kernel = { path = "./kernel", features = ["net_stack"] }

# 新版 .toml 配置
[dependencies]
kernel = { path = "./kernel", features = ["networking"] }
```

#### 1.2 图形子系统整合

**旧特性** → **新特性**：
- `graphics_subsystem` + `web_engine` → `graphics`

**影响**：
- 图形和Web引擎功能合并为单一特性
- 简化了图形功能的启用配置

**迁移步骤**：
```toml
# 旧版
[dependencies]
kernel = { path = "./kernel", features = ["graphics_subsystem", "web_engine"] }

# 新版
[dependencies]
kernel = { path = "./kernel", features = ["graphics"] }
```

#### 1.3 内存页特性整合

**旧特性** → **新特性**：
- `hpage_2mb` → 合并到 `huge_pages`
- `hpage_1gb` → 合并到 `huge_pages`
- **新特性**：`huge_pages`（统一管理2MB和1GB大页）

**影响**：
- 不再区分2MB和1GB大页配置
- 通过运行时配置选择大页大小

**迁移步骤**：
```toml
# 旧版
[dependencies]
kernel = { path = "./kernel", features = ["hpage_2mb", "hpage_1gb"] }

# 新版
[dependencies]
kernel = { path = "./kernel", features = ["huge_pages"] }
```

#### 1.4 性能优化特性整合

**旧特性** → **新特性**：
- `fast_syscall` → 合并到 `performance`
- `zero_copy` → 合并到 `performance`
- `batch_syscalls` → 合并到 `performance`
- `sched_opt` → 合并到 `performance`
- **新特性**：`performance`（统一性能优化功能）

**影响**：
- 所有性能优化特性统一管理
- 简化了高性能配置

**迁移步骤**：
```toml
# 旧版
[dependencies]
kernel = { path = "./kernel", features = ["fast_syscall", "zero_copy", "batch_syscalls"] }

# 新版
[dependencies]
kernel = { path = "./kernel", features = ["performance"] }
```

#### 1.5 调试与可观测性整合

**旧特性** → **新特性**：
- `debug` → 保留
- `debug_subsystems` → 合并到 `debug`
- `observability` → 合并到 `debug`
- **新特性**：`debug`（统一的调试和可观测性）

**影响**：
- 所有调试功能集中在单一特性下
- 日志、追踪、监控统一配置

**迁移步骤**：
```toml
# 旧版
[dependencies]
kernel = { path = "./kernel", features = ["debug", "debug_subsystems", "observability"] }

# 新版
[dependencies]
kernel = { path = "./kernel", features = ["debug"] }
```

#### 1.6 安全特性整合

**旧特性** → **新特性**：
- `security_audit` → 合并到 `security`
- `formal_verification` → 合并到 `security`
- **新特性**：`security`（统一安全功能）

**影响**：
- 安全审计和形式化验证统一管理
- 简化安全配置

**迁移步骤**：
```toml
# 旧版
[dependencies]
kernel = { path = "./kernel", features = ["security_audit", "formal_verification"] }

# 新版
[dependencies]
kernel = { path = "./kernel", features = ["security"] }
```

#### 1.7 完整特性映射表

| 旧特性名称 | 新特性名称 | 状态 |
|-----------|-----------|------|
| `net_stack` | `networking` | 重命名 |
| `graphics_subsystem` | `graphics` | 合并 |
| `web_engine` | `graphics` | 合并 |
| `hpage_2mb` | `huge_pages` | 合并 |
| `hpage_1gb` | `huge_pages` | 合并 |
| `fast_syscall` | `performance` | 合并 |
| `zero_copy` | `performance` | 合并 |
| `batch_syscalls` | `performance` | 合并 |
| `sched_opt` | `performance` | 合并 |
| `debug_subsystems` | `debug` | 合并 |
| `observability` | `debug` | 合并 |
| `security_audit` | `security` | 合并 |
| `formal_verification` | `security` | 合并 |

---

### 2. 模块路径变更

**变更说明**：优化模块组织结构，提升代码可维护性。

#### 2.1 系统调用优化模块重构

**旧路径** → **新路径**：
- `kernel::subsystems::syscalls::optimization_core` → `kernel::subsystems::syscalls::optimization::core`
- `kernel::subsystems::syscalls::optimization_common` → `kernel::subsystems::syscalls::optimization::common`
- `kernel::subsystems::syscalls::optimization_framework` → `kernel::subsystems::syscalls::optimization::framework`
- `kernel::subsystems::syscalls::optimization_tests` → `kernel::subsystems::syscalls::optimization::tests`

**影响**：
- 所有系统调用优化相关代码移至 `optimization/` 子模块
- 需要更新所有import语句

**迁移步骤**：
```rust
// 旧版 import
use kernel::subsystems::syscalls::optimization_core::UnifiedSyscallStats;
use kernel::subsystems::syscalls::optimization_common::OptimizationConfig;

// 新版 import
use kernel::subsystems::syscalls::optimization::core::UnifiedSyscallStats;
use kernel::subsystems::syscalls::optimization::common::OptimizationConfig;
```

#### 2.2 静态分析模块重构

**旧路径** → **新路径**：
- `kernel::subsystems::formal_verification::static_analyzer` → `kernel::subsystems::formal_verification::static_analyzer::mod`

**影响**：
- 静态分析功能拆分为多个子模块
- 需要更新import路径

**迁移步骤**：
```rust
// 旧版
use kernel::subsystems::formal_verification::static_analyzer::StaticAnalyzer;

// 新版
use kernel::subsystems::formal_verification::static_analyzer::mod::StaticAnalyzer;
```

#### 2.3 其他模块路径调整

**变更**：
- 部分内部模块重新组织以优化依赖关系
- 循环依赖问题的解决导致路径调整

**建议**：
- 使用 `cargo build` 编译错误提示找到需要调整的路径
- 参考编译错误信息中的建议路径

---

### 3. 文件结构变更

**变更说明**：大文件拆分为模块化结构，提升代码可维护性。

#### 3.1 POSIX测试模块拆分

**原文件**：`kernel/src/posix_tests/core_tests.rs`

**新结构**：
```
kernel/src/posix_tests/
  ├── core/           # 核心测试模块
  ├── thread/         # 线程测试模块
  └── signal/         # 信号测试模块
```

**影响**：
- 原单文件测试代码拆分为多个模块
- 需要更新测试文件的import路径

**迁移步骤**：
```rust
// 旧版
#[path = "posix_tests/core_tests.rs"]
mod core_tests;

// 新版
#[path = "posix_tests/core/mod.rs"]
mod core;

// 或使用模块声明
mod posix_tests {
    pub mod core;
    pub mod thread;
    pub mod signal;
}
```

#### 3.2 虚拟内存模块拆分

**原文件**：`kernel/src/subsystems/mm/vm.rs`

**新结构**：
```
kernel/src/subsystems/mm/vm/
  ├── mod.rs          # 主模块
  ├── mapping.rs      # 内存映射
  ├── page_table.rs   # 页表管理
  └── region.rs       # 内存区域
```

**影响**：
- 虚拟内存功能模块化
- 需要更新内部import

**迁移步骤**：
```rust
// 旧版
use kernel::subsystems::mm::vm::{VirtualMemory, PageTable};

// 新版 - 根据具体功能选择模块
use kernel::subsystems::mm::vm::mapping::VirtualMemory;
use kernel::subsystems::mm::vm::page_table::PageTable;
```

#### 3.3 EXT4文件系统模块拆分

**原文件**：`kernel/src/subsystems/fs/ext4.rs`

**新结构**：
```
kernel/src/subsystems/fs/ext4/
  ├── mod.rs                    # 主模块
  ├── ext4_enhanced_impl.rs     # 增强实现
  ├── ext4_persistence.rs       # 持久化
  └── ext4_unified.rs           # 统一接口
```

**影响**：
- EXT4文件系统功能模块化
- 提升代码可维护性

**迁移步骤**：
```rust
// 旧版
use kernel::subsystems::fs::ext4::Ext4Filesystem;

// 新版
use kernel::subsystems::fs::ext4::Ext4Filesystem;  // 主导出保持兼容
```

#### 3.4 系统调用优化模块拆分

**原文件**：
- `kernel/src/subsystems/syscalls/optimization_core.rs`
- `kernel/src/subsystems/syscalls/optimization_common.rs`
- `kernel/src/subsystems/syscalls/optimization_framework.rs`

**新结构**：
```
kernel/src/subsystems/syscalls/optimization/
  ├── mod.rs
  ├── core.rs
  ├── common.rs
  ├── framework.rs
  └── tests.rs
```

**影响**：
- 系统调用优化功能集中管理
- 优化模块间依赖关系

**迁移步骤**：
```rust
// 旧版
use kernel::subsystems::syscalls::optimization_core;

// 新版
use kernel::subsystems::syscalls::optimization::core;
```

---

### 4. 包结构变更

**变更说明**：优化crate组织结构，减少依赖复杂度。

#### 4.1 错误处理包合并

**变更**：`nos-error-handling` crate 合并到 `nos-api`

**原因**：
- 错误处理是API的核心部分
- 减少crate数量和依赖复杂度
- 统一错误类型定义

**影响**：
- 不再需要单独依赖 `nos-error-handling`
- 所有错误类型现在从 `nos-api` 导入

**迁移步骤**：

**Cargo.toml 更新**：
```toml
# 旧版
[dependencies]
nos-api = { path = "./nos-api" }
nos-error-handling = { path = "./nos-error-handling" }

# 新版
[dependencies]
nos-api = { path = "./nos-api" }  # 已包含错误处理
```

**Rust代码更新**：
```rust
// 旧版
use nos_error_handling::UnifiedError;
use nos_error_handling::ErrorContext;

// 新版
use nos_api::error::UnifiedError;
use nos_api::error::ErrorContext;

// 或者使用重导出
use nos_api::{UnifiedError, ErrorContext};
```

#### 4.2 API包结构优化

**变更**：`nos-api` 内部模块重组

**新结构**：
```
nos-api/src/
  ├── core/           # 核心类型和trait
  ├── error/          # 错误处理（从nos-error-handling迁移）
  ├── event/          # 事件系统
  ├── interfaces/     # 接口定义
  └── syscall/        # 系统调用类型
```

**影响**：
- 更清晰的模块组织
- 需要更新部分import路径

**迁移步骤**：
```rust
// 旧版
use nos_api::CoreTrait;
use nos_error_handling::UnifiedError;

// 新版
use nos_api::core::CoreTrait;
use nos_api::error::UnifiedError;
```

---

### 5. 删除的功能

**变更说明**：清理死代码和桩代码，提升代码质量。

#### 5.1 网络桩代码删除

**删除位置**：`kernel/src/lib.rs:122-169`

**删除内容**（16项）：
- `TcpSocket` 桩实现
- `UdpSocket` 桩实现
- `TcpListener` 桩实现
- `NetworkInterface` 桩实现
- 网络缓冲区管理桩
- 网络统计桩
- 其他网络相关桩代码

**影响**：
- 如果项目依赖这些桩代码，需要使用实际实现替代
- 推荐使用 `networking` 特性中的完整实现

**替代方案**：
```rust
// 旧版 - 使用桩代码
use kernel::{TcpSocket, UdpSocket};  // 桩实现

// 新版 - 使用实际实现
use kernel::subsystems::net::tcp::TcpSocket;
use kernel::subsystems::net::udp::UdpSocket;
```

#### 5.2 VirtIO驱动桩代码删除

**删除位置**：`kernel/src/platform/drivers/mod.rs:93-128`

**删除内容**：
- `VirtIOBlk` 桩驱动
- `VirtIONet` 桩驱动
- `VirtIOGpu` 桩驱动
- VirtIO队列管理桩

**影响**：
- VirtIO相关功能需要使用实际驱动实现
- 桩代码不再可用

**替代方案**：
```rust
// 旧版 - 使用桩代码
use kernel::platform::drivers::{VirtIOBlk, VirtIONet};

// 新版 - 使用实际驱动
use kernel::platform::drivers::virtio_blk::VirtIOBlk;
use kernel::platform::drivers::virtio_net::VirtIONet;
```

#### 5.3 进程辅助类型删除

**删除位置**：`kernel/src/subsystems/process/elf.rs:453`

**删除内容**：
- `AuxType` 枚举（未使用的辅助类型）

**影响**：
- 如果有代码依赖此类型，需要使用标准ELF辅助向量类型
- 该枚举定义重复，应使用标准定义

**替代方案**：
```rust
// 旧版
use kernel::subsystems::process::elf::AuxType;

// 新版 - 使用标准定义
use kernel::subsystems::process::elf::elf_auxv::AT_NULL;
```

#### 5.4 其他删除项

- **重复的类型定义**：删除了多个crate中重复的类型定义
- **未使用的测试工具**：清理了长期未使用的测试辅助函数
- **过时的宏定义**：删除了已被新宏替代的旧宏

---

### 6. 依赖版本更新

**变更说明**：更新关键依赖到最新稳定版本。

#### 6.1 核心依赖更新

| 包名 | 旧版本 | 新版本 | 破坏性变更 |
|-----|-------|--------|-----------|
| `spin` | 0.10.0 | 0.13.0 | API改进，性能提升 |
| `bitflags` | 2.4 | 2.8.0 | 新增宏特性 |
| `heapless` | 0.7 | 0.8.0 | API调整 |
| `criterion` | 0.8 (错误) | 0.5.1 | 版本修正 |

#### 6.2 spin 0.13.0 变更

**主要变更**：
- 改进的 `Mutex` API
- 更好的 `Once` 实现
- 性能优化

**迁移步骤**：
```rust
// 旧版
use spin::Mutex;

// 新版 - API基本兼容，但建议检查具体用法
use spin::Mutex;

// 新版推荐用法
let mutex = Mutex::new(0);
let guard = mutex.lock();  // 返回类型可能不同
```

#### 6.3 bitflags 2.8.0 变更

**主要变更**：
- 新增 `bitflags!` 宏特性
- 更好的文档生成
- 改进的错误信息

**迁移步骤**：
```rust
// 旧版
bitflags! {
    pub struct Flags: u32 {
        const A = 1;
        const B = 2;
    }
}

// 新版 - 基本兼容，可使用新特性
bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Flags: u32 {
        const A = 1;
        const B = 2;
    }
}
```

#### 6.4 heapless 0.8.0 变更

**主要变更**：
- `Vec` API调整
- `String` API改进
- 更好的错误处理

**迁移步骤**：
```rust
// 旧版
use heapless::Vec;

// 新版 - 检查API变更
use heapless::Vec;

let mut vec: Vec<u8, 16> = Vec::new();
vec.push(1).unwrap();  // API可能不同
```

#### 6.5 criterion 版本修正

**问题**：之前使用的 0.8 版本不存在

**修正**：更新到正确的 0.5.1 版本

**影响**：
- 基准测试API可能需要调整
- 建议检查基准测试代码

**迁移步骤**：
```rust
// 旧版（错误版本）
use criterion::{black_box, Criterion};

// 新版（正确版本）
use criterion::{black_box, criterion_group, criterion_main, Criterion};
```

---

## 升级步骤

本章节提供完整的升级流程指导。

### 步骤1：准备工作

**备份当前代码**：
```bash
# 创建备份分支
git checkout -b backup-before-v0.2.0-upgrade
git push origin backup-before-v0.2.0-upgrade

# 创建升级分支
git checkout -b upgrade-to-v0.2.0
```

**检查当前版本**：
```bash
# 查看当前Git标签
git tag --sort=-version | head -5

# 确认当前在正确分支
git branch --show-current
```

### 步骤2：更新依赖

**更新Cargo.lock**：
```bash
# 更新所有依赖
cargo update

# 或者只更新特定依赖
cargo update spin bitflags heapless criterion
```

**验证依赖更新**：
```bash
# 检查依赖版本
cargo tree | grep -E "spin|bitflags|heapless|criterion"
```

### 步骤3：更新特性标志

**查找所有使用旧特性的位置**：
```bash
# 搜索所有 Cargo.toml 文件
find . -name "Cargo.toml" -type f -exec grep -l "features" {} \;

# 搜索旧特性名称
grep -r "net_stack" --include="*.toml" .
grep -r "graphics_subsystem" --include="*.toml" .
grep -r "hpage_2mb\|hpage_1gb" --include="*.toml" .
grep -r "fast_syscall\|zero_copy\|batch_syscalls" --include="*.toml" .
```

**批量替换**：
```bash
# 使用sed或手动编辑Cargo.toml文件
# 将旧特性名称替换为新特性名称
```

**验证特性配置**：
```bash
# 检查特性是否正确配置
cargo check --features "networking,performance,debug"
```

### 步骤4：更新模块import路径

**查找需要更新的import**：
```bash
# 搜索旧的模块路径
grep -r "use kernel::subsystems::syscalls::optimization_core" \
  --include="*.rs" .

grep -r "use kernel::subsystems::formal_verification::static_analyzer" \
  --include="*.rs" .
```

**批量更新import**（推荐使用IDE）：
```bash
# 使用rust-analyzer的重构功能
# 或手动替换
find . -name "*.rs" -type f -exec sed -i '' \
  's/use kernel::subsystems::syscalls::optimization_core::/use kernel::subsystems::syscalls::optimization::core::/g' {} +
```

### 步骤5：删除已移除功能的引用

**查找使用已删除代码的位置**：
```bash
# 搜索网络桩代码的使用
grep -r "use kernel::TcpSocket" --include="*.rs" .

# 搜索VirtIO桩代码的使用
grep -r "use kernel::platform::drivers::VirtIO" --include="*.rs" .
```

**替换为实际实现**：
```rust
// 根据实际需求，将桩代码替换为真实实现
// 参考本文档"删除的功能"章节的替代方案
```

### 步骤6：重新编译项目

**完整编译**：
```bash
# 清理之前的构建
cargo clean

# 完整编译workspace
cargo build --workspace 2>&1 | tee compile.log

# 如果启用所有特性
cargo build --workspace --all-features 2>&1 | tee compile_all_features.log
```

**处理编译错误**：
```bash
# 查看编译错误数量
grep "error" compile.log | wc -l

# 分类错误
grep "E0425" compile.log | head -10  # 找不到名称
grep "E0433" compile.log | head -10  # 找不到模块
grep "E0277" compile.log | head -10  # 类型不匹配
```

### 步骤7：运行测试套件

**运行所有测试**：
```bash
# 单元测试
cargo test --workspace 2>&1 | tee test.log

# 集成测试
cargo test --workspace --test-threads=1 2>&1 | tee test_integration.log

# 文档测试
cargo test --workspace --doc 2>&1 | tee test_doc.log
```

**运行基准测试**：
```bash
# 基准性能测试
cargo bench --workspace 2>&1 | tee bench.log
```

### 步骤8：验证功能

**功能检查清单**：
- [ ] 网络功能正常（TCP/UDP）
- [ ] 图形系统正常渲染
- [ ] 文件系统正常挂载和读写
- [ ] 进程调度正常工作
- [ ] 系统调用正确执行
- [ ] 内存管理无泄漏
- [ ] 安全机制生效

**性能回归测试**：
```bash
# 对比升级前后的性能
# 保存基准测试结果
cargo bench --workspace -- --save-baseline main

# 对比基准
cargo bench --workspace -- --baseline main
```

### 步骤9：文档更新

**更新项目文档**：
```bash
# 更新 README.md
# 更新 API 文档
cargo doc --workspace --no-deps --open

# 更新开发者文档
# 根据需要更新相关文档
```

### 步骤10：提交更改

**审查更改**：
```bash
# 查看所有修改
git status

# 查看具体变更
git diff

# 查看新增文件
git status --short | grep "^??"
```

**提交更改**：
```bash
# 添加所有更改
git add -A

# 提交
git commit -m "Upgrade to v0.2.0: Migration and modernization

- Update feature flags (30+ -> 15)
- Refactor module paths
- Consolidate error handling to nos-api
- Remove stub code
- Update dependencies to latest versions
- Update import paths

BREAKING CHANGES: See docs/MIGRATION_GUIDE.md for details"

# 推送（如果需要）
git push origin upgrade-to-v0.2.0
```

---

## 常见问题

### Q1: 编译时找不到模块

**症状**：
```
error[E0433]: failed to resolve: use of undeclared crate or module `optimization_core`
```

**原因**：
- 模块路径变更
- import语句未更新

**解决方案**：
1. 检查本文档"模块路径变更"章节
2. 更新import路径为新路径
3. 使用 `cargo build` 的错误提示定位问题
4. 运行 `cargo fix --allow-dirty` 自动修复部分问题

**示例**：
```rust
// 错误
use kernel::subsystems::syscalls::optimization_core;

// 正确
use kernel::subsystems::syscalls::optimization::core;
```

### Q2: 特性标志不工作

**症状**：
```
error: unused manifest key: package.features.net_stack
```

**原因**：
- 使用了旧特性名称
- 特性已在v0.2.0中重命名或合并

**解决方案**：
1. 参考本文档"特性标志变更"章节
2. 使用新的特性名称
3. 检查 `Cargo.toml` 中的特性配置

**示例**：
```toml
# 错误
[dependencies]
kernel = { path = "./kernel", features = ["net_stack"] }

# 正确
[dependencies]
kernel = { path = "./kernel", features = ["networking"] }
```

### Q3: 类型不匹配错误

**症状**：
```
error[E0308]: mismatched types
   --> src/main.rs:10:5
    |
10  |     let x: UnifiedError = ...;
    |     ^^^^^^^^^^^^^^^^^^^^^^ expected enum `UnifiedError`, found enum `nos_api::error::UnifiedError`
```

**原因**：
- 错误处理包合并导致类型路径变更
- 依赖版本更新导致API变化

**解决方案**：
1. 检查import语句
2. 确保使用 `nos_api::error::UnifiedError`
3. 删除对 `nos-error-handling` 的引用
4. 运行 `cargo update` 同步依赖

**示例**：
```rust
// 错误
use nos_error_handling::UnifiedError;

// 正确
use nos_api::error::UnifiedError;
```

### Q4: 依赖冲突

**症状**：
```
error: failed to select a version for `spin`.
    ... required by `kernel v0.2.0`
    ... which is depended on by `user-space v0.1.0`
```

**原因**：
- 依赖版本要求冲突
- 中间库的依赖约束

**解决方案**：
1. 更新 `Cargo.toml` 锁定版本
2. 运行 `cargo update -p spin`
3. 如果持续冲突，考虑使用 `cargo upgrade`
4. 检查传递依赖是否需要更新

**示例**：
```toml
# 在 Cargo.toml 中明确指定版本
[dependencies]
spin = "0.13"  # 或 "0.13.0"
```

### Q5: 编译时间过长

**症状**：升级后编译时间显著增加

**原因**：
- 新增特性导致编译范围扩大
- 依赖版本更新可能影响编译速度

**解决方案**：
1. 使用 `cargo check` 快速检查语法
2. 分模块编译：`cargo build -p kernel`
3. 启用增量编译：`CARGO_INCREMENTAL=1 cargo build`
4. 使用 `sccache` 加速编译
5. 考虑只编译需要的特性

**示例**：
```bash
# 快速检查
cargo check --workspace

# 增量编译
CARGO_INCREMENTAL=1 cargo build --workspace

# 只编译kernel
cargo build -p kernel
```

### Q6: 测试失败

**症状**：升级后部分测试失败

**原因**：
- 测试代码import路径未更新
- 测试使用的API变更
- 特性标志影响测试行为

**解决方案**：
1. 更新测试代码中的import路径
2. 检查测试是否依赖特定特性
3. 运行特定测试：`cargo test test_name`
4. 查看详细错误：`cargo test -- --nocapture`

**示例**：
```bash
# 运行特定测试
cargo test test_syscall_optimization

# 查看测试输出
cargo test -- --nocapture --test-threads=1

# 只运行单元测试
cargo test --lib
```

### Q7: 性能下降

**症状**：升级后性能指标下降

**原因**：
- 特性配置不当
- 编译优化设置问题
- 新增的检查或日志

**解决方案**：
1. 确认启用了 `performance` 特性
2. 使用 `--release` 模式编译
3. 检查是否启用了调试日志
4. 运行基准测试对比
5. 使用 `perf` 或 `flamegraph` 分析

**示例**：
```bash
# 确保使用性能特性
cargo build --release --features "performance"

# 运行基准测试
cargo bench --bench syscall_benchmark

# 性能分析
cargo install flamegraph
cargo flamegraph --bench syscall_benchmark
```

### Q8: 文档生成失败

**症状**：`cargo doc` 报错

**原因**：
- 文档中的代码示例import错误
- 文档链接失效

**解决方案**：
1. 更新文档中的代码示例
2. 修复失效的文档链接
3. 使用 `--no-deps` 跳过依赖文档
4. 检查文档注释中的宏使用

**示例**：
```bash
# 只生成当前crate的文档
cargo doc --no-deps

# 打开文档
cargo doc --open

# 检查文档链接
cargo doc --document-private-items
```

---

## 回滚方案

如果升级过程中遇到无法解决的问题，可以回滚到升级前的状态。

### 临时回滚（保留更改）

```bash
# 切换到备份分支
git checkout backup-before-v0.2.0-upgrade

# 或者重置到升级前的提交
git reset --hard HEAD~1  # 如果已经提交
```

### 完全回滚（丢弃更改）

```bash
# 删除升级分支
git branch -D upgrade-to-v0.2.0

# 恢复到稳定版本
git checkout v0.1.0
```

### 选择性回滚部分更改

```bash
# 回滚特定文件
git checkout backup-before-v0.2.0-upgrade -- path/to/file.rs

# 回滚特定提交
git revert <commit-hash>
```

### 回滚后验证

```bash
# 确认可以正常编译
cargo build --workspace

# 运行测试确保功能正常
cargo test --workspace

# 检查依赖版本
cargo tree
```

---

## 升级检查清单

在完成升级后，使用此检查清单验证升级是否成功。

### 代码变更

- [ ] 所有 `Cargo.toml` 文件已更新特性标志
- [ ] 所有import路径已更新为新路径
- [ ] 删除了对已移除功能的引用
- [ ] 更新了使用 `nos-error-handling` 的代码
- [ ] 检查并修复了编译警告

### 编译验证

- [ ] `cargo check --workspace` 无错误
- [ ] `cargo build --workspace` 成功
- [ ] `cargo build --workspace --release` 成功
- [ ] `cargo build --workspace --all-features` 成功
- [ ] 编译时间在可接受范围内

### 测试验证

- [ ] `cargo test --workspace` 全部通过
- [ ] `cargo test --workspace --release` 全部通过
- [ ] 集成测试全部通过
- [ ] 基准测试可运行
- [ ] 无测试泄漏

### 功能验证

- [ ] 网络功能正常
- [ ] 图形系统正常
- [ ] 文件系统正常
- [ ] 进程管理正常
- [ ] 系统调用正常
- [ ] 内存管理正常
- [ ] 安全特性生效

### 性能验证

- [ ] 无明显性能下降
- [ ] 基准测试结果可接受
- [ ] 内存使用无异常
- [ ] 启动时间正常

### 文档更新

- [ ] README.md 已更新
- [ ] API文档生成正常
- [ ] 迁移指南完成
- [ ] 代码注释正确

### Git工作流

- [ ] 所有更改已提交
- [ ] 提交信息清晰
- [ ] 标签已更新（如需要）
- [ ] 分支策略正确

---

## 获取帮助

如果在升级过程中遇到问题，可以通过以下途径获取帮助。

### 文档资源

- **项目文档**：`docs/` 目录
- **API文档**：运行 `cargo doc --open` 查看
- **开发者指南**：`docs/DEVELOPER_GUIDE.md`
- **架构文档**：`docs/ARCHITECTURE.md`

### 代码示例

- **示例代码**：`examples/` 目录
- **测试用例**：`kernel/src/tests/` 和 `kernel/src/*/tests.rs`
- **集成测试**：`kernel/tests/` 目录

### 调试技巧

**启用详细日志**：
```bash
RUST_LOG=debug cargo build --workspace
```

**查看完整编译输出**：
```bash
cargo build --workspace 2>&1 | tee build.log
```

**使用编译器建议**：
```bash
cargo fix --allow-dirty --edition-idioms
```

### 性能分析

**生成Flamegraph**：
```bash
cargo install flamegraph
cargo flamegraph --bench benchmark_name
```

**使用perf分析**（Linux）：
```bash
cargo build --release --features "performance"
perf record ./target/release/kernel
perf report
```

### 社区支持

- **GitHub Issues**：提交问题和bug报告
- **Discussions**：参与讨论和问答
- **Pull Requests**：贡献修复和改进

### 联系方式

- **项目维护者**：通过GitHub Issues联系
- **安全漏洞**：使用私人报告渠道
- **功能请求**：提交Feature Request

---

## 附录

### A. 完整特性映射表

| 旧特性 | 新特性 | 变更类型 | 备注 |
|-------|-------|---------|------|
| `net_stack` | `networking` | 重命名 | 网络栈统一管理 |
| `graphics_subsystem` | `graphics` | 合并 | 图形相关功能合并 |
| `web_engine` | `graphics` | 合并 | Web引擎并入图形 |
| `hpage_2mb` | `huge_pages` | 合并 | 大页功能统一 |
| `hpage_1gb` | `huge_pages` | 合并 | 大页功能统一 |
| `fast_syscall` | `performance` | 合并 | 性能优化统一 |
| `zero_copy` | `performance` | 合并 | 零拷贝优化 |
| `batch_syscalls` | `performance` | 合并 | 批量系统调用 |
| `sched_opt` | `performance` | 合并 | 调度器优化 |
| `debug` | `debug` | 保留 | 调试功能 |
| `debug_subsystems` | `debug` | 合并 | 子系统调试 |
| `observability` | `debug` | 合并 | 可观测性 |
| `security_audit` | `security` | 合并 | 安全审计 |
| `formal_verification` | `security` | 合并 | 形式化验证 |

### B. 模块路径映射表

| 旧路径 | 新路径 | 文件 |
|-------|-------|------|
| `kernel::subsystems::syscalls::optimization_core` | `kernel::subsystems::syscalls::optimization::core` | optimization/core.rs |
| `kernel::subsystems::syscalls::optimization_common` | `kernel::subsystems::syscalls::optimization::common` | optimization/common.rs |
| `kernel::subsystems::syscalls::optimization_framework` | `kernel::subsystems::syscalls::optimization::framework` | optimization/framework.rs |
| `kernel::subsystems::syscalls::optimization_tests` | `kernel::subsystems::syscalls::optimization::tests` | optimization/tests.rs |
| `kernel::subsystems::formal_verification::static_analyzer` | `kernel::subsystems::formal_verification::static_analyzer::mod` | static_analyzer/mod.rs |

### C. 依赖版本清单

```toml
[dependencies]
# 核心依赖
spin = "0.13.0"
bitflags = "2.8.0"
heapless = "0.8.0"

# 开发依赖
criterion = "0.5.1"

# 其他依赖保持不变或根据需要更新
```

### D. 升级脚本示例

创建自动化升级脚本：

```bash
#!/bin/bash
# upgrade_to_v0.2.0.sh

set -e

echo "=== NOS v0.2.0 升级脚本 ==="

# 1. 备份当前代码
echo "Step 1: 创建备份分支..."
git checkout -b backup-before-v0.2.0-upgrade || git checkout backup-before-v0.2.0-upgrade

# 2. 创建升级分支
echo "Step 2: 创建升级分支..."
git checkout -b upgrade-to-v0.2.0 || git checkout upgrade-to-v0.2.0

# 3. 更新依赖
echo "Step 3: 更新依赖..."
cargo update

# 4. 批量替换特性标志
echo "Step 4: 更新特性标志..."
find . -name "Cargo.toml" -type f -exec sed -i '' \
  -e 's/"net_stack"/"networking"/g' \
  -e 's/"graphics_subsystem"/"graphics"/g' \
  -e 's/"web_engine"/"graphics"/g' \
  -e 's/"hpage_2mb"/"huge_pages"/g' \
  -e 's/"hpage_1gb"/"huge_pages"/g' \
  {} +

# 5. 编译检查
echo "Step 5: 编译检查..."
cargo check --workspace || {
  echo "编译检查失败，请查看错误信息"
  exit 1
}

# 6. 运行测试
echo "Step 6: 运行测试..."
cargo test --workspace || {
  echo "测试失败，请查看错误信息"
  exit 1
}

echo "=== 升级完成 ==="
echo "请检查并提交更改："
echo "  git status"
echo "  git diff"
echo "  git commit -m 'Upgrade to v0.2.0'"
```

### E. 验证脚本示例

创建验证脚本：

```bash
#!/bin/bash
# verify_upgrade.sh

set -e

echo "=== 验证 v0.2.0 升级 ==="

# 编译验证
echo "1. 编译验证..."
cargo check --workspace --all-features

# 测试验证
echo "2. 测试验证..."
cargo test --workspace --all-features

# 文档验证
echo "3. 文档验证..."
cargo doc --workspace --no-deps

# 特性验证
echo "4. 特性验证..."
cargo build --features "networking,performance,debug,security,huge_pages,graphics"

echo "=== 所有验证通过 ==="
```

---

**版本信息**：
- 文档版本：v1.0
- 适用升级：v0.1.0 → v0.2.0
- 最后更新：2025-12-29

**变更日志**：
- v1.0 (2025-12-29): 初始版本，完整迁移指南

---

**注意事项**：
1. 升级前务必备份代码
2. 建议在测试环境先进行升级验证
3. 大型项目建议分阶段升级
4. 保留升级日志以便问题追踪
5. 升级完成后进行全面的回归测试

**祝你升级顺利！** 如有问题，请参考"常见问题"章节或寻求社区支持。
