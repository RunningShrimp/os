#!/bin/bash

# 修复 GLib 类型导入问题

# 为所有 GLib 文件添加必要的导入
for file in user/src/glib/*.rs; do
    if [[ -f "$file" ]]; then
        # 检查是否已经导入了 types
        if ! grep -q "types::*" "$file"; then
            # 在第一个 use 语句后添加 types 导入
            sed -i '' '/^use crate::glib::{/a\
    types::*,' "$file"
        fi
    fi
done

echo "类型导入修复完成"
