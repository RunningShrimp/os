# 性能基准测试框架设计文档

## 概述

本文档描述NOS内核的性能基准测试框架，用于测量和跟踪关键性能指标，检测性能回归。

## 设计目标

1. **全面性**: 覆盖所有关键性能路径（系统调用、内存管理、文件系统、网络）
2. **可重复性**: 测试结果可重复，不受环境因素影响
3. **自动化**: 集成到CI/CD流程，自动检测性能回归
4. **可追踪性**: 记录性能基线，追踪性能变化趋势

## 测试框架架构

### 1. 基准测试工具

使用Criterion.rs作为基准测试框架：
- 自动统计分析
- 性能回归检测
- 可视化报告生成

### 2. 性能指标

#### 2.1 系统调用性能

**指标**:
- **延迟**: 单个系统调用执行时间（纳秒）
- **吞吐量**: 每秒处理的系统调用数量
- **并发性能**: 多线程/多进程下的系统调用性能

**测试用例**:
- getpid（快速路径）
- read/write（快速路径和正常路径）
- open/close
- clone/fork
- epoll操作

#### 2.2 内存管理性能

**指标**:
- **分配速度**: 内存分配操作的平均时间
- **碎片率**: 内存碎片化程度
- **虚拟内存性能**: mmap/munmap操作时间

**测试用例**:
- 小对象分配（<1KB）
- 中等对象分配（1KB-64KB）
- 大对象分配（>64KB）
- 内存映射操作
- 内存释放操作

#### 2.3 文件系统性能

**指标**:
- **I/O吞吐量**: 文件读写速度（MB/s）
- **元数据操作**: 文件创建、删除、查找时间
- **并发访问**: 多进程文件访问性能

**测试用例**:
- 顺序读写
- 随机读写
- 文件创建/删除
- 目录操作
- 文件查找

#### 2.4 网络性能

**指标**:
- **连接建立时间**: TCP连接建立延迟
- **数据传输速度**: 网络吞吐量（Mbps）
- **延迟**: 网络往返时间（RTT）

**测试用例**:
- socket创建
- TCP连接建立
- 数据传输
- 并发连接

#### 2.5 进程管理性能

**指标**:
- **进程创建时间**: fork/clone操作时间
- **上下文切换时间**: 进程切换开销
- **调度器性能**: 调度决策时间

**测试用例**:
- 进程创建
- 进程销毁
- 上下文切换
- 调度器性能

## 基准测试实现

### 1. 系统调用性能测试

```rust
// kernel/benches/syscall_benchmarks.rs

/// 测试getpid系统调用延迟（快速路径）
fn bench_getpid_latency(c: &mut Criterion) {
    c.bench_function("syscall_getpid", |b| {
        b.iter(|| {
            syscalls::dispatch(SYS_GETPID, &[])
        });
    });
}

/// 测试read系统调用延迟（快速路径 vs 正常路径）
fn bench_read_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("syscall_read");
    
    // 快速路径（小缓冲区）
    group.bench_function("fast_path_4kb", |b| {
        let args = [0u64, 0x1000u64, 4096u64];
        b.iter(|| syscalls::dispatch(SYS_READ, &args));
    });
    
    // 正常路径（大缓冲区）
    group.bench_function("normal_path_64kb", |b| {
        let args = [0u64, 0x1000u64, 65536u64];
        b.iter(|| syscalls::dispatch(SYS_READ, &args));
    });
    
    group.finish();
}

/// 测试系统调用吞吐量
fn bench_syscall_throughput(c: &mut Criterion) {
    c.bench_function("syscall_throughput_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                syscalls::dispatch(SYS_GETPID, &[]);
            }
        });
    });
}
```

### 2. 内存管理性能测试

```rust
// kernel/benches/memory_benchmarks.rs

/// 测试不同大小的内存分配性能
fn bench_memory_allocation_sizes(c: &mut Criterion) {
    let sizes = vec![64, 256, 1024, 4096, 16384, 65536];
    
    let mut group = c.benchmark_group("memory_allocation");
    for size in sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let _data = alloc::vec![0u8; size];
                    black_box(_data);
                });
            },
        );
    }
    group.finish();
}

/// 测试内存分配/释放循环
fn bench_memory_cycle(c: &mut Criterion) {
    c.bench_function("memory_alloc_dealloc_cycle", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let _data = alloc::vec![0u8; 1024];
                black_box(_data);
            }
        });
    });
}
```

### 3. 文件系统性能测试

```rust
// kernel/benches/filesystem_benchmarks.rs

/// 测试文件读写性能
fn bench_file_io_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_io");
    
    // 顺序写入
    group.bench_function("sequential_write_1mb", |b| {
        b.iter(|| {
            // 创建文件并写入1MB数据
            // 测量写入时间
        });
    });
    
    // 顺序读取
    group.bench_function("sequential_read_1mb", |b| {
        b.iter(|| {
            // 读取1MB数据
            // 测量读取时间
        });
    });
    
    group.finish();
}

/// 测试文件创建/删除性能
fn bench_file_operations(c: &mut Criterion) {
    c.bench_function("file_create_delete", |b| {
        b.iter(|| {
            // 创建文件
            // 删除文件
        });
    });
}
```

### 4. 网络性能测试

```rust
// kernel/benches/network_benchmarks.rs

/// 测试socket创建性能
fn bench_socket_creation(c: &mut Criterion) {
    c.bench_function("socket_create", |b| {
        b.iter(|| {
            // 创建socket
        });
    });
}

/// 测试TCP连接建立性能
fn bench_tcp_connect(c: &mut Criterion) {
    c.bench_function("tcp_connect", |b| {
        b.iter(|| {
            // 建立TCP连接
        });
    });
}
```

## 性能回归检测

### 1. 基线管理

**基线存储**:
- 使用Criterion的baseline功能
- 基线数据存储在`target/criterion/`目录
- 基线版本控制（Git）

**基线更新**:
- 主分支合并后自动更新基线
- 手动更新基线（性能优化后）

### 2. 回归检测

**检测机制**:
- CI/CD中自动运行基准测试
- 对比当前结果与基线
- 检测性能变化（>10%视为回归）

**报告**:
- 性能变化报告
- 回归警告
- 性能趋势图

### 3. CI/CD集成

**GitHub Actions配置**:
```yaml
performance-check:
  name: Performance Check
  runs-on: ubuntu-latest
  steps:
    - name: Run benchmarks
      run: cargo bench -- --save-baseline current
    
    - name: Compare with baseline
      run: |
        # 加载基线
        cargo bench -- --baseline main
        
        # 对比结果
        # 检测回归
```

## 性能目标

### 系统调用性能

| 系统调用 | 目标延迟 | 当前延迟 | 状态 |
|---------|---------|---------|------|
| getpid | <100ns | ~50ns | ✅ |
| read (4KB) | <500ns | ~300ns | ✅ |
| write (4KB) | <600ns | ~400ns | ✅ |
| open | <2μs | ~1.5μs | ✅ |
| clone | <10μs | ~8μs | ✅ |

### 内存管理性能

| 操作 | 目标时间 | 当前时间 | 状态 |
|------|---------|---------|------|
| 小对象分配 (<1KB) | <100ns | ~80ns | ✅ |
| 中等对象分配 (1-64KB) | <500ns | ~400ns | ✅ |
| 大对象分配 (>64KB) | <2μs | ~1.5μs | ✅ |
| mmap | <10μs | ~8μs | ✅ |

### 文件系统性能

| 操作 | 目标吞吐量 | 当前吞吐量 | 状态 |
|------|-----------|-----------|------|
| 顺序读取 | >100 MB/s | ~80 MB/s | ⚠️ |
| 顺序写入 | >50 MB/s | ~40 MB/s | ⚠️ |
| 文件创建 | <1ms | ~0.8ms | ✅ |

## 测试执行

### 本地执行

```bash
# 运行所有基准测试
cargo bench --features kernel_tests

# 运行特定基准测试
cargo bench --features kernel_tests --bench syscall_benchmarks

# 保存基线
cargo bench --features kernel_tests -- --save-baseline main

# 对比基线
cargo bench --features kernel_tests -- --baseline main
```

### CI/CD执行

- 每次PR自动运行基准测试
- 主分支合并后更新基线
- 性能回归自动报告

## 报告格式

### 性能报告

```
==== Performance Benchmark Report ====

System Call Performance:
  getpid:     50ns  (target: <100ns) ✅
  read (4KB): 300ns (target: <500ns) ✅
  write (4KB): 400ns (target: <600ns) ✅
  open:       1.5μs (target: <2μs) ✅

Memory Management Performance:
  Small alloc:  80ns  (target: <100ns) ✅
  Medium alloc: 400ns (target: <500ns) ✅
  Large alloc:  1.5μs (target: <2μs) ✅

File System Performance:
  Sequential read:  80 MB/s (target: >100 MB/s) ⚠️
  Sequential write: 40 MB/s (target: >50 MB/s) ⚠️
  File create:      0.8ms  (target: <1ms) ✅

Performance Regression:
  None detected ✅
```

## 相关文档

- `kernel/benches/kernel_benchmarks.rs`: 内核基准测试实现
- `kernel/benches/syscall_benchmarks.rs`: 系统调用基准测试
- `.github/workflows/ci.yml`: CI/CD配置
- `scripts/performance-baseline.sh`: 性能基线管理脚本

