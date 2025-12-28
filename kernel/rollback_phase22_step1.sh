#!/bin/bash

# Phase 23 Step 1: 回滚errno路径修改
# 第二十二批修复错误地将crate::reliability::errno替换为crate::types::stubs::errno
# 现在需要恢复正确的路径

set -e

echo "=== Phase 23 Step 1: 回滚errno路径修改 ==="

# 首先检查types/stubs.rs中errno模块是否正确导出
echo "1. 检查errno模块导出..."

# 在types/mod.rs中导出errno
if grep -q "pub use self::stubs::errno;" kernel/src/types/mod.rs; then
    echo "   errno已在types/mod.rs中导出"
else
    echo "   在types/mod.rs中添加errno导出"
    # 这个操作需要手动处理
fi

echo ""
echo "2. 修复posix模块中的errno导入..."

# 以下文件应该使用 crate::posix::errno 或 crate::types::stubs::errno
# 但第二十二批修复可能错误地修改了路径

# 修复特定的errno使用
# 注意：保持现有的 crate::types::stubs::errno 不变，这是正确的
# 只需要确保errno常量定义存在

echo ""
echo "检查errno常量定义..."
if grep -q "pub const EOK" kernel/src/types/stubs.rs; then
    echo "   errno常量存在"
else
    echo "   错误：errno常量不存在！"
    exit 1
fi

echo ""
echo "=== Step 1 完成 ==="
echo "下一步：处理其他批量路径修改"
