#!/bin/bash

# 图形模块安全检查脚本

set -e

echo "Checking graphics module safety..."

ERRORS_FOUND=false

GRAPHICS_DIR="bootloader/src/graphics"

if [ ! -d "$GRAPHICS_DIR" ]; then
    echo "Graphics directory not found, skipping safety checks"
    exit 0
fi

# 检查显存访问安全性
echo "Checking graphics memory access safety..."

GRAPHICS_FILES=$(find "$GRAPHICS_DIR" -name "*.rs")

for file in $GRAPHICS_FILES; do
    # 检查原始指针使用
    if grep -q "\*mut\|\*const" "$file"; then
        echo "⚠️  File $file uses raw pointers - ensure safety checks are in place"
        
        # 检查是否有边界检查
        if ! grep -q "bounds\|check\|validate" "$file"; then
            echo "❌ Raw pointer usage in $file lacks bounds checking"
            ERRORS_FOUND=true
        fi
    fi
    
    # 检查不安全代码块
    if grep -q "unsafe" "$file"; then
        echo "⚠️  File $file contains unsafe code"
        
        # 检查是否有安全文档
        if ! grep -q "# Safety" "$file"; then
            echo "❌ Unsafe code in $file lacks safety documentation"
            ERRORS_FOUND=true
        fi
    fi
    
    # 检查内存映射操作
    if grep -q "memory_map\|mmap\|MMIO" "$file"; then
        echo "⚠️  File $file performs memory mapping operations"
        
        # 检查是否有适当的权限控制
        if ! grep -q "permission\|access\|protect" "$file"; then
            echo "❌ Memory mapping in $file lacks permission checks"
            ERRORS_FOUND=true
        fi
    fi
done

# 检查VBE模式设置安全性
echo "Checking VBE mode setting safety..."

VBE_FILES=$(find "$GRAPHICS_DIR" -name "*vbe*" -o -name "*mode*")

for file in $VBE_FILES; do
    # 检查模式参数验证
    if grep -q "set_mode\|VBE" "$file"; then
        if ! grep -q "validate\|check\|verify" "$file"; then
            echo "❌ VBE mode setting in $file lacks parameter validation"
            ERRORS_FOUND=true
        fi
    fi
done

# 检查缓冲区操作安全性
echo "Checking buffer operation safety..."

BUFFER_FILES=$(find "$GRAPHICS_DIR" -name "*buffer*" -o -name "*surface*")

for file in $BUFFER_FILES; do
    # 检查缓冲区边界检查
    if grep -q "buffer\|surface" "$file"; then
        # 检查是否有索引或长度检查
        if ! grep -q "index\|len\|size\|bounds" "$file"; then
            echo "❌ Buffer operations in $file may lack bounds checking"
            ERRORS_FOUND=true
        fi
    fi
done

if [ "$ERRORS_FOUND" = true ]; then
    echo ""
    echo "❌ Graphics safety check failed"
    exit 1
else
    echo ""
    echo "✅ Graphics safety check passed"
    exit 0
fi