# Feature 使用审查：实施计划与 TODO（基于全仓库特性审查）

> 日期：2025-12-15
>
> 目标：
> 1) 纠正 feature 漂移（`cfg(feature = "X")` 但 `Cargo.toml` 未声明/声明但无使用）。
> 2) 收敛 nightly/不稳定特性影响面，避免“为了一个点把整个 workspace 锁死 nightly”。
> 3) 建立可持续的 feature 治理机制（CI/脚本强约束），防止回归。

---

## 0. 范围、术语与验收基线

### 0.1 范围
- **Rust 不稳定特性**：`#![feature(...)]`（nightly-only）。
- **Cargo features**：`[features]`、`cfg(feature = "...")`、`cfg!(feature = "...")`。
- **跨 crate 传播**：`crateA/featureX` 依赖传播、`default-features = false` 组合。

### 0.2 不在本计划范围（避免无限膨胀）
- 不进行子系统功能重写（network/fs 等实现细节不在此整改）。
- 不对性能做广泛重构（除非是因 feature 开关引发的编译/链接失败）。

### 0.3 验收基线（每阶段都要可验证）
- Baseline-F0：任意 `cfg(feature = "X")` **必须**在同 crate 的 `Cargo.toml [features]` 出现（否则 CI 失败）。
- Baseline-F1：workspace 内的 nightly 使用被**隔离到必要 crate/必要模块**；能明确指出 nightly 的“最小原因集”。
- Baseline-F2：features 命名体系一致（同一概念不出现 `monitoring`/`observability` 等多套叫法并存）。

---

## 1. 当前发现摘要（来自本次审查）

### 1.1 工作区被强制 nightly
- 工具链：`channel = "nightly"`（见 [rust-toolchain.toml](rust-toolchain.toml#L1-L4)）。
- 已发现 `#![feature]`：
  - 内核：`c_variadic`（见 [kernel/src/main.rs](../kernel/src/main.rs#L3)）
  - Bootloader：`allocator_api`、`alloc_error_handler`（见 [bootloader/src/lib.rs](../bootloader/src/lib.rs#L7-L9)）
  - microbench：`test`（criterion 已存在，可能完全可去掉）

### 1.2 Feature 漂移（典型滥用症状）
- **代码引用了未声明的 feature**，导致开关“名存实亡”或永远走默认分支：
  - `kernel/src/features/mod.rs` 中使用 `networking/security/monitoring`，但 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 未声明这些 feature。
  - `kernel/src/mm/hugepage.rs` 使用 `hpage_2mb/hpage_1gb`，但 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 未声明。
  - `kernel/src/main.rs` 使用 `journaling_fs`，以及测试/基准中出现 `numa/hw_accel/ml`，但 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 未声明。
- 同时存在 `cfg!(feature = "...")` 的常量判断：它不会剔除代码，仅改变运行时常量分支，容易留下“关了 feature 也编不过”的隐患。

### 1.3 Meta-features 扩散但缺少治理
- 多个库重复声明 `minimal/embedded/server/desktop` 以及 `debug_subsystems/formal_verification/security_audit`（见各 `nos-*` 的 `Cargo.toml`）。
- 大量 feature 可能属于“产品档位”而非“库能力开关”，更适合集中到集成层，而不是在每个库复制粘贴。

### 1.4 已确认的“滥用/失控”清单（可直接转 TODO）

> 说明：这里的“滥用”优先指**可验证的问题**（漂移/名存实亡/命名分裂/过度 gate），而不是主观的“feature 太多”。

#### 1.4.1 Feature 漂移（代码引用，但 `Cargo.toml [features]` 未声明）

- bootloader
  - `secure_boot_support`
    - 使用点：
      - [bootloader/src/uefi/main.rs](../bootloader/src/uefi/main.rs#L75)
      - [bootloader/src/uefi/mod.rs](../bootloader/src/uefi/mod.rs#L230)
    - 现状：在 [bootloader/Cargo.toml](../bootloader/Cargo.toml#L47-L92) 未声明。

- kernel
  - `networking` / `security` / `monitoring`
    - 使用点： [kernel/src/features/mod.rs](../kernel/src/features/mod.rs#L53-L58)
    - 现状：在 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 未声明（同时 kernel 已有 `net_stack/observability/...` 等相近命名）。
  - `realtime`
    - 使用点： [kernel/src/sync/mod.rs](../kernel/src/sync/mod.rs#L162)
    - 现状：在 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 未声明。
  - `journaling_fs`
    - 使用点： [kernel/src/main.rs](../kernel/src/main.rs#L220-L231)
    - 现状：在 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 未声明。
  - `hpage_2mb` / `hpage_1gb`
    - 使用点： [kernel/src/mm/hugepage.rs](../kernel/src/mm/hugepage.rs#L48-L55)
    - 现状：在 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 未声明。
  - `numa` / `hw_accel` / `ml`
    - 使用点（测试/基准路径）：
      - [kernel/src/test/integration.rs](../kernel/src/test/integration.rs#L401)
      - [kernel/src/test/benchmark_final.rs](../kernel/src/test/benchmark_final.rs#L644)
    - 现状：在 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 未声明。
  - `debug`（疑似与 `debug_subsystems` 命名冲突/拼写分裂）
    - 使用点： [kernel/src/mm/prefetch.rs](../kernel/src/mm/prefetch.rs#L321)
    - 现状：在 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 未声明。

#### 1.4.2 过度细粒度 gate（可读性差，且容易形成“伪支持”）

- `alloc` 的 item 级 `#[cfg(feature = "alloc")]` 在多个 `nos-*` crate 中高频出现（本次抽样最明显的是 `nos-error-handling`）：
  - 入口文件： [nos-error-handling/src/lib.rs](../nos-error-handling/src/lib.rs#L37-L155)
  - 现象：同一文件内大量函数/结构体/impl 用 `#[cfg(feature = "alloc")]` 分散 gate。
  - 风险：读者难以判断 crate 在 `no-alloc` 下是否“可用”；实际很容易出现某个模块内部仍引用 `alloc` 导致构建腐烂。

#### 1.4.3 `cfg!(feature = "...")` 误用风险（裁剪失败/隐藏编译问题）

- `cfg!()` 只会生成常量 bool，不会把未选分支从编译中移除；对 OS 这类需要裁剪/隔离的工程，通常应优先 `#[cfg]` 隔离模块或函数。
  - 例： [kernel/src/mm/hugepage.rs](../kernel/src/mm/hugepage.rs#L48-L55)

#### 1.4.4 “空 feature / 占位 feature”扩散（定义无语义）

- 多个库中存在 `alloc = []` / `std = []` / `minimal/embedded/server/desktop` 等 feature 的复制粘贴式存在，但缺少明确的：
  - 使用点（代码 `cfg`）
  - 组合表（这些档位到底打开了哪些能力）
  - 组合测试（哪些组合被 CI 覆盖）
  - 说明文档（为何存在、谁来维护）

---

---

## 2. 实施原则（用于判定“滥用”与整改方向）

1) **feature 必须可解释**：每个 feature 在 `Cargo.toml` 附 1 行用途说明（或在 docs 中有表格）。
2) **feature 必须可验证**：CI 至少跑 `--no-default-features`、`--all-features`、以及关键组合（见 4.3）。
3) **库能力开关优先，产品档位后置**：库内只保留 `alloc/std/log` 和少量“真正可选模块”；`server/desktop/embedded/minimal` 这类“档位”集中到顶层。
4) **nightly 最小化**：能用稳定替代就替代；不能替代就隔离在单独 crate 或明确的 feature gate。

---

## 3. 分阶段实施计划（可并行，但建议按序验收）

### Phase A（P0）：建立治理基线（防回归）
**目标**：先把“漂移”变成 CI 可见的硬失败，否则后续修复会反复回潮。

- [ ] A1：新增脚本 `scripts/check-feature-drift.sh`（或 xtask 子命令）
  - 扫描所有 `cfg(feature = "X")`/`cfg!(feature = "X")`
  - 对照同 crate `Cargo.toml [features]`
  - 输出：未声明 feature、声明但无使用 feature、以及疑似拼写分裂（如 `monitoring` vs `observability`）
- [ ] A2：CI 接入：对每个 crate 跑一次漂移检查
- [ ] A3：建立 docs 表：`Feature 名称 / 所属 crate / 目的 / 默认值 / 依赖传播`

**验收**：Baseline-F0 达成（出现漂移即 CI 失败）。

### Phase B（P0）：修复 kernel 的 feature 漂移与命名分裂
**目标**：先把 kernel 这类“feature 使用量最大”的 crate 拉回一致状态。

- [ ] B1：对齐 `KernelFeatures` 的字段与 `kernel/Cargo.toml`
  - 选择统一命名：例如用 `net_stack` 替代 `networking`，用 `observability` 或 `monitoring` 二选一
  - 修改 [kernel/src/features/mod.rs](../kernel/src/features/mod.rs) 使其只引用已声明的 feature
- [ ] B2：处理 `hpage_2mb/hpage_1gb`
  - 方案 1：在 [kernel/Cargo.toml](../kernel/Cargo.toml#L42-L67) 显式声明并说明
  - 方案 2：删除这些 feature 判断，改为运行期探测或固定策略
- [ ] B3：处理 `journaling_fs/numa/hw_accel/ml` 等未声明 feature
  - 若只是测试/benchmark：改用 `cfg(test)` 或 `required-features` 的 bench 配置
  - 若是正式能力：补齐 `Cargo.toml [features]` 定义与依赖
- [ ] B4：审计 `cfg!()` 使用点
  - 若分支内引用了不应编译的符号，改成 `#[cfg(feature = "...")]` + 模块隔离

**验收**：
- kernel 相关的“未声明 feature 使用”清零。
- `--no-default-features` 与默认 features 的 `cargo check -p kernel` 能通过（允许 warning）。

### Phase C（P1）：收敛 nightly 不稳定特性影响面
**目标**：把 nightly 需求从“workspace 全局”收敛为“必要点”。

- [ ] C1：定位 `c_variadic` 的真实需求与替代路径
  - 已发现内核里声明了 C 变参函数（见 `printf/open` 的 `...`，位于 [kernel/src/libc/newlib.rs](../kernel/src/libc/newlib.rs) 附近）
  - 备选整改路径（择一落地）：
    - 路径 1：用 C shim 提供非变参 API（例如 `vprintf`/`vsnprintf` 包装），Rust 侧不再声明 `...`
    - 路径 2：完全避免调用变参函数，Rust 侧实现格式化并输出到串口/console
- [ ] C2：评估 `allocator_api` 的必要性
  - 若仅为了自定义 allocator：优先确认是否能改为稳定接口；不能则隔离到最小模块，并写清原因
- [ ] C3：bench 移除 `#![feature(test)]`
  - 已使用 Criterion（见 microbench 的 Cargo.toml），通常无需 nightly `test`
- [ ] C4：将 nightly 固定到具体版本（可复现构建）
  - 在 [rust-toolchain.toml](../rust-toolchain.toml) 改为带日期的 nightly（例如 `nightly-YYYY-MM-DD`）

**验收**：
- 能说明 nightly 的“最小原因集”（哪几个 crate/模块、为什么需要）。
- bench 不再依赖 `#![feature(test)]`。

### Phase D（P1）：治理 meta-features（避免复制粘贴扩散）
**目标**：让 feature 体系可维护、可组合、可预测。

- [ ] D1：决定 meta-features 策略
  - 方案 A：把 `server/desktop/embedded/minimal` 从各库移除，仅在顶层集成 crate 定义组合
  - 方案 B：保留但必须“落地使用点”与“组合表”，并禁止在库里无意义占位
- [ ] D2：清理“声明但无使用”的 features
  - 对每个 `nos-*` crate：列出未用 features 并删除/或补齐使用点
- [ ] D3：统一可传播 feature 的命名与传播方式
  - 例如：所有库的 `alloc/log/std` 一致，且 `kernel` 作为集成层负责统一开启

**验收**：
- `nos-*` crates 的 feature 列表明显收敛；每个 feature 都有用途与使用点。

### Phase E（P2）：补齐组合测试矩阵（防止“某组合永远没跑过”）
- [ ] E1：确定最小矩阵（建议）
  - `--no-default-features`
  - `--all-features`
  - `default`
  - kernel 关键组合：`+kernel_tests`、`+baremetal`、`+net_stack`（按你们目标平台调整）
- [ ] E2：引入 `cargo hack`（如团队接受）或用 xtask 显式跑组合

**验收**：每次 PR 至少跑过最小矩阵，避免 feature 组合腐烂。

---

## 4. TODO 清单（按模块/仓库位置拆分，便于分配）

### 4.1 Workspace / 总控
- [ ] W1：在 docs 中建立 feature 总表（单一事实来源）
- [ ] W2：CI 接入 feature 漂移检查 + 组合矩阵
- [ ] W3：固定 nightly 版本（若仍需要）

### 4.2 kernel
- [ ] K1：修复 `KernelFeatures` 引用未声明 feature（`networking/security/monitoring`）
- [ ] K2：补齐或移除 `hpage_2mb/hpage_1gb` feature 开关
- [ ] K3：补齐或移除 `journaling_fs/numa/hw_accel/ml` 等 feature
- [ ] K4：梳理 `cfg!()` 使用点，必要时改为 `#[cfg]` 模块隔离
- [ ] K5：处理 `c_variadic`：消除或隔离（见 Phase C1）

### 4.3 bootloader
- [ ] BTL1：评估 `allocator_api` 是否可替代；若不可，写明原因与最小化范围
- [ ] BTL2：避免把 `bootloader/src/lib_backup/*` 作为“活代码”误导：在 docs 标注或从构建/扫描中排除

### 4.4 benchmarks/microbench
- [ ] MB1：移除 `#![feature(test)]`，统一使用 Criterion
- [ ] MB2：确保 bench 只在明确的 `required-features` 下启用（避免默认编译矩阵膨胀）

### 4.5 nos-api / nos-syscalls / nos-services / nos-memory-management / nos-error-handling / tests
- [ ] N1：对每个 crate 列出 `Cargo.toml [features]` 中“声明但无使用”的条目并处理
- [ ] N2：统一 `alloc/log/std` 语义与传播策略（谁负责打开、默认是什么）
- [ ] N3：清理/收敛 `server/desktop/embedded/minimal` 这类 meta-features（按 Phase D 决策）

---

## 5. 风险与回滚策略
- 风险：修复 feature 漂移可能暴露隐藏的编译路径问题（以前从未真正启用/禁用过）。
- 风险控制：每一步都以 CI 组合矩阵验证；先做“治理基线”（Phase A），再逐步修复。
- 回滚策略：所有 feature 变更以“增量 PR”提交；每个 PR 都保持可编译。

---

## 6. 交付物（最终应得到什么）
- 一个可执行的 feature 治理脚本/xtask（漂移检查）。
- 一份权威的 feature 总表（文档）。
- 一个可复现的 toolchain 策略（固定 nightly 或尽可能 stable）。
- 一个最小但覆盖关键组合的 CI 矩阵。
