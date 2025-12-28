#!/bin/bash
files=(
  "kernel/src/ids/host_ids/host_ids.rs"
  "kernel/src/ids/mod.rs"
  "kernel/src/ids/network_ids.rs"
)
for file in "${files[@]}"; do
  if [ -f "$file" ]; then
    if grep -q "use alloc::string::ToString" "$file"; then
      echo "$file already has ToString"
    else
      echo "Adding ToString to $file"
      sed -i '' '/^use alloc::[^s]/a\
use alloc::string::ToString;
' "$file" 2>/dev/null || sed -i '/^use alloc::[^s]/a\
use alloc::string::ToString;
' "$file"
    fi
  fi
done
