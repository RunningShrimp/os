# NOS 内核调试工具

本文档描述了为 NOS 内核实现的完整调试工具集。

## 概述

内核调试工具提供全面的系统级调试支持，包括：
- 内核调试器 (KDB)
- 动态调试控制
- 崩溃转储生成
- 符号解析
- 栈回溯
- 变量监视点

## 模块说明

### 1. 内核调试器 (KDB) - `kernel/src/debug/kdb.rs`

**功能：**
- 命令行调试接口
- 断点管理（软件/硬件断点）
- 单步执行（Step Into/Over/Out）
- 内存检查和修改
- 寄存器查看和修改
- 调用栈回溯
- 反汇编支持

**主要类型：**
- `KdbInterpreter`: 命令解释器
- `BreakpointManager`: 断点管理器
- `MemoryViewer`: 内存查看器
- `RegisterViewer`: 寄存器查看器

**使用示例：**
```rust
use kernel::debug::kdb::KdbInterpreter;

let mut kdb = KdbInterpreter::new();
kdb.init().unwrap();

// 设置断点
kdb.execute_command("break 0x1000").unwrap();

// 单步执行
kdb.execute_command("step").unwrap();

// 查看寄存器
kdb.execute_command("registers").unwrap();
```

**可用命令：**
```
break/b <addr>      - Set breakpoint at address
clear <id>          - Clear breakpoint
continue/c          - Continue execution
step/s              - Single step into
next/n              - Single step over
finish/fin          - Step out of current function
print/p <expr>      - Print expression
x/nx <addr>         - Examine memory
info <topic>        - Show information (breakpoints, registers, threads)
set <var> = <val>   - Set variable value
backtrace/bt        - Show stack trace
frame <n>           - Select stack frame
thread <id>         - Select thread
quit/q              - Quit debugger
history             - Show command history
alias <name>=<cmd>  - Define alias
disassemble <addr>  - Disassemble code
registers/regs      - Show registers
```

### 2. 动态调试控制 - `kernel/src/debug/dynamic_debug.rs`

**功能：**
- 基于模式的调试启用/禁用
- 运行时调试级别调整
- 分类调试输出
- 通配符模式匹配
- 调试统计信息

**主要类型：**
- `DynamicDebugManager`: 动态调试管理器
- `DebugPattern`: 调试模式（包含/排除/通配符）
- `DebugCategory`: 调试类别
- `DebugOutputBuffer`: 调试输出缓冲区
- `DebugStatistics`: 调试统计信息

**调试级别：**
- `Off`: 关闭
- `Error`: 仅错误
- `Warn`: 警告及以上
- `Info`: 信息及以上
- `Debug`: 调试及以上
- `Trace`: 所有输出

**使用示例：**
```rust
use kernel::debug::dynamic_debug::{DynamicDebugManager, DebugLevel};

let manager = DynamicDebugManager::new();
manager.init().unwrap();

// 设置全局调试级别
manager.set_debug_level(DebugLevel::Debug);

// 启用特定类别
manager.enable_category("memory", DebugLevel::Trace).unwrap();

// 添加调试模式
manager.add_pattern("kernel/src/memory/*").unwrap();

// 使用宏输出调试信息
dynamic_debug!("memory", DebugLevel::Debug, "Allocated {} bytes", size);
```

### 3. 崩溃转储生成 - `kernel/src/debug/crash_dump.rs`

**功能：**
- 内核崩溃时自动生成转储
- CPU 状态保存（寄存器、控制寄存器、FPU、SIMD）
- 内存快照（物理/虚拟内存）
- 进程和线程信息
- 设备和网络状态
- 转储压缩
- 转储分析

**主要类型：**
- `CrashDumpManager`: 崩溃转储管理器
- `CrashDump`: 崩溃转储
- `CrashReason`: 崩溃原因
- `CpuState`: CPU 状态
- `MemorySnapshot`: 内存快照
- `DumpAnalysis`: 转储分析结果

**崩溃原因类型：**
- `KernelPanic`: 内核恐慌
- `DoubleFault`: 双重错误
- `TripleFault`: 三重错误
- `PageFault`: 页错误
- `GeneralProtectionFault`: 一般保护错误
- `SegmentationFault`: 段错误
- `AssertionFailed`: 断言失败
- `DeadlockDetected`: 死锁检测
- `StackOverflow`: 栈溢出
- `HeapCorruption`: 堆损坏
- `OutOfMemory`: 内存耗尽

**使用示例：**
```rust
use kernel::debug::crash_dump::{CrashDumpManager, CrashReason};

let mut manager = CrashDumpManager::new();
manager.init().unwrap();

// 生成崩溃转储
let reason = CrashReason::KernelPanic("Test panic".to_string());
let dump_id = manager.generate_dump(reason).unwrap();

// 分析转储
let analysis = manager.analyze_dump(dump_id).unwrap();
println!("Crash location: {:?}", analysis.crash_location);
println!("Recommendations: {:?}", analysis.recommendations);
```

### 4. 符号解析 - `kernel/src/debug/symbol.rs`

**功能：**
- ELF 符号表解析
- 地址到符号查找
- 符号到地址查找
- 函数边界查找
- 源代码行号映射
- 符号统计信息

**主要类型：**
- `SymbolResolver`: 符号解析器
- `SymbolTable`: 符号表
- `Symbol`: 符号
- `SymbolLookupResult`: 符号查找结果
- `SourceLocation`: 源代码位置

**使用示例：**
```rust
use kernel::debug::symbol::SymbolResolver;

let mut resolver = SymbolResolver::new();

// 加载 ELF 符号表
resolver.load_elf_symbols(&elf_data, base_address).unwrap();

// 根据地址查找符号
let result = resolver.lookup_address(0x1000);
if let Some(symbol) = result {
    println!("Function: {}", symbol.name);
    println!("Offset: 0x{:x}", symbol.offset);
}

// 根据名称查找符号
let results = resolver.lookup_name("function_name");
for result in results {
    println!("Found at 0x{:x}", result.address);
}
```

### 5. 栈回溯 - `kernel/src/debug/traceback.rs`

**功能：**
- 栈回溯捕获
- 调用栈重建
- 符号化栈帧
- 多种输出格式
- 栈回溯比较
- 公共祖先查找

**主要类型：**
- `StackTraceback`: 栈回溯器
- `StackFrame`: 栈帧
- `OutputFormat`: 输出格式（Short/Standard/Verbose/JSON）
- `TracebackOptions`: 回溯选项
- `StackDiff`: 栈差异

**使用示例：**
```rust
use kernel::debug::traceback::{StackTraceback, OutputFormat};

let traceback = StackTraceback::new();
traceback.set_output_format(OutputFormat::Verbose);

// 捕获当前栈回溯
let frames = traceback.capture(None).unwrap();

// 格式化输出
let output = traceback.format_traceback(&frames);
println!("{}", output);

// 便捷函数
use kernel::debug::traceback::print_traceback;
println!("Current stack:\n{}", print_traceback());
```

### 6. 变量监视点 - `kernel/src/debug/watch.rs`

**功能：**
- 变量监视点设置
- 硬件断点（x86_64 DR0-DR3）
- 数据断点（读/写/执行）
- 监视条件（等于/大于/位掩码等）
- 实时监控
- 触发历史记录

**主要类型：**
- `WatchpointManager`: 监视点管理器
- `Watchpoint`: 监视点
- `WatchType`: 监视类型（Software/Hardware/DataBreakpoint）
- `AccessType`: 访问类型（Read/Write/ReadWrite/Execute）
- `WatchCondition`: 监视条件
- `WatchpointTrigger`: 触发记录

**使用示例：**
```rust
use kernel::debug::watch::{WatchpointManager, AccessType, WatchType};

let manager = WatchpointManager::new();

// 设置硬件断点
let id = manager.set_hardware_breakpoint(
    0x1000,
    AccessType::Write,
    8
).unwrap();

// 监视变量变化
let id = manager.watch_variable(
    0x2000,
    4,
    "my_variable".to_string()
).unwrap();

// 检查监视点触发
let trigger = manager.check_watchpoint(
    0x2000,
    AccessType::Write,
    Some(vec![1, 2, 3, 4]),
    Some(1),  // thread_id
    Some(100), // process_id
);

// 获取统计信息
let stats = manager.get_statistics();
println!("Total watchpoints: {}", stats.total_watchpoints);
println!("Total triggers: {}", stats.total_triggers);
```

## 集成到内核

### 初始化

在内核初始化时添加调试工具初始化：

```rust
// 在 kernel/src/core/init.rs 中
use kernel::debug::{
    kdb::KdbInterpreter,
    dynamic_debug::init_dynamic_debug,
    crash_dump::CrashDumpManager,
    symbol::SymbolResolver,
    traceback::StackTraceback,
    watch::WatchpointManager,
};

pub fn init_debug_subsystem() {
    // 初始化动态调试
    init_dynamic_debug();

    // 初始化崩溃转储管理器
    let mut crash_manager = CrashDumpManager::new();
    crash_manager.init().unwrap();

    // 加载内核符号表
    let mut symbol_resolver = SymbolResolver::new();
    symbol_resolver.load_elf_symbols(&kernel_elf, kernel_base).unwrap();

    log::info!("Debug subsystem initialized");
}
```

### 在恐慌处理程序中使用

```rust
use kernel::debug::crash_dump::{CrashDumpManager, CrashReason};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 生成崩溃转储
    let reason = CrashReason::KernelPanic(info.to_string());
    if let Ok(dump_id) = crash_manager.generate_dump(reason) {
        log::error!("Crash dump {} generated", dump_id);
    }

    // 打印栈回溯
    use kernel::debug::traceback::print_traceback;
    println!("Stack trace:\n{}", print_traceback());

    loop {}
}
```

## 测试

所有模块都包含单元测试：

```bash
# 测试所有调试模块
cargo test --lib debug::kdb
cargo test --lib debug::dynamic_debug
cargo test --lib debug::crash_dump
cargo test --lib debug::symbol
cargo test --lib debug::traceback
cargo test --lib debug::watch
```

## 限制和注意事项

1. **硬件断点限制**: x86_64 架构仅支持 4 个硬件断点（DR0-DR3）
2. **内存对齐**: 硬件断点要求地址对齐（1/2/4/8 字节）
3. **性能影响**: 软件监视点可能导致显著的性能下降
4. **符号信息**: 需要 ELF 调试信息才能进行完整的符号化
5. **转储大小**: 崩溃转储可能非常大，建议启用压缩

## 未来改进

1. 添加远程调试支持（GDB 协议）
2. 实现 JIT 符号解析
3. 添加性能分析工具集成
4. 支持实时跟踪点
5. 添加更多的条件表达式支持
6. 实现自动化的崩溃分析
7. 添加调试脚本支持

## 参考文档

- [Intel SDM - Debug Registers](https://software.intel.com/content/www/us/en/develop/articles/intel-sdm.html)
- [DWARF Debugging Format](https://dwarfstd.org/)
- [ELF Format](https://refspecs.linuxfoundation.org/elf/elf.pdf)
- [Linux Kernel Debugging](https://www.kernel.org/doc/html/latest/)

## 许可证

遵循 NOS 内核项目的许可证。
