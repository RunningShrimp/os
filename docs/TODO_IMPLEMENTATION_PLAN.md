# TODO 审查 — 可执行实施计划（阶段化）

生成时间: 2025-12-08

目标: 基于代码级 TODO 扫描与分析（327 个标记、热点分布见 /docs/STUB_REPORT.md & /docs/TODO_AUDIT_ANALYSIS.md），制定一个包含阶段里程碑、资源配置、依赖映射、风险缓释措施和复盘节点的可执行计划。该计划以“优先处理影响系统正确性与安全的缺陷”为导向，同时保证逐步替换大量 type/stub 带来的技术债务。

---

## 约定与设计原则
- 最小破坏：对高影响改动（如 types/stubs 替换）采取 Adapter/compat 层 + 渐进式替换；每步须有回滚方案。
- 自动化优先：每个里程碑交付必须配套自动化回归/集成测试（强制在 CI 上通过）。
- 分阶段交付：按可交付物（buildable、可回归测试、可部署）拆分迭代。
- 风险逐层缓解：先解决高安全/正确性风险，再处理性能与可扩展性问题。

---

## 总体时间线（建议）
> 下面给出一个可调整的路线：总周期（MVP->stable）约 6–9 个月（取决于团队规模/并行度）。

- 阶段 0 (0–4 周) — 准备与门控
- 阶段 1 (4–16 周) — 核心正确性与基础设施稳定（Process/Thread/VFS/MM 基线）
- 阶段 2 (16–28 周) — 安全/权限/驱动/平台实现（Seccomp/ASLR/Drivers）
- 阶段 3 (28–40 周) — 性能/扩展（zero-copy/io_uring/advanced mmap）
- 阶段 4 (40–48 周) — 验证/固化/发布准备与后续功能（GUI/graphics/optional）

注：里程碑与时间可根据团队资源并行推进加速。

---

## 阶段 0（0–4 周） — 基础准备、检测门控与最低前置条件 ✅
目标：建立可视化 TODO 行级清单、CI 阻断、最小可复现测试集。

主要任务：
- 生成逐条 CSV 列表（文件/行号/上下文/推荐优先级/依赖人）并导入 Issue Tracker。
- 在 CI 中增加 TODO 增量检测：阻止在核心路径(systems, mm, vfs, security)产生新的 TODO/placeholder PRs。
- 定义并锁定「核心 API Contract」: Process/Thread/VM/FS/Driver 基本接口规范。
- 建立/扩展核心单元测试 + 最小化的集成测试（fork/exec/wait, vfs read/write, mmap basic）

交付物：
- /docs/STUB_REPORT.md（已存在）扩展为每条 TODO 行级 CSV 以便精确分派
- CI 新检查项（PR 阻断关键路径 TODO）
- 基线测试套件 + 测试度量（覆盖率基线）

资源与角色建议：
- 1 工程师（自动化/CI） + 1 内核工程师（前期驱动/测试 harness）

风险与缓解：
- 风险：未能构建可靠的行级清单 → 缓解：使用工具（scripts/scan_stubs.sh）再生成细化 CSV。

---

## 阶段 1（4–16 周） — 核心正确性优先（Critical）
目标： 修补与替换关键核心功能的占位实现，确保系统的基本语义正确（进程、线程、VFS、基础内存操作）。

主要任务（并行可做，但按优先级还是建议步进进行）：
1. Process & Thread syscall 完整性修复（fork/exec/wait, clone/futex 等）
   - 里程碑 1.1 (week 4–8): fork/exec/wait 基线功能 + 边界/错误处理 + 单元/集成测试
   - 里程碑 1.2 (week 8–12): 完整 clone/clone flag 支持、futex robustness、CLONE_* 行为测试

2. VFS / FS 基础实现
   - 里程碑 1.3 (week 6–12): inode read/write、directory lookup、truncate、root init 和最小一致性测试

3. Memory 基线（mm）
   - 里程碑 1.4 (week 6–14): 基线 vm/mmap 行为、copyin/copyout 正确性、页面分配/释放一致性测试

4. Types/stubs 渐进替换策略启动
   - 里程碑 1.5 (week 10–16): 为关键 stub 提供 Adapter 层（兼容旧接口），并把核心模块切换到真实类型的实验性 PR

交付物：
- 完整的系统调用核心用例通过 CI
- 基线 VFS 操作可在集成测试通过
- types/stubs 替换原型（不破坏 mainline）

资源建议：
- 3 核心内核工程师（Process/Thread/VM）
- 2 文件系统与驱动工程师（VFS, FS）
- 1 QA/自动化工程师（测试覆盖、回归套件）

风险与缓解：
- 风险：types/stubs 替换破坏性大 → 缓解：Adapter→逐文件替换→CI 未通过不合并
- 风险：核心行为边界不完整 → 缓解：增加 syscall fuzz 测试与用户态互操作性测试

---

## 阶段 2（16–28 周） — 安全与平台支撑（High）
目标：填补安全功能缺失并完成关键驱动以支持真实 I/O 平台。

主要任务：
- Security 体系实现（permission_check 集成 seccomp/capabilities、ASLR、smap/smep）
  - 里程碑 2.1: 安全策略框架 + 最小策略集成测试
  - 里程碑 2.2: 强化权限检查与准入测试

- 驱动与平台（VirtIO、device binding、block/net driver）
  - 里程碑 2.3: VirtIO 基本实现 + CI 磁盘/网络吞吐测试

- Formal verification target 确定（选择 1–2 个高价值子系统做形式化验真）

交付物：
- 安全功能集成下的合规测试
- 虚拟化场景（VM）下正常运行的 I/O 驱动

资源建议：
- 2 安全工程师/架构师
- 2 驱动工程师
- 1 验证/形式化工程师

风险与缓解：
- 风险：安全策略与现有行为冲突 → 缓解：设计兼容层、分阶段启用策略（feature flag） + 大量回归测试
- 风险：驱动与平台差异 → 缓解：提前锁定目标平台与测试矩阵

---

## 阶段 3（28–40 周） — 性能与扩展（Medium→High）
目标：解决性能债务（zero-copy、io_uring、advanced mmap、page pinning），支持高吞吐、低延迟场景。

主要任务：
- zero-copy 的真实现（page reference moves / vmsplice / splice）
- io_uring 支持 (setup/enter/register)、异步 I/O 抽象
- advanced mmap: on-demand paging / file backing / pinning
- 大规模并发与压力测试（长期稳定性）

交付物：
- 零拷贝路径在关键场景下的性能基准提升
- io_uring 基本 API 可用并有测试用例

资源建议：
- 2–3 性能/系统工程师
- 1 测试/benchmark 工程师

风险与缓解：
- 风险：复杂内核数据结构改动导致稳定性问题 → 缓解：先在实验分支完成 micro‑bench + 强制回归

---

## 阶段 4（40–48 周） — 验证、固化、发布准备
主要任务：
- 集成所有变更到 release candidate
- 执行全面回归/压力/安全测试
- 性能回归比较与确认
- 文档与操作手册更新

交付物：
- 新的 release candidate（RC）镜像和发版说明
- 发布前安全/合规报告

---

## 依赖映射（简明）
- `execve` → 依赖 `vfs`、`fs`、`types`、`mm`（page tables）
- `clone/futex` → 依赖 `thread` 实现、`process::manager`、`futex` 数据结构
- `zero_copy` → 依赖 `fs` (VFS read)、`socket`、`drivers`、future io_uring
- `security` → 依赖 `types/stubs`、`syscall` 拦截点、内核权限模型
- `drivers` → 依赖 `platform`、`device_manager`、block/net subsystems

策略：优先修复无上游依赖或能被分段替换/适配的模块；对跨模块依赖采用 Interface/Adapter 断点以降低联动风险。

---

## 风险缓释矩阵（示例）
- 大型替换(Types Stubs)
  - 风险: 高
  - 缓解: Adapter 层 + 小步迁移 + 拆分 PR + 自动化回归 + 里程碑式合并
- 安全特性启用
  - 风险: 高（可能导致兼容性 break）
  - 缓解: 使用 feature flags + 渐进启用 + 回滚策略
- 性能特性（zero-copy）
  - 风险: 中
  - 缓解: 先功能 + 基准 → 再扩展；写回退路径

---

## 复盘与检查节点（节奏）
- 周会：每周 30–60 分钟进度 + 风险更新
- Sprint（两周）交付：每两周给出小版本的交付与回顾
- 阶段里程碑审查：阶段末一次设计/代码审查 + 一次安全/测试审查
- 月度全栈回顾：产品/工程/QA/安全联合回顾

---

## 快速动作建议（优先）
1. 阶段 0 中执行：把 327 个 TODO 逐条入 issue tracker，并给出 owner 与估时。
2. 先锁定并实现一套覆盖度高的核心 syscall 集合（fork/exec/wait、vfs 基线、mmap 基线）。
3. 对 `types/stubs.rs` 设计 Adapter 策略，选取 3 个低耦合类型做“先行替换实验”，把替换流程模板化。

---

## 下一步（可执行）
- 我可以：
  1) 将 327 条 TODO 逐条细化为 CSV + 转化为跟踪 issue（包含建议优先级、估时、owner）
  2) 生成详细 Gantt / Sprint 划分（基于你同意的团队规模）
  3) 生成 PR 模板/迁移策略文档 (types/stubs replacement)

请告诉我你希望我现在做哪一项（1/2/3），或给出团队规模/期望完工时间，我会据此调整计划并输出可分配的 issue 列表与时间估算。
