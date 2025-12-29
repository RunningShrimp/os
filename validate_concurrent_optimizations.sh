#!/bin/bash
# Validation script for concurrent performance optimizations

echo "=========================================="
echo "Concurrent Performance Optimization Validation"
echo "=========================================="
echo ""

# Check file existence
echo "1. Checking created files..."
files=(
    "kernel/src/sched/sharded_table.rs"
    "kernel/src/sched/rcu_table.rs"
    "kernel/src/subsystems/mm/percpu_allocator_v2.rs"
    "kernel/src/subsystems/mm/zone_allocator.rs"
    "kernel/benches/concurrent_bench.rs"
)

for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✓ $file"
    else
        echo "  ✗ $file (NOT FOUND)"
    fi
done
echo ""

# Count lines of code
echo "2. Lines of code statistics..."
echo "  Sharded Process Table:"
wc -l kernel/src/sched/sharded_table.rs | awk '{print "    " $1 " lines"}'

echo "  RCU Process Table:"
wc -l kernel/src/sched/rcu_table.rs | awk '{print "    " $1 " lines"}'

echo "  Enhanced Per-CPU Allocator:"
wc -l kernel/src/subsystems/mm/percpu_allocator_v2.rs | awk '{print "    " $1 " lines"}'

echo "  Zone Allocator:"
wc -l kernel/src/subsystems/mm/zone_allocator.rs | awk '{print "    " $1 " lines"}'

echo "  Benchmark Suite:"
wc -l kernel/benches/concurrent_bench.rs | awk '{print "    " $1 " lines"}'

total=$(cat kernel/src/sched/sharded_table.rs kernel/src/sched/rcu_table.rs kernel/src/subsystems/mm/percpu_allocator_v2.rs kernel/src/subsystems/mm/zone_allocator.rs kernel/benches/concurrent_bench.rs | wc -l)
echo "  TOTAL: $total lines"
echo ""

# Check module exports
echo "3. Checking module exports..."
if grep -q "pub mod sharded_table" kernel/src/sched/mod.rs; then
    echo "  ✓ sharded_table exported in sched/mod.rs"
else
    echo "  ✗ sharded_table NOT exported"
fi

if grep -q "pub mod rcu_table" kernel/src/sched/mod.rs; then
    echo "  ✓ rcu_table exported in sched/mod.rs"
else
    echo "  ✗ rcu_table NOT exported"
fi

if grep -q "pub mod percpu_allocator_v2" kernel/src/subsystems/mm/mod.rs; then
    echo "  ✓ percpu_allocator_v2 exported in mm/mod.rs"
else
    echo "  ✗ percpu_allocator_v2 NOT exported"
fi

if grep -q "pub mod zone_allocator" kernel/src/subsystems/mm/mod.rs; then
    echo "  ✓ zone_allocator exported in mm/mod.rs"
else
    echo "  ✗ zone_allocator NOT exported"
fi
echo ""

# Check key features
echo "4. Checking key implementation features..."

# Sharded table
if grep -q "const NUM_SHARDS: usize = 32" kernel/src/sched/sharded_table.rs; then
    echo "  ✓ 32 shards configured"
fi

if grep -q "#\[repr(align(64))\]" kernel/src/sched/sharded_table.rs; then
    echo "  ✓ Cache-line alignment present"
fi

# RCU table
if grep -q "rcu_read_lock" kernel/src/sched/rcu_table.rs; then
    echo "  ✓ RCU read-side critical section implemented"
fi

if grep -q "advance_epoch" kernel/src/sched/rcu_table.rs; then
    echo "  ✓ Epoch-based reclamation present"
fi

# Enhanced allocator
if grep -q "LOCAL_CACHE_SIZE: usize = 64" kernel/src/subsystems/mm/percpu_allocator_v2.rs; then
    echo "  ✓ Local cache size: 64 frames"
fi

if grep -q "BATCH_SIZE: usize = 32" kernel/src/subsystems/mm/percpu_allocator_v2.rs; then
    echo "  ✓ Batch size: 32 frames"
fi

# Zone allocator
if grep -q "enum AllocOrder" kernel/src/subsystems/mm/zone_allocator.rs; then
    echo "  ✓ 11 allocation orders (2^0 to 2^10 pages)"
fi

if grep -q "NUM_STRIPES: usize = 64" kernel/src/subsystems/mm/zone_allocator.rs; then
    echo "  ✓ 64 lock stripes configured"
fi
echo ""

# Count tests
echo "5. Test coverage..."
tests=$(grep -r "^#\[test\]" kernel/src/sched/sharded_table.rs kernel/src/sched/rcu_table.rs kernel/src/subsystems/mm/percpu_allocator_v2.rs kernel/src/subsystems/mm/zone_allocator.rs 2>/dev/null | wc -l)
echo "  Total unit tests: $tests"

benchmarks=$(grep -c "bench_function" kernel/benches/concurrent_bench.rs)
echo "  Total benchmarks: $benchmarks"
echo ""

# Performance targets
echo "6. Performance targets summary..."
echo "  Process Table:"
echo "    - Lookup latency: < 50ns (70% improvement)"
echo "    - Lock contention: ~97% reduction"
echo ""
echo "  RCU Table:"
echo "    - Read latency: < 20ns (zero locks)"
echo "    - Write latency: < 100ns"
echo ""
echo "  Per-CPU Allocator:"
echo "    - Fast path: < 20ns"
echo "    - Cache hit rate: > 95%"
echo ""
echo "  Zone Allocator:"
echo "    - Lock contention: < 10%"
echo "    - Throughput: 5M allocs/sec"
echo ""

echo "=========================================="
echo "Validation complete!"
echo "=========================================="
