# 系统调用概览与分发策略

## 范围与编号
- 进程：0x1000-0x1FFF
- 文件 I/O：0x2000-0x2FFF
- 内存：0x3000-0x3FFF
- 网络：0x4000-0x4FFF
- 信号：0x5000-0x5FFF
- 时间：0x6000-0x6FFF
- 文件系统：0x7000-0x7FFF
- 线程：0x8000-0x8FFF
- 零拷贝：0x9000-0x9FFF
- epoll：0xA000-0xAFFF
- GLib：0xB000-0xBFFF
- AIO：0xC000-0xCFFF
- 消息队列：0xD000-0xDFFF
- 实时调度：0xE000-0xEFFF
- 安全：0xF000-0xFFFF

## 分发
- 快路径：`fast_dispatcher` 与性能优化分发器；失败回退至遗留分发
- 遗留分发：按范围路由到 `process/fs/mm/network/signal/time/...`
- 特性化回退：`zero_copy`/`sched_opt` 不启用时回退到 `zero_copy`/`sched`

## 优化特性
- `fast_syscall`：启用快速分发与性能优化器
- `batch_syscalls`：批处理
- `zero_copy`：零拷贝优化
- `net_opt`：网络优化
- `sched_opt`：调度优化

## 观测与测试
- 服务层度量：注册/查找统计
- 微基准：高频系统调用快路径与批处理
- 覆盖率：关键模块设定阈值

