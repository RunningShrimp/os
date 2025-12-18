# Bootloader 审查整改实施计划与 TODO（基于审查报告 + 当前上下文）

> 日期：2025-12-14
> 
> 目标：把审查报告中所有 P0/P1/P2 建议落地为可执行任务，并补齐“当前上下文”中暴露的构建健康问题（否则后续整改无法验证）。

---

## 0. 范围与前置条件（不做会阻塞后续验收）

### 0.1 当前上下文：构建健康度
- 现状：`cargo check -p nos-bootloader` 失败（大量错误）。其中至少包含：事务模块 `TransactionId` 私有导入错误，且存在大量未使用 import/风格告警。
- 影响：没有稳定的 `cargo check`，图形/引导整改无法通过 CI/本地验证闭环。

### 0.2 本计划的验收基线
- Baseline-1：`cargo check -p nos-bootloader` 通过（可先允许 warning）。
- Baseline-2：新增/修改的关键路径具备最小单元测试覆盖（针对 framebuffer pitch、BootMenu 小分辨率、双缓冲安全）。

---

## 1. P0（必须先做）：安全性 + 正确性闭环

### P0-1 修复双缓冲的 pitch/越界写风险（最高优先级）
- 背景：`DoubleBuffer` 后备缓冲按 `width*height` 分配，但初始化清零按 `height*pitch`；当 `pitch > width*4` 会越界写。
- 涉及文件：
  - bootloader/src/graphics/mod.rs
- 任务拆分：
  1) 统一后备缓冲的“真实字节大小”计算：使用 `fb.pitch` 与 `fb.height`。
  2) 所有像素寻址使用 `stride_pixels = pitch / bytes_per_pixel`（ARGB8888 为 4）。
  3) `copy_region`/`copy_full_buffer` 使用 stride 与 pitch 做行拷贝，而不是假设紧密布局。
  4) 为 `pitch != width*4` 添加单元测试（虚拟内存 framebuffer）。
- 验收标准：
  - 不存在对 back buffer 的越界 `write_bytes`/`copy_nonoverlapping`。
  - `pitch != width*4` 的测试用例通过。

### P0-2 修复 BootMenu 图形布局的下溢/错误坐标
- 背景：`menu_x = (renderer.width() - menu_width)/2` 在 `width < menu_width` 时发生 u32 下溢。
- 涉及文件：
  - bootloader/src/boot_menu/mod.rs
- 任务拆分：
  1) 采用 `saturating_sub` 或显式 guard（不足分辨率自动退回 Text/Serial）。
  2) 对“最小分辨率门槛”制定规则并写测试。
- 验收标准：
  - 小分辨率不会出现极大坐标。
  - 图形模式不足时行为确定（回退或裁剪）且可测试。

### P0-3 修复 `graphics/mod.rs` 的非 x86_64 不可移植导入
- 背景：文件顶部无条件引入 `core::arch::x86_64::*`。
- 涉及文件：
  - bootloader/src/graphics/mod.rs
- 任务拆分：
  1) 把 x86_64 SIMD import 放到 `#[cfg(target_arch = "x86_64")]` 分支或子模块。
  2) 保证非 x86_64 仍可编译（走纯 Rust 路径）。
- 验收标准：
  - 非 x86_64 目标不因 import 编译失败。

### P0-4 修复 VBE 签名写入的高风险指针/UB
- 背景：对寄存器字段地址进行错误类型转换写入，存在 UB 风险。
- 涉及文件：
  - bootloader/src/graphics/vbe.rs
- 任务拆分：
  1) 使用对 `info.signature` 的直接写入（安全、可读）。
  2) 补充“签名写入正确”的单元测试（不依赖真实 BIOS 中断）。
- 验收标准：
  - 不再出现将 `regs.di` 地址当作结构体指针的写法。

### P0-5 让“图形初始化成功”与实际 framebuffer 语义一致
- 背景：GraphicsBackend 当前为占位实现：UEFI GOP address=0；VBE address 硬编码；绘制/清屏/文本为空操作。
- 涉及文件：
  - bootloader/src/infrastructure/graphics_backend.rs
  - bootloader/src/application/boot_orchestrator.rs
- 任务拆分：
  1) 明确最小可用标准：初始化必须返回真实 `FramebufferInfo`（address/pitch/bpp/width/height）。
  2) 上层在发布 `GraphicsInitializedEvent` 时应使用真实 address/pixel format。
  3) 若后端仍未实现真实硬件调用：上层必须显式标注“未提供 framebuffer”（不要假装成功）。
- 验收标准：
  - `GraphicsInitializedEvent` 不再填 0 地址。
  - `clear_screen/draw_pixel` 至少对 framebuffer 生效（或明确返回未实现错误）。

### P0-6 修复协议类型来源，禁止固定回落 BIOS
- 背景：`get_protocol_type()` 固定返回 `BootProtocolType::Bios`。
- 涉及文件：
  - bootloader/src/application/boot_orchestrator.rs
  - bootloader/src/infrastructure/di_container.rs（容器已持有 protocol_type）
- 任务拆分：
  1) 在 `BootApplicationService` 中保存真实 `BootProtocolType`（构造时注入/从容器读取）。
  2) `create_boot_info()` 使用真实协议。
- 验收标准：
  - UEFI/Multiboot2 启动路径不会生成 BIOS 协议类型。

---

## 2. P1（紧随其后）：维护性与架构一致性

### P1-1 统一 FramebufferInfo 类型，避免重复定义与误适配
- 背景：存在 `protocol::FramebufferInfo` 与 `graphics::FramebufferInfo` 双定义。
- 涉及文件：
  - bootloader/src/protocol/mod.rs
  - bootloader/src/graphics/mod.rs
  - bootloader/src/infrastructure/graphics_backend.rs
- 任务拆分：
  1) 选定权威结构体（建议 protocol 层）并迁移引用。
  2) 提供显式转换函数（如必须保留两者）。
- 验收标准：
  - 跨层传递 framebuffer 不再需要“猜字段语义”。

### P1-2 修复 DI 容器：单例缓存的 clone 不自洽 + 错误 leak
- 背景：`Box<dyn Any>` 被 `.clone()`；循环依赖报错用 `format!(..).leak()`。
- 涉及文件：
  - bootloader/src/infrastructure/di_container.rs
- 任务拆分：
  1) 单例缓存改用 `Arc<dyn Any + Send + Sync>`（或其他共享策略）。
  2) resolve 返回结构化错误类型（替换 `&'static str`），删除 `.leak()`。
  3) 更新依赖解析/作用域缓存逻辑与测试。
- 验收标准：
  - 不再对 `Box<dyn Any>` 进行 clone。
  - 不再存在 `.leak()` 构造 `'static` 错误字符串。

### P1-3 修复 SerializerRegistry：避免“注册可用但获取永远 None”的占位逻辑
- 背景：注册表用 `Box<dyn Serializer>`，但 get_serializer 由于不可 clone 永远返回 None。
- 涉及文件：
  - bootloader/src/infrastructure/serialization/serializer_registry.rs
- 任务拆分：
  1) 存储改为 `Arc<dyn Serializer + Send + Sync>`。
  2) `get_serializer/get_serializer_by_name` 返回 `Arc<...>`。
  3) 补齐测试：注册后可取回、格式冲突检测。
- 验收标准：
  - registry 的“注册→获取”链路可用。

---

## 3. P2（优化/体验）：性能与可测试性增强

### P2-1 渲染刷新策略与 dirty tracking 验证
- 背景：dirty tracking 存在，但在 BootMenu 场景可能仍会全屏 clear+大面积绘制。
- 涉及文件：
  - bootloader/src/graphics/mod.rs
  - bootloader/src/boot_menu/mod.rs
- 任务拆分：
  1) 规定 UI 刷新时机：仅输入变化时重绘。
  2) 在图形菜单渲染中减少全屏操作；引入“背景一次绘制+局部更新”的路径。
- 验收标准：
  - 在模拟输入多次切换时，全屏拷贝次数下降（可用计数器/统计验证）。

### P2-2 VBE 初始化输出与模式枚举的性能治理
- 背景：VBE initialize 使用大量 `println!` 且线性扫描模式列表。
- 涉及文件：
  - bootloader/src/graphics/vbe.rs
- 任务拆分：
  1) 将输出受 LogLevel/feature gate 控制。
  2) 对候选模式筛选（只 preload 常用模式，或设枚举上限）。
- 验收标准：
  - release/默认配置下 VBE 初始化不会产生大量 I/O 输出。

---

## 4. 构建健康（来自上下文，但不在图形审查范围内；仍需纳入计划）

### B-1 让 `cargo check -p nos-bootloader` 可通过
- 背景：当前有大量错误，包含 `TransactionId` 私有导入等。
- 涉及文件（初步）：
  - bootloader/src/infrastructure/transactions/transaction_log.rs
  - bootloader/src/domain/transactions.rs
  - 以及其他报错文件（以编译输出为准分批修复）
- 任务拆分：
  1) 先修复阻塞性可见性/导入错误（如 TransactionId）。
  2) 再逐批处理剩余 errors，直到 `cargo check` 通过。
  3) 最后收敛 warnings（可延后）。
- 验收标准：
  - `cargo check -p nos-bootloader` 通过。

---

## 5. 追踪矩阵（审查建议 → 任务）

| 审查建议 | 任务ID |
|---|---|
| 双缓冲 pitch/越界修复 | P0-1 |
| BootMenu 小分辨率下溢修复 | P0-2 |
| x86_64 SIMD import 可移植性 | P0-3 |
| VBE 签名写入 UB 修复 | P0-4 |
| GraphicsBackend 真实语义/事件地址 | P0-5 |
| 协议类型禁止固定 BIOS 回落 | P0-6 |
| FramebufferInfo 统一 | P1-1 |
| DI 单例 clone 不自洽 + leak | P1-2 |
| SerializerRegistry 可用性 | P1-3 |
| dirty tracking/刷新策略优化 | P2-1 |
| VBE 输出与枚举性能治理 | P2-2 |
| 构建健康（上下文） | B-1 |
