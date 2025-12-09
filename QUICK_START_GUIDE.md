# NOS改进计划 - 快速启动指南

> **创建日期**: 2025-12-09  
> **目标**: 6个月内将NOS从中等水平提升到优秀水平

---

## 📋 已完成的准备工作

✅ **全面评估报告** - 分析了项目当前状态  
✅ **TODO清理计划** (`docs/TODO_CLEANUP_PLAN.md`) - 261个TODO的详细追踪  
✅ **改进路线图** (`NOS_IMPROVEMENT_ROADMAP.md`) - 完整的6个月实施计划  
✅ **待办事项列表** - 15个主要任务已创建

---

## 🎯 核心目标

| 指标 | 当前 | 6个月目标 |
|------|------|-----------|
| 可维护性评分 | 6.2/10 | **8.5/10** |
| 系统性能 | 基线 | **+100-300%** |
| 功能完整性 | 50% | **80%+** |
| TODO技术债务 | 261个 | **<50个** |

---

## 🚀 立即开始：第一周任务

### 第1天：根目录清理（2小时）
```bash
# 1. 创建目录结构
cd /Users/didi/Desktop/nos
mkdir -p temp/build_logs temp/analysis docs/reports

# 2. 移动临时文件
mv *_errors.txt temp/build_logs/
mv *_output.txt temp/build_logs/
mv *error*.txt temp/analysis/
mv *REPORT.md docs/reports/
mv *PLAN.md docs/reports/

# 3. 更新.gitignore
echo "temp/" >> .gitignore
echo "*.profraw" >> .gitignore

# 4. 提交清理
git add .
git commit -m "chore: 清理根目录，建立项目结构规范"
```

**预期结果**: 根目录从25+个文件减少到<10个

---

### 第2-3天：进程管理核心功能（16小时）

#### 任务1: 实现基础进程操作
**文件**: `kernel/src/syscalls/process_service/handlers.rs`

```rust
// 当前状态：所有函数都是TODO占位符
// 需要实现的函数（优先级排序）：
// 1. sys_getpid() - 最简单，先实现
// 2. sys_getppid() - 简单
// 3. sys_exit() - 核心功能
// 4. sys_fork() - 复杂但关键
// 5. sys_execve() - 复杂但关键
// 6. sys_waitpid() - 需要进程状态管理
// 7. sys_kill() - 需要信号支持
```

**实施步骤**:
1. 阅读现有进程管理代码 (`kernel/src/process/`)
2. 实现getpid/getppid（参考现有实现）
3. 实现exit逻辑（进程清理）
4. 实现fork逻辑（进程复制）
5. 添加单元测试

**关键文件**:
- `kernel/src/syscalls/process_service/handlers.rs` - 系统调用handler
- `kernel/src/process/mod.rs` - 进程管理核心
- `kernel/src/syscalls/process.rs` - 现有实现参考

---

### 第4-5天：文件系统核心功能（16小时）

#### 任务2: 实现基础文件I/O
**文件**: `kernel/src/syscalls/fs_service/handlers.rs`

```rust
// 实现优先级：
// 1. sys_open() - 打开文件
// 2. sys_close() - 关闭文件
// 3. sys_read() - 读取数据
// 4. sys_write() - 写入数据
// 5. sys_lseek() - 移动文件指针
// 6. sys_stat() / sys_fstat() - 获取文件信息
```

**实施步骤**:
1. 研究VFS接口 (`kernel/src/fs/`)
2. 实现open/close（文件描述符管理）
3. 实现read/write（调用VFS接口）
4. 实现stat（获取inode信息）
5. 添加集成测试

---

## 📅 第一周时间表

| 时间 | 任务 | 预计产出 |
|------|------|----------|
| **第1天** | 根目录清理 | 整洁的项目结构 |
| **第2-3天** | 进程管理实现 | 基础进程操作可用 |
| **第4-5天** | 文件系统实现 | 基础文件I/O可用 |
| **第6天** | 测试和验证 | 所有测试通过 |
| **第7天** | 文档和总结 | 进度报告 |

---

## 🔧 开发环境设置

### 必需工具
```bash
# 1. 确认Rust工具链
rustc --version
cargo --version

# 2. 安装开发工具
cargo install cargo-watch   # 自动重新编译
cargo install cargo-expand  # 宏展开
cargo install cargo-tree    # 依赖树

# 3. 设置代码格式化
rustup component add rustfmt clippy

# 4. 运行初始构建
cd /Users/didi/Desktop/nos
cargo build --release
```

### 推荐IDE配置
**VS Code扩展**:
- rust-analyzer (必需)
- CodeLLDB (调试)
- Error Lens (错误高亮)
- GitLens (Git增强)

---

## 📊 进度跟踪

### 每日检查清单
- [ ] 早上回顾当天目标
- [ ] 提交代码时添加详细commit message
- [ ] 运行`cargo test`确保测试通过
- [ ] 运行`cargo clippy`检查代码质量
- [ ] 更新TODO列表状态
- [ ] 晚上记录完成情况和遇到的问题

### 每周检查清单
- [ ] 完成本周任务目标
- [ ] 更新`NOS_IMPROVEMENT_ROADMAP.md`进度
- [ ] 运行完整测试套件
- [ ] 代码review
- [ ] 撰写周报

---

## 💡 编码最佳实践

### 1. 实现前先阅读
```bash
# 查找相关代码
grep -r "fork" kernel/src/process/
grep -r "open" kernel/src/fs/

# 查找现有实现
rg "fn sys_fork" --type rust
```

### 2. 小步迭代
- 不要一次实现所有功能
- 每个函数单独实现和测试
- 频繁提交（每个功能一次commit）

### 3. 测试驱动
```rust
// 先写测试
#[test]
fn test_getpid_returns_current_process_id() {
    let pid = sys_getpid();
    assert!(pid > 0);
}

// 再实现功能
pub fn sys_getpid() -> ProcessId {
    // 实现...
}
```

### 4. 文档化决策
```rust
// ✅ 好的注释
/// Returns the current process ID.
/// 
/// # Implementation Notes
/// This function reads from the per-CPU current process pointer
/// and returns its PID. It never fails.
///
/// # Safety
/// Safe to call from any context.
pub fn sys_getpid() -> ProcessId { ... }

// ❌ 不好的注释
// Get PID
pub fn sys_getpid() -> ProcessId { ... }
```

---

## 🆘 遇到问题时

### 常见问题和解决方案

#### 1. 找不到现有实现
```bash
# 搜索整个代码库
rg "function_name" --type rust

# 搜索类似功能
rg "similar_keyword" kernel/src/
```

#### 2. 编译错误
```bash
# 查看详细错误
cargo build --verbose

# 检查依赖问题
cargo tree
```

#### 3. 测试失败
```bash
# 运行单个测试
cargo test test_name -- --nocapture

# 查看详细输出
RUST_BACKTRACE=1 cargo test
```

#### 4. 设计决策困难
- 参考Linux内核实现
- 查看现有NOS代码模式
- 记录决策理由（添加注释）
- 可以先实现简化版本

---

## 📚 关键文档引用

### 必读文档
1. `docs/TODO_CLEANUP_PLAN.md` - TODO详细清单
2. `NOS_IMPROVEMENT_ROADMAP.md` - 完整路线图
3. `docs/MODULAR_DEVELOPMENT_STANDARDS.md` - 开发标准
4. `docs/ERROR_HANDLING_SPECIFICATION.md` - 错误处理规范

### 代码结构参考
```
kernel/src/
├── syscalls/          # 系统调用入口
│   ├── process_service/  # 进程管理服务
│   ├── fs_service/       # 文件系统服务
│   └── ...
├── process/           # 进程管理核心
├── fs/                # 文件系统核心
├── mm/                # 内存管理核心
└── arch/              # 架构特定代码
```

---

## 🎉 第一周成功标准

完成以下即视为第一周成功：

✅ **代码**:
- [ ] 根目录清理完成
- [ ] 实现5+个进程管理函数
- [ ] 实现5+个文件系统函数
- [ ] 所有新代码有测试
- [ ] 所有测试通过

✅ **文档**:
- [ ] 更新TODO列表
- [ ] 记录技术决策
- [ ] 撰写周报

✅ **质量**:
- [ ] 无编译警告
- [ ] Clippy检查通过
- [ ] 代码格式化符合标准

✅ **进度**:
- [ ] TODO数量: 261 → ~251 (-10个)
- [ ] 核心功能完整性: 50% → 60%

---

## 📞 下一步行动

**立即执行**（今天）:
```bash
# 1. 清理根目录
cd /Users/didi/Desktop/nos
./scripts/cleanup_root.sh  # 或手动执行清理命令

# 2. 创建工作分支
git checkout -b feature/week1-core-implementations

# 3. 开始第一个任务
# 打开 kernel/src/syscalls/process_service/handlers.rs
# 实现 sys_getpid() 函数
```

**本周目标**:
- 专注于进程和文件系统基础功能
- 每天提交代码
- 保持动力和进度

**获取帮助**:
- 遇到问题先搜索代码库
- 参考现有实现
- 记录问题和解决方案

---

## 🏁 准备好了吗？

1. ✅ 已阅读本指南
2. ✅ 开发环境已配置
3. ✅ 理解第一周目标
4. ✅ 知道如何开始

**Let's build an amazing OS! 🚀**

---

*最后更新: 2025-12-09*  
*下次更新: 2025-12-16（第一周结束）*
