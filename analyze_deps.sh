#!/bin/bash

# 分析循环依赖的脚本

echo "=== 检测 Rust 循环依赖 ==="
echo ""

KERNEL_SRC="/Users/wangbiao/Desktop/project/nos/kernel/src"

cd "$KERNEL_SRC" || exit 1

# 查找所有 use 语句并分析依赖关系
echo "=== 分析模块导入关系 ==="
echo ""

# 检查根级别模块的导入
for mod_file in */mod.rs; do
    if [ -f "$mod_file" ]; then
        mod_name=$(dirname "$mod_file")
        echo "Module: $mod_name"
        echo "  Imports:"
        grep -h "^use crate::" "$mod_file" 2>/dev/null | head -20 | sed 's/^/    /'
        echo ""
    fi
done | head -200

echo ""
echo "=== 检查潜在循环依赖 ==="
echo ""

# 检查 subsystems 和根级别模块之间的相互依赖
echo "检查根模块 -> subsystems 的依赖:"
grep -h "^use crate::subsystems::" */mod.rs 2>/dev/null | cut -d: -f1 | sort -u

echo ""
echo "检查 subsystems -> 根模块的依赖:"
grep -h "^use crate::" subsystems/*/mod.rs subsystems/*/*/mod.rs 2>/dev/null | \
    grep -v "use crate::subsystems::" | \
    grep -v "^use crate::error" | \
    grep -v "^use crate::api" | \
    head -30

echo ""
echo "=== 检查 vfs 和 subsystems::fs 的依赖 ==="
echo ""
echo "vfs -> subsystems:"
grep -h "^use crate::subsystems::" vfs/mod.rs 2>/dev/null

echo ""
echo "subsystems::fs -> vfs:"
grep -h "^use crate::vfs" subsystems/fs/mod.rs 2>/dev/null

echo ""
echo "=== 检查 api 和 error 的依赖 ==="
echo ""
echo "api -> error:"
grep -h "^use crate::error" api/mod.rs 2>/dev/null

echo ""
echo "error -> api:"
grep -h "^use crate::api" error/mod.rs 2>/dev/null

echo ""
echo "分析完成"
