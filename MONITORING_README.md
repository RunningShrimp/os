# NOS 内核性能监控框架

## 概述

本框架为NOS内核提供了完整的性能监控基础设施,支持指标收集、性能采样、CPU分析和数据导出功能。

## 快速开始

### 1. 使用性能采样器

```rust
use kernel::monitoring::sampling::PerformanceSampler;

// 创建采样器(名称, 采样率N, 缓冲区大小)
static SAMPLER: PerformanceSampler = 
    PerformanceSampler::new("my_operation", 100, 1000);

// 包装函数进行采样
fn my_function() {
    SAMPLER.sample("my_function", || {
        // 你的代码
        do_work();
    });
}

// 获取统计信息
let summary = SAMPLER.summary();
println!("P95延迟: {} ns", summary.p95);
```

### 2. 使用指标收集

```rust
use kernel::monitoring::metrics::get_metrics_collector;

// 获取全局收集器
let collector = get_metrics_collector();

// 记录指标
collector.increment_counter("operations_total", 1);
collector.set_gauge("memory_usage_bytes", 1024);

// 批量获取所有指标
let metrics = collector.collect_metrics();
```

### 3. 使用CPU Profiler

```rust
use kernel::monitoring::profiler::{init_profiler, start_profiling, stop_profiling};

// 初始化(采样间隔1ms, 最大100000样本)
init_profiler(1_000_000, 100_000);

// 开始profiling
start_profiling();

// 运行工作负载
run_workload();

// 停止profiling
stop_profiling();

// 导出数据
if let Some(profiler) = get_profiler() {
    println!("{}", profiler.format_flame_graph());
}
```

### 4. 导出数据

```rust
use kernel::monitoring::export::{export_metrics, ExportFormat};
use kernel::monitoring::metrics::get_metrics_collector;

let collector = get_metrics_collector();

// Prometheus格式
let prometheus = export_metrics(&collector, ExportFormat::Prometheus);

// JSON格式
let json = export_metrics(&collector, ExportFormat::Json);

// 文本格式
let text = export_metrics(&collector, ExportFormat::Text);
```

## 架构

```
┌─────────────────────────────────────────────────────────┐
│                     应用代码                              │
└───────────────────┬─────────────────────────────────────┘
                    │
┌───────────────────▼─────────────────────────────────────┐
│              性能采样器 (Sampling)                        │
│  - PerformanceSampler                                    │
│  - 可配置采样率                                          │
│  - 循环缓冲区                                            │
└───────────────────┬─────────────────────────────────────┘
                    │
┌───────────────────▼─────────────────────────────────────┐
│              指标收集器 (Metrics)                         │
│  - MetricsCollector                                      │
│  - Counter/Gauge/Histogram                               │
│  - 全局注册表                                            │
└───────────────────┬─────────────────────────────────────┘
                    │
┌───────────────────▼─────────────────────────────────────┐
│               数据导出 (Export)                           │
│  - Prometheus格式                                        │
│  - JSON格式                                              │
│  - 文本格式                                              │
└───────────────────┬─────────────────────────────────────┘
                    │
┌───────────────────▼─────────────────────────────────────┐
│           /proc 或 /sysfs 接口                            │
└──────────────────────────────────────────────────────────┘
```

## 模块说明

### sampling.rs - 性能采样器

提供低开销的性能采样功能,支持:
- 可配置采样率(1-N)
- 百分位数统计(P50/P95/P99)
- 最小开销快速路径
- 运行时启用/禁用

**适用场景:**
- 系统调用延迟监控
- 内存分配性能分析
- I/O操作性能跟踪
- 函数执行时间统计

### profiler.rs - CPU Profiler

提供CPU性能分析功能,支持:
- 定时器采样
- 火焰图生成
- 栈回溯(待实现)
- 符号解析(待实现)

**适用场景:**
- CPU热点分析
- 函数调用频率统计
- 性能瓶颈识别
- 优化方向指导

### metrics.rs - 指标收集

提供系统指标收集功能,支持:
- Counter(单调递增)
- Gauge(可增可减)
- Histogram(分布统计)
- 自动注册系统指标

**预定义指标:**
- 系统调用: syscalls_total, syscalls_success_total等
- 内存: memory_used_bytes, memory_free_bytes等
- 调度: context_switches_total等
- 锁: locks_spin_acquire_total等

### export.rs - 数据导出

提供多格式数据导出,支持:
- Prometheus文本格式
- JSON格式
- 纯文本格式
- 流式导出

## 集成示例

### 系统调用集成

```rust
static SYSCALL_SAMPLER: PerformanceSampler = 
    PerformanceSampler::new("syscall", 100, 1000);

pub fn handle_syscall(id: u64) -> i64 {
    get_metrics_collector().increment_counter("syscalls_total", 1);
    
    SYSCALL_SAMPLER.sample("syscall_dispatch", || {
        dispatch_syscall(id)
    })
}
```

### 内存分配集成

```rust
static ALLOC_SAMPLER: PerformanceSampler = 
    PerformanceSampler::new("memory_alloc", 1000, 500);

pub fn kmalloc(size: usize) -> *mut u8 {
    ALLOC_SAMPLER.sample("allocate", || {
        internal_allocate(size)
    })
}
```

## 性能特征

| 操作 | 开销 | 说明 |
|------|------|------|
| 指标增量 | <10ns | 原子fetch_add |
| 采样检查 | <20ns | 原子操作 + 分支 |
| 采样记录 | ~150ns | 时间戳 + 缓冲区写入 |
| 批量收集 | ~100ns/指标 | 遍历注册表 |

## 配置

### 采样率配置

| 场景 | 推荐采样率 | 说明 |
|------|-----------|------|
| 系统调用 | 100 (1%) | 平衡精度与开销 |
| 内存分配 | 1000 (0.1%) | 频繁操作,低采样率 |
| 调度器 | 50 (2%) | 关键路径,高采样率 |
| 文件I/O | 200 (0.5%) | 中等频率 |

### 缓冲区大小

| 组件 | 默认大小 | 说明 |
|------|---------|------|
| PerformanceSampler | 1000样本 | 循环缓冲区 |
| Profiler | 100000样本 | 可配置 |

## 导出接口

### /proc/metrics
Prometheus格式的系统指标

### /proc/sampling
采样统计信息(文本格式)

### /proc/profiler
CPU profiling数据(JSON格式)

## 测试

```bash
# 运行所有monitoring测试
cargo test --lib monitoring

# 运行特定模块测试
cargo test --lib metrics
cargo test --lib sampling
cargo test --lib profiler
cargo test --lib export
```

## 文档

- [执行日志](./trackI_execution_log.md) - 详细的实施记录
- [验证报告](./trackI_verification.md) - 完整的验证清单
- [集成示例](./kernel/src/monitoring/examples/integration_example.rs) - 代码示例

## 贡献

欢迎贡献!请查看:
- [TODO列表](./trackI_verification.md#后续改进方向)
- [代码规范](./DEVELOPER_GUIDE.md)

## 许可

与NOS内核项目相同

## 联系方式

- Issue tracker: GitHub Issues
- 文档: [项目Wiki](./)

---

**版本**: 1.0.0
**最后更新**: 2025-12-31
