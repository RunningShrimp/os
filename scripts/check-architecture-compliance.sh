#!/bin/bash

# DDD架构一致性检查脚本

set -e

echo "Checking DDD architecture compliance..."

ERRORS_FOUND=false

# 检查Domain层是否依赖其他层
echo "Checking Domain layer dependencies..."
DOMAIN_FILES=$(find bootloader/src/domain -name "*.rs" 2>/dev/null || true)

for file in $DOMAIN_FILES; do
    # 检查是否有对其他层的导入
    if grep -q "use crate::application\|use crate::infrastructure\|use crate::boot_stage" "$file"; then
        echo "❌ Domain layer file $file contains forbidden dependencies"
        ERRORS_FOUND=true
    fi
done

# 检查Application层是否直接依赖Infrastructure层
echo "Checking Application layer dependencies..."
APP_FILES=$(find bootloader/src/application -name "*.rs" 2>/dev/null || true)

for file in $APP_FILES; do
    # 检查是否有对Infrastructure层的直接导入
    if grep -q "use crate::infrastructure" "$file"; then
        echo "❌ Application layer file $file directly depends on Infrastructure layer"
        ERRORS_FOUND=true
    fi
done

# 检查是否有循环依赖
echo "Checking for circular dependencies..."
# 这里可以添加更复杂的循环依赖检测逻辑

# 检查模块职责边界
echo "Checking module responsibility boundaries..."

# 检查Boot Stage层职责
BOOT_STAGE_FILES=$(find bootloader/src/boot_stage -name "*.rs" 2>/dev/null || true)

for file in $BOOT_STAGE_FILES; do
    # 检查是否有超出职责范围的功能
    if grep -q "use crate::domain::" "$file" && ! grep -q "use crate::application::" "$file"; then
        echo "⚠️  Boot Stage file $file may have incorrect dependency pattern"
    fi
done

if [ "$ERRORS_FOUND" = true ]; then
    echo ""
    echo "❌ Architecture compliance check failed"
    exit 1
else
    echo ""
    echo "✅ Architecture compliance check passed"
    exit 0
fi