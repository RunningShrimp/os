#!/bin/bash

# 图形性能检查脚本

set -e

echo "Checking graphics performance..."

ERRORS_FOUND=false

GRAPHICS_DIR="bootloader/src/graphics"

if [ ! -d "$GRAPHICS_DIR" ]; then
    echo "Graphics directory not found, skipping performance checks"
    exit 0
fi

# 检查性能关键路径
echo "Checking performance-critical paths..."

GRAPHICS_FILES=$(find "$GRAPHICS_DIR" -name "*.rs")

for file in $GRAPHICS_FILES; do
    # 检查是否有不必要的内存分配
    if grep -q "Vec::new\|Box::new\|alloc" "$file"; then
        echo "⚠️  File $file contains memory allocations - review for optimization opportunities"
        
        # 检查是否有缓存或重用机制
        if ! grep -q "cache\|pool\|reuse\|static" "$file"; then
            echo "❌ Memory allocations in $file may benefit from caching or pooling"
            ERRORS_FOUND=true
        fi
    fi
    
    # 检查是否有循环优化机会
    if grep -q "for.*in\|while" "$file"; then
        # 检查循环内是否有重复计算
        if grep -A 5 -B 5 "for\|while" "$file" | grep -q "calculate\|compute"; then
            echo "⚠️  Loop in $file may contain repeated calculations - consider optimization"
        fi
    fi
    
    # 检查是否有同步等待
    if grep -q "wait\|sync\|block" "$file"; then
        echo "⚠️  File $file contains blocking operations - review for async alternatives"
        
        # 检查是否有超时机制
        if ! grep -q "timeout\|deadline\|async" "$file"; then
            echo "❌ Blocking operations in $file lack timeout mechanisms"
            ERRORS_FOUND=true
        fi
    fi
done

# 检查绘制操作性能
echo "Checking drawing operation performance..."

DRAWING_FILES=$(find "$GRAPHICS_DIR" -name "*draw*" -o -name "*render*" -o -name "*composite*")

for file in $DRAWING_FILES; do
    # 检查是否有批量操作
    if grep -q "draw\|render\|blit" "$file" && ! grep -q "batch\|bulk\|array" "$file"; then
        echo "⚠️  Drawing operations in $file may benefit from batching"
    fi
    
    # 检查是否有脏区域优化
    if grep -q "update\|refresh\|invalidate" "$file" && ! grep -q "dirty\|region\|area" "$file"; then
        echo "⚠️  Update operations in $file may benefit from dirty region optimization"
    fi
done

# 检查内存带宽使用
echo "Checking memory bandwidth usage..."

MEMORY_FILES=$(find "$GRAPHICS_DIR" -name "*buffer*" -o -name "*surface*" -o -name "*framebuffer*")

for file in $MEMORY_FILES; do
    # 检查是否有内存拷贝优化
    if grep -q "copy\|memcpy\|clone" "$file"; then
        echo "⚠️  File $file contains memory copy operations - review for optimization"
        
        # 检查是否有零拷贝或引用计数
        if ! grep -q "zero_copy\|ref_count\|borrow\|reference" "$file"; then
            echo "❌ Memory operations in $file may benefit from zero-copy or reference counting"
            ERRORS_FOUND=true
        fi
    fi
done

# 检查硬件加速利用
echo "Checking hardware acceleration utilization..."

HW_FILES=$(find "$GRAPHICS_DIR" -name "*hw*" -o -name "*accel*" -o -name "*gpu*")

for file in $HW_FILES; do
    # 检查是否有硬件特性检测
    if ! grep -q "detect\|query\|capability" "$file"; then
        echo "⚠️  Hardware acceleration file $file may lack capability detection"
    fi
    
    # 检查是否有软件回退
    if grep -q "hardware\|acceleration" "$file" && ! grep -q "fallback\|software\|software_render" "$file"; then
        echo "⚠️  Hardware acceleration in $file may lack software fallback"
    fi
done

if [ "$ERRORS_FOUND" = true ]; then
    echo ""
    echo "❌ Graphics performance check failed"
    exit 1
else
    echo ""
    echo "✅ Graphics performance check passed"
    exit 0
fi