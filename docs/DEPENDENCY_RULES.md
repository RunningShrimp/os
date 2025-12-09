# 依赖与分层规则

- 核心层不得依赖上层（subsystems/services/observability/security）
- 跨层调用通过 facade/traits 暴露统一接口
- 服务层对外暴露稳定 API，版本化管理（major.minor）
- 测试与基准不引入生产路径的额外依赖
- CI 校验越层依赖并阻断违规变更

