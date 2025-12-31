# Track B - 阶段1-1: 详细文件清单

## A. Enhanced/Optimized 文件完整列表

### A.1 内存管理 (Memory Management)

1. **subsystems/mm/percpu_allocator_v2.rs** (334 行)
   - 状态: 未被使用
   - 功能: 增强的 Per-CPU 分配器，带批分配和本地缓存
   - 决策: **集成**到主模块后删除

2. **subsystems/mm/optimized_page_allocator.rs**
   - 状态: 未知
   - 功能: 优化的页分配器
   - 决策: **需人工审查**

### A.2 网络子系统 (Network)

3. **subsystems/net/tcp_optimized.rs** (658 行)
   - 状态: 被测试代码引用
   - 功能: BBR 拥塞控制、零拷贝、连接池
   - 决策: **集成**到 tcp.rs 后删除

4. **subsystems/net/enhanced_network.rs**
   - 状态: 被 syscalls/network 引用
   - 功能: 增强网络管理器
   - 决策: **保留**（已在使用）

5. **subsystems/net/icmp_enhanced.rs**
   - 状态: 未知
   - 功能: 增强 ICMP 实现
   - 决策: **需审查**与 icmp.rs 的区别

### A.3 IPC 子系统

6. **subsystems/ipc/enhanced_ipc.rs** (1,046 行)
   - 状态: 被 ipc/mod.rs 引用
   - 功能: 完整的增强 IPC 系统（消息队列、共享内存、RPC 等）
   - 决策: **保留**（功能独特且完整）

### A.4 文件系统 (Filesystem)

7. **subsystems/fs/io_optimized.rs**
   - 状态: 0 引用
   - 功能: 优化的 I/O 路径
   - 决策: **删除**

8. **subsystems/fs/ext4_enhanced_impl.rs**
   - 状态: 未知
   - 功能: ext4 增强实现
   - 决策: **集成**到 ext4/mod.rs

9. **subsystems/fs/journaling_enhanced.rs**
   - 状态: 未知
   - 功能: 增强日志
   - 决策: **集成**到 journaling_fs.rs

### A.5 系统调用 (Syscalls)

10. **subsystems/syscalls/enhanced_error_handler.rs**
    - 状态: 未知
    - 功能: 增强错误处理
    - 决策: **集成**到统一错误处理框架

11. **subsystems/syscalls/ipc/enhanced_handlers.rs**
    - 状态: 被 enhanced_ipc 使用
    - 功能: 增强 IPC 系统调用处理
    - 决策: **保留**（与 enhanced_ipc 配套）

12. **subsystems/syscall/optimized_arg_handler.rs**
    - 状态: 未知
    - 功能: 优化的参数处理
    - 决策: **删除**（未使用）

13. **subsystems/syscalls/optimization/mod.rs**
    - 状态: 系统框架
    - 功能: 系统调用优化框架
    - 决策: **保留**（架构组件）

### A.6 进程管理

14. **subsystems/process/lock_optimized.rs**
    - 状态: 可选模块
    - 功能: 优化的进程表锁定
    - 决策: **保留**（性能分析工具，已有说明）

### A.7 同步原语

15. **subsystems/sync/rwlock_optimized.rs**
    - 状态: 未知
    - 功能: 优化的读写锁
    - 决策: **基准测试对比后决定**

### A.8 安全模块

16. **security/enhanced_permissions.rs**
    - 状态: 安全模块
    - 功能: 增强权限系统
    - 决策: **保留**（安全关键组件）

---

## B. 测试文件完整列表

### B.1 顶层测试文件

1. **tests.rs** - 主测试文件
2. **test_prelude.rs** - 测试前导
3. **test_macros.rs** - 测试宏
4. **test_reporting.rs** - 测试报告
5. **fuzz_testing_main.rs** - 模糊测试入口
   - 决策: **删除**（未集成）

### B.2 Benchmark 目录 (6 文件, 16 KB)

6. **benchmark/mod.rs** (221 行)
7. **benchmark/io.rs** (3,100 行)
8. **benchmark/memory.rs** (4,339 行)
9. **benchmark/network.rs** (2,544 行)
10. **benchmark/scheduler.rs** (2,371 行)
11. **benchmark/syscall.rs** (4,165 行)

### B.3 Testing 目录 (12 文件, 202 KB)

12. **testing/mod.rs** (4,785 行)
13. **testing/baseline.rs** (13,320 行)
14. **testing/benchmarks.rs** (15,709 行)
15. **testing/docs.rs** (12,425 行)
16. **testing/framework.rs** (21,936 行)
17. **testing/integration_suite.rs** (12,211 行)
18. **testing/integration_tests.rs** (21,222 行)
19. **testing/performance_tests.rs** (16,317 行)
20. **testing/security_tests.rs** (20,923 行)
21. **testing/stress_tests.rs** (28,025 行)
22. **testing/test_automation.rs** (27,227 行)
23. **testing/test_runner.rs** (23,017 行)

### B.4 Benches 目录

24. **benches/syscall_optimization_bench.rs** - 系统调用优化基准

### B.5 POSIX 测试 (7 文件)

25. **posix_tests/mod.rs** (3,328 行)
26. **posix_tests/core/mod.rs**
27. **posix_tests/core/basic_tests.rs**
28. **posix_tests/core/signal_tests.rs**
29. **posix_tests/core/thread_tests.rs**
30. **posix_tests/core/time_tests.rs**

31. **posix/advanced_tests.rs**
32. **posix/integration_tests.rs**
33. **posix/tests.rs**

### B.6 LibC 测试 (3 文件)

34. **libc/io_tests.rs**
35. **libc/memory_tests.rs**
36. **libc/standard_tests.rs**

### B.7 子系统测试文件

37. **subsystems/fs/tests.rs**
38. **subsystems/mm/tests.rs**
39. **subsystems/mm/tests/alloc_bench.rs**
40. **subsystems/net/tests.rs**
41. **subsystems/net/ipv6_tests.rs**
42. **subsystems/net/tcp/congestion_tests.rs**
43. **subsystems/process/tests.rs**
44. **subsystems/sync/tests.rs**
45. **subsystems/sync/futex_tests.rs**
46. **subsystems/ipc/tests.rs**
47. **subsystems/syscalls/tests.rs**
48. **subsystems/syscalls/advanced_mmap_tests.rs**
49. **subsystems/syscalls/advanced_mmap/tests.rs**
50. **subsystems/syscalls/memfd_test.rs**
51. **subsystems/syscalls/posix_integration_test.rs**
52. **subsystems/syscalls/posix_tests.rs**

### B.8 VFS 测试

53. **vfs/tests.rs**

### B.9 Sync 测试

54. **sync/tests.rs**

### B.10 Test 目录 (3 文件)

55. **test/mod.rs**
56. **test/integration.rs**
57. **test/benchmark_final.rs**

---

## C. 重复功能模块对比

### C.1 PerCPU 分配器对比

| 特性 | percpu_allocator.rs (379 行) | percpu_allocator_v2.rs (334 行) |
|------|------------------------------|---------------------------------|
| 本地缓存 | ❌ | ✅ 64 frames |
| 批分配 | ❌ | ✅ 32 frames/batch |
| 快速路径 | ❌ | ✅ O(1) 分配 |
| 缓存统计 | 基础 | ✅ 详细（命中率） |
| 负载均衡 | ❌ | ✅ 自动平衡 |
| 性能目标 | 功能性 | <20ns, >95% 命中率 |
| **使用情况** | 主模块 | 0 引用 |

**决策**: 集成 V2 功能到主模块

### C.2 TCP 实现对比

| 特性 | tcp.rs (820 行) | tcp_optimized.rs (658 行) |
|------|-----------------|---------------------------|
| 拥塞控制 | 基础 | ✅ Reno/Cubic/BBR |
| 零拷贝 | ❌ | ✅ |
| 连接池 | ❌ | ✅ 1024 连接 |
| 批处理 ACK | ❌ | ✅ 4 包聚合 |
| SACK | ❌ | ✅ |
| **使用情况** | 主模块 | 1 引用（测试） |

**决策**: 集成优化功能到主模块

---

## D. 需要人工审查的文件

### D.1 包含未实现代码的文件 (16 个)

1. **vfs/mount.rs** - 包含 unreachable!()
2. **subsystems/net/processor.rs** - 包含未实现代码
3. **subsystems/net/icmp_enhanced.rs** - 包含未实现代码
4. **subsystems/net/mod.rs** - 包含未实现代码
5. **subsystems/net/icmp.rs** - 包含未实现代码
6. **subsystems/net/ipv6/icmpv6.rs** - 包含未实现代码
7. **subsystems/net/udp.rs** - 包含未实现代码
8. **subsystems/formal_verification/type_checker.rs** - 包含未实现代码
9. **subsystems/mm/mpu.rs** - 包含未实现代码
10. **subsystems/sync/futex_validation.rs** - 包含未实现代码
11. **subsystems/sync/mod.rs** - 包含未实现代码
12. **subsystems/sync/lazy.rs** - 包含未实现代码
13. **subsystems/fs/api/mod.rs** - 包含未实现代码
14. **subsystems/process/thread.rs** - 包含未实现代码
15. **sync/mod.rs** - 包含未实现代码
16. **error/unified.rs** - 包含未实现代码

**建议**: 逐个审查，决定完成实现或删除

---

## E. 引用分析

### E.1 0 引用的 Enhanced/Optimized 文件

- `subsystems/mm/percpu_allocator_v2.rs` - **0 引用**
- `subsystems/fs/io_optimized.rs` - **0 引用**

### E.2 被 1 个文件引用的

- `subsystems/net/tcp_optimized.rs` - 被 testing/performance_tests.rs 引用
- `subsystems/net/enhanced_network.rs` - 被 syscalls/network/mod.rs 引用
- `subsystems/ipc/enhanced_ipc.rs` - 被 subsystems/ipc/mod.rs 引用

---

## F. 代码统计

### F.1 按类别统计

| 类别 | 文件数 | 总行数 |
|------|--------|--------|
| Enhanced/Optimized | 16 | 11,840 |
| 测试文件 | 54 | 11,769 |
| **总计** | **70** | **23,609** |

### F.2 可清理代码量

| 优先级 | 预计删除文件 | 预计减少行数 |
|--------|-------------|-------------|
| 高（0 引用） | 3 | ~700 |
| 中（重复整合） | 4 | ~992 |
| 低（审查后） | 2-8 | ~500 |
| **总计** | **9-15** | **2,192** |

---

## G. 执行跟踪

### G.1 已完成

- ✅ 识别所有临时/实验性文件
- ✅ 分析重复功能模块
- ✅ 统计代码规模
- ✅ 制定分类决策
- ✅ 创建清理报告

### G.2 待执行

- ⏳ 阶段 1: 删除 0 引用文件
- ⏳ 阶段 2: 整合重复模块
- ⏳ 阶段 3: 文档和审查
- ⏳ 阶段 4: 测试验证
