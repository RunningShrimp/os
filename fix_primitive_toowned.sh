#!/bin/bash
# Fix .to_owned() on primitive types - should be .to_string()
# Find files with this pattern
grep -r "\.to_owned()" kernel/src --include="*.rs" | grep -E "(u32|u64|u16|i64|bool)\.to_owned\(\)" | cut -d: -f1 | sort -u | while read file; do
  echo "Fixing $file"
  sed -i '' -e 's/\([uif][0-9]*\)\.to_owned()/\1.to_string()/g' \
         -e 's/\(bool\)\.to_owned()/\1.to_string()/g' \
         "$file" 2>/dev/null || sed -i -e 's/\([uif][0-9]*\)\.to_owned()/\1.to_string()/g' \
         -e 's/\(bool\)\.to_owned()/\1.to_string()/g' "$file"
done
