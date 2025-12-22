#!/bin/bash

# 模块边界检查脚本

set -e

echo "Checking module boundaries..."

ERRORS_FOUND=false

# 定义允许的模块间依赖关系
declare -A ALLOWED_DEPENDENCIES
ALLOWED_DEPENDENCIES=(
    ["application"]="domain"
    ["infrastructure"]="domain"
    ["boot_stage"]="application domain"
    ["graphics"]="domain infrastructure"
    ["security"]="domain infrastructure"
    ["drivers"]="domain infrastructure"
)

# 检查每个模块的依赖
for module in "${!ALLOWED_DEPENDENCIES[@]}"; do
    echo "Checking module: $module"
    
    MODULE_PATH="bootloader/src/$module"
    if [ ! -d "$MODULE_PATH" ]; then
        continue
    fi
    
    MODULE_FILES=$(find "$MODULE_PATH" -name "*.rs")
    
    for file in $MODULE_FILES; do
        # 检查是否有不允许的依赖
        for dep in $(grep -h "^use crate::" "$file" | sed 's/use crate::\([^:]*\).*/\1/' | sort -u); do
            if [ "$dep" != "$module" ]; then
                # 检查是否在允许的依赖列表中
                if [[ ! " ${ALLOWED_DEPENDENCIES[$module]} " =~ " $dep " ]]; then
                    echo "❌ Module $module has forbidden dependency on $dep in file $file"
                    ERRORS_FOUND=true
                fi
            fi
        done
    done
done

# 检查公共API的合理性
echo "Checking public API boundaries..."

# 检查是否有不合理的pub use
for file in $(find bootloader/src -name "mod.rs"); do
    if grep -q "pub use.*::" "$file"; then
        echo "⚠️  File $file contains pub use statements - review for proper API boundaries"
    fi
done

if [ "$ERRORS_FOUND" = true ]; then
    echo ""
    echo "❌ Module boundaries check failed"
    exit 1
else
    echo ""
    echo "✅ Module boundaries check passed"
    exit 0
fi