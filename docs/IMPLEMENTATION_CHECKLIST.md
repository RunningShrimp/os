# 架构实施清单与阶段里程碑

## 清单
- 分层门面与兼容层建立（core/subsystems/observability/security/syscalls/core/optimizations）
- 特性旗标与分发/路由门控（fast_syscall/zero_copy/batch_syscalls/net_opt/sched_opt/lazy_init）
- 服务 traits（Versioned/ProvidesCapabilities/Capabilities 位集合）
- 观测节点（servicestats/servicehealth/features/initlazy/processstats/timeline/perfsummary）
- 测试与微基准（读取测试、快路径、批处理、零拷贝、调度）
- 覆盖率与回归阈值（CI）
- 依赖地图与越层阻断（CI 规则）

## 当前完成
- 门面与特性门控：已接入初始化与回退
- 观测节点：已新增 `/proc/servicestats`、`/proc/servicehealth`、`/proc/features`、`/proc/initlazy`、`/proc/processstats`
- 文档：`ARCHITECTURE_OVERVIEW.md`、`DEPENDENCY_RULES.md`、`SYSCALLS_OVERVIEW.md`、`PROCFS.md`、`CONTRIBUTING.md`、`ROADMAP_6_12M.md`

## 待办
- 迁移优化实现到分层并完善特性化接入
- 初始化时间线与性能摘要节点
- CI 阻断越层依赖与性能回归报告

