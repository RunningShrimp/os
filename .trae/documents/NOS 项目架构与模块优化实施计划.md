## 目标
- 完成对目录结构、模块化设计、整体架构的系统性审查与落地优化
- 强化内核混合架构的分层边界、服务化接口与可测试性，支持未来 6–12 个月的演进
- 在不影响线上与现有开发流程的前提下平滑过渡

## 范围
- 目录结构重构（聚合域、文档/脚本/测试归档、lint 约束）
- 系统调用层的模块化与特性化（核心/服务/优化分层）
- 服务层接口治理（统一 traits、版本化与注册发现）
- 初始化流程优化（按需/延迟加载、可观测性增强）
- 测试与 CI 体系完善（分层、基准、覆盖与矩阵）
- 性能治理（快路径、批处理、零拷贝与度量）
- 文档与工程治理（架构图、贡献指南、依赖地图）

## 实施步骤

### Phase 0：基线盘点与保护
1. 收敛工作区与构建别名：确认 `Cargo.toml` 工作区成员与 `.cargo/config.toml` 别名一致
2. 冻结当前主路径与关键入口：`kernel/src/main.rs` 入口（`rust_main*`、调度器）与 `syscalls` 路由
3. 建立“变更防护”分支策略与 CI 门槛：编译、单测、微基准、覆盖率阈值

### Phase 1：目录结构整序
1. 在 `kernel/src/` 建立逻辑分层：
   - `core/`（`arch/mm/cpu/sync/trap/types/collections`）
   - `subsystems/`（`vfs/fs/net/ipc/process/time/drivers`）
   - `services/`（服务注册、调度与公共 traits）
   - `observability/`（`debug/monitoring/profiling/tracing/metrics/symbols`）
   - `security/`（`security/security_audit/formal_verification/error_handling/reliability`）
   - 通过 `mod.rs` 与 `pub use` 提供兼容层，避免调用路径大规模改动
2. `syscalls/` 归档：
   - `syscalls/core/`（`fast_dispatcher.rs/batch.rs/cache.rs/common.rs/validation.rs`）
   - `syscalls/services/`（现有 `*_service/` 与 `services/{dispatcher,registry,traits}.rs`）
   - `syscalls/optimizations/`（`*_optimized.rs/zero_copy_optimized.rs/scheduler_optimized.rs`）
   - `syscalls/tests/` 与 `syscalls/docs/`（移动 `README_OPTIMIZATION.md/OPTIMIZATION_SUMMARY.md`）
3. 顶层归档：新增 `docs/`（架构/优化/路线图）、`ops/`（脚本与运维）、`tests/`（跨模块集成测试）

### Phase 2：模块化与特性化
1. 建立优化特性旗标：`features = { fast_syscall, zero_copy, batch_syscalls, net_opt, sched_opt }`
2. 在 `syscalls` 与相关子系统中，用 `#[cfg(feature = "...")]` 选择优化实现，默认保守实现
3. 为服务层抽象统一 `traits`：请求/响应、错误码、版本号（`ServiceVersion`）与能力声明（`Capabilities`）
4. 引入 `service_registry` 的可插拔路由与度量钩子（延迟与失败率）

### Phase 3：依赖关系治理
1. 输出模块依赖地图：生成 `depgraph.md` 并在 CI 进行规则校验（核心层不得依赖上层）
2. 在 `cargo deny`/`dep-tree` 或自研 lint 脚本中强制边界规则
3. 为跨层调用建立 `facade` 模式（上层只见接口，不见内部）

### Phase 4：初始化流程与可观测性
1. 将非关键子系统改为按需初始化：网卡、图形、web 引擎采用 lazy-init（可通过服务注册触发）
2. 在关键入口注入观测：启动阶段时序、各子系统 init 耗时与错误分布
3. 引入轻量级 `boot timeline` 输出与可选持久化到 `procfs`

### Phase 5：测试与 CI
1. 单元测试：保留 `no_std` 场景，扩展 `#[cfg(test)]` 主机模拟测试（mockall/proptest）
2. 集成测试：`tests/` 目录下用特性矩阵执行（架构/特性组合）
3. 微基准：完善 `benchmarks/microbench` 并在 CI `microbench.yml` 输出对比报告
4. 覆盖率：`tarpaulin.toml` 约束关键模块覆盖阈值

### Phase 6：性能治理
1. 快路径：`fast_dispatcher` 热路由，缓存命中率与尾延迟度量
2. 批处理：建立批量系统调用 API 与内核侧合并提交（VFS/NET）
3. 零拷贝：在 `vfs/net` 打通页缓存与 DMA 区域映射；建立安全边界检查
4. 调度优化：为 I/O 密集与 CPU 密集负载提供策略切换（`sched_opt`）

### Phase 7：文档与工程治理
1. 在 `docs/` 输出架构图、模块边界、依赖规则与演进路线图
2. 完善贡献指南（变更规范、命名与层级约定、测试要求）
3. 为运维与脚本写使用说明与基准报告模板

## 验收标准
- 所有模块迁移后编译与测试通过（含特性矩阵）
- CI 规则生效并阻断越层依赖
- 性能基线不低于当前值；开启优化特性后在目标场景达到预期
- 文档完整，开发者能够按规范扩展模块与新增服务

## 平滑过渡
- 分阶段引入，先加壳（`pub use` 兼容层）再迁移实现
- 默认关闭优化特性，保持现有行为；在实验环境逐步打开并对比
- 通过 `xtask` 增加迁移辅助命令（仅文档与脚本，不改代码行为）

请确认以上计划，确认后我将开始逐步实施，每一步都提供具体改动与收益说明，并确保不会影响现有服务。