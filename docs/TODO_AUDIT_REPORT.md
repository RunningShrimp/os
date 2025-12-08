# 全面 TODO 审计与可执行实施计划（终稿）

生成时间: 2025-12-08

说明: 本审计与计划基于对源码层面自动扫描得到的 TODO/FIXME/STUB 标记（scan 脚本输出 /docs/STUB_REPORT.md），并独立做出优先级、依赖、风险与资源评估；未直接依赖已有策略文档来制定决策。报告包含：清单统计、优先级与风险分配、技术债务评估、阶段性里程碑（含资源建议）与复盘节点。

---

## 一、执行摘要（2–3 行）
- 全仓扫描结果：共发现 327 处 TODO/FIXME/STUB 标记，热点集中在 syscalls、types/stubs、VFS/FS、内存与驱动模块。
- 建议优先级：先修复影响正确性/安全的核心模块（Process/Thread/VFS/MM/Security），其次处理驱动与性能路径（Zero‑copy/io_uring），types/stubs 的替换采取渐进式迁移与 Adapter 层。

---

## 二、范围与方法
- 范围: 仓库内所有源码文件（尤其 kernel/*）的 TODO、FIXME、STUB、placeholder、Temporary、hack 等标记。
- 方法: 使用项目内扫描脚本与正则检索收集行级信息，随后人工/代码审阅定位“临时实现/占位/重要未实现”并按影响范围、可测性与依赖度评分。

---

## 三、关键数据快照
- 总计 TODO/FIXME/STUB: 327 处
- Top 熱点（按条目数）:
  - `kernel/src/syscalls/process.rs`: 34
  - `kernel/src/syscalls/thread.rs`: 22
  - `kernel/src/ids/host_ids/host_ids.rs`: 20
  - `kernel/src/syscalls/memory.rs`: 19
  - `kernel/src/types/stubs.rs`: 15
  - 其他高关注：`vfs/ext4.rs`, `fs/fs_impl.rs`, `security/permission_check.rs`, `syscalls/advanced_mmap.rs`, `syscalls/zero_copy.rs`

（完整分布见 /docs/STUB_REPORT.md）

---

## 四、总体优先级与风险矩阵（概览）
- Critical: process/thread syscalls, vfs/fs 基础、核心 memory 操作、security 权限检查。
- High: types/stubs 替换、drivers (VirtIO)、IO 基础（copy vs mmap）
- Medium: zero-copy / io_uring（性能）、formal verification（保证层面）
- Low: GUI/graphics、部分测试辅助代码

风险要点：types/stubs 的普遍使用构成一次重大技术债务，替换风险高；security 模块未集成标准访问控制会引发高危安全缺陷；VFS/FS 与驱动缺陷会导致数据损坏/不稳定。

---

## 五、技术债务与扩展性问题评估（摘要）
1. 大量占位实现（placeholder/prints）:
   - 影响: 隐藏运行时错误、阻止真实场景测试。
   - 处置: 编写最小可测实现并在后续阶段对功能增强。

2. 类型/接口 Stubs (`types/stubs.rs`):
   - 影响: 广泛依赖导致替换成本高、容易引入回归。
   - 处置: 1) 设计 Adapter 层，2) 逐个模块替换并严格 CI 测试。

3. 性能债务（zero-copy、io_uring、advanced mmap）:
   - 影响: 高吞吐场景下表现不佳。
   - 处置: 阶段性优化，先保证正确性后逐步切换到零拷贝/异步 I/O

4. 安全债务（permission_check, smap_smep, ASLR）:
   - 影响: 重大，需高优先级修补。
   - 处置: 定义 threat model 与策略，分阶段逐步启用并回归。

---

## 六、分阶段（Phase）可执行实施计划（精简版）
注：每一阶段均包含：里程碑、资源配置、关键交付物与验收标准。

Phase 0 — 准备（0–4 周）
- 任务: 导出逐条 CSV、设置 CI 门控（阻止核心路径新 TODO）、构建基线测试
- 交付: CSV 列表、CI 禁止规则、基础测试集
- 资源: 1 CI/自动化 + 1 内核 engineer

Phase 1 — 核心正确性（4–16 周）
- 任务: 完善 Process/Thread syscalls、VFS/FS 基线、MM 基线实现、开始 types/stubs Adapter
- 交付: 基本 syscall 集合通过 CI、VFS 基线可用、types/stubs 替换原型
- 资源: 3 core kernel + 2 fs/driver + 1 QA

Phase 2 — 安全/平台（16–28 周）
- 任务: security 整合 (seccomp/SELinux/capabilities)、VirtIO 与关键驱动、formal verification (选取目标)
- 交付: 安全框架能够在 CI 下运行、驱动在目标平台可用
- 资源: 2 security + 2 driver + 1 verification

Phase 3 — 性能/扩展（28–40 周）
- 任务: true zero-copy 实现、io_uring 支持、advanced mmap 完整功能、压力测试
- 交付: 性能指标达标、稳定性验证
- 资源: 2–3 perf engineers + 1 QA

Phase 4 — 验证/发布（40–48 周）
- 任务: 集成、回归、性能比对、发布候选
- 交付: RC 镜像 + 发布说明 + 安全合规检查

---

## 七、依赖映射（行动优先级指导）
- Execve → 需 VFS + FS + MM
- fork/clone/futex → thread/process manager
- zero-copy → VFS/FS + socket + drivers
- security → syscall interception points + types/stubs

策略：优先处理“被依赖度高”的节点以释放大量下游工作（例如：先把 VFS/FS 稳定下来，再做 execve 的扩展）。

---

## 八、风险缓释（示例措施）
- Types/stubs 替换: Adapter 层 + 小批量替换 + CI 强制覆盖
- 添加安全策略: feature flags + 阶段性启用 + 可回滚
- 驱动适配: 锁定目标平台并做到驱动层可模拟（unit tests + device simulation）

---

## 九、可交付的短期行动建议（立即执行）
1. 将 327 条 TODO 导入 Issue Tracker（带 owner/估时/优先级）
2. 阶段 0：在 CI 中对核心路径添加 TODO/placeholder 阻断规则
3. 选择 2–3 个可以并行推进的“关键小目标”（例如：a) fork/exec/wait 的完整测试覆盖，b) VFS inode 基本功能，c) Adapter 原型替换一个 types/stub）

---

## 十、我接下来可以做的具体项（请选择其中之一或多个）
1. 逐条把 327 条 TODO 转化为 Issue Tracker（推荐）并给出 item‑level 估时/owner。
2. 根据你提供的团队规模，生成详细的 Sprint/Gantt（PR‑level 工作分配与交付时间）以便直接分配任务。
3. 自动化生成行级 CSV 与 PR 模板，包含替换 types/stubs 的示例流程和测试 checklist。

---

如需我继续执行（例如把所有 TODO 生成 Issue 列表并附上优先级/估时/owner），请回复数字选择（1/2/3）或指定你想要的下一步。我会按你选项继续并以可合并 PR 或 Issue 导入的形式交付。

(本次审计不会单纯引用仓库中现有设计文档作为结论依据，而是基于源码扫描与代码上下文而得出建议 — 若你希望把文档中已存在的历史决策纳入考量，我可以在下一轮把这些文档作为对照项并调整优先级与里程碑。)
