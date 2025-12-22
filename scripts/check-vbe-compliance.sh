#!/bin/bash

# VBE合规性检查脚本

set -e

echo "Checking VBE compliance..."

ERRORS_FOUND=false

GRAPHICS_DIR="bootloader/src/graphics"

if [ ! -d "$GRAPHICS_DIR" ]; then
    echo "Graphics directory not found, skipping VBE compliance checks"
    exit 0
fi

# 检查VBE标准实现
echo "Checking VBE standard implementation..."

VBE_FILES=$(find "$GRAPHICS_DIR" -name "*vbe*" -o -name "*vesa*")

if [ -z "$VBE_FILES" ]; then
    echo "⚠️  No VBE implementation files found"
    exit 0
fi

for file in $VBE_FILES; do
    echo "Checking VBE file: $file"
    
    # 检查VBE功能实现
    VBE_FUNCTIONS=(
        "VBEController"
        "get_mode_info"
        "set_mode"
        "get_current_mode"
        "find_mode"
    )
    
    for func in "${VBE_FUNCTIONS[@]}"; do
        if ! grep -q "$func" "$file"; then
            echo "⚠️  VBE function $func not found in $file"
        fi
    done
    
    # 检查VBE模式信息结构
    if ! grep -q "VBEModeInfo\|VbeModeInfo" "$file"; then
        echo "❌ VBE mode info structure not found in $file"
        ERRORS_FOUND=true
    fi
    
    # 检查VBE控制器信息
    if ! grep -q "VBEControllerInfo\|VbeControllerInfo" "$file"; then
        echo "❌ VBE controller info structure not found in $file"
        ERRORS_FOUND=true
    fi
    
    # 检查错误处理
    if grep -q "VBE\|vbe" "$file" && ! grep -q "Result\|Error\|fail" "$file"; then
        echo "❌ VBE operations in $file lack proper error handling"
        ERRORS_FOUND=true
    fi
done

# 检查EDID支持
echo "Checking EDID support..."

EDID_FILES=$(find "$GRAPHICS_DIR" -name "*edid*" -o -name "*monitor*")

for file in $EDID_FILES; do
    # 检查EDID解析
    if ! grep -q "parse\|decode\|read" "$file"; then
        echo "⚠️  EDID file $file may lack parsing functionality"
    fi
    
    # 检查显示器能力检测
    if ! grep -q "capabilities\|support\|features" "$file"; then
        echo "⚠️  EDID file $file may lack capability detection"
    fi
done

# 检查多显示器支持
echo "Checking multi-monitor support..."

MULTI_FILES=$(find "$GRAPHICS_DIR" -name "*multi*" -o -name "*display*")

for file in $MULTI_FILES; do
    # 检查显示器枚举
    if grep -q "display\|monitor" "$file" && ! grep -q "enumerate\|list\|count" "$file"; then
        echo "⚠️  Display file $file may lack enumeration support"
    fi
done

if [ "$ERRORS_FOUND" = true ]; then
    echo ""
    echo "❌ VBE compliance check failed"
    exit 1
else
    echo ""
    echo "✅ VBE compliance check passed"
    exit 0
fi