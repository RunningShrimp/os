#!/bin/bash

# NOS根目录清理脚本
# 用途: 整理项目根目录，移动临时文件到合适位置
# 日期: 2025-12-09

set -e  # 遇到错误立即退出

PROJECT_ROOT="/Users/didi/Desktop/nos"
cd "$PROJECT_ROOT"

echo "================================================"
echo "  NOS 根目录清理脚本"
echo "================================================"
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 1. 创建目标目录
echo -e "${BLUE}[1/6] 创建目标目录...${NC}"
mkdir -p temp/build_logs
mkdir -p temp/analysis
mkdir -p docs/reports
mkdir -p docs/plans
echo -e "${GREEN}✓ 目录创建完成${NC}"
echo ""

# 2. 移动构建日志
echo -e "${BLUE}[2/6] 移动构建日志和输出文件...${NC}"
count=0
for file in build_*.txt *_output.txt; do
    if [ -f "$file" ]; then
        mv -v "$file" temp/build_logs/
        ((count++))
    fi
done
echo -e "${GREEN}✓ 移动了 $count 个构建日志文件${NC}"
echo ""

# 3. 移动错误分析文件
echo -e "${BLUE}[3/6] 移动错误分析文件...${NC}"
count=0
for file in *error*.txt error_*.txt compile*.txt compilation*.txt current_errors.txt; do
    if [ -f "$file" ]; then
        mv -v "$file" temp/analysis/
        ((count++))
    fi
done
echo -e "${GREEN}✓ 移动了 $count 个分析文件${NC}"
echo ""

# 4. 移动报告文档
echo -e "${BLUE}[4/6] 移动报告文档...${NC}"
count=0
for file in *REPORT.md *SUMMARY.md *AUDIT*.md *ANALYSIS*.md *ASSESSMENT*.md; do
    if [ -f "$file" ] && [ "$file" != "README.md" ]; then
        mv -v "$file" docs/reports/
        ((count++))
    fi
done
echo -e "${GREEN}✓ 移动了 $count 个报告文档${NC}"
echo ""

# 5. 移动计划文档
echo -e "${BLUE}[5/6] 移动计划文档...${NC}"
count=0
for file in *PLAN.md *ROADMAP.md *TODO.md; do
    if [ -f "$file" ]; then
        mv -v "$file" docs/plans/
        ((count++))
    fi
done
echo -e "${GREEN}✓ 移动了 $count 个计划文档${NC}"
echo ""

# 6. 清理临时文件
echo -e "${BLUE}[6/6] 清理临时文件...${NC}"
count=0
for file in *.profraw; do
    if [ -f "$file" ]; then
        rm -v "$file"
        ((count++))
    fi
done
echo -e "${GREEN}✓ 删除了 $count 个临时文件${NC}"
echo ""

# 7. 更新.gitignore
echo -e "${BLUE}[7/7] 更新 .gitignore...${NC}"
if [ ! -f .gitignore ]; then
    touch .gitignore
fi

# 检查并添加规则
if ! grep -q "^temp/$" .gitignore 2>/dev/null; then
    echo "" >> .gitignore
    echo "# 临时文件和构建产物" >> .gitignore
    echo "temp/" >> .gitignore
    echo "*.profraw" >> .gitignore
    echo "*_errors.txt" >> .gitignore
    echo "*_output.txt" >> .gitignore
    echo -e "${GREEN}✓ 已更新 .gitignore${NC}"
else
    echo -e "${YELLOW}⚠ .gitignore 已包含相关规则${NC}"
fi
echo ""

# 8. 显示清理结果
echo "================================================"
echo -e "${GREEN}  清理完成！${NC}"
echo "================================================"
echo ""
echo "根目录文件统计:"
echo "  - Rust配置文件: $(ls -1 *.toml 2>/dev/null | wc -l | tr -d ' ')"
echo "  - Markdown文档: $(ls -1 *.md 2>/dev/null | wc -l | tr -d ' ')"
echo "  - 其他文件: $(ls -1 * 2>/dev/null | grep -v '/$' | grep -v '\.toml$' | grep -v '\.md$' | wc -l | tr -d ' ')"
echo ""
echo "临时文件目录:"
echo "  - temp/build_logs: $(ls -1 temp/build_logs 2>/dev/null | wc -l | tr -d ' ') 个文件"
echo "  - temp/analysis: $(ls -1 temp/analysis 2>/dev/null | wc -l | tr -d ' ') 个文件"
echo ""
echo "文档组织:"
echo "  - docs/reports: $(ls -1 docs/reports 2>/dev/null | wc -l | tr -d ' ') 个报告"
echo "  - docs/plans: $(ls -1 docs/plans 2>/dev/null | wc -l | tr -d ' ') 个计划"
echo ""

# 9. 建议下一步
echo -e "${BLUE}建议的下一步操作:${NC}"
echo "  1. 查看清理结果: ls -la"
echo "  2. 提交更改: git add . && git commit -m 'chore: 清理根目录，建立项目结构规范'"
echo "  3. 查看文档: cat docs/reports/TODO_CLEANUP_PLAN.md"
echo ""
echo -e "${YELLOW}注意: temp/ 目录已添加到 .gitignore，不会被提交到Git${NC}"
echo ""
