#!/bin/bash

# TODO Tracker Script
# 用于跟踪TODO数量变化和生成报告

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Project root
PROJECT_ROOT="/Users/wangbiao/Desktop/project/nos"
cd "$PROJECT_ROOT"

echo -e "${BLUE}=== TODO Tracker ===${NC}"
echo ""

# Function to count TODOs by type
count_todos() {
    local pattern=$1
    local description=$2
    local count=$(grep -r "$pattern" kernel/src --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
    echo -e "${GREEN}✓${NC} $description: ${YELLOW}$count${NC}"
}

# Function to count TODOs by priority
count_priority() {
    local pattern=$1
    local description=$2

    # Count by keyword patterns
    local count=$(grep -r "$pattern" kernel/src --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
    echo -e "${GREEN}✓${NC} $description: ${YELLOW}$count${NC}"
}

echo -e "${BLUE}📊 TODO 统计${NC}"
echo ""
echo "按类型:"
count_todos "TODO" "TODO"
count_todos "FIXME" "FIXME"
count_todos "XXX" "XXX"
count_todos "HACK" "HACK"

echo ""
echo "按内容分类:"
count_todos "实现具体测试逻辑" "测试桩代码"
count_todos "TODO: {::" "格式字符串问题"
count_todos "Implement actual" "核心功能待实现"
count_todos "实现实际\|实现真正的" "中文核心功能"
count_todos "get.*time\|clock" "时间相关"
count_todos "implement.*timestamp\|proper timestamp" "时间戳"

echo ""
echo "按模块:"
for dir in posix subsystems vfs ids security graphics i18n error; do
    if [ -d "kernel/src/$dir" ]; then
        count=$(grep -r "TODO\|FIXME\|XXX\|HACK" "kernel/src/$dir" --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
        if [ "$count" -gt 0 ]; then
            echo -e "${GREEN}✓${NC} kernel/src/$dir: ${YELLOW}$count${NC}"
        fi
    fi
done

echo ""
echo -e "${BLUE}🔥 热点文件 (TODO最多的10个)${NC}"
echo ""
grep -r "TODO\|FIXME\|XXX\|HACK" kernel/src --include="*.rs" -c 2>/dev/null | \
    sort -t: -k2 -rn | \
    head -10 | \
    while IFS=: read -r file count; do
        short_file=${file#kernel/src/}
        echo -e "${GREEN}✓${NC} ${YELLOW}$count${NC} - $short_file"
    done

echo ""
echo -e "${BLUE}📈 趋势分析${NC}"
echo ""

# Check if history exists
HISTORY_FILE=".todo_history"
if [ -f "$HISTORY_FILE" ]; then
    echo "历史记录:"
    cat "$HISTORY_FILE" | tail -5
else
    echo "未找到历史记录"
fi

# Save current count
current_total=$(grep -r "TODO\|FIXME\|XXX\|HACK" kernel/src --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
echo "$(date +%Y-%m-%d) | Total: $current_total" >> "$HISTORY_FILE"

echo ""
echo -e "${BLUE}💡 建议${NC}"
echo ""

if [ "$current_total" -gt 300 ]; then
    echo -e "${RED}⚠️  TODO数量过多 ($current_total)${NC}"
    echo "   建议: 立即执行快速清理任务"
elif [ "$current_total" -gt 200 ]; then
    echo -e "${YELLOW}⚠️  TODO数量较多 ($current_total)${NC}"
    echo "   建议: 每周至少处理20个TODO"
elif [ "$current_total" -gt 100 ]; then
    echo -e "${GREEN}✓ TODO数量适中 ($current_total)${NC}"
    echo "   建议: 持续清理，保持下降趋势"
else
    echo -e "${GREEN}✅ TODO数量良好 ($current_total)${NC}"
    echo "   建议: 保持现状，专注于质量"
fi

echo ""
echo -e "${BLUE}🚀 快速行动${NC}"
echo ""
echo "1. 查看详细报告: cat TODO_SUMMARY.md"
echo "2. 查看快速修复: cat TODO_QUICK_WINS.md"
echo "3. 运行格式字符串检查: ./fix_format_strings.sh"
echo "4. 运行测试桩分析: ./analyze_test_stubs.sh"

echo ""
echo -e "${BLUE}✨ 完成!${NC}"
echo "下次运行: $(date -v+7d +%Y-%m-%d)"
