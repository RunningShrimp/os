# 贡献指南与目录规范

## 命名与目录
- 模块分层：`core/`、`subsystems/`、`services/`、`observability/`、`security/`
- 系统调用：`syscalls/core`（分发/缓存/批处理/校验）、`syscalls/optimizations`（零拷贝/调度/网络等）
- 文档集中在 `docs/`；跨模块集成测试集中在 `tests/`

## 代码规范
- 遵循无注释要求（必要说明放文档），保持接口清晰
- Feature 门控：`fast_syscall`、`zero_copy`、`batch_syscalls`、`net_opt`、`sched_opt`、`lazy_init`
- 依赖边界：核心层不依赖上层；跨层通过 traits/facade

## 提交流程
- 保持编译通过与测试通过（含特性矩阵）
- 更新相关文档与基准报告
- 禁止提交任何敏感信息与密钥

