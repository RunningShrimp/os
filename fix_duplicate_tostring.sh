#!/bin/bash
# Find and fix duplicate ToString imports
grep -l "use alloc::string::ToString" kernel/src/**/*.rs kernel/src/**/**/*.rs 2>/dev/null | while read file; do
  # Count occurrences
  count=$(grep -c "use alloc::string::ToString" "$file")
  if [ "$count" -gt 1 ]; then
    echo "Fixing duplicates in $file (found $count occurrences)"
    # Keep first occurrence, remove the rest
    awk '
      /^use alloc::string::ToString;/ {
        if (!seen) {
          print
          seen = 1
        }
        next
      }
      {print}
    ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
  fi
done
