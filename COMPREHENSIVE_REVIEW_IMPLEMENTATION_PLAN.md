# NOS项目全面审查实施计划

## 执行摘要

本文档基于全面代码审查结果，制定了NOS项目从当前不可编译状态恢复至生产级别的详细实施计划。计划分为4个阶段，共12周，预期投入为5-8人月。

---

## 项目当前状态基线

- **代码规模**: 205,510行
- **编译状态**: ❌ 334个编译错误（无法构建）
- **测试覆盖**: <30%（多数框架未集成）
- **生产就绪度**: 2/10（关键缺陷）

---

## 第1阶段：基础稳定化（第1-2周）

**目标**: 消除编译错误，清理临时代码，建立稳定基础

### Phase 1.1：修复编译错误（P0）

#### 任务 1.1.1：修复 optimization_service.rs Trait 不匹配
- **问题**: Service trait 方法签名不匹配（334个编译错误的根源）
- **文件**: `kernel/src/syscalls/optimization_service.rs`
- **具体修复**:
  - [ ] 移除不存在的方法: `service_type()`, `restart()`, `health_check()`
  - [ ] 修正 `get_supported_syscalls()` → `supported_syscalls()`
  - [ ] 验证所有实现与 `traits.rs` 中 Service trait 签名一致
  - [ ] 运行 `cargo check --lib` 确认无错误

#### 任务 1.1.2：修复 Service Registry 初始化错误
- **文件**: `kernel/src/syscalls/services/`
- **具体修复**:
  - [ ] 验证 `traits.rs` 中 Service trait 的完整定义
  - [ ] 检查 `registry.rs` 中 ServiceEntry 的兼容性
  - [ ] 修复所有 impl Service 的地方，确保方法签名一致

#### 任务 1.1.3：禁用或完成 Service 实现
- **选择方案**: 禁用未完成的服务（临时方案）
- [ ] 在 `Cargo.toml` 中注释掉有问题的 feature flag
- [ ] 或完成 `process_service/` 和 `fs_service/` 的实现

**验收标准**: 
```bash
cargo check --lib  # 输出: 0 errors
```

---

### Phase 1.2：删除临时实验代码

#### 任务 1.2.1：清理 syscalls 目录下的工具脚本
- **删除文件**:
  - [ ] `kernel/src/syscalls/optimization_cli.rs` → 移到 `tools/cli/`
  - [ ] `kernel/src/syscalls/optimization_service.rs` → 移到 `tools/services/`
  - [ ] `kernel/src/syscalls/optimization_tests.rs` → 移到 `tools/tests/`
  - [ ] `kernel/src/syscalls/OPTIMIZATION_SUMMARY.md` → 移到 `docs/`
  - [ ] `kernel/src/syscalls/README_OPTIMIZATION.md` → 移到 `docs/`

#### 任务 1.2.2：隔离孤立的测试文件
- [ ] `kernel/src/enhanced_tests.rs` 重新集成或删除
- [ ] 确保所有测试通过 `lib.rs` 模块声明注册

#### 任务 1.2.3：清理 syscalls/fs.rs.bak
- [ ] 删除备份文件: `kernel/src/syscalls/fs.rs.bak`

**验收标准**:
```bash
find kernel/src -name "*.bak" | wc -l  # 结果: 0
find kernel/src -name "OPTIMIZATION_*" | wc -l  # 结果: 0
```

---

### Phase 1.3：统一内存分配器

#### 任务 1.3.1：选定参考实现
- [ ] 确认 `optimized_buddy.rs` 优于 `buddy.rs`
- [ ] 确认 `optimized_slab.rs` 优于 `slab.rs`
- [ ] 分析两个版本的性能对比，选定单一实现

#### 任务 1.3.2：删除冗余实现
- [ ] 删除 `kernel/src/mm/buddy.rs`
- [ ] 删除 `kernel/src/mm/slab.rs`
- [ ] 将 `optimized_buddy.rs` 重命名为 `buddy.rs`
- [ ] 将 `optimized_slab.rs` 重命名为 `slab.rs`

#### 任务 1.3.3：创建 Allocator Trait 抽象
- [ ] 在 `kernel/src/mm/traits.rs` 中定义:
  ```rust
  pub trait MemoryAllocator: Send + Sync {
      fn allocate(&mut self, layout: Layout) -> Result<*mut u8>;
      fn deallocate(&mut self, ptr: *mut u8, layout: Layout);
      fn stats(&self) -> AllocatorStats;
  }
  ```
- [ ] 为 BuddyAllocator 实现此 trait
- [ ] 为 SlabAllocator 实现此 trait

#### 任务 1.3.4：删除其他冗余分配器
- [ ] 删除 `kernel/src/mm/copy_optimized.rs`（临时实验）
- [ ] 整合 `kernel/src/mm/optimized_allocator.rs` 到主分配器
- [ ] 评估 `percpu_allocator.rs` 是否必要

**验收标准**:
```bash
ls kernel/src/mm/*buddy* | grep -v "\.rs$"  # 仅 buddy.rs
ls kernel/src/mm/*slab* | grep -v "\.rs$"   # 仅 slab.rs
cargo check --lib  # 0 errors
```

---

### Phase 1.4：统一错误处理

#### 任务 1.4.1：定义统一错误类型
- [ ] 在 `kernel/src/error_handling/mod.rs` 中创建 `KernelError` 作为单一错误源
- [ ] 定义所有可能的错误变体（Memory、Process、Network、Fs等）
- [ ] 实现 `as_errno()` 方法映射到 POSIX errno

#### 任务 1.4.2：创建错误转换函数
- [ ] 实现 `impl From<MemoryError> for KernelError`
- [ ] 实现 `impl From<VfsError> for KernelError`
- [ ] 实现 `impl From<NetworkError> for KernelError`
- [ ] 实现 `impl From<SyscallError> for KernelError`

#### 任务 1.4.3：建立 errno 映射表
- [ ] 创建完整的 POSIX errno 映射
- [ ] 文件: `kernel/src/reliability/errno_mapping.rs`
- [ ] 涵盖所有系统调用可能的错误代码

**验收标准**:
```bash
# 所有模块都使用 KernelError
grep -r "enum.*Error" kernel/src/syscalls/ | wc -l  # 应减少50%+
```

---

## 第2阶段：架构解耦（第3-4周）

**目标**: 消除高耦合的硬编码依赖，实现动态分发

### Phase 2.1：重构系统调用分发器

#### 任务 2.1.1：创建动态分发机制
- [ ] 创建 `kernel/src/syscalls/dispatcher.rs` 新文件
- [ ] 定义 `SyscallDispatcher` 结构体和 `register()` 方法
- [ ] 实现 `dispatch()` 方法支持动态路由（无硬编码）

#### 任务 2.1.2：消除硬编码导入
- [ ] 重构 `kernel/src/syscalls/mod.rs`:
  - [ ] 移除所有 287 个 `use crate::...` 硬编码导入
  - [ ] 替换为动态 `dispatcher.register()` 调用
  - [ ] 将注册逻辑移到模块初始化函数

#### 任务 2.1.3：实现模块化注册接口
- [ ] 为每个系统调用模块（process/fs/mm/ipc/net）定义 `register_syscalls()` 函数
- [ ] 在内核初始化时按顺序调用注册函数
- [ ] 编译时减少模块间的直接依赖

**验收标准**:
```bash
# mod.rs 中的 use crate:: 导入减少到 <10 个
grep "^use crate::" kernel/src/syscalls/mod.rs | wc -l  # <10

cargo check --lib  # 0 errors, 同时验证分发器工作
```

---

### Phase 2.2：合并重复的系统调用实现

#### 任务 2.2.1：合并文件 I/O 系统调用
- [ ] 创建 `kernel/src/syscalls/file_io/` 目录
- [ ] 合并 `file_io.rs` 和 `file_io_optimized.rs`:
  - [ ] 选择优化版本作为基础
  - [ ] 使用 `#[inline]` 和 `#[inline(always)]` 代替代码复制
  - [ ] 为热路径添加条件编译 feature flag
- [ ] 删除旧文件
- [ ] 验证所有 open/close/read/write 测试通过

#### 任务 2.2.2：合并进程系统调用
- [ ] 合并 `process.rs` 和 `process_optimized.rs`
- [ ] 优化实现成为主实现
- [ ] 删除冗余文件

#### 任务 2.2.3：合并内存系统调用
- [ ] 合并 `memory.rs` 和 `memory_optimized.rs`
- [ ] 删除冗余文件

#### 任务 2.2.4：合并信号处理
- [ ] 创建 `kernel/src/syscalls/signal/` 目录
- [ ] 合并 `signal.rs` + `signal_advanced.rs` + `signal_optimized.rs`
- [ ] 使用 feature flag (`advanced_signals`) 控制高级特性

#### 任务 2.2.5：合并网络系统调用
- [ ] 合并 `network.rs` 和 `network_optimized.rs`

#### 任务 2.2.6：合并零拷贝 I/O
- [ ] 合并 `zero_copy.rs` 和 `zero_copy_optimized.rs`

**验收标准**:
```bash
# 检查是否消除了冗余的 *_optimized.rs 文件
ls kernel/src/syscalls/*_optimized.rs 2>/dev/null | wc -l  # 应该 = 0

# 验证代码行数减少
find kernel/src/syscalls -name "*.rs" | xargs wc -l | tail -1
# 预期从 ~28,000 行减少到 ~20,000 行
```

---

### Phase 2.3：完成 Service Registry 规范

#### 任务 2.3.1：建立统一的 Service Trait
- [ ] 审查并修正 `kernel/src/syscalls/services/traits.rs`
- [ ] 确保方法签名的一致性
- [ ] 添加完整的文档注释

#### 任务 2.3.2：完成 ServiceRegistry 实现
- [ ] 实现 service 动态注册/注销
- [ ] 实现 service 依赖解析
- [ ] 实现 service 生命周期管理

#### 任务 2.3.3：修复或删除占位符服务
- **选项A**: 完成实现
  - [ ] 实现 `kernel/src/syscalls/process_service/` 中的所有处理器
  - [ ] 实现 `kernel/src/syscalls/fs_service/` 中的所有处理器
- **选项B**: 临时禁用
  - [ ] 注释掉这些模块的初始化代码
  - [ ] 留下 TODO 标记供后续完成

**验收标准**:
```bash
# 检查编译通过
cargo build --lib 2>&1 | grep -c "^error"  # 应该 = 0

# 验证 Service trait 实现一致
grep -r "fn.*(&self)" kernel/src/syscalls/services/traits.rs > /tmp/methods.txt
grep -r "impl.*Service" kernel/src/syscalls/*_service/ | grep "fn" > /tmp/impls.txt
# 手动对比，确保方法匹配
```

---

## 第3阶段：功能补全（第5-8周）

**目标**: 完成关键缺失功能，实现完整的 POSIX 接口

### Phase 3.1：完成 IPC 实现

#### 任务 3.1.1：实现管道（Pipe）
- **文件**: `kernel/src/syscalls/ipc/handlers.rs` line 33
- [ ] 实现 `sys_pipe()` 系统调用
- [ ] 创建内核管道结构体
- [ ] 实现管道的读写操作
- [ ] 处理 EOF 和阻塞/非阻塞模式
- [ ] 添加单元测试

#### 任务 3.1.2：实现消息队列
- **文件**: `kernel/src/syscalls/ipc/handlers.rs` line 197-286
- [ ] 实现 `sys_msgget()` - 创建/获取消息队列
- [ ] 实现 `sys_msgsnd()` - 发送消息
- [ ] 实现 `sys_msgrcv()` - 接收消息
- [ ] 实现 `sys_msgctl()` - 控制消息队列
- [ ] 处理消息优先级和超时
- [ ] 添加集成测试

#### 任务 3.1.3：实现共享内存
- **文件**: `kernel/src/syscalls/ipc/handlers.rs` line 88-170
- [ ] 实现 `sys_shmget()` - 创建/获取共享内存段
- [ ] 实现 `sys_shmat()` - 附加共享内存
- [ ] 实现 `sys_shmdt()` - 分离共享内存
- [ ] 实现 `sys_shmctl()` - 控制共享内存
- [ ] 实现缓存一致性机制
- [ ] 添加集成测试

#### 任务 3.1.4：实现信号量
- **文件**: `kernel/src/syscalls/ipc/handlers.rs` line 314-371
- [ ] 实现 `sys_semget()` - 创建/获取信号量集合
- [ ] 实现 `sys_semop()` - 执行信号量操作
- [ ] 实现 `sys_semctl()` - 控制信号量
- [ ] 处理原子操作和死锁检测
- [ ] 添加集成测试

**验收标准**:
```bash
# IPC 测试通过
cargo test --lib ipc::tests 2>&1 | grep -c "test result: ok"  # > 0

# TODO 标记减少
grep "TODO.*ipc" kernel/src/syscalls/ipc/ | wc -l  # 应该 < 5
```

---

### Phase 3.2：完成网络系统调用

#### 任务 3.2.1：完成 Socket 选项处理
- **文件**: `kernel/src/syscalls/network/options.rs` line 62-68
- [ ] 实现 `sys_getsockname()`
- [ ] 实现 `sys_getpeername()`
- [ ] 实现完整的 `sys_getsockopt()`/`sys_setsockopt()`

#### 任务 3.2.2：完成数据传输系统调用
- **文件**: `kernel/src/syscalls/network/data.rs` line 246-252
- [ ] 实现 `sys_sendmsg()`
- [ ] 实现 `sys_recvmsg()`
- [ ] 实现 `sys_sendto()`/`sys_recvfrom()` 的完整版本

#### 任务 3.2.3：实现高级网络特性
- [ ] 实现 `sys_socketpair()` (line 658)
- [ ] 实现 `sys_poll()` (line 649 epoll 相关)
- [ ] 实现 TCP keepalive 和时间戳选项

**验收标准**:
```bash
# 网络测试通过
cargo test --lib net::tests 2>&1 | grep "test result: ok"

# TODO 标记减少
grep "TODO.*network\|socket" kernel/src/syscalls/network/ | wc -l  # < 5
```

---

### Phase 3.3：完成内存管理系统调用

#### 任务 3.3.1：完成 mmap 实现
- **文件**: `kernel/src/syscalls/memory.rs` line 252
- [ ] 实现文件支持的映射 (file-backed mmap)
- [ ] 实现 COW (Copy-on-Write) 优化
- [ ] 实现 MADV_DONTNEED 等 madvise 操作

#### 任务 3.3.2：完成页表操作（架构特定）
- **文件**: `kernel/src/mm/vm.rs` 多处
- [ ] 完成 x86_64 页表 walk：
  - [ ] line 551-558: `translate_address()` 完整实现
  - [ ] line 713-981: `unmap_pages()` 实现
- [ ] 完成 aarch64 页表 walk：
  - [ ] line 1053-1104: 物理页跟踪
  - [ ] line 1055-1111: 解映射实现
- [ ] 完成 RISC-V 页表支持

#### 任务 3.3.3：完成内存锁定
- **文件**: `kernel/src/syscalls/mm/handlers.rs` line 545-718
- [ ] 实现 `sys_mlock()`
- [ ] 实现 `sys_munlock()`
- [ ] 实现 `sys_mlockall()`
- [ ] 实现 `sys_munlockall()`
- [ ] 实现 `sys_mincore()`
- [ ] 实现 `sys_mremap()`

**验收标准**:
```bash
# 内存管理测试通过
cargo test --lib mm::tests 2>&1 | grep "test result: ok"

# 架构特定 TODO 清除
grep "TODO.*aarch64\|x86_64\|riscv" kernel/src/mm/ | wc -l  # = 0
```

---

### Phase 3.4：完成文件系统实现

#### 任务 3.4.1：完成 VFS 核心功能
- [ ] 完成 inotify 事件生成（vfs/mod.rs line 545）
- [ ] 实现完整的路径解析和缓存策略
- [ ] 实现文件系统挂载/卸载

#### 任务 3.4.2：完成 ext4 支持
- [ ] 实现日志功能（journal）
- [ ] 实现目录操作（mkdir/rmdir/unlink）
- [ ] 实现文件属性操作（chmod/chown）

#### 任务 3.4.3：完成 procfs/sysfs
- [ ] procfs：进程信息导出
- [ ] sysfs：内核对象导出

**验收标准**:
```bash
# 文件系统测试通过
cargo test --lib vfs::tests 2>&1 | grep "test result: ok"

# 文件系统相关 TODO < 3
grep "TODO.*fs\|vfs\|inode" kernel/src/ | wc -l  # < 10
```

---

### Phase 3.5：完成 POSIX 兼容性

#### 任务 3.5.1：完成 errno 映射
- [ ] 建立完整的 POSIX errno 到 Rust Result 的映射
- [ ] 所有系统调用返回 errno 而不是 KernelError
- [ ] 文件: `kernel/src/reliability/posix_errno.rs`

#### 任务 3.5.2：完成缺失的系统调用
- [ ] `sys_execve()` - 程序执行（process/exec.rs）
- [ ] `sys_wait4()` - 进程等待变体
- [ ] `sys_getrusage()` - 资源使用统计（process.rs line 470）
- [ ] 异步 I/O: `aio_read()`, `aio_write()`, `aio_suspend()`
- [ ] 定时器: `timer_create()`, `timer_settime()`

#### 任务 3.5.3：实现 POSIX 实时扩展
- [ ] 实时信号处理
- [ ] 消息队列（POSIX mqueue）
- [ ] 共享内存（POSIX shm）
- [ ] 信号量（POSIX sem）

**验收标准**:
```bash
# POSIX 兼容性测试通过
cargo test --lib posix_tests 2>&1 | grep "test result: ok"

# 核心系统调用实现完整
grep "TODO.*execve\|waitpid\|getrusage\|aio_\|timer_" kernel/src/ | wc -l  # = 0
```

---

## 第4阶段：生产化与优化（第9-12周）

**目标**: 达到生产级别质量，通过完整测试和性能优化

### Phase 4.1：完整的测试覆盖

#### 任务 4.1.1：创建集成测试矩阵
- [ ] 创建 `kernel/tests/integration/` 目录
- [ ] 定义测试组合矩阵：
  ```
  [架构] × [特性] × [系统调用组合]
  = x86_64, aarch64, riscv64
  × default, fast_syscall, zero_copy, batch_syscalls
  × [25个关键系统调用组合]
  ≈ 3 × 4 × 25 = 300 个集成测试
  ```
- [ ] 为每个组合编写至少 1 个集成测试
- [ ] 验收标准: 所有 300+ 测试通过

#### 任务 4.1.2：压力与长期运行测试
- [ ] 创建 `kernel/tests/stress/` 目录
- [ ] 高并发进程创建/销毁测试 (1000+ 进程)
- [ ] 内存分配/释放压力测试 (1GB+ 总分配)
- [ ] 文件 I/O 吞吐量测试
- [ ] 网络连接建立/关闭压力测试
- [ ] 运行 24 小时以上检测内存泄漏

#### 任务 4.1.3：POSIX 兼容性验证
- [ ] 针对 Linux 对照比较测试
- [ ] 验证 30+ 个关键系统调用的行为一致性
- [ ] 测试文件: `kernel/tests/posix_compliance.rs`

**验收标准**:
```bash
# 集成测试覆盖
cargo test --test '*' 2>&1 | grep "test result: ok"  # 所有集成测试通过

# 代码覆盖率（使用 tarpaulin）
cargo tarpaulin --out Html  # 预期 >= 70% 覆盖率
```

---

### Phase 4.2：性能基准测试与优化

#### 任务 4.2.1：建立性能基准
- [ ] 运行 `kernel/benches/` 中所有基准
- [ ] 记录基准值：
  - [ ] 系统调用延迟（getpid、open、read）
  - [ ] 进程创建时间（fork）
  - [ ] 内存分配速度（各大小）
  - [ ] 文件 I/O 吞吐量
  - [ ] 网络包处理速度
- [ ] 保存基准数据到 `docs/performance_baseline.json`

#### 任务 4.2.2：性能优化
- [ ] 零拷贝 I/O：验证和优化 `zero_copy.rs`
- [ ] 系统调用缓存：实现频繁调用缓存
- [ ] 批处理支持：`batch.rs` 集成到分发器
- [ ] 调度器优化：评估 `sched/` 中的 O(1) 实现

#### 任务 4.2.3：性能回归测试（CI）
- [ ] 在 GitHub Actions 中集成基准测试
- [ ] 每次提交后运行基准，对比基线
- [ ] 设置告警：如果性能下降 >10%，CI 失败

**验收标准**:
```bash
# 性能基准生成
cat docs/performance_baseline.json | jq '.syscalls.getpid.latency_ns'
# 预期: < 500ns (在现代 CPU 上)

# 没有性能回归
cargo bench 2>&1 | grep "regressed"  # 应该为空
```

---

### Phase 4.3：安全加固

#### 任务 4.3.1：启用所有安全特性
- [ ] ASLR：验证地址随机化
- [ ] SMAP/SMEP：验证内存保护
- [ ] DEP/NX：验证执行保护
- [ ] 栈金丝雀：编译时启用
- [ ] 整数溢出检查：debug 配置启用

#### 任务 4.3.2：Fuzzing 测试
- [ ] 启用 `kernel/src/fuzz_testing.rs`
- [ ] 为系统调用实现 libFuzzer 目标
- [ ] 运行 fuzzing 至少 100 小时，无 panic

#### 任务 4.3.3：安全审计
- [ ] 使用 `cargo-audit` 检查依赖漏洞
- [ ] 代码审查检查表（unsafe 块、边界检查、错误处理）
- [ ] 形式化验证关键路径（可选，P3）

**验收标准**:
```bash
# 无依赖漏洞
cargo audit 2>&1 | grep "vulnerabilities"  # 应该为 0

# Fuzzing 无 panic
cargo fuzz fuzz_syscalls -- -max_len=1024 -timeout=1 -runs=1000000
# 预期：全部通过，无 panic
```

---

### Phase 4.4：生产就绪清单

#### 任务 4.4.1：文档完善
- [ ] 生成 rustdoc：`cargo doc --open`
- [ ] 验证所有 pub 函数都有文档
- [ ] 创建 `docs/PRODUCTION_READINESS.md` 检查清单
- [ ] 编写系统调用 API 参考（与 Linux 对照）
- [ ] 编写驱动开发指南

#### 任务 4.4.2：发布准备
- [ ] 更新 `Cargo.toml` 版本号为 `0.1.0-alpha.1`
- [ ] 创建 CHANGELOG.md
- [ ] 创建 CONTRIBUTING.md 和代码风格指南
- [ ] 标记首个 git tag: `v0.1.0-alpha.1`

#### 任务 4.4.3：部署验证
- [ ] 在物理硬件上（或高保真模拟器）启动内核
- [ ] 验证基本操作（进程创建、文件 I/O、网络）
- [ ] 运行 24 小时以上无崩溃

**验收标准**:
```bash
# 文档覆盖率 100%
cargo doc --lib 2>&1 | grep "warning: missing"  # 应该为 0

# 首个版本发布
git tag -l | grep "v0.1.0"  # 存在此标签
```

---

## 交叉项目活动（全程进行）

### 持续集成与质量保证

#### 任务：建立 GitHub Actions CI/CD 流程
- [ ] 每次 push 运行:
  - [ ] `cargo check --lib` (5min)
  - [ ] `cargo test --lib` (10min)
  - [ ] 代码覆盖率检查 (5min)
  - [ ] Clippy lint 检查 (3min)
  - [ ] 文档生成 (2min)
- [ ] 允许失败的步骤（可选，用于实验）

#### 任务：代码审查流程
- [ ] PR 必须通过 CI
- [ ] 至少 1 个审查者 Approve
- [ ] 要求签名提交

#### 任务：变更日志维护
- [ ] 每个 PR 必须更新 CHANGELOG.md
- [ ] 按类别组织：Features、Fixes、Refactoring、Docs

---

## 详细 Todo 列表

### 第 1 周 (第 1-2 阶段的前 5 天)

#### Day 1-2: 编译错误修复
- [ ] Task 1.1.1: 修复 optimization_service.rs
  - [ ] 移除 service_type() 方法
  - [ ] 移除 restart() 方法
  - [ ] 移除 health_check() 方法
  - [ ] 修正方法名 get_supported_syscalls → supported_syscalls
  - [ ] `cargo check --lib` 验证

- [ ] Task 1.1.2: 修复 Service Registry
  - [ ] 审查 services/traits.rs
  - [ ] 审查 services/registry.rs
  - [ ] 修复 impl Service 块

- [ ] Task 1.1.3: 禁用或完成服务
  - [ ] 选择处理方案：禁用 vs 完成
  - [ ] 更新 Cargo.toml feature flags

#### Day 3-4: 清理临时代码
- [ ] Task 1.2.1: 移动工具脚本到 tools/
  - [ ] 创建 tools/cli/, tools/services/, tools/tests/
  - [ ] 移动 optimization_cli.rs
  - [ ] 移动 optimization_service.rs
  - [ ] 移动 optimization_tests.rs
  - [ ] 删除 .md 文档（移到 docs/）

- [ ] Task 1.2.2: 隔离测试文件
  - [ ] enhanced_tests.rs 处理（重新集成或删除）
  - [ ] 验证所有测试在 lib.rs 中注册

- [ ] Task 1.2.3: 清理备份
  - [ ] 删除 fs.rs.bak

#### Day 5: 分配器统一
- [ ] Task 1.3.1-1.3.4: 内存分配器整合
  - [ ] 确认 optimized_buddy/slab 更优
  - [ ] 删除基础版本（buddy.rs, slab.rs）
  - [ ] 重命名优化版本为基础名
  - [ ] 创建 MemoryAllocator trait
  - [ ] 实现两个 allocator 的 trait
  - [ ] 删除 copy_optimized.rs
  - [ ] cargo check 验证

**周末检查点**：
```bash
cargo check --lib  # 0 errors
find kernel/src -name "*_optimized.rs" | wc -l  # <= 5 (其他模块)
```

---

### 第 2 周 (继续第 1-2 阶段)

#### Day 6: 错误处理统一
- [ ] Task 1.4.1-1.4.3: 建立 KernelError
  - [ ] 创建统一 KernelError enum
  - [ ] 实现 From 转换
  - [ ] 创建 errno_mapping.rs
  - [ ] 建立完整 errno 映射表

#### Day 7-8: 系统调用分发器重构 (Phase 2.1)
- [ ] Task 2.1.1: 创建动态分发器
  - [ ] 创建 dispatcher.rs
  - [ ] 实现 SyscallDispatcher 结构体
  - [ ] 实现 register() 和 dispatch() 方法

- [ ] Task 2.1.2: 消除硬编码导入
  - [ ] 重构 mod.rs：移除 287 个 use 导入
  - [ ] 使用 dispatcher.register() 替换
  - [ ] 创建 module-level register_syscalls() 函数

- [ ] Task 2.1.3: 模块化注册接口
  - [ ] process::register_syscalls()
  - [ ] fs::register_syscalls()
  - [ ] mm::register_syscalls()
  - [ ] ipc::register_syscalls()
  - [ ] net::register_syscalls()

#### Day 9-10: 系统调用合并 (Phase 2.2)
- [ ] Task 2.2.1: 合并文件 I/O
  - [ ] 创建 file_io/ 子目录
  - [ ] 合并 file_io.rs + file_io_optimized.rs
  - [ ] 删除旧文件
  - [ ] 验证测试通过

- [ ] Task 2.2.2-2.2.6: 合并其他系统调用
  - [ ] process.rs + process_optimized.rs
  - [ ] memory.rs + memory_optimized.rs
  - [ ] signal.rs + signal_advanced.rs + signal_optimized.rs
  - [ ] network.rs + network_optimized.rs
  - [ ] zero_copy.rs + zero_copy_optimized.rs

**周末检查点**：
```bash
cargo build --lib  # 0 errors
ls kernel/src/syscalls/*_optimized.rs | wc -l  # 0 (已删除)
cargo test --lib  # 所有单元测试通过
```

---

### 第 3-4 周 (第 2 阶段后半段及 Phase 2.3)

#### Day 11: Service 规范化 (Phase 2.3)
- [ ] Task 2.3.1: Service Trait 统一
  - [ ] 审查 services/traits.rs
  - [ ] 确认方法签名一致
  - [ ] 添加完整文档

- [ ] Task 2.3.2: ServiceRegistry 完成
  - [ ] 实现动态注册/注销
  - [ ] 实现依赖解析
  - [ ] 实现生命周期管理

- [ ] Task 2.3.3: 占位符服务处理
  - [ ] 决定：完成 vs 禁用
  - [ ] 如果完成：实现所有处理器
  - [ ] 如果禁用：注释掉初始化代码

**第 2 阶段完成验收**：
```bash
cargo build --lib --release  # 0 errors, 成功编译
cargo test --lib 2>&1 | tail -1  # "test result: ok"
grep "use crate::" kernel/src/syscalls/mod.rs | wc -l  # < 10
```

---

### 第 5-8 周 (第 3 阶段：功能补全)

#### Week 5 (Day 29-33): IPC 实现
- [ ] Day 29: 管道实现
  - [ ] 实现 sys_pipe()
  - [ ] 创建 Pipe 结构体
  - [ ] 实现读写操作
  - [ ] 添加单元测试

- [ ] Day 30: 消息队列实现
  - [ ] msgget/msgsnd/msgrcv/msgctl
  - [ ] 消息优先级支持
  - [ ] 超时处理

- [ ] Day 31: 共享内存实现
  - [ ] shmget/shmat/shmdt/shmctl
  - [ ] 缓存一致性
  - [ ] 集成测试

- [ ] Day 32-33: 信号量实现
  - [ ] semget/semop/semctl
  - [ ] 原子操作
  - [ ] 死锁检测

#### Week 6 (Day 34-38): 网络与内存系统调用
- [ ] Day 34: 网络系统调用完成
  - [ ] getsockname/getpeername
  - [ ] sendmsg/recvmsg
  - [ ] socketpair
  - [ ] poll 改进

- [ ] Day 35-37: 内存管理系统调用
  - [ ] mmap 文件支持
  - [ ] 页表 walk (x86_64/aarch64)
  - [ ] mlock/munlock/mlockall/mincore/mremap

- [ ] Day 38: 文件系统完成
  - [ ] VFS inotify
  - [ ] ext4 日志
  - [ ] procfs/sysfs

#### Week 7-8 (Day 39-56): POSIX 兼容与测试基础
- [ ] Day 39-42: POSIX syscall 完成
  - [ ] execve 完整实现
  - [ ] wait4, getrusage
  - [ ] aio_* 系列
  - [ ] timer_*

- [ ] Day 43-56: 测试框架建立
  - [ ] 创建 tests/integration/
  - [ ] 创建 25+ 集成测试
  - [ ] POSIX 兼容性对标
  - [ ] 压力测试

**第 3 阶段完成验收**：
```bash
cargo test --lib 2>&1 | grep -c "test result: ok"  # >= 200
grep "^[[:space:]]*\[" kernel/tests/integration/*.rs | wc -l  # >= 25
grep "TODO" kernel/src/syscalls/ | wc -l  # < 20 (从 150+ 减少)
```

---

### 第 9-12 周 (第 4 阶段：生产化)

#### Week 9 (Day 57-61): 集成与压力测试
- [ ] Day 57-59: 完整集成测试矩阵
  - [ ] 3 架构 × 4 特性 × 25 组合
  - [ ] 300+ 集成测试全部通过

- [ ] Day 60-61: 压力与长期运行测试
  - [ ] 高并发进程创建 (1000+ 进程)
  - [ ] 内存压力 (1GB+ 分配)
  - [ ] 文件 I/O 吞吐量
  - [ ] 网络连接建立

#### Week 10 (Day 62-66): 性能基准与优化
- [ ] Day 62: 基准测试建立
  - [ ] 运行所有 benches/
  - [ ] 记录基准值
  - [ ] 生成 performance_baseline.json

- [ ] Day 63-65: 性能优化
  - [ ] 零拷贝 I/O 优化
  - [ ] 系统调用缓存
  - [ ] 批处理集成
  - [ ] 调度器优化验证

- [ ] Day 66: 性能回归测试设置
  - [ ] GitHub Actions 集成
  - [ ] 基准对比和告警

#### Week 11 (Day 67-71): 安全加固
- [ ] Day 67: 安全特性启用
  - [ ] ASLR 验证
  - [ ] SMAP/SMEP 启用
  - [ ] DEP/NX 验证
  - [ ] 栈金丝雀启用

- [ ] Day 68-70: Fuzzing 与审计
  - [ ] libFuzzer 目标实现
  - [ ] 100 小时 fuzzing 运行
  - [ ] cargo-audit 检查
  - [ ] 代码审查检查表

- [ ] Day 71: 安全文档
  - [ ] 安全特性文档
  - [ ] 已知漏洞列表

#### Week 12 (Day 72-76): 最终发布准备
- [ ] Day 72: 文档完善
  - [ ] rustdoc 100% 覆盖
  - [ ] 系统调用 API 参考
  - [ ] 驱动开发指南
  - [ ] PRODUCTION_READINESS.md

- [ ] Day 73: 发布准备
  - [ ] 更新版本号到 0.1.0-alpha.1
  - [ ] 创建 CHANGELOG.md
  - [ ] 创建 CONTRIBUTING.md
  - [ ] git tag v0.1.0-alpha.1

- [ ] Day 74-76: 部署验证
  - [ ] 物理硬件/模拟器启动
  - [ ] 基本操作验证
  - [ ] 24 小时无崩溃运行

**最终验收**：
```bash
cargo build --release  # 成功，无 warnings
cargo test --all  # 所有测试通过
cargo doc --open  # 100% 文档覆盖
git tag -l | grep "v0.1.0-alpha.1"  # 标签存在
```

---

## 资源需求

### 人力配置（5-8 人月）

| 角色 | 人数 | 投入 | 职责 |
|------|------|------|------|
| 内核架构师 | 1 | 全程 | 架构决策、解耦方案、性能优化 |
| 系统编程工程师 | 2-3 | 全程 | 系统调用实现、内存管理、驱动 |
| 测试工程师 | 1 | Week 5+ | 测试框架、CI/CD、性能基准 |
| 安全工程师 | 1 | Week 9+ | 安全加固、fuzzing、审计 |
| 文档编写 | 0.5 | Week 12 | API 文档、用户指南 |

### 时间表摘要

| 阶段 | 周数 | 目标 | 风险 |
|------|------|------|------|
| Phase 1: 稳定化 | 2 | 编译无误、代码清理 | 低 |
| Phase 2: 解耦 | 2 | 架构重构、消除耦合 | 中 |
| Phase 3: 补全 | 4 | 功能完成、测试覆盖 | 高 (IPC、POSIX) |
| Phase 4: 生产化 | 4 | 性能优化、安全加固、发布 | 中 |
| **总计** | **12** | **生产就绪 alpha.1** | **可控** |

### 依赖与风险

**风险 1: IPC 实现复杂性**
- 概率: 中
- 影响: 高
- 缓解: 从简单功能(pipe)开始，逐步升级(msgqueue/shm/sem)

**风险 2: 架构特定代码（页表 walk）**
- 概率: 高
- 影响: 中
- 缓解: 优先完成 x86_64，其他架构 fallback 到基础实现

**风险 3: 性能基准设定过高**
- 概率: 低
- 影响: 中
- 缓解: 基准基于当前代码，逐步优化

---

## 成功指标

### 第 1 阶段完成 (Week 2)

```
✓ cargo check --lib → 0 errors
✓ 临时代码清理完毕
✓ 内存分配器统一 (1 个 buddy, 1 个 slab)
✓ 错误处理统一 (单一 KernelError)
```

### 第 2 阶段完成 (Week 4)

```
✓ cargo build --lib → 成功
✓ syscalls/mod.rs 硬编码导入 < 10 个
✓ *_optimized.rs 文件 = 0 个
✓ Service trait 实现一致
✓ cargo test --lib → 所有单元测试通过
```

### 第 3 阶段完成 (Week 8)

```
✓ IPC 实现完毕 (pipe, msgqueue, shm, sem)
✓ 网络和内存系统调用完整
✓ POSIX syscall 集合完整
✓ 25+ 集成测试
✓ 代码覆盖率 >= 50%
✓ TODO 标记 < 20 个
```

### 第 4 阶段完成 (Week 12)

```
✓ 300+ 集成测试全部通过
✓ 压力测试 24h+ 无崩溃
✓ 性能基准建立并回归检查
✓ Fuzzing 100h+ 无 panic
✓ 安全审计完成
✓ 100% 文档覆盖
✓ git tag v0.1.0-alpha.1
✓ 物理硬件/模拟器可启动
```

---

## 后续维护与长期规划

### 短期 (Month 2-3)

- [ ] Beta 版本发布（v0.1.0-beta.1）
- [ ] 社区反馈收集与快速迭代
- [ ] 容器运行时集成验证

### 中期 (Month 4-6)

- [ ] 第一个稳定版本（v0.1.0）
- [ ] Kubernetes 适配器开发
- [ ] eBPF 支持启用

### 长期 (Month 7-12)

- [ ] 微服务化继续深化
- [ ] 形式化验证关键路径
- [ ] 生态工具链完善

---

**文档更新日期**: 2025-12-09
**状态**: 待执行
**优先级**: ⚠️ **关键** - 项目当前无法编译，立即启动 Phase 1
