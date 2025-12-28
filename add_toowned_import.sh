#!/bin/bash
# Files that need ToOwned import
files=(
  "kernel/src/platform/drivers/device_manager.rs"
  "kernel/src/platform/drivers/nvme.rs"
  "kernel/src/subsystems/cloud_native/cgroups.rs"
  "kernel/src/subsystems/cloud_native/container.rs"
  "kernel/src/subsystems/cloud_native/oci.rs"
)

for file in "${files[@]}"; do
  if [ -f "$file" ]; then
    # Check if file already has ToOwned import
    if grep -q "use alloc::borrow::ToOwned" "$file"; then
      echo "$file already has ToOwned import"
    else
      echo "Adding ToOwned to $file"
      # Add after extern crate alloc or after use alloc::
      if grep -q "^extern crate alloc" "$file"; then
        sed -i '' '/^extern crate alloc/a\
use alloc::borrow::ToOwned;
' "$file" 2>/dev/null || sed -i '/^extern crate alloc/a\
use alloc::borrow::ToOwned;
' "$file"
      elif grep -q "^use alloc::" "$file"; then
        # Find first use alloc:: and add after it
        sed -i '' '/^use alloc::/a\
use alloc::borrow::ToOwned;
' "$file" 2>/dev/null || sed -i '/^use alloc::/a\
use alloc::borrow::ToOwned;
' "$file"
      fi
    fi
  fi
done
