#!/bin/bash

# 深度分析循环依赖的脚本

KERNEL_SRC="/Users/wangbiao/Desktop/project/nos/kernel/src"
cd "$KERNEL_SRC" || exit 1

echo "=== 深度循环依赖分析 ==="
echo ""

# 分析关键模块的依赖关系
analyze_module_deps() {
    local module=$1
    echo "=== $module 模块依赖 ==="
    echo "内部依赖 (use crate::):"
    grep -rh "^use crate::" "$module" 2>/dev/null | grep -v "^use crate::${module}::" | sort -u | sed 's/^use crate:://;s/::.*//;s/;$//' | sort -u | sed 's/^/  - /'
    echo ""
}

# 分析关键模块
analyze_module_deps "api"
analyze_module_deps "error"
analyze_module_deps "vfs"
analyze_module_deps "subsystems/fs"
analyze_module_deps "subsystems/syscalls"
analyze_module_deps "services"

echo ""
echo "=== 检测关键循环依赖 ==="
echo ""

# 检查 vfs <-> subsystems::fs
echo "1. vfs <-> subsystems::fs 循环:"
echo "  vfs -> subsystems::fs:"
grep -h "^use crate::subsystems::fs" vfs/*.rs 2>/dev/null | grep -v "^//" | head -3
echo "  subsystems::fs -> vfs:"
grep -h "^use crate::vfs" subsystems/fs/*.rs 2>/dev/null | head -3
echo ""

# 检查 api <-> error
echo "2. api <-> error 循环:"
echo "  api -> error:"
grep -h "^use crate::error" api/*.rs 2>/dev/null | head -3
echo "  error -> api:"
grep -h "^use crate::api" error/*.rs 2>/dev/null | head -3
echo ""

# 检查 syscalls <-> 其他根模块
echo "3. subsystems::syscalls -> 根模块依赖:"
for dep in vfs memory services sync posix; do
    count=$(grep -rh "^use crate::${dep}[^a-z]" subsystems/syscalls 2>/dev/null | wc -l)
    if [ $count -gt 0 ]; then
        echo "  - $dep: $count 处引用"
        grep -rh "^use crate::${dep}[^a-z]" subsystems/syscalls 2>/dev/null | head -2 | sed 's/^/    /'
    fi
done
echo ""

# 检查是否有反向依赖
echo "4. 根模块 -> subsystems::syscalls 依赖:"
for mod in vfs memory services; do
    if [ -d "$mod" ]; then
        count=$(grep -rh "^use crate::subsystems::syscalls" "$mod" 2>/dev/null | wc -l)
        if [ $count -gt 0 ]; then
            echo "  - $mod -> syscalls: $count 处引用"
        fi
    fi
done
echo ""

echo "=== 模块组织问题 ==="
echo ""
echo "检测重复的模块:"
duplicate_modules=$(find . -name "mod.rs" -exec grep -l "^pub mod " {} \; | xargs -I {} bash -c 'grep "^pub mod " {} | sed "s|^|{}: |"' | grep -E "(vfs|fs|syscalls|memory|services|posix)" | sort)
echo "$duplicate_modules"
echo ""

echo "检测 root 级别与 subsystems 级别的重复:"
for name in fs syscalls services; do
    if [ -d "$name" ] && [ -d "subsystems/$name" ]; then
        echo "发现重复: $name (根) 和 subsystems/$name"
    fi
done
echo ""

echo "=== 建议的移动操作 ==="
echo ""
echo "以下模块应该移动到 subsystems/:"
for mod in vfs posix; do
    if [ -d "$mod" ] && [ ! -d "subsystems/$mod" ]; then
        # 检查是否被其他模块依赖
        dependents=$(grep -rl "use crate::${mod}[^a-z/]" . 2>/dev/null | grep -v "^${mod}/" | wc -l)
        echo "- $mod: 被 $dependents 个模块依赖"
    fi
done

echo ""
echo "以下根级别模块可能应该合并或重构:"
for mod in $(ls -d */ 2>/dev/null | grep -v "^subsystems/" | grep -v "^arch/" | grep -v "^platform/" | sed 's|/||'); do
    if [ -f "$mod/mod.rs" ]; then
        size=$(find "$mod" -name "*.rs" | wc -l)
        if [ $size -lt 3 ]; then
            echo "- $mod: 只有 $size 个文件，考虑合并"
        fi
    fi
done
