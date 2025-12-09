# NOS 架构总览

## 分层结构

- core：arch、mm、cpu、sync、trap、types、collections
- subsystems：vfs、fs、net、ipc、process、time、drivers
- services：统一注册、发现与通信
- observability：debug、monitoring、profiling、tracing、metrics、symbols
- security：security、security_audit、formal_verification、error_handling、reliability

## 启动流程

- 入口：`kernel/src/main.rs` → 初始化核心与子系统 → 调度器
- 可选按需与延迟初始化：网络、图形、web 引擎等通过服务触发

## 系统调用与优化

- core：快速分发、批处理、缓存、校验
- services：`*_service` 与统一 traits
- optimizations：零拷贝、调度优化、网络优化

## 测试与基准

- 单元测试与主机模拟测试（mock/属性测试）
- 集成测试矩阵（架构 × 特性组合）
- 微基准与性能基线报告

