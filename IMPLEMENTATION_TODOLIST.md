# NOS 项目实施 Todo 列表

**生成日期**: 2025-12-09
**项目状态**: Phase 1 - 基础稳定化（第1-2周）
**当前优先级**: P0 - 关键（项目无法编译）

---

## 第1周 Task 清单

### Day 1-2: 编译错误修复 (P0)

#### [ ] 1.1.1 修复 optimization_service.rs 方法签名
- **文件**: `kernel/src/syscalls/optimization_service.rs`
- **问题**: Service trait 方法不存在（导致334个编译错误）
- **具体步骤**:
  - [ ] 打开文件，定位到 line 42 (OptimizationManagerService impl)
  - [ ] 删除方法 `service_type(&self) -> ServiceType`（line 42-44）
  - [ ] 删除方法 `restart(&mut self) -> Result<(), ServiceError>`（line 72-75）
  - [ ] 删除方法 `health_check(&self) -> Result<ServiceHealth, ServiceError>`（line 77-86）
  - [ ] 查找并删除同样的方法在其他 impl 块中（line 154, 183, 188, 245 等）
  - [ ] 修正方法名: `get_supported_syscalls()` → `supported_syscalls()`（line 104, 215）
- **验收**: `cargo check --lib` 运行不出错于此文件

#### [ ] 1.1.2 修复 PerformanceOptimizationService impl
- **文件**: `kernel/src/syscalls/optimization_service.rs`
- **问题**: PerformanceOptimizationService 实现有错误的方法
- **具体步骤**:
  - [ ] 定位到 line 154 (PerformanceOptimizationService impl)
  - [ ] 删除或修正同样的不存在的方法
  - [ ] 确保只实现 Service trait 中定义的方法

#### [ ] 1.1.3 修复 SchedulerOptimizationService impl
- **文件**: `kernel/src/syscalls/optimization_service.rs`
- **问题**: SchedulerOptimizationService 实现有错误的方法
- **具体步骤**:
  - [ ] 定位到 line 245 (SchedulerOptimizationService impl)
  - [ ] 删除或修正同样的不存在的方法

#### [ ] 1.1.4 验证 services/traits.rs 定义
- **文件**: `kernel/src/syscalls/services/traits.rs`
- **具体步骤**:
  - [ ] 打开文件，确认 Service trait 定义了什么方法
  - [ ] 确认 SyscallService trait 的方法签名
  - [ ] 与所有实现对比，确保一致性

#### [ ] 1.1.5 修复所有 impl Service 块
- **文件**: `kernel/src/syscalls/mm/service.rs`, `kernel/src/syscalls/net/service.rs` 等
- **具体步骤**:
  - [ ] 搜索所有 `impl Service` 的地方
  - [ ] 验证每个实现都只包含 trait 定义的方法
  - [ ] 删除任何额外的方法

#### [ ] 1.1.6 编译验证
```bash
cd /Users/didi/Desktop/nos
cargo check --lib 2>&1 | tail -20
# 预期结果: 无 error[E0407] 关于 optimization_service.rs
```

---

#### [ ] 1.1.7 修复 Service Registry 初始化问题
- **文件**: `kernel/src/syscalls/services/mod.rs`
- **具体步骤**:
  - [ ] 检查 `ServiceRegistry::new()` 是否返回正确的类型
  - [ ] 确认 `SyscallDispatcher::with_default_config()` 存在
  - [ ] 修复任何类型错配问题

#### [ ] 1.1.8 最终编译验证
```bash
cargo check --lib 2>&1
# 预期: 无任何编译错误
cargo build --lib 2>&1 | tail -5
# 预期: "Finished" 消息
```

---

### Day 3-4: 清理临时实验代码 (P0)

#### [ ] 1.2.1 创建 tools/ 目录结构
```bash
mkdir -p /Users/didi/Desktop/nos/tools/{cli,services,tests}
```

#### [ ] 1.2.2 移动优化工具脚本
- [ ] 移动 `kernel/src/syscalls/optimization_cli.rs` → `tools/cli/src/main.rs`
- [ ] 移动 `kernel/src/syscalls/optimization_service.rs` → `tools/services/optimization.rs`
- [ ] 移动 `kernel/src/syscalls/optimization_tests.rs` → `tools/tests/optimization.rs`

#### [ ] 1.2.3 移动文档到 docs/
- [ ] 移动 `kernel/src/syscalls/OPTIMIZATION_SUMMARY.md` → `docs/OPTIMIZATION_SUMMARY.md`
- [ ] 移动 `kernel/src/syscalls/README_OPTIMIZATION.md` → `docs/README_OPTIMIZATION.md`
- [ ] 移动 `kernel/src/syscalls/PROCESS_SYSCALLS_ANALYSIS_REPORT.md` → `docs/`

#### [ ] 1.2.4 处理孤立测试文件
- **文件**: `kernel/src/enhanced_tests.rs`
- **选项**: 
  - [ ] 如果重要，集成到 `kernel/src/tests/` 并在 `lib.rs` 中注册
  - [ ] 如果不重要，删除
- **验收**: 编译通过，无"unused file"警告

#### [ ] 1.2.5 删除备份文件
```bash
rm -f /Users/didi/Desktop/nos/kernel/src/syscalls/fs.rs.bak
```

#### [ ] 1.2.6 验证清理完成
```bash
find /Users/didi/Desktop/nos/kernel/src -name "*.bak" | wc -l  # 预期: 0
find /Users/didi/Desktop/nos/kernel/src -name "OPTIMIZATION_*" | wc -l  # 预期: 0
cargo check --lib  # 验证: 0 errors
```

---

### Day 5: 统一内存分配器 (P1)

#### [ ] 1.3.1 分析分配器实现对比
- **文件对比**:
  - [ ] `kernel/src/mm/buddy.rs` vs `kernel/src/mm/optimized_buddy.rs`
  - [ ] `kernel/src/mm/slab.rs` vs `kernel/src/mm/optimized_slab.rs`
- **对比维度**:
  - [ ] 代码行数
  - [ ] 性能特性（bitmap 优化）
  - [ ] 内存开销
  - [ ] 代码清晰度
- **结论**: [ ] 优化版本确实更优

#### [ ] 1.3.2 删除基础版本
```bash
rm /Users/didi/Desktop/nos/kernel/src/mm/buddy.rs
rm /Users/didi/Desktop/nos/kernel/src/mm/slab.rs
```

#### [ ] 1.3.3 重命名优化版本
```bash
cd /Users/didi/Desktop/nos/kernel/src/mm
mv optimized_buddy.rs buddy.rs
mv optimized_slab.rs slab.rs
```

#### [ ] 1.3.4 修改 mod.rs 导入
- **文件**: `kernel/src/mm/mod.rs`
- **具体步骤**:
  - [ ] 移除 `pub mod buddy;` 和 `pub mod optimized_buddy;` 中的重复
  - [ ] 保留 `pub mod buddy;`
  - [ ] 移除 `pub use optimized_buddy::*;`
  - [ ] 保留 `pub use buddy::*;`
  - [ ] 同样处理 slab

#### [ ] 1.3.5 创建 MemoryAllocator Trait
- **文件**: `kernel/src/mm/traits.rs`
- **代码**:
  ```rust
  pub trait MemoryAllocator: Send + Sync {
      fn allocate(&mut self, layout: Layout) -> Result<*mut u8>;
      fn deallocate(&mut self, ptr: *mut u8, layout: Layout);
      fn stats(&self) -> AllocatorStats;
  }
  
  // 为 BuddyAllocator 实现
  impl MemoryAllocator for BuddyAllocator { ... }
  
  // 为 SlabAllocator 实现
  impl MemoryAllocator for SlabAllocator { ... }
  ```

#### [ ] 1.3.6 删除其他冗余分配器
- [ ] 删除 `kernel/src/mm/copy_optimized.rs`（临时实验代码）
- [ ] 评估 `kernel/src/mm/optimized_allocator.rs`：
  - [ ] 如果是 hybrid allocator，集成到主分配器
  - [ ] 如果是工具，移到 tools/
- [ ] 评估 `kernel/src/mm/percpu_allocator.rs`：
  - [ ] 保留或删除（基于项目需求）

#### [ ] 1.3.7 编译验证
```bash
cargo check --lib 2>&1 | grep "optimized"  # 预期: 无结果
ls /Users/didi/Desktop/nos/kernel/src/mm/*optimized* 2>&1  # 预期: 无文件
cargo build --lib 2>&1 | tail -1  # 预期: "Finished"
```

---

### 第1周末完成检查点

```bash
# 检查点 1: 编译通过
cargo check --lib 2>&1 | grep -c "^error"  # 预期: 0

# 检查点 2: 临时代码清理
find kernel/src -name "*optimization*" -o -name "*.bak" | wc -l  # 预期: 0

# 检查点 3: 内存分配器统一
ls kernel/src/mm/*buddy* | grep -v "\.rs$"  # 仅 buddy.rs
ls kernel/src/mm/*slab* | grep -v "\.rs$"   # 仅 slab.rs

# 检查点 4: 代码量减少
find kernel/src -name "*.rs" | xargs wc -l | tail -1
# 预期: 相比开始时减少 3000+ 行

# 检查点 5: 测试通过
cargo test --lib 2>&1 | tail -1  # 预期: "test result: ok"
```

---

## 第2周 Task 清单

### Day 6: 错误处理统一 (P1)

#### [ ] 1.4.1 定义统一 KernelError 枚举
- **文件**: `kernel/src/error_handling/mod.rs`
- **具体步骤**:
  - [ ] 打开文件
  - [ ] 在文件顶部创建新的 enum KernelError:
    ```rust
    #[derive(Debug, Clone)]
    pub enum KernelError {
        IoError(isize),  // errno 值
        MemoryError(String),
        ProcessError(String),
        NetworkError(String),
        FileSystemError(String),
        SecurityError(String),
        NotSupported,
        Other(String),
    }
    ```
  - [ ] 为 KernelError 实现 Display 和 Error traits

#### [ ] 1.4.2 创建 errno 映射表
- **文件**: `kernel/src/reliability/errno_mapping.rs` (新建)
- **代码结构**:
  ```rust
  pub fn kernel_error_to_errno(err: &KernelError) -> isize {
      match err {
          KernelError::MemoryError(_) => ENOMEM,
          KernelError::IoError(e) => *e,
          // ... 完整映射
      }
  }
  ```
- [ ] 创建此文件并实现完整映射
- [ ] 映射至少30个常见 errno 值

#### [ ] 1.4.3 创建错误转换函数
- **文件**: `kernel/src/error_handling/mod.rs`
- **具体步骤**:
  - [ ] 实现 `impl From<MemoryError> for KernelError`
  - [ ] 实现 `impl From<VfsError> for KernelError`
  - [ ] 实现 `impl From<NetworkError> for KernelError`
  - [ ] 实现 `impl From<SyscallError> for KernelError`
  - [ ] 实现 `impl From<String> for KernelError`

#### [ ] 1.4.4 验证编译
```bash
cargo check --lib 2>&1 | grep "KernelError"  # 无错误
```

---

### Day 7-8: 系统调用分发器重构 (Phase 2.1 - P1)

#### [ ] 2.1.1 创建动态分发器骨架
- **文件**: `kernel/src/syscalls/dispatcher.rs` (新建)
- **代码框架**:
  ```rust
  use alloc::collections::BTreeMap;
  
  pub type SyscallHandler = fn(&[u64]) -> isize;
  
  pub struct SyscallDispatcher {
      handlers: BTreeMap<u32, SyscallHandler>,
  }
  
  impl SyscallDispatcher {
      pub fn new() -> Self {
          Self {
              handlers: BTreeMap::new(),
          }
      }
      
      pub fn register(&mut self, num: u32, handler: SyscallHandler) {
          self.handlers.insert(num, handler);
      }
      
      pub fn dispatch(&self, num: u32, args: &[u64]) -> isize {
          self.handlers
              .get(&num)
              .map(|h| h(args))
              .unwrap_or(-1)  // ENOSYS
      }
  }
  ```

#### [ ] 2.1.2 在 mod.rs 中声明 dispatcher 模块
- **文件**: `kernel/src/syscalls/mod.rs`
- [ ] 添加 `pub mod dispatcher;`
- [ ] 添加 `pub use dispatcher::*;`

#### [ ] 2.1.3 创建模块级 register_syscalls 函数骨架
- [ ] 在 `kernel/src/process/mod.rs` 添加:
  ```rust
  pub fn register_syscalls(dispatcher: &mut SyscallDispatcher) {
      // 注册所有进程相关 syscall
  }
  ```
- [ ] 同样为 fs, mm, ipc, net 模块添加

#### [ ] 2.1.4 重构 mod.rs 以消除硬编码导入
- **文件**: `kernel/src/syscalls/mod.rs`
- **具体步骤**:
  - [ ] 统计当前 `use crate::` 导入数量（应该是287个）
  - [ ] 创建 init 函数：
    ```rust
    pub fn init_syscall_dispatcher() -> SyscallDispatcher {
        let mut dispatcher = SyscallDispatcher::new();
        crate::process::register_syscalls(&mut dispatcher);
        crate::fs::register_syscalls(&mut dispatcher);
        crate::mm::register_syscalls(&mut dispatcher);
        crate::ipc::register_syscalls(&mut dispatcher);
        crate::net::register_syscalls(&mut dispatcher);
        dispatcher
    }
    ```
  - [ ] 删除硬编码的 match 块（数百行）

#### [ ] 2.1.5 验证编译与导入数量
```bash
cargo check --lib
grep "^use crate::" /Users/didi/Desktop/nos/kernel/src/syscalls/mod.rs | wc -l
# 预期: < 20（从287减少）
```

---

### Day 9-10: 系统调用实现合并 (Phase 2.2 - P1)

#### [ ] 2.2.1 合并文件 I/O 系统调用
- **步骤**:
  - [ ] 创建 `kernel/src/syscalls/file_io/` 目录
  - [ ] 移动 `file_io_optimized.rs` → `kernel/src/syscalls/file_io/mod.rs`
  - [ ] 删除原始 `file_io.rs`
  - [ ] 在 `syscalls/mod.rs` 中：
    - [ ] 移除 `pub mod file_io;`
    - [ ] 添加 `pub mod file_io;`（指向新目录）
  - [ ] 验证编译

#### [ ] 2.2.2 合并进程系统调用
- [ ] 创建 `kernel/src/syscalls/process/mod.rs`
- [ ] 选择 `process_optimized.rs` 作为基础
- [ ] 删除原始 `process.rs` 和优化版本
- [ ] 验证编译和测试

#### [ ] 2.2.3 合并内存系统调用
- [ ] 创建 `kernel/src/syscalls/memory/mod.rs`
- [ ] 选择 `memory_optimized.rs` 作为基础
- [ ] 删除原始 `memory.rs` 和优化版本
- [ ] 验证编译和测试

#### [ ] 2.2.4 合并信号处理
- [ ] 创建 `kernel/src/syscalls/signal/mod.rs`
- [ ] 合并 `signal.rs`, `signal_advanced.rs`, `signal_optimized.rs`
- [ ] 使用 feature flag `advanced_signals` 控制高级特性
- [ ] 删除三个原始文件
- [ ] 在 `Cargo.toml` 添加 feature: `advanced_signals`

#### [ ] 2.2.5 合并网络系统调用
- [ ] 创建 `kernel/src/syscalls/net/mod.rs`
- [ ] 合并 `network.rs` 和 `network_optimized.rs`
- [ ] 删除原始文件

#### [ ] 2.2.6 合并零拷贝 I/O
- [ ] 创建 `kernel/src/syscalls/zero_copy/mod.rs`
- [ ] 合并 `zero_copy.rs` 和 `zero_copy_optimized.rs`
- [ ] 删除原始文件

#### [ ] 2.2.7 编译验证
```bash
cargo check --lib
cargo build --lib
ls /Users/didi/Desktop/nos/kernel/src/syscalls/*_optimized.rs 2>&1 | wc -l
# 预期: 0
```

---

### Day 11: Service 规范化 (Phase 2.3 - P1)

#### [ ] 2.3.1 审查 Service Trait 定义
- **文件**: `kernel/src/syscalls/services/traits.rs`
- **检查清单**:
  - [ ] 打开文件
  - [ ] 列出 Service trait 的所有方法
  - [ ] 列出 SyscallService trait 的所有方法
  - [ ] 记录每个方法的签名

#### [ ] 2.3.2 检查所有 Service 实现
- **文件**: `kernel/src/syscalls/*/service.rs`
- **具体步骤**:
  - [ ] 在 `mm/service.rs` 中检查 impl Service for MemoryService
  - [ ] 在 `net/service.rs` 中检查 impl Service for NetworkService
  - [ ] 比较实现方法与 trait 定义
  - [ ] 修正任何不匹配的地方

#### [ ] 2.3.3 完成或禁用 process_service
- **选项 A: 完成实现**
  - [ ] 打开 `kernel/src/syscalls/process_service/service.rs`
  - [ ] 找到所有 `// TODO:` 注释
  - [ ] 实现每个处理程序（fork, exec, exit, wait, kill, sched_yield, nice）
  - [ ] 添加单元测试
- **选项 B: 禁用（临时）**
  - [ ] 在 `Cargo.toml` 中注释掉 process_service 特性
  - [ ] 在 `mod.rs` 中注释掉导入

#### [ ] 2.3.4 完成或禁用 fs_service
- **同上选项 A/B**
  - [ ] 实现或禁用文件系统服务

#### [ ] 2.3.5 编译验证
```bash
cargo build --lib 2>&1
# 预期: "Finished" 消息
cargo test --lib 2>&1 | tail -1
# 预期: "test result: ok"
```

---

### 第2周末完成检查点

```bash
# 检查点 1: 编译成功
cargo build --lib 2>&1 | grep -c "^error"  # 预期: 0

# 检查点 2: 优化文件消除
ls kernel/src/syscalls/*_optimized.rs 2>&1 | wc -l  # 预期: 0
ls kernel/src/syscalls/*_advanced.rs 2>&1 | wc -l  # 预期: 0

# 检查点 3: 硬编码导入减少
grep "^use crate::" kernel/src/syscalls/mod.rs | wc -l  # 预期: < 20

# 检查点 4: 测试通过
cargo test --lib 2>&1 | grep "test result: ok"  # 预期: 显示

# 检查点 5: 代码行数统计
find kernel/src -name "*.rs" | xargs wc -l | tail -1
# 预期: 比周一减少 5000+ 行
```

---

## 第3-4周 Task 清单（Phase 2.3 继续 + Phase 3 开始）

### Day 12-14: Service 规范化完成和初步测试框架

#### [ ] 2.3.5 优化 ServiceRegistry 实现
- **文件**: `kernel/src/syscalls/services/registry.rs`
- [ ] 实现 service 动态注册
- [ ] 实现 service 依赖解析
- [ ] 添加生命周期管理

#### [ ] 3.1.1 创建基础集成测试框架
- **文件**: `kernel/tests/integration/` (新建)
- [ ] 创建目录
- [ ] 创建 `lib.rs` 或 test helper
- [ ] 为基本 syscall 创建 5+ 集成测试

### Day 15-18: IPC 实现开始

#### [ ] 3.1.2 实现管道 (Pipe)
- **文件**: `kernel/src/syscalls/ipc/handlers.rs` line 33
- [ ] 实现 `sys_pipe()` 完整逻辑
- [ ] 创建 Pipe 结构体
- [ ] 实现读写操作
- [ ] 添加错误处理

#### [ ] 3.1.3 实现消息队列基础
- **文件**: `kernel/src/syscalls/ipc/handlers.rs` line 197-226
- [ ] 实现 `sys_msgget()`
- [ ] 实现 `sys_msgsnd()`
- [ ] 基础测试

---

## 后续 Task 优先级列表

**P0（关键，必须完成）**:
- [ ] Phase 1: 稳定化（第1-2周）
- [ ] Phase 2: 架构解耦（第3-4周）

**P1（高优先级）**:
- [ ] Phase 3: 功能补全（第5-8周）
  - [ ] IPC 实现（管道、消息队列、共享内存、信号量）
  - [ ] 网络系统调用完成
  - [ ] 内存管理系统调用完成
  - [ ] 文件系统完成
  - [ ] POSIX syscall 集合

**P2（中优先级）**:
- [ ] Phase 4: 生产化（第9-12周）
  - [ ] 集成测试覆盖
  - [ ] 性能基准与优化
  - [ ] 压力与长期运行测试

**P3（低优先级，后续）**:
- [ ] 安全加固与 Fuzzing
- [ ] 文档完善
- [ ] 版本发布准备

---

## 快速命令参考

```bash
# 编译检查
cargo check --lib
cargo build --lib

# 测试运行
cargo test --lib
cargo test --lib ipc::tests
cargo test --test '*'

# 代码统计
find kernel/src -name "*.rs" | xargs wc -l | tail -1
grep "^use crate::" kernel/src/syscalls/mod.rs | wc -l
grep "TODO\|FIXME" kernel/src/ -r | wc -l

# 文件搜索
find kernel/src -name "*_optimized.rs"
find kernel/src -name "*.bak"
find kernel/src -name "*optimization*"

# 版本控制
git status
git add -A
git commit -m "Phase 1: [具体改动]"
git push origin feature/week1-core-implementations
```

---

## 完成标记

### Phase 1 完成标记（预计 Day 10）

- [ ] 编译无误（cargo build --lib）
- [ ] 所有单元测试通过
- [ ] 临时代码清理完毕
- [ ] 内存分配器统一
- [ ] 错误处理统一

### Phase 2 完成标记（预计 Day 24）

- [ ] syscalls/mod.rs 硬编码导入 < 10
- [ ] 所有 *_optimized.rs 文件删除
- [ ] Service Registry 正常工作
- [ ] 集成测试框架建立
- [ ] 代码覆盖率 >= 30%

### 后续阶段完成标记

- Phase 3: 所有 IPC/网络/内存 syscall 完成，TODO < 20
- Phase 4: 300+ 集成测试通过，版本 v0.1.0-alpha.1 标记

---

**最后更新**: 2025-12-09
**状态**: ⚠️ 等待执行
**下一步**: 立即开始 Day 1 - 修复编译错误
