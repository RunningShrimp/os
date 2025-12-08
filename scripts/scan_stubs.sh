#!/bin/bash
# 存根扫描脚本
# 用于扫描项目中的TODO/FIXME/STUB标记并生成报告

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_FILE="${PROJECT_ROOT}/docs/STUB_REPORT.md"
TIMESTAMP=$(date +"%Y-%m-%d %H:%M:%S")

echo "# 存根扫描报告" > "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "生成时间: $TIMESTAMP" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 统计总数
TOTAL=$(grep -rn "TODO\|FIXME\|STUB\|stub" "${PROJECT_ROOT}/kernel/src" --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
echo "## 统计信息" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "- **总计**: $TOTAL 处存根标记" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 按文件分组统计
echo "## 按文件分布" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"
echo "| 文件 | 数量 |" >> "$OUTPUT_FILE"
echo "|------|------|" >> "$OUTPUT_FILE"
grep -rn "TODO\|FIXME\|STUB\|stub" "${PROJECT_ROOT}/kernel/src" --include="*.rs" 2>/dev/null | \
    awk -F: '{print $1}' | sort | uniq -c | sort -rn | \
    awk '{printf "| %s | %d |\n", $2, $1}' >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"
echo "## 详细列表" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# 按模块分类
for module in syscalls posix vfs fs net drivers security ids types; do
    echo "### $module 模块" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    grep -rn "TODO\|FIXME\|STUB\|stub" "${PROJECT_ROOT}/kernel/src/${module}" --include="*.rs" 2>/dev/null | \
        head -20 | \
        awk -F: '{printf "- **%s:%s**: %s\n", $1, $2, substr($0, index($0,$3))}' >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done

echo "报告已生成: $OUTPUT_FILE"
echo "总计: $TOTAL 处存根标记"

