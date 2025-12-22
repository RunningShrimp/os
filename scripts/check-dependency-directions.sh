#!/bin/bash

# 依赖方向检查脚本

set -e

echo "Checking dependency directions..."

ERRORS_FOUND=false

# 定义正确的依赖方向
# Domain <- Application <- Infrastructure
# Domain <- Boot Stage
# Domain <- Graphics
# Domain <- Security
# Domain <- Drivers

# 检查依赖方向违规
echo "Checking for dependency direction violations..."

# 检查Domain层是否依赖其他层
DOMAIN_DEPS=$(find bootloader/src/domain -name "*.rs" -exec grep -l "use crate::" {} \; 2>/dev/null || true)
for file in $DOMAIN_DEPS; do
    DEPS=$(grep "^use crate::" "$file" | sed 's/use crate::\([^:]*\).*/\1/')
    for dep in $DEPS; do
        if [ "$dep" != "domain" ]; then
            echo "❌ Domain layer depends on $dep layer (violates dependency direction)"
            ERRORS_FOUND=true
        fi
    done
done

# 检查Application层是否依赖Infrastructure层
APP_DEPS=$(find bootloader/src/application -name "*.rs" -exec grep -l "use crate::infrastructure" {} \; 2>/dev/null || true)
for file in $APP_DEPS; do
    echo "❌ Application layer directly depends on Infrastructure layer in $file"
    ERRORS_FOUND=true
done

# 检查是否有循环依赖
echo "Checking for circular dependencies..."

# 构建依赖图
TEMP_DEP_FILE=$(mktemp)
find bootloader/src -name "*.rs" -exec grep -H "^use crate::" {} \; | \
    sed 's|bootloader/src/\([^/]*\)/.*:use crate::\([^:]*\).*|\1 \2|' | \
    sort -u > "$TEMP_DEP_FILE"

# 检查循环依赖（简化版）
while read -r from to; do
    if [ "$from" != "$to" ]; then
        # 检查反向依赖是否存在
        if grep -q "^$to $from$" "$TEMP_DEP_FILE"; then
            echo "❌ Circular dependency detected between $from and $to"
            ERRORS_FOUND=true
        fi
    fi
done < "$TEMP_DEP_FILE"

rm "$TEMP_DEP_FILE"

if [ "$ERRORS_FOUND" = true ]; then
    echo ""
    echo "❌ Dependency directions check failed"
    exit 1
else
    echo ""
    echo "✅ Dependency directions check passed"
    exit 0
fi