#!/bin/bash

# NOS改进计划 - 快速开始脚本
# 用途: 验证环境并开始第一天工作
# 日期: 2025-12-09

set -e

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo ""
echo "================================================"
echo "  🚀 NOS 改进计划 - 快速开始"
echo "================================================"
echo ""

# 1. 确认目录
echo -e "${BLUE}[1/8] 确认项目目录...${NC}"
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ 错误: 不在NOS项目根目录${NC}"
    exit 1
fi
echo -e "${GREEN}✓ 当前目录正确${NC}"
echo ""

# 2. 检查Rust工具链
echo -e "${BLUE}[2/8] 检查Rust工具链...${NC}"
if ! command -v rustc &> /dev/null; then
    echo -e "${RED}❌ 错误: 未找到rustc${NC}"
    echo "请安装Rust: https://rustup.rs"
    exit 1
fi
rustc --version
cargo --version
echo -e "${GREEN}✓ Rust工具链正常${NC}"
echo ""

# 3. 检查Git状态
echo -e "${BLUE}[3/8] 检查Git状态...${NC}"
git status --short
current_branch=$(git branch --show-current)
echo "当前分支: $current_branch"
echo -e "${GREEN}✓ Git状态正常${NC}"
echo ""

# 4. 测试构建
echo -e "${BLUE}[4/8] 测试项目构建...${NC}"
echo "这可能需要几分钟..."
if cargo build --quiet 2>/dev/null; then
    echo -e "${GREEN}✓ 项目构建成功${NC}"
else
    echo -e "${YELLOW}⚠ 项目构建失败，但可以继续${NC}"
    echo "建议: 先解决编译错误"
fi
echo ""

# 5. 检查代码质量工具
echo -e "${BLUE}[5/8] 检查代码质量工具...${NC}"
if command -v cargo-fmt &> /dev/null; then
    echo -e "${GREEN}✓ cargo fmt 可用${NC}"
else
    echo -e "${YELLOW}⚠ cargo fmt 未安装${NC}"
    echo "安装: rustup component add rustfmt"
fi

if command -v cargo-clippy &> /dev/null; then
    echo -e "${GREEN}✓ cargo clippy 可用${NC}"
else
    echo -e "${YELLOW}⚠ cargo clippy 未安装${NC}"
    echo "安装: rustup component add clippy"
fi
echo ""

# 6. 显示文档
echo -e "${BLUE}[6/8] 关键文档列表...${NC}"
echo "📋 核心文档:"
echo "  - QUICK_START_GUIDE.md          快速启动指南"
echo "  - NOS_IMPROVEMENT_ROADMAP.md    6个月路线图"
echo "  - docs/TODO_CLEANUP_PLAN.md     TODO详细清单"
echo "  - docs/plans/WEEK1_DETAILED_GUIDE.md  第一周指南"
echo "  - EXECUTION_CHECKLIST.md        执行检查清单"
echo ""

# 7. 显示第一天任务
echo -e "${BLUE}[7/8] 第一天任务提醒...${NC}"
echo "📅 今天的任务 (Day 1):"
echo "  1. 执行根目录清理脚本 (30分钟)"
echo "     $ ./scripts/cleanup_root.sh"
echo ""
echo "  2. 分析代码结构 (2小时)"
echo "     $ grep -r 'Process' kernel/src/process/"
echo "     $ cat kernel/src/syscalls/process_service/handlers.rs"
echo ""
echo "  3. 创建工作分支 (5分钟)"
echo "     $ git checkout -b feature/week1-core-implementations"
echo ""

# 8. 提供选项
echo -e "${BLUE}[8/8] 选择下一步操作...${NC}"
echo ""
echo "请选择:"
echo "  1) 立即执行根目录清理"
echo "  2) 查看快速启动指南"
echo "  3) 查看第一周详细指南"
echo "  4) 查看执行检查清单"
echo "  5) 创建工作分支"
echo "  6) 退出"
echo ""

read -p "请输入选项 (1-6): " choice

case $choice in
    1)
        echo ""
        echo -e "${GREEN}执行根目录清理...${NC}"
        if [ -f "scripts/cleanup_root.sh" ]; then
            chmod +x scripts/cleanup_root.sh
            ./scripts/cleanup_root.sh
        else
            echo -e "${RED}错误: 找不到清理脚本${NC}"
        fi
        ;;
    2)
        echo ""
        less QUICK_START_GUIDE.md
        ;;
    3)
        echo ""
        less docs/plans/WEEK1_DETAILED_GUIDE.md
        ;;
    4)
        echo ""
        less EXECUTION_CHECKLIST.md
        ;;
    5)
        echo ""
        echo -e "${GREEN}创建工作分支...${NC}"
        git checkout -b feature/week1-core-implementations
        echo -e "${GREEN}✓ 分支创建成功${NC}"
        ;;
    6)
        echo ""
        echo -e "${GREEN}退出。祝开发顺利！${NC}"
        ;;
    *)
        echo ""
        echo -e "${YELLOW}无效选项${NC}"
        ;;
esac

echo ""
echo "================================================"
echo -e "${GREEN}  环境检查完成！${NC}"
echo "================================================"
echo ""
echo "📚 推荐阅读顺序:"
echo "  1. QUICK_START_GUIDE.md"
echo "  2. docs/plans/WEEK1_DETAILED_GUIDE.md"
echo "  3. EXECUTION_CHECKLIST.md"
echo ""
echo "🚀 准备好了就开始吧！Good luck!"
echo ""
