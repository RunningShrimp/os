# TODO 快速修复指南

## 立即可执行的清理任务 (预计 2-4 小时)

### 1. 格式字符串修复 (19个) - 优先级：P0

这些是简单的语法错误，可以批量修复。

#### 修复模式

```bash
# 运行查看详细信息的脚本
./fix_format_strings.sh
```

#### 手动修复示例

**文件**: `kernel/src/deploy/backup.rs:198`
```rust
// 修复前:
alloc::string::String::from("sha256:") + /* TODO: {::016x} */ &hash.to_string()

// 修复后:
format!("sha256:{:016x}", hash)
```

**文件**: `kernel/src/i18n/locale.rs:319`
```rust
// 修复前:
result = result.replace("%m", &/* TODO: {::02} */ &month.to_string());

// 修复后:
result = result.replace("%m", &format!("{:02}", month));
```

#### 自动化脚本
创建以下脚本来自动修复：

```bash
#!/bin/bash
# auto_fix_format_strings.sh

files=(
    "kernel/src/deploy/backup.rs"
    "kernel/src/deploy/container_build.rs"
    "kernel/src/i18n/translation.rs"
    "kernel/src/i18n/locale.rs"
    "kernel/src/accessibility/high_contrast.rs"
    "kernel/src/accessibility/screen_reader.rs"
    "kernel/src/subsystems/mm/user_space_isolation.rs"
    "kernel/src/subsystems/security/audit.rs"
)

echo "警告: 此脚本将修改文件。建议先提交更改。"
read -p "继续? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 1
fi

# 应用修复 (需要手动验证每个)
echo "请手动修复以下文件中的格式字符串问题:"
for file in "${files[@]}"; do
    if grep -q "TODO: {::" "$file"; then
        echo "  - $file"
    fi
done
```

---

### 2. 测试桩代码清理 (102个) - 优先级：P2

#### 方案A: 添加 #[ignore] 属性（推荐）

如果功能尚未实现，添加忽略属性：

```rust
// 在 kernel/src/posix_tests/core/basic_tests.rs

// 修复前:
pub fn test_stat() -> PosixTestResult {
    // TODO: 实现具体测试逻辑
    Ok(())
}

// 修复后:
#[ignore = "stat syscall not yet implemented"]
pub fn test_stat() -> PosixTestResult {
    // TODO: 实现具体测试逻辑
    Ok(())
}
```

批量添加脚本：

```bash
#!/bin/bash
# add_ignore_to_tests.sh

file="kernel/src/posix_tests/core/basic_tests.rs"

# 查找所有 test 函数
grep -n "pub fn test_" "$file" | while read -r line; do
    line_num=$(echo "$line" | cut -d: -f1)
    func_name=$(echo "$line" | cut -d: -f3 | cut -d'(' -f1)

    # 检查下一行是否有 TODO
    if sed -n "$((line_num+1))p" "$file" | grep -q "TODO: 实现具体测试逻辑"; then
        echo "需要 #[ignore]: $func_name (line $line_num)"
    fi
done
```

#### 方案B: 删除不需要的测试

如果某些测试永远不会实现，直接删除：

```bash
# 删除整个 test 函数
# 需要手动审查每个函数
```

---

### 3. 显然已完成的TODO (估计30-50个) - 优先级：P1

需要逐个审查，但很多可能已经实现。

#### 审查步骤

1. **查看代码上下文**
```bash
# 查看特定TODO的上下文
sed -n '368,375p' kernel/src/posix/timer.rs
```

2. **判断是否已完成**
   - 如果代码看起来完整，删除TODO
   - 如果注释不够详细，改进注释
   - 如果真的未完成，更新TODO描述

3. **示例**

**文件**: `kernel/src/vfs/ext4.rs:188`
```rust
// TODO: Sync all dirty blocks to disk

// 检查下面是否有实际实现代码
// 如果有，改为：
// Sync all dirty blocks to disk
```

**文件**: `kernel/src/libc/implementations.rs:497`
```rust
// TODO: 根据 errnum 返回对应的错误消息

// 检查是否有 switch 或 match 语句
// 如果有实现，删除TODO
```

---

### 4. 占位符API函数 (8个) - 优先级：P1

**文件**: `kernel/src/subsystems/mm/api/page.rs`

```rust
// TODO: Implement this function
pub fn allocate_pages(count: usize) -> Result<PhysAddr, Error> {
    // 检查是否有实际实现
    // 如果没有，要么实现，要么删除
    Err(Error::NotImplemented)
}
```

#### 决策树
```
函数被调用?
├─ 是 → 实现它
└─ 否 → 删除它
```

检查函数是否被使用：

```bash
# 搜索函数调用
grep -r "allocate_pages" kernel/src --include="*.rs"
```

---

## 批量清理脚本

### 一键检查脚本

```bash
#!/bin/bash
# quick_cleanup_check.sh

echo "=== TODO 快速清理检查 ==="
echo ""

echo "1. 格式字符串问题:"
echo "   数量: $(grep -r 'TODO: {::' kernel/src --include="*.rs" | wc -l)"
echo "   文件:"
grep -r 'TODO: {::' kernel/src --include="*.rs" -l | sed 's/^/     - /'
echo ""

echo "2. 测试桩代码:"
echo "   数量: $(grep -r 'TODO: 实现具体测试逻辑' kernel/src --include="*.rs" | wc -l)"
echo "   文件:"
grep -r 'TODO: 实现具体测试逻辑' kernel/src --include="*.rs" -l | sed 's/^/     - /'
echo ""

echo "3. 占位符实现:"
echo "   数量: $(grep -r 'TODO: Implement this function' kernel/src --include="*.rs" | wc -l)"
echo "   文件:"
grep -r 'TODO: Implement this function' kernel/src --include="*.rs" -l | sed 's/^/     - /'
echo ""

echo "4. 可能已完成的TODO:"
echo "   (需要手动审查)"
echo "   检查这些文件中的简单TODO:"
echo "   - kernel/src/vfs/ext4.rs"
echo "   - kernel/src/libc/implementations.rs"
echo "   - kernel/src/graphics/input.rs"
```

---

## 时间估算

| 任务 | 数量 | 预计时间 | 优先级 |
|------|------|----------|--------|
| 格式字符串修复 | 19 | 1-2小时 | P0 |
| 测试桩代码添加 #[ignore] | 102 | 1小时 | P2 |
| 显然已完成的TODO审查 | 30-50 | 2-3小时 | P1 |
| 占位符API决策 | 8 | 30分钟 | P1 |
| **总计** | **159-177** | **4.5-6.5小时** | - |

---

## 执行计划

### 第1小时：格式字符串
1. 运行 `./fix_format_strings.sh` 查看详情
2. 手动修复所有19个格式字符串问题
3. 编译验证

### 第2小时：测试桩代码
1. 运行 `./analyze_test_stubs.sh` 查看详情
2. 批量添加 `#[ignore]` 属性
3. 或删除不需要的测试

### 第3-5小时：审查已完成的TODO
1. 从 vfs/ext4.rs 开始
2. 逐个审查简单的TODO
3. 删除明显已完成的标记

### 第6小时：占位符API
1. 检查函数是否被调用
2. 决定实现或删除

---

## 验证步骤

完成清理后，运行验证：

```bash
# 重新统计TODO
echo "剩余 TODO: $(grep -r 'TODO\|FIXME\|XXX\|HACK' kernel/src --include="*.rs" | wc -l)"

# 编译检查
cargo build

# 测试检查
cargo test

# Git diff 检查
git diff --stat
```

---

## 注意事项

1. **先提交代码**: 在批量修改前先提交当前状态
2. **逐文件修改**: 不要一次性修改太多文件
3. **编译验证**: 每修改几个文件就编译一次
4. **保留有价值的信息**: 不要删除有实质内容的注释
5. **更新文档**: 如果删除了TODO，相关文档也要更新

---

## 成功标准

- [ ] 所有格式字符串TODO已修复
- [ ] 所有测试桩已处理（添加#[ignore]或删除）
- [ ] 所有简单的已完成TODO已删除
- [ ] 所有占位符API已决策（实现或删除）
- [ ] 代码编译通过
- [ ] 测试运行正常
- [ ] TODO数量减少 40% 以上 (从380降到230以下)

---

**最后更新**: 2025-12-29
