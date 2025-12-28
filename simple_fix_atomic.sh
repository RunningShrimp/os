#!/bin/bash
# Simple sed script to replace += with fetch_add
for file in kernel/src/libc/newlib.rs kernel/src/security/memory_audit.rs kernel/src/subsystems/io/uring/buffers.rs kernel/src/subsystems/mm/stats.rs; do
  if [ -f "$file" ]; then
    sed -i.bak 's/\.total_allocations += /.total_allocations.fetch_add(/g; s/;$/, Ordering::Relaxed);/g' "$file"
  fi
done
