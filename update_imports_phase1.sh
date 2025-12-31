#!/bin/bash

# Track E - 导入路径更新脚本（Phase 1）
# 自动更新所有受影响的导入路径

set -e

KERNEL_SRC="/Users/wangbiao/Desktop/project/nos/kernel/src"
cd "$KERNEL_SRC" || exit 1

echo "========================================="
echo "Track E - 更新导入路径（Phase 1）"
echo "========================================="
echo ""

# 创建备份
find . -name "*.rs" -type f -exec cp {} {}.phase1_backup \; 2>/dev/null || true

update_count=0

# 处理每个 .rs 文件
while IFS= read -r -d '' file; do
    if [ ! -f "$file.phase1_backup" ]; then
        cp "$file" "$file.phase1_backup"
    fi

    changed=false

    # 规则 1: use crate::vfs::xxx -> use crate::subsystems::fs::vfs::xxx
    if grep -q "use crate::vfs::" "$file"; then
        sed -i.tmp '
            s/use crate::vfs::/use crate::subsystems::fs::vfs::/g
        ' "$file"
        rm -f "$file.tmp"
        changed=true
    fi

    # 规则 2: use crate::vfs{...} -> use crate::subsystems::fs::vfs{...}
    if grep -q "use crate::vfs{" "$file"; then
        sed -i.tmp '
            s/use crate::vfs{/use crate::subsystems::fs::vfs{/g
        ' "$file"
        rm -f "$file.tmp"
        changed=true
    fi

    # 规则 3: use crate::vfs_interface::xxx -> use crate::subsystems::fs::vfs_interface::xxx
    if grep -q "use crate::vfs_interface::" "$file"; then
        sed -i.tmp '
            s/use crate::vfs_interface::/use crate::subsystems::fs::vfs_interface::/g
        ' "$file"
        rm -f "$file.tmp"
        changed=true
    fi

    # 规则 4: use crate::vfs_interface{...} -> use crate::subsystems::fs::vfs_interface{...}
    if grep -q "use crate::vfs_interface{" "$file"; then
        sed -i.tmp '
            s/use crate::vfs_interface{/use crate::subsystems::fs::vfs_interface{/g
        ' "$file"
        rm -f "$file.tmp"
        changed=true
    fi

    # 规则 5: use crate::posix::xxx -> use crate::subsystems::posix::xxx
    if grep -q "use crate::posix::" "$file"; then
        sed -i.tmp '
            s/use crate::posix::/use crate::subsystems::posix::/g
        ' "$file"
        rm -f "$file.tmp"
        changed=true
    fi

    # 规则 6: use crate::posix{...} -> use crate::subsystems::posix{...}
    if grep -q "use crate::posix{" "$file"; then
        sed -i.tmp '
            s/use crate::posix{/use crate::subsystems::posix{/g
        ' "$file"
        rm -f "$file.tmp"
        changed=true
    fi

    if [ "$changed" = true ]; then
        update_count=$((update_count + 1))
        echo "✓ 已更新: $file"
    fi

done < <(find . -name "*.rs" -type f -print0)

echo ""
echo "========================================="
echo "更新完成！"
echo "========================================="
echo "总计更新文件数: $update_count"
echo ""
echo "备份文件: *.rs.phase1_backup"
echo ""
echo "如需恢复备份:"
echo "  find . -name '*.rs.phase1_backup' -exec sh -c 'mv \"\$1\" \"\${1%.phase1_backup}\"' _ {} \;"
echo ""

# 显示需要手动检查的文件
echo "可能需要手动检查的文件:"
grep -r "use crate::vfs[^/]" . --include="*.rs" 2>/dev/null | grep -v "subsystems/fs" | head -10 || echo "  无"
echo ""
grep -r "use crate::vfs_interface" . --include="*.rs" 2>/dev/null | grep -v "subsystems/fs" | head -10 || echo "  无"
echo ""
grep -r "use crate::posix[^/]" . --include="*.rs" 2>/dev/null | grep -v "subsystems/posix" | head -10 || echo "  无"
echo ""

echo "完成！"
