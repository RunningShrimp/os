# 内核特性开关使用说明

## 可用特性
- `fast_syscall`：启用快速系统调用分发与性能优化器
- `zero_copy`：启用零拷贝 I/O 优化
- `batch_syscalls`：启用批量系统调用
- `net_opt`：启用网络栈优化
- `sched_opt`：启用调度器优化
- `lazy_init`：对网络/图形/Web 子系统采用延迟初始化
- `observability`：启用观测系统（metrics/health/alerting 及 /proc 时间线）
- `debug_subsystems`：启用调试/监控/剖析/追踪/指标/符号子系统
- `cloud_native`：启用云原生特性

## 启用方式
- 单项启用：`cargo build -p kernel --features fast_syscall`
- 组合启用：`cargo build -p kernel --features "fast_syscall,zero_copy,sched_opt"`
- 观测/调试：`cargo build -p kernel --features "observability,debug_subsystems"`
- 云原生：`cargo build -p kernel --features cloud_native`
- 测试场景：`cargo test -p kernel --features "kernel_tests,fast_syscall"`

## 运行时观测
- `/proc/features`：查看当前启用的特性列表
- `/proc/servicestats`：服务注册与查找统计
- `/proc/perfsummary`：性能概览（服务 + 进程统计）
- `/proc/timeline`：启动时间线（关键阶段标签）
- `/proc/initlazy`：在启用 `lazy_init` 时触发延迟初始化

## 兼容与回退
- 未启用特性时默认使用保守实现，不改变现有行为
- 已在分发与路由中对 `zero_copy`、`sched_opt` 加入回退逻辑
