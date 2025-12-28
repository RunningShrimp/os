#!/bin/bash
for file in kernel/src/api/error.rs kernel/src/api/interfaces.rs kernel/src/error/unified_framework.rs kernel/src/error/unified.rs; do
  echo "=== $file ==="
  grep "ToOwned\|ToString" "$file" | head -3
done
