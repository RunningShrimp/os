#!/bin/bash
find kernel/src -name "*.rs" -type f | while read file; do
  if grep -q "^use alloc::{" "$file"; then
    if grep -A1 "^use alloc::{" "$file" | grep -q "use alloc::string::ToString"; then
      echo "$file"
    fi
  fi
done
