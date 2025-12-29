# Performance Comparison Guide

## Overview

This guide explains how to run and interpret the performance benchmarks for the concurrent optimizations implemented in Workflow 2.

## Quick Start

```bash
# Validate implementation
./validate_concurrent_optimizations.sh

# Run all concurrent benchmarks (when dependencies are resolved)
cargo bench --bench concurrent_bench

# Run specific benchmark groups
cargo bench --bench concurrent_bench -- sharded_benches
cargo bench --bench concurrent_bench -- rcu_benches
cargo bench --bench concurrent_bench -- percpu_benches
cargo bench --bench concurrent_bench -- zone_benches
```

## Benchmark Categories

### 1. Sharded Process Table Benchmarks

#### `sharded_alloc_single_thread`
- **Purpose**: Baseline single-threaded allocation performance
- **Expected**: ~100-200ns per allocation
- **Comparison**: Baseline for multi-threaded tests

#### `sharded_concurrent_alloc/1`, `/2`, `/4`, `/8`
- **Purpose**: Measure scalability with 1-8 threads
- **Expected**: Near-linear scaling up to 8 cores
- **Metric**: Operations per second

#### `sharded_lookup/10`, `/50`, `/100`, `/500`
- **Purpose**: Lookup performance with different table sizes
- **Target**: < 50ns per lookup
- **Improvement**: 70% vs linear search

#### `sharded_load_balance`
- **Purpose**: Measure load distribution across shards
- **Target**: Standard deviation < 2.0
- **Lower is better**: Indicates even distribution

### 2. RCU Process Table Benchmarks

#### `rcu_read_lockfree`
- **Purpose**: Lock-free read performance
- **Target**: < 20ns per read
- **Key**: Zero mutex acquisitions

#### `rcu_write`
- **Purpose**: Write/update performance
- **Expected**: < 100ns
- **Note**: Includes copy and pointer swap

#### `rcu_reclamation`
- **Purpose**: Epoch-based memory reclamation
- **Metric**: Time to reclaim 100 entries
- **Safety**: No use-after-free

### 3. Per-CPU Allocator Benchmarks

#### `percpu_fast_path`
- **Purpose**: Fast path allocation from cache
- **Target**: < 20ns
- **Key**: No global lock

#### `percpu_hit_rate`
- **Purpose**: Measure cache effectiveness
- **Target**: > 95% hit rate
- **Impact**: Higher hit rate = better performance

### 4. Zone Allocator Benchmarks

#### `zone_alloc_fine_grained`
- **Purpose**: Per-zone locking performance
- **Expected**: < 100ns per allocation
- **Comparison**: vs global lock

#### `zone_contention_estimate`
- **Purpose**: Measure lock contention
- **Target**: Score < 1.0
- **Lower is better**: Less contention

### 5. Comparison Benchmarks

#### `lookup_comparison`
- **Baseline**: Linear search O(n)
- **Optimized**: Hash-based O(1)
- **Expected**: 70-90% improvement

#### `locking_comparison`
- **Baseline**: Global lock
- **Optimized**: Fine-grained locks
- **Expected**: 90% contention reduction

### 6. Scalability Benchmarks

#### `scalability/process_table/1` to `/8`
- **Purpose**: Multi-core scalability
- **Expected**: Near-linear scaling
- **Metric**: Operations per second vs thread count

## Interpreting Results

### Performance Metrics

| Metric | Good | Excellent |
|--------|------|-----------|
| Lookup latency | < 100ns | < 50ns |
| Read latency (RCU) | < 50ns | < 20ns |
| Cache hit rate | > 90% | > 95% |
| Lock contention | < 20% | < 10% |
| Scalability (8-core) | > 5x | > 7x |

### Key Performance Indicators

1. **Throughput Scaling**
   - Ideal: Linear (8 cores = 8x speedup)
   - Good: > 6x on 8 cores
   - Poor: < 4x on 8 cores

2. **Latency Distribution**
   - P50 (median): Should be fast
   - P99: Should be < 3x P50
   - Max: Should be bounded

3. **Cache Effectiveness**
   - Hit rate > 95%: Excellent
   - Hit rate > 90%: Good
   - Hit rate < 85%: Needs tuning

## Expected Performance Improvements

### Before Optimization

```
Process lookup (O(n) linear):
  100 processes: ~5,000ns
  500 processes: ~25,000ns

Memory allocation (global lock):
  8-core contention: ~90%
  Throughput: ~1M ops/sec
```

### After Optimization

```
Process lookup (O(1) sharded):
  100 processes: ~50ns (99% improvement)
  500 processes: ~50ns (99.8% improvement)

Memory allocation (per-CPU + fine-grained):
  8-core contention: < 10%
  Throughput: ~5M ops/sec (5x improvement)
```

## Generating Performance Reports

```bash
# Save baseline measurements
cargo bench --bench concurrent_bench -- --save-baseline before

# After changes, compare
cargo bench --bench concurrent_bench -- --baseline before

# Generate HTML report
cargo bench --bench concurrent_bench -- --output-format html
```

## Troubleshooting

### Low Cache Hit Rate

**Symptoms**: Per-CPU allocator hit rate < 90%

**Solutions**:
1. Increase `LOCAL_CACHE_SIZE` (default: 64)
2. Increase `BATCH_SIZE` (default: 32)
3. Check allocation pattern fragmentation

### High Lock Contention

**Symptoms**: Zone allocator contention score > 1.0

**Solutions**:
1. Increase `NUM_STRIPES` (default: 64)
2. Enable fine-grained locking: `enable_fine_grained()`
3. Check for hot spots in allocation pattern

### Poor Scalability

**Symptoms**: < 4x speedup on 8 cores

**Solutions**:
1. Verify NUMA awareness
2. Check for false sharing (use `repr(align(64))`)
3. Profile lock contention

## Advanced Analysis

### Flame Graph Generation

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bench concurrent_bench -- sharded_lookup
```

### Lock Contention Analysis

```bash
# Use perf to analyze lock contention
perf record -g -e lock:lock_acquire cargo bench --bench concurrent_bench
perf report
```

## Continuous Integration

Add to CI pipeline:

```yaml
benchmarks:
  script:
    - cargo bench --bench concurrent_bench -- --nocapture
  artifacts:
    paths:
      - target/criterion/
```

## Performance Regression Detection

Set up performance regression in CI:

```bash
# Store baseline
cargo bench --bench concurrent_bench -- --save-baseline main

# In CI PRs
cargo bench --bench concurrent_bench -- --baseline main --threshold 10
```

This will fail if performance regresses by > 10%.

## Summary

The concurrent optimizations target the following improvements:

- **70% reduction** in process lookup latency
- **Zero locks** for read-heavy workloads (RCU)
- **95%+ cache hit rate** for memory allocation
- **90% reduction** in lock contention
- **5x throughput** improvement on 8-core systems

All benchmarks are designed to validate these targets and detect regressions.
