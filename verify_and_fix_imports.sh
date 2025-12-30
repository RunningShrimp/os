#!/bin/bash

# Track E - 验证和修复导入脚本
# 检测所有未更新的导入路径并提供修复建议

KERNEL_SRC="/Users/wangbiao/Desktop/project/nos/kernel/src"
cd "$KERNEL_SRC" || exit 1

echo "========================================="
echo "Track E - 导入路径验证和修复"
echo "========================================="
echo ""

# 临时文件
TEMP_REPORT=$(mktemp)
TEMP_FIXES=$(mktemp)

# 分析函数
analyze_imports() {
    local module=$1
    local new_path=$2

    echo "========================================="
    echo "检查模块: $module"
    echo "新路径: $new_path"
    echo "========================================="

    # 查找所有旧导入
    local old_imports=$(grep -rn "use crate::$module" . --include="*.rs" 2>/dev/null | \
        grep -v "use crate::$module[^a-z/]" | \
        grep -v "/.phase1_backup:" | \
        grep -v "/.backup:" | \
        grep -v "$new_path" || true)

    if [ -z "$old_imports" ]; then
        echo "✅ 所有 $module 导入已更新"
        echo ""
        return 0
    fi

    echo "⚠️ 发现未更新的导入:"
    echo "$old_imports"
    echo ""

    # 生成修复命令
    echo "$old_imports" | while read -r line; do
        file=$(echo "$line" | cut -d':' -f1)
        import=$(echo "$line" | cut -d':' -f2-)

        # 生成 sed 命令
        if echo "$import" | grep -q "use crate::$module{"; then
            new_import=$(echo "$import" | sed "s/use crate::$module{/use crate::$new_path{/g")
            echo "sed -i '' 's|use crate::$module{|use crate::$new_path{|g' $file" >> "$TEMP_FIXES"
        elif echo "$import" | grep -q "use crate::$module::"; then
            new_import=$(echo "$import" | sed "s/use crate::$module::/use crate::$new_path::/g")
            echo "sed -i '' 's|use crate::$module::|use crate::$new_path::|g' $file" >> "$TEMP_FIXES"
        fi
    done

    return 1
}

# 执行分析
{
    echo "# Track E 导入路径验证报告"
    echo "# 生成时间: $(date)"
    echo ""
} > "$TEMP_REPORT"

errors=0

# 检查 vfs
analyze_imports "vfs" "subsystems/fs/vfs" | tee -a "$TEMP_REPORT" || errors=$((errors + 1))

# 检查 vfs_interface
analyze_imports "vfs_interface" "subsystems/fs/vfs_interface" | tee -a "$TEMP_REPORT" || errors=$((errors + 1))

# 检查 posix
analyze_imports "posix" "subsystems/posix" | tee -a "$TEMP_REPORT" || errors=$((errors + 1))

# 检查 compat
analyze_imports "compat" "subsystems/compat" | tee -a "$TEMP_REPORT" || errors=$((errors + 1))

echo ""
echo "========================================="
echo "验证总结"
echo "========================================="
echo "发现的问题: $errors"
echo ""
echo "详细报告已保存到: $TEMP_REPORT"
echo "修复命令已保存到: $TEMP_FIXES"
echo ""

if [ $errors -gt 0 ]; then
    echo "⚠️ 发现未更新的导入"
    echo ""
    echo "自动修复选项:"
    echo "1. 查看修复命令: cat $TEMP_FIXES"
    echo "2. 应用修复（谨慎）: bash $TEMP_FIXES"
    echo "3. 手动修复参考上述报告"
    echo ""
else
    echo "✅ 所有导入路径正确！"
    echo ""
fi

# 显示前 20 个需要修复的文件
if [ -s "$TEMP_FIXES" ]; then
    echo "需要修复的文件（前 20 个）:"
    cat "$TEMP_FIXES" | cut -d' ' -f3 | sort -u | head -20
    echo ""
fi

# 生成详细统计
echo "========================================="
echo "导入路径统计"
echo "========================================="
echo ""
echo "VFS 导入:"
echo "  正确: $(grep -r "use crate::subsystems::fs::vfs" . --include="*.rs" 2>/dev/null | wc -l | xargs)"
echo "  需修复: $(grep -r "use crate::vfs[^a-z/]" . --include="*.rs" 2>/dev/null | wc -l | xargs)"
echo ""
echo "VFS interface 导入:"
echo "  正确: $(grep -r "use crate::subsystems::fs::vfs_interface" . --include="*.rs" 2>/dev/null | wc -l | xargs)"
echo "  需修复: $(grep -r "use crate::vfs_interface" . --include="*.rs" 2>/dev/null | grep -v "subsystems/fs" | wc -l | xargs)"
echo ""
echo "POSIX 导入:"
echo "  正确: $(grep -r "use crate::subsystems::posix" . --include="*.rs" 2>/dev/null | wc -l | xargs)"
echo "  需修复: $(grep -r "use crate::posix[^a-z/]" . --include="*.rs" 2>/dev/null | wc -l | xargs)"
echo ""

# 保存报告
REPORT_FILE="/Users/wangbiao/Desktop/project/nos/trackE_import_verification_report.txt"
cp "$TEMP_REPORT" "$REPORT_FILE"
echo "完整报告已保存到: $REPORT_FILE"
echo ""

# 清理临时文件
trap "rm -f $TEMP_REPORT $TEMP_FIXES" EXIT

echo "验证完成！"
