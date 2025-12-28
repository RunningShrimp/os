#!/bin/bash
# Files that need AtomicU64 += converted to fetch_add
files=(
  "kernel/src/libc/newlib.rs"
  "kernel/src/security/memory_audit.rs"
  "kernel/src/subsystems/io/uring/buffers.rs"
  "kernel/src/subsystems/mm/stats.rs"
)

for file in "${files[@]}"; do
  if [ -f "$file" ]; then
    echo "Fixing $file"
    # Replace field += value with field.fetch_add(value, Ordering::Relaxed)
    # But be careful not to replace fetch_add lines
    sed -i '' -e 's/\([a-z_]*\)\.total_allocations += \([0-9]\+\);\?$/\1.total_allocations.fetch_add(\2, Ordering::Relaxed);/g' \
           -e 's/\([a-z_]*\)\.total_deallocations += \([0-9]\+\);\?$/\1.total_deallocations.fetch_add(\2, Ordering::Relaxed);/g' \
           -e 's/\([a-z_]*\)\.current_allocations += \([0-9]\+\);\?$/\1.current_allocations.fetch_add(\2, Ordering::Relaxed);/g' \
           -e 's/\([a-z_]*\)\.total_allocated_bytes += \([a-z_0-9.]\+\);\?$/\1.total_allocated_bytes.fetch_add(\2, Ordering::Relaxed);/g' \
           -e 's/\([a-z_]*\)\.current_allocated_bytes += \([a-z_0-9.]\+\);\?$/\1.current_allocated_bytes.fetch_add(\2, Ordering::Relaxed);/g' \
           "$file" 2>/dev/null || echo "  macOS sed failed, trying GNU sed"
  fi
done
