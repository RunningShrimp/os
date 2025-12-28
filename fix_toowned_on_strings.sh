#!/bin/bash
# Files that need ToString import for to_owned() on string literals
files=(
  "kernel/src/api/error.rs"
  "kernel/src/api/interfaces.rs"
  "kernel/src/error/unified_framework.rs"
  "kernel/src/error/unified.rs"
  "kernel/src/platform/drivers/device_manager.rs"
  "kernel/src/platform/drivers/nvme.rs"
  "kernel/src/subsystems/cloud_native/cgroups.rs"
  "kernel/src/subsystems/cloud_native/container.rs"
  "kernel/src/subsystems/cloud_native/oci.rs"
)

for file in "${files[@]}"; do
  if [ -f "$file" ]; then
    # Check if file already has ToString import
    if grep -q "ToString" "$file"; then
      echo "$file already has ToString or ToOwned"
    else
      echo "Adding ToString to $file"
      # Add ToString import after other alloc imports
      if grep -q "^use alloc::string::String" "$file"; then
        # Change String to {String, ToString}
        sed -i '' 's/^use alloc::string::String;/use alloc::string::{String, ToString};/g' "$file" 2>/dev/null || \
        sed -i 's/^use alloc::string::String;/use alloc::string::{String, ToString};/g' "$file"
      else
        # Add after extern crate alloc or first use alloc::
        if grep -q "^extern crate alloc" "$file"; then
          sed -i '' '/^extern crate alloc/a\
use alloc::string::ToString;
' "$file" 2>/dev/null || sed -i '/^extern crate alloc/a\
use alloc::string::ToString;
' "$file"
        fi
      fi
    fi
  fi
done
