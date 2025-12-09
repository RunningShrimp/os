# ProcFS 使用说明

## 节点
- `/proc/servicestats`：服务注册表统计（注册/查找次数、服务列表）
- `/proc/servicehealth`：服务健康状态（默认 5 秒超时，列出异常服务）
- `/proc/features`：内核特性开关状态（`fast_syscall`、`zero_copy`、`batch_syscalls`、`net_opt`、`sched_opt`、`lazy_init`）
- `/proc/initlazy`：触发延迟初始化（启用 `lazy_init` 特性时生效）
  - 触发后会在 `/proc/timeline` 追加事件：`lazy_init_start`、`lazy_net_init`、`lazy_graphics_init`、`lazy_web_init`、`lazy_init_complete`
- `/proc/processstats`：进程统计（总数、可运行、睡眠、僵尸）
- `/proc/perfsummary`：性能概览（服务统计 + 进程统计汇总）
- `/proc/timeline`：启动时间线（记录关键阶段的时间戳与标签）
- `/proc/timesummary`：时间线汇总（计算各阶段耗时与总耗时）
- `/proc/perfmonitor`：性能监控报告（系统调用与基础事件统计）
- `/proc/heapstats`：堆分配器统计（Buddy/Slab 使用与碎片情况）

## 读取示例
```
# 读取服务统计
cat /proc/servicestats

# 查看健康状态
cat /proc/servicehealth

# 查看特性开关
cat /proc/features

# 触发延迟初始化（需启用 lazy_init 特性）
cat /proc/initlazy

# 查看进程统计
cat /proc/processstats

# 查看性能概览
cat /proc/perfsummary

# 查看启动时间线
cat /proc/timeline

# 查看性能监控报告
cat /proc/perfmonitor

# 查看堆分配器统计
cat /proc/heapstats
```

## 说明
- 节点内容为即时生成，适用于观测与排障；建议结合日志与微基准使用。
- 后续将添加更多节点（初始化时间线、错误统计、性能摘要等）。
