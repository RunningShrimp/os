#!/bin/bash
files=(
  "kernel/src/api/error.rs"
  "kernel/src/api/interfaces.rs"
  "kernel/src/error/unified_framework.rs"
  "kernel/src/error/unified.rs"
)

for file in "${files[@]}"; do
  if [ -f "$file" ]; then
    if grep -q "use alloc::borrow::ToOwned" "$file"; then
      echo "$file already has ToOwned"
    else
      echo "Adding ToOwned to $file"
      # Add after use alloc::string
      sed -i '' '/^use alloc::string/a\
use alloc::borrow::ToOwned;
' "$file" 2>/dev/null || sed -i '/^use alloc::string/a\
use alloc::borrow::ToOwned;
' "$file"
    fi
  fi
done
