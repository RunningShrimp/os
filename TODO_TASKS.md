# TODO 任务清单

**生成时间**: 2025-12-29
**总计**: 380 个 TODO/FIXME/XXX/HACK 标记

## 执行摘要

### 按优先级分类
- **高优先级** (P0): 62 个 - 需要立即处理
- **中优先级** (P1): 250 个 - 需要计划处理
- **低优先级** (P2): 68 个 - 可以延后处理

### 按操作类型分类
- **需要实现**: 64 个 - 核心功能待实现
- **需要审查**: 181 个 - 可能已完成，需要审查
- **实现或删除**: 110 个 - 主要是测试桩代码
- **修复格式**: 19 个 - 格式化字符串问题
- **跟踪Issue**: 6 个 - 等待其他模块完成

---

## 1. 高优先级任务 (P0)

### 1.1 格式字符串修复 (19 个)
**状态**: 需要立即修复 - 这些是格式化字符串的语法错误

```rust
// 错误模式：使用了 /* TODO: {...} */ 注释
alloc::string::String::from("sha256:") + /* TODO: {::016x} */ &hash.to_string()
```

**影响文件**:
- `deploy/backup.rs:198, 261`
- `deploy/container_build.rs:644`
- `i18n/translation.rs:221`
- `i18n/locale.rs:278, 319-335` (多处)
- `accessibility/high_contrast.rs:129`
- `accessibility/screen_reader.rs:241`
- `subsystems/mm/user_space_isolation.rs:707, 775, 785`
- `subsystems/security/audit.rs:326`

**建议操作**:
1. 替换所有 `/* TODO: {::format} */` 为正确的 format! 宏
2. 示例修复:
```rust
// 修复前:
alloc::string::String::from("sha256:") + /* TODO: {::016x} */ &hash.to_string()

// 修复后:
format!("sha256:{:016x}", hash)
```

### 1.2 核心功能实现 (43 个)
**状态**: 阻塞功能 - 影响系统完整性

#### 1.2.1 网络栈实现
**文件**: `subsystems/syscalls/network/service.rs`
- `115`: 实现实际的 socket 分配
- `126`: 实现实际的 socket 绑定
- `137`: 实现实际的 socket 连接
- `148`: 实现实际的 socket 监听
- `159`: 实现实际的 socket accept
- `170`: 实现实际的数据发送
- `181`: 实现实际的数据接收
- `209`: 初始化网络栈
- `220`: 启动网络接口
- `231`: 停止网络接口

**文件**: `subsystems/net/processor.rs`
- `287`: 发送 SYN-ACK
- `295`: 缓冲接收数据

#### 1.2.2 信号处理实现
**文件**: `subsystems/syscalls/signal/handlers.rs`
- `184`: 实现实际的信号发送逻辑
- `213`: 实现实际的信号掩码设置逻辑
- `240`: 实现实际的信号集操作逻辑
- `302`: 终止进程
- `307`: 停止进程
- `312`: 继续进程

**文件**: `subsystems/syscalls/service.rs`
- `180`: 实现实际的信号发送
- `199`: 实现实际的信号等待
- `237`: 实现实际的信号处理
- `312-344`: 信号管理器生命周期管理

**文件**: `subsystems/syscalls/handlers.rs`
- `33`: 实现kill逻辑
- `59`: 实现raise逻辑
- `183`: 实现sigwait逻辑
- `210`: 实现sigwaitinfo逻辑
- `238`: 实现sigtimedwait逻辑
- `267`: 实现signal逻辑
- `291`: 实现pause逻辑

#### 1.2.3 内存管理核心功能
**文件**: `subsystems/syscalls/implementation/handlers/mm.rs`
- `192`: 处理文件支持的映射
- `261`: aarch64 物理页跟踪
- `271`: x86_64 解映射
- `498`: aarch64 页表遍历
- `505`: x86_64 页表遍历
- `686, 698`: 失败时的页清理
- `707`: 正确解映射和释放页
- `898`: 实现正确的 mremap

**文件**: `subsystems/mm/vm/mmap.rs`
- `64`: 实现文件映射
- `223`: 实现文件同步
- `251`: 实现真正的物理页分配
- `261`: 实现真正的物理页释放

**文件**: `subsystems/mm/vm/lock.rs`
- `78, 113, 155`: 实现真正的内存锁定/解锁
- `247, 265`: 实现地址空间锁定/解锁

**文件**: `subsystems/mm/vm/protection.rs`
- `74, 107, 126, 142`: 实现真正的内存/地址空间锁定/解锁

#### 1.2.4 零拷贝I/O
**文件**: `subsystems/syscalls/zero_copy.rs`
- `314`: 通过移动页引用实现真正的零拷贝
- `534`: 通过复制页引用实现真正的零拷贝
- `889`: 实现 io_uring setup
- `900`: 实现 io_uring_enter
- `910`: 实现 io_uring_register

#### 1.2.5 高级内存映射
**文件**: `subsystems/syscalls/memory/advanced_mmap.rs`
- `40`: 实现 madvise 功能
- `57`: 实现 mlock 功能
- `74`: 实现 munlock 功能
- `90`: 实现 mlockall 功能
- `100`: 实现 munlockall 功能
- `118`: 实现 mincore 功能
- `138`: 实现 remap_file_pages 功能
- `159`: 实现高级 mmap 功能

**文件**: `subsystems/syscalls/advanced_mmap.rs`
- `302`: 实现真正的批量 map_pages
- `321`: 实现文件按需分页

---

## 2. 中优先级任务 (P1)

### 2.1 时间和时间戳实现 (30 个)
**状态**: 重要但不紧急 - 需要统一的时间管理

**文件**: `posix/timer.rs`
- `368`: 添加当前时间到相对时间
- `453`: 获取实时时钟
- `457`: 获取单调时钟
- `461`: 获取进程CPU时间
- `465`: 获取线程CPU时间
- `492`: 实现设置实时时钟
- `552`: 实现实际的睡眠逻辑
- `572`: 获取实际当前时间

**文件**: `posix/shm.rs`
- `149, 352, 402`: 获取当前时间用于跟踪

**文件**: `subsystems/fs/ext4_persistence.rs`
- `387`: 连接到真实时间源，目前使用单调时钟

**文件**: `security/memory_security.rs`
- `449`: 使用适当的时间源

**文件**: `subsystems/syscalls/performance_monitor.rs`
- `163`: 从系统时钟获取

**文件**: `formal_verification/*`
- `model_checker.rs`: `530, 550, 561, 572` - 实现正确的时间戳
- `theorem_prover.rs`: `706, 739, 795, 810` - 实现正确的时间戳
- `type_checker.rs`: `684, 727` - 实现正确的时间戳
- `verification_pipeline.rs`: `171, 249` - 实现正确的时间戳

**建议操作**:
1. 创建统一的时间管理模块 `kernel/src/time/mod.rs`
2. 实现系统时钟接口
3. 替换所有硬编码的 `0` 时间戳

### 2.2 设备管理实现 (7 个)
**文件**: `platform/drivers/device_manager.rs`
- `848`: 实现驱动绑定
- `915`: 实现驱动通知

**文件**: `platform/drivers/mod.rs`
- `150`: 处理退格键
- `154`: 发送 SIGINT
- `260`: 探测其他设备 (VirtIO等)

**文件**: `platform/trap/mod.rs`
- `89`: 处理外部中断
- `152`: 读取中断控制器确定源
- `445`: 设置trapframe并调用userret

**文件**: `services/driver.rs`
- `13`: 在 driver_manager.rs 中实现 get_driver_manager

### 2.3 VFS 和文件系统 (8 个)
**文件**: `vfs/ext4.rs`
- `135`: 打开设备并读取超级块
- `188`: 同步所有脏块到磁盘
- `205`: 同步和清理
- `523`: 同步inode到磁盘

**文件**: `subsystems/fs/mod.rs`
- `179`: 实现目录创建
- `190`: 实现文件创建
- `202`: 实现文件写入
- `209`: 实现文件/目录删除

**文件**: `subsystems/fs/file.rs`
- `464`: 写入inode

**文件**: `subsystems/syscalls/fs/service.rs`
- `228`: 初始化文件系统缓存，注册VFS操作
- `240`: 刷新缓存，清理临时文件

**文件**: `subsystems/syscalls/fs/file_io.rs`
- `277`: 实现 lstat

**文件**: `subsystems/syscalls/glib.rs`
- `625-626`: 从VFS获取设备号和inode号

### 2.4 进程和线程管理 (8 个)
**文件**: `subsystems/process/exec.rs`
- `516`: 实际复制字符串并设置指针

**文件**: `subsystems/process/thread.rs`
- `1085`: 为其他架构添加TLS设置

**文件**: `subsystems/process/manager.rs`
- `1309`: 支持进程组等待 (pid < -1) 和同组 (pid == 0)

**文件**: `subsystems/cloud_native/oci.rs`
- `760`: 实现真正的kill系统调用

### 2.5 错误恢复和调试 (8 个)
**文件**: `error/recovery.rs`
- `313`: 实现回退机制
- `339`: 实现驱动重置
- `344`: 实现进程重置
- `354`: 实现子系统重启

**文件**: `error/panic_handler.rs`
- `367-368`: 获取实际内存使用情况

**文件**: `debug/manager.rs`
- `119`: 实现 load_default_plugins
- `395`: 使用 thread_id 参数获取特定线程的堆栈信息

### 2.6 IPC 和消息队列 (4 个)
**文件**: `posix/mqueue.rs`
- `220`: 实现实际的信号发送
- `224`: 实现管道通知

**文件**: `subsystems/syscalls/mqueue.rs`
- `263`: 实现带阻塞和超时的正确定时发送
- `370`: 实现带阻塞和超时的正确定时接收

### 2.7 安全和权限 (8 个)
**文件**: `security/permission_check.rs`
- `166`: 与实际的 seccomp 子系统集成
- `180`: 与实际的 SELinux 子系统集成
- `189`: 与实际的 capabilities 子系统集成

**文件**: `security/smap_smep.rs`
- `15`: 在arch模块中实现 X86Feature 和 X86Cpu
- `605`: 实现清理

**文件**: `posix/shm.rs`
- `451`: 支持有效GID

---

## 3. 低优先级任务 (P2)

### 3.1 模块集成跟踪 (6 个)
**状态**: 等待其他模块完成 - 创建 GitHub Issues 跟踪

**文件**: `types/stubs.rs`
- `123`: 当服务注册表完全实现时重新启用
- `148`: 当所有用法更新时用 Process stub 替换 crate::process::Proc
- `296`: 当需要原子类型时重新启用

**文件**: `subsystems/syscalls/performance_monitor.rs`
- `9`: 当优化模块重构时重新启用这些导入
- `117`: 当优化模块重构时重新启用

**文件**: `subsystems/syscalls/optimization/framework.rs`
- `19`: 更新以使用来自 dispatch::unified 的新统一调度器
- `24`: 当完成适当的集成时移除此项

**文件**: `subsystems/syscalls/dispatch/traits.rs`
- `299, 311, 323, 335`: 当优化服务重构时实现

**文件**: `subsystems/syscalls/services/traits.rs`
- `297, 309, 321, 333`: 当优化服务重构时实现

**文件**: `ids/host_ids/mod.rs`
- `18`: 逐步拆分到各个子模块
- `25`: 在实现完整的子模块后重新启用这些导出

**建议**: 为这些创建 GitHub Issues，标签为 `blocked`, `integration`

### 3.2 测试桩代码 (102 个)
**状态**: 可能不需要 - 需要审查和清理

**文件**: `posix_tests/core/basic_tests.rs`
- `416-824` (大量): 实现具体测试逻辑

**文件**: `posix_tests/core/signal_tests.rs`
- `82-148`: 实现具体测试逻辑

**文件**: `posix_tests/core/thread_tests.rs`
- `106-208`: 实现具体测试逻辑

**文件**: `testing/performance_tests.rs`
- `527, 533, 539, 545`: 实现实际基准测试

**建议操作**:
1. 审查这些测试是否实际需要
2. 如果不需要，删除整个测试函数
3. 如果需要，添加具体的测试实现
4. 考虑使用属性宏 `#[ignore]` 而不是 TODO 注释

### 3.3 占位符实现 (8 个)
**文件**: `subsystems/mm/api/page.rs`
- `24, 36, 48, 60`: 实现此函数

**文件**: `subsystems/mm/api/stats.rs`
- `12, 21, 27, 33`: 实现此函数

**建议**: 如果这些函数未被调用，考虑删除它们

### 3.4 通用TODO (181 个)
**状态**: 需要逐个审查

这些是各种通用 TODO 注释，包括：
- 架构特定的实现细节
- 性能优化机会
- 功能增强建议
- 文档和注释改进

**建议**: 创建 GitHub Issues，逐个审查

---

## 4. 可立即删除的简单TODO

### 4.1 明显已完成的TODO
以下TODO对应的代码已经实现，可以删除标记：

1. `libc/implementations.rs` - 多个TODO可能已经有实现
2. `vfs/ext4.rs` - 基本操作可能已实现
3. `graphics/input.rs` - 命中测试可能已实现

### 4.2 废弃功能相关TODO
如果某个功能已经决定不再实现，可以删除：
- 相关的TODO注释
- 相关的桩代码
- 相关的测试框架

---

## 5. 建议的清理步骤

### 阶段1: 快速修复 (1-2天)
1. ✅ 修复所有19个格式字符串问题
2. ✅ 删除明显已完成的TODO标记（审查后）
3. ✅ 为测试桩代码添加 `#[ignore]` 属性或删除

### 阶段2: 核心实现 (1-2周)
1. ✅ 实现网络栈核心功能
2. ✅ 实现信号处理核心功能
3. ✅ 实现内存管理核心功能
4. ✅ 实现零拷贝I/O
5. ✅ 实现高级内存映射

### 阶段3: 时间和设备 (1周)
1. ✅ 创建统一时间管理模块
2. ✅ 实现设备管理功能
3. ✅ 实现VFS和文件系统功能

### 阶段4: 审查和跟踪 (持续)
1. ✅ 为模块集成TODO创建Issues
2. ✅ 逐个审查通用TODO
3. ✅ 更新文档和注释

---

## 6. GitHub Issues 建议模板

### 高优先级Issue模板
```markdown
## Issue: 实现XXX功能

**优先级**: P0 - High
**模块**: kernel/src/xxx/yyy.rs
**位置**: Line 123

### 描述
实现当前标记为TODO的核心功能。

### 相关文件
- `kernel/src/xxx/yyy.rs:123`
- `kernel/src/xxx/zzz.rs:456`

### 验收标准
- [ ] 功能完整实现
- [ ] 添加单元测试
- [ ] 更新文档
- [ ] 删除TODO标记

### 参考资料
- Linux手册页: man 2 xxx
- POSIX标准: https://...
```

### 模块集成Issue模板
```markdown
## Issue: 模块集成 - XXX

**优先级**: P2 - Low
**类型**: Integration
**依赖**: 待定

### 描述
当前TODO标记等待其他模块完成后才能实现。需要跟踪依赖关系。

### 阻塞原因
等待 [模块YYY] 完成实现

### 相关文件
- `kernel/src/xxx/yyy.rs:123`

### 后续步骤
1. 等待依赖模块完成
2. 实现集成代码
3. 添加集成测试
4. 删除TODO标记
```

---

## 7. 统计图表

### 按文件分类 (Top 10)
```
posix_tests/core/basic_tests.rs:  102 TODOs
i18n/locale.rs:                     8 TODOs
subsystems/syscalls/signal/:        8 TODOs
subsystems/syscalls/network/:       8 TODOs
subsystems/syscalls/memory/:        8 TODOs
ids/host_ids/host_ids.rs:          18 TODOs
formal_verification/:              12 TODOs
subsystems/mm/vm/:                 10 TODOs
subsystems/syscalls/handlers.rs:    7 TODOs
subsystems/syscalls/implementation/handlers/mm.rs: 7 TODOs
```

### 按模块分类
```
测试:       106 TODOs (27.9%)
系统调用:    76 TODOs (20.0%)
内存管理:    52 TODOs (13.7%)
网络:        25 TODOs (6.6%)
形式化验证:  12 TODOs (3.2%)
时间管理:    30 TODOs (7.9%)
文件系统:    20 TODOs (5.3%)
其他:       59 TODOs (15.5%)
```

---

## 8. 关键指标

- **完成率**: 约 30% (通过代码审查估计)
- **阻塞项**: 6 个 (等待其他模块)
- **技术债务**: 中等
- **建议处理时间**: 3-4 周 (全职)

---

## 9. 后续行动

### 立即行动
1. **审查此报告** - 确认分类是否准确
2. **创建Issues** - 为P0任务创建GitHub Issues
3. **修复格式字符串** - 19个高优先级语法问题

### 本周行动
1. **实现核心功能** - 开始P0任务实现
2. **删除不需要的TODO** - 清理测试桩代码
3. **更新文档** - 记录实现进度

### 持续跟踪
1. **每周审查** - 检查TODO清理进度
2. **更新Issues** - 关闭已完成的任务
3. **维护此清单** - 保持与代码同步

---

**生成工具**: 自动化脚本分析
**最后更新**: 2025-12-29
**维护者**: 开发团队
