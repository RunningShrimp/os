#!/bin/bash

# Track E - Phase 1: 移动 VFS 和 POSIX 模块
# 解决主要循环依赖

set -e  # 遇到错误立即退出

KERNEL_SRC="/Users/wangbiao/Desktop/project/nos/kernel/src"
cd "$KERNEL_SRC" || exit 1

echo "========================================="
echo "Track E - Phase 1: 移动 VFS 和 POSIX"
echo "========================================="
echo ""

# 创建备份
echo "Step 0: 创建备份点"
git add -A
git commit -m "Track E: Backup before Phase 1 refactoring" || echo "Already committed"
echo "✅ 备份完成"
echo ""

# Step 1: 移动 vfs/
echo "Step 1: 移动 vfs/ 到 subsystems/fs/vfs/"
if [ -d "vfs" ] && [ ! -d "subsystems/fs/vfs" ]; then
    mkdir -p subsystems/fs/vfs
    mv vfs/*.rs subsystems/fs/vfs/ 2>/dev/null || echo "  某些文件移动失败（可能已移动）"
    echo "✅ vfs/ 移动完成"
else
    echo "⚠️ vfs/ 已存在或目标已存在"
fi
echo ""

# Step 2: 移动 vfs_interface/
echo "Step 2: 移动 vfs_interface/ 到 subsystems/fs/vfs_interface/"
if [ -d "vfs_interface" ] && [ ! -d "subsystems/fs/vfs_interface" ]; then
    mkdir -p subsystems/fs/vfs_interface
    mv vfs_interface/*.rs subsystems/fs/vfs_interface/ 2>/dev/null || echo "  某些文件移动失败"
    echo "✅ vfs_interface/ 移动完成"
else
    echo "⚠️ vfs_interface/ 已存在或目标已存在"
fi
echo ""

# Step 3: 移动 posix/
echo "Step 3: 移动 posix/ 到 subsystems/posix/"
if [ -d "posix" ] && [ ! -d "subsystems/posix" ]; then
    mkdir -p subsystems/posix
    mv posix/*.rs subsystems/posix/ 2>/dev/null || echo "  某些文件移动失败"
    echo "✅ posix/ 移动完成"
else
    echo "⚠️ posix/ 已存在或目标已存在"
fi
echo ""

# Step 4: 更新导入路径
echo "Step 4: 批量更新导入路径"

# 备份所有 .rs 文件
find . -name "*.rs" -type f -exec cp {} {}.backup \;

# 更新 vfs 导入
echo "  更新 crate::vfs 导入..."
find . -name "*.rs" -type f ! -name "*.backup" -exec sed -i.bak '
    s/use crate::vfs::/use crate::subsystems::fs::vfs::/g
    s/use crate::vfs{/use crate::subsystems::fs::vfs{/g
' {} \;

# 更新 vfs_interface 导入
echo "  更新 crate::vfs_interface 导入..."
find . -name "*.rs" -type f ! -name "*.backup" -exec sed -i.bak '
    s/use crate::vfs_interface::/use crate::subsystems::fs::vfs_interface::/g
    s/use crate::vfs_interface{/use crate::subsystems::fs::vfs_interface{/g
' {} \;

# 更新 posix 导入
echo "  更新 crate::posix 导入..."
find . -name "*.rs" -type f ! -name "*.backup" -exec sed -i.bak '
    s/use crate::posix::/use crate::subsystems::posix::/g
    s/use crate::posix{/use crate::subsystems::posix{/g
' {} \;

# 清理 .bak 文件
find . -name "*.bak" -delete
echo "✅ 导入路径更新完成"
echo ""

# Step 5: 更新 lib.rs
echo "Step 5: 更新 lib.rs 模块声明"
if [ -f "lib.rs" ]; then
    # 创建 lib.rs.patch
    cat > librs_vfs_posix_patch.txt << 'EOF'
# 需要手动修改 lib.rs，添加以下 re-export:

# VFS re-export (移动到 subsystems/fs/vfs 后)
pub use crate::subsystems::fs::vfs;

# VFS interface re-export
pub use crate::subsystems::fs::vfs_interface;

# POSIX re-export (移动到 subsystems/posix 后)
pub use crate::subsystems::posix;
EOF

    # 自动添加 re-export（如果不存在）
    if ! grep -q "pub use crate::subsystems::fs::vfs" lib.rs; then
        # 在适当位置插入（在 subsystems 声明之后）
        sed -i.bak '/^pub mod subsystems;/a\
\
// Re-export VFS (moved to subsystems/fs/vfs)\
pub use crate::subsystems::fs::vfs;\
pub use crate::subsystems::fs::vfs_interface;\
\
// Re-export POSIX (moved to subsystems/posix)\
pub use crate::subsystems::posix;\
' lib.rs
        echo "✅ lib.rs 已更新（自动）"
    else
        echo "⚠️ lib.rs 可能已包含 re-export"
    fi
fi
echo ""

# Step 6: 验证目录结构
echo "Step 6: 验证目录结构"
echo "VFS 文件:"
ls -1 subsystems/fs/vfs/*.rs 2>/dev/null | wc -l | xargs echo "  文件数:"
echo "VFS interface 文件:"
ls -1 subsystems/fs/vfs_interface/*.rs 2>/dev/null | wc -l | xargs echo "  文件数:"
echo "POSIX 文件:"
ls -1 subsystems/posix/*.rs 2>/dev/null | wc -l | xargs echo "  文件数:"
echo ""

# Step 7: 编译测试
echo "Step 7: 尝试编译"
cd /Users/wangbiao/Desktop/project/nos/kernel
echo "运行 cargo build..."
if cargo build 2>&1 | tee build_phase1.log | tail -50; then
    echo "✅ 编译成功！"
else
    ERROR_COUNT=$(grep -c "^error" build_phase1.log || echo "0")
    echo "⚠️ 编译失败，发现 $ERROR_COUNT 个错误"
    echo "详细日志: build_phase1.log"
    echo ""
    echo "常见错误类型:"
    grep "^error\[" build_phase1.log | cut -d':' -f3 | sort | uniq -c | sort -rn | head -10
fi
echo ""

echo "========================================="
echo "Phase 1 完成！"
echo "========================================="
echo ""
echo "下一步:"
echo "1. 检查编译错误"
echo "2. 手动修复未自动更新的导入"
echo "3. 运行测试验证"
echo ""
echo "如需回滚:"
echo "  git reset --hard HEAD"
echo ""
