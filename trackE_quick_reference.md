# Track E 快速参考指南

## 📋 概览

**目标**: 优化 NOS 内核目录结构，解决循环依赖
**当前进度**: Phase 1 准备中
**关键文件**:
- 完整报告: `trackE_directory_optimization.md`
- 实施清单: `trackE_implementation_checklist.md`
- 重构脚本: `refactor_step1_move_vfs.sh`
- 导入更新: `update_imports_phase1.sh`
- 验证脚本: `verify_and_fix_imports.sh`

---

## 🚀 快速开始

### 第一步: 创建分支

```bash
cd /Users/wangbiao/Desktop/project/nos
git checkout master
git pull
git checkout -b trackE-phase1-vfs-posix
git tag trackE-phase1-start
```

### 第二步: 执行重构

```bash
chmod +x refactor_step1_move_vfs.sh
./refactor_step1_move_vfs.sh
```

### 第三步: 更新导入

```bash
chmod +x update_imports_phase1.sh
cd kernel/src
./../update_imports_phase1.sh
```

### 第四步: 验证导入

```bash
chmod +x verify_and_fix_imports.sh
cd kernel/src
./../verify_and_fix_imports.sh
```

### 第五步: 编译检查

```bash
cd /Users/wangbiao/Desktop/project/nos/kernel
cargo build 2>&1 | tee build_phase1.log
```

### 第六步: 查看错误

```bash
# 错误数量
grep -c "^error" build_phase1.log

# 错误类型
grep "^error\[" build_phase1.log | cut -d':' -f3 | sort | uniq -c | sort -rn
```

---

## 🔍 关键路径变化

### VFS 移动

| 旧路径 | 新路径 |
|--------|--------|
| `crate::vfs` | `crate::subsystems::fs::vfs` |
| `crate::vfs::Mount` | `crate::subsystems::fs::vfs::Mount` |
| `crate::vfs::File` | `crate::subsystems::fs::vfs::file` |

### VFS Interface 移动

| 旧路径 | 新路径 |
|--------|--------|
| `crate::vfs_interface` | `crate::subsystems::fs::vfs_interface` |
| `crate::vfs_interface::VfsError` | `crate::subsystems::fs::vfs_interface::VfsError` |

### POSIX 移动

| 旧路径 | 新路径 |
|--------|--------|
| `crate::posix` | `crate::subsystems::posix` |
| `crate::posix::types` | `crate::subsystems::posix::types` |
| `crate::posix::stat` | `crate::subsystems::posix::stat` |

---

## 🛠️ 常用命令

### 检查特定模块的导入

```bash
cd kernel/src
grep -rn "use crate::vfs" . --include="*.rs" | head -20
```

### 批量查找替换

```bash
# 查找所有旧导入
grep -rn "use crate::vfs[^/]" . --include="*.rs"

# 替换导入
find . -name "*.rs" -exec sed -i '' \
  's|use crate::vfs::|use crate::subsystems::fs::vfs::|g' {} \;
```

### 查看目录结构

```bash
cd kernel/src
tree -L 2 -I target
# 或
find . -type d -maxdepth 2 | sort
```

### 验证模块依赖

```bash
cd kernel/src
# 查看模块导入
grep "^use crate::" subsystems/fs/vfs/mod.rs

# 检查循环依赖
grep "use crate::subsystems" vfs/mod.rs
grep "use crate::vfs" subsystems/fs/mod.rs
```

---

## 📊 进度跟踪

### Phase 1 (进行中)

- [x] 分析当前结构
- [x] 检测循环依赖
- [x] 设计新架构
- [x] 准备重构脚本
- [ ] 创建分支
- [ ] 移动文件
- [ ] 更新导入
- [ ] 编译验证
- [ ] 测试验证
- [ ] 提交代码

### Phase 2 (待开始)

- [ ] 合并 fs/ 模块
- [ ] 移动 compat/
- [ ] 合并 memory/

### Phase 3 (待开始)

- [ ] 合并 syscalls/
- [ ] 合并 services/
- [ ] 清理根目录

---

## 🐛 常见问题

### Q1: 编译错误: `failed to resolve: use of undeclared crate::vfs`

**原因**: 导入路径未更新

**修复**:
```bash
# 手动更新
sed -i '' 's/use crate::vfs::/use crate::subsystems::fs::vfs::/g' <file>.rs

# 或批量更新
find . -name "*.rs" -exec sed -i '' \
  's|use crate::vfs::|use crate::subsystems::fs::vfs::|g' {} \;
```

### Q2: 错误: `cannot find type VfsError in this scope`

**原因**: 缺少 re-export

**修复**: 在 `lib.rs` 中添加:
```rust
pub use crate::subsystems::fs::vfs_interface::{VfsError, FileSystemType};
```

### Q3: 错误: `cyclic dependency`

**原因**: 模块间循环导入

**修复**:
1. 检查模块导入: `grep "^use crate::" <module>/mod.rs`
2. 使用接口解耦
3. 调整依赖方向

### Q4: 如何回滚?

```bash
# 回滚到 Phase 1 开始
git reset --hard trackE-phase1-start

# 回滚到 master
git checkout master
git branch -D trackE-phase*
```

---

## 📈 成功标准

✅ **编译通过**:
```bash
cargo build
# 输出: Finished dev [unoptimized + debuginfo] target(s)
```

✅ **无警告**:
```bash
cargo build 2>&1 | grep "warning:" | wc -l
# 输出: 0
```

✅ **测试通过**:
```bash
cargo test --lib
# 输出: test result: ok
```

✅ **Clippy 通过**:
```bash
cargo clippy -- -D warnings
# 输出: Finished dev [unoptimized + debuginfo] target(s)
```

✅ **无循环依赖**:
```bash
# 运行验证脚本
./verify_and_fix_imports.sh
# 输出: ✅ 所有导入路径正确！
```

---

## 📝 检查点

### 检查点 1: 文件移动后

```bash
# 验证文件已移动
ls -la kernel/src/subsystems/fs/vfs/
ls -la kernel/src/subsystems/fs/vfs_interface/
ls -la kernel/src/subsystems/posix/

# 验证旧目录已删除
ls -la kernel/src/vfs 2>&1 | grep "No such file"
ls -la kernel/src/posix 2>&1 | grep "No such file"
```

### 检查点 2: 导入更新后

```bash
# 统计更新数量
grep -r "use crate::subsystems::fs::vfs" kernel/src --include="*.rs" | wc -l
# 应该 > 0

# 统计未更新
grep -r "use crate::vfs[^/]" kernel/src --include="*.rs" | wc -l
# 应该 = 0
```

### 检查点 3: 编译前

```bash
# 检查 lib.rs
grep "pub use crate::subsystems::fs::vfs" kernel/src/lib.rs
# 应该有输出

# 检查模块声明
grep "pub mod posix" kernel/src/subsystems/mod.rs
# 应该有输出
```

### 检查点 4: 编译后

```bash
# 检查错误
grep "^error" kernel/build_phase1.log | wc -l
# 应该 = 0

# 检查警告
grep "warning:" kernel/build_phase1.log | wc -l
# 应该 = 0
```

---

## 🎯 下一步行动

### 立即执行

1. **创建分支** (5分钟)
   ```bash
   git checkout -b trackE-phase1-vfs-posix
   ```

2. **运行重构脚本** (10分钟)
   ```bash
   ./refactor_step1_move_vfs.sh
   ```

3. **更新导入** (5分钟)
   ```bash
   ./update_imports_phase1.sh
   ```

4. **验证** (5分钟)
   ```bash
   ./verify_and_fix_imports.sh
   ```

5. **编译检查** (5分钟)
   ```bash
   cd kernel && cargo build
   ```

### 预期总时间

- Phase 1: 2-4小时
- Phase 2: 2-3小时
- Phase 3: 1-2小时
- **总计**: 5-9小时

---

## 📞 获取帮助

### 文档

- 完整分析报告: `trackE_directory_optimization.md`
- 实施检查清单: `trackE_implementation_checklist.md`
- 本快速参考: `trackE_quick_reference.md`

### 脚本

- 重构脚本: `refactor_step1_move_vfs.sh`
- 导入更新: `update_imports_phase1.sh`
- 依赖分析: `circular_deps_analysis.sh`
- 验证脚本: `verify_and_fix_imports.sh`

### Git 操作

```bash
# 查看所有 tag
git tag | grep trackE

# 查看分支
git branch | grep trackE

# 查看状态
git status
```

---

## ✅ 完成标准

当以下所有条件满足时，Track E Phase 1 完成:

- [x] 分析报告已完成
- [x] 重构脚本已准备
- [ ] 分支已创建
- [ ] 文件已移动
- [ ] 导入已更新
- [ ] 编译通过 (0 errors)
- [ ] 无警告 (0 warnings)
- [ ] 测试通过
- [ ] 已提交代码
- [ ] 已标记完成

**当前状态**: 30% (准备阶段完成)

---

**最后更新**: 2025-12-30
**维护者**: Track E Team
**版本**: 1.0
