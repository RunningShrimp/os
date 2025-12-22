#!/bin/bash

# 性能回归检查脚本
# 用法: ./check-performance-regression.sh <baseline_branch> <regression_threshold_percent>

set -e

BASELINE_BRANCH=$1
REGRESSION_THRESHOLD=${2:-10}  # 默认10%回归阈值

if [ -z "$BASELINE_BRANCH" ]; then
    echo "Usage: $0 <baseline_branch> [regression_threshold_percent]"
    exit 1
fi

echo "Checking performance regression against $BASELINE_BRANCH baseline"
echo "Regression threshold: $REGRESSION_THRESHOLD%"

# 检查基线数据是否存在
if [ ! -d ".baselines/$BASELINE_BRANCH" ]; then
    echo "No baseline data found for branch $BASELINE_BRANCH"
    exit 0
fi

# 比较基准测试结果
REGRESSION_FOUND=false

# 检查关键基准测试
BENCHMARKS=(
    "bootloader::boot_time"
    "kernel::memory_allocation"
    "kernel::system_call"
    "graphics::framebuffer_operations"
)

for benchmark in "${BENCHMARKS[@]}"; do
    if [ -f "target/criterion/$benchmark/benchmark.json" ] && [ -f ".baselines/$BASELINE_BRANCH/$benchmark/benchmark.json" ]; then
        CURRENT_TIME=$(jq -r '.median.est' "target/criterion/$benchmark/benchmark.json")
        BASELINE_TIME=$(jq -r '.median.est' ".baselines/$BASELINE_BRANCH/$benchmark/benchmark.json")
        
        if [ "$CURRENT_TIME" != "null" ] && [ "$BASELINE_TIME" != "null" ]; then
            REGRESSION_PERCENT=$(echo "scale=2; (($CURRENT_TIME - $BASELINE_TIME) / $BASELINE_TIME) * 100" | bc)
            
            echo "Benchmark: $benchmark"
            echo "  Current:  ${CURRENT_TIME}s"
            echo "  Baseline: ${BASELINE_TIME}s"
            echo "  Change:   ${REGRESSION_PERCENT}%"
            
            # 检查是否超过回归阈值
            if (( $(echo "$REGRESSION_PERCENT > $REGRESSION_THRESHOLD" | bc -l) )); then
                echo "  ❌ REGRESSION DETECTED: Performance regression of ${REGRESSION_PERCENT}% exceeds threshold of ${REGRESSION_THRESHOLD}%"
                REGRESSION_FOUND=true
            else
                echo "  ✅ No significant regression"
            fi
        fi
    fi
done

if [ "$REGRESSION_FOUND" = true ]; then
    echo ""
    echo "❌ Performance regression detected in one or more benchmarks"
    exit 1
else
    echo ""
    echo "✅ No significant performance regressions detected"
    exit 0
fi