# TODO 审查 — 优先级 / 风险 / 前置条件评估

生成时间: 2025-12-08

说明: 本文基于对项目源码扫描产生的 TODO/FIXME/STUB 扫描结果（运行时扫描产生的 /docs/STUB_REPORT.md），独立分析代码中的待办项、临时实现、技术债务与依赖关系；不直接基于现有设计文档或已有 TODO 处置方案。

---

## 一、关键发现摘要（要点）
- 扫描统计: 共发现 327 处 TODO/FIXME/STUB 标记，分布集中在若干核心模块（见下）。
- 热点模块（按标记数量排序）:
  - `kernel/src/syscalls/process.rs` — 34 处（核心进程/系统调用）
  - `kernel/src/syscalls/thread.rs` — 22 处（线程/clone/futex 等）
  - `kernel/src/ids/host_ids/host_ids.rs` — 20 处（IDS 行为分析）
  - `kernel/src/syscalls/memory.rs` — 19 处（内存 / mm）
  - `kernel/src/types/stubs.rs` — 15 处（大量类型/接口 stub）
  - `kernel/src/syscalls/signal.rs`、`posix/timer.rs`、`vfs/ext4.rs`、`fs/fs_impl.rs` 等也表现出较高密度。

- 主要问题类型(分层):
  1. 核心功能缺失或占位实现（Process/Thread、VFS、FS、Memory/advanced_mmap） — 影响系统正确性、兼容性与稳定性。
  2. 安全/权限相关 stub（permission_check、smap_smep、aslr）— 安全边界存在明显风险。
  3. 性能/扩展（zero_copy、io_uring、advanced mmap）— 当前使用临时/缓冲实现，约束吞吐和扩展性。
  4. 驱动/平台空缺（VirtIO、device binding）— 限制平台功能和 I/O 子系统能力。
  5. 类型 stub（types/stubs.rs）广泛存在：替换为真实类型将是一次高影响、跨模块的 refactor。

---

## 二、优先级分配原则（用于后续计划）
优先级分为 Critical / High / Medium / Low，评估维度：
- 影响范围 (影响多少模块/用户场景)
- 阻塞发布 (是否阻塞一个稳定可用的 release)
- 安全/崩溃风险 (是否会引入安全漏洞或数据损坏)
- 迁移成本 (实现替换需要的涉及面/风险)

---

## 三、按模块的优先级、依赖与风险（摘要）
> 下表为高优先级模块的结论性评估（示例）。完整分解会包含每个 TODO 行的逐条评估（可按需生成 CSV/清单）。

1) Core process / thread syscalls (`syscalls/process.rs`, `syscalls/thread.rs`)  — Priority: Critical
- 核心理由: 影响进程创建、调度、执行与退出，是系统正确性的基础。
- 典型 TODO: fork/execve 边界检查、sched_* 系列、waitpid/wait4、CLONE 参数边界支持。
- 关键依赖: `process::manager`、`proc_table`、vfs（execve 文件读取）、memory/pagetable。
- 风险: 未正确实现导致进程泄漏、资源没释放、崩溃或权限绕过。
- 前置条件: 稳定单元测试（fork/exec/wait）、API 兼容性定义、CI 回归测试。

2) VFS / FS (`vfs/ext4.rs`, `fs/fs_impl.rs`, `vfs/mod.rs`) — Priority: Critical → High
- 核心理由: 文件系统为 exec、加载、持久化提供基础；inode、目录操作未实现会阻塞用户空间程序。
- 关键依赖: block device drivers、buffer cache、inode layout
- 风险: 数据损坏、fs 一致性问题、启动或程序加载失败
- 前置条件: 低级驱动稳定，持久化测试、文件系统一致性测试（fsck）

3) Memory / Advanced mmap / mm (`syscalls/memory/advanced_mmap.rs`, `mm/vm.rs`) — Priority: High
- 核心理由: 内存管理影响安全、性能与隔离；占位实现（mlock、madvise、mincore）会限制高级优化与性能工具。
- 关键依赖: page allocator、page tables、swap/IO 子系统
- 风险: 内存泄漏、权限泄露、性能退化
- 前置条件: 明确虚拟内存模型与 page pinning API、测试 harness

4) Security (`security/permission_check.rs`, `smap_smep.rs`, `aslr.rs`) — Priority: Critical/High
- 核心理由: 未集成 seccomp/SELinux/capability 支撑会显著扩展安全面板漏洞。
- 风险: 权限绕过、信息泄露、root 提权攻击面
- 前置条件: 定义安全模型、Threat model、CI 安全测试用例、可测的安全策略实现

5) Types / Stubs (`types/stubs.rs`) — Priority: High (tech-debt centre)
- 核心理由: 大量模块依赖于 types/stubs；替换将触达大量代码并有潜在破坏性
- 风险: 一次性改动会破坏多个模块；缺乏迁移测试会导致回归
- 前置条件: 建立 adapter 层、兼容 shim、逐步替换计划、自动化回归测试

6) Zero-copy / IO (`syscalls/zero_copy.rs`) — Priority: Medium → High (performance)
- 核心理由: 当前实现使用缓冲拷贝；io_uring 未实现；对高吞吐服务影响大
- 风险: 性能退化、无法满足高并发 I/O 场景
- 前置条件: 先实现测试驱动路径（小步支持），定义 async I/O 抽象，逐步引入 io_uring

7) Drivers / Platform (`drivers/mod.rs`, `drivers/device_manager.rs`) — Priority: High
- 核心理由: VirtIO / device binding 未实现会阻塞网络/磁盘 I/O 的真实运行
- 风险: 无法部署到真实平台或 CI 虚拟化环境
- 前置条件: 设备接口规范、设备测试平台、驱动兼容测试

8) Formal verification modules (`formal_verification/*`) — Priority: Medium
- 核心理由: 适用于高保障目标；当前模块含 stub 需与工程实现对齐
- 风险: 研究/维护成本高；若没有测试/代码联动价值减弱
- 前置条件: 选择关键组件作为 first‑class verification target（例如 memory 或 scheduler）

9) GUI / Graphics / Input  — Priority: Low → Medium
- 核心理由: 仅影响 UI 层和非核心服务；仍需为图形子系统构建更完整实现以支持 UI 需求
- 风险: 功能缺失不会阻断内核功能

---

## 四、临时/简易实现（technical debt）评估（总结）
- 广泛占位实现（println/placeholder/None 返回）在短期可维持编译与开发，但会掩盖运行时错误与逻辑漏洞。
- 类型 stub (`types/stubs.rs`) 是技术债务的核心：
  - 替换为真实类型需全仓协作（高风险改动）。
  - 建议通过 Adapter / Compatibility layer 逐步替换，配合大量回归测试与小步提交。
- 性能债务（zero-copy, io_uring, batch mmap）会随着生产负载逐步显现，如果目标产品强调吞吐，需要在中期解决。
- 安全债务（permission_check、smap_smep、ASLR）优先级应高，因潜在漏洞代价极高。

---

## 五、短期（0–4 周）前置工作与条件（蓝图）
1. 建立完整的 TODO 列表导出（逐条行级 CSV），并把每个标记关联到文件、行号与上下文（已生成 /docs/STUB_REPORT.md，可再细化）。
2. 引入临时自动化检查（CI）: 阻止新的 TODO/PLACEHOLDER 在关键路径（core/syscalls/mm/fs/security）提交。
3. 增加覆盖面测试：先保证 syscall 的最小可用测试（fork/exec/wait, mmap 基线, vfs 基本读写）
4. 确定核心 API contract（Process/Thread/VM/FS）并锁定 public API 作为稳定边界。

---

## 六、下步计划（接下来将进入详细实施规划）
- 目标: 根据上面的优先级和前置步骤，制订 3 个阶段（稳固核心→实现关键功能→性能/扩展）里程碑与资源估算、依赖地图与风险缓释措施。
- 下一步: 基于上述评估，生成包含里程碑、资源分配、依赖映射的可执行实施计划（含每日/每周检查点与回顾节点），并把高风险点拆成小步验收项。


---

(注: 我已完成源码扫描并完成优先级与技术债务评估。下一步将把这些条目转换为一个阶段性、可执行的实施计划 — 包括时间线、里程碑、所需人员类型与风险缓释措施。)
