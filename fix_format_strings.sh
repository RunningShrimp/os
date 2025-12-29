#!/bin/bash

# Script to identify format string TODOs that can be auto-fixed
# This script analyzes the code and generates suggested fixes

echo "=== Format String TODO Fix Suggestions ==="
echo ""
echo "Scanning for format string TODOs..."
echo ""

# Find all format string TODOs
grep -r "TODO: {::" /Users/wangbiao/Desktop/project/nos/kernel/src --include="*.rs" -n | while read -r line; do
    file=$(echo "$line" | cut -d: -f1)
    lineno=$(echo "$line" | cut -d: -f2)
    content=$(echo "$line" | cut -d: -f3-)

    echo "File: $file"
    echo "Line: $lineno"
    echo "Original: $content"
    echo ""

    # Extract context from the file
    sed -n "${lineno}p" "$file"
    echo ""
    echo "---"
    echo ""
done

echo "=== Summary ==="
total=$(grep -r "TODO: {::" /Users/wangbiao/Desktop/project/nos/kernel/src --include="*.rs" | wc -l)
echo "Found $total format string TODOs"
echo ""
echo "Common patterns:"
echo "1. /* TODO: {::016x} */ -> format!(\"{:016x}\", value)"
echo "2. /* TODO: {::08x} */  -> format!(\"{:08x}\", value)"
echo "3. /* TODO: {::02} */   -> format!(\"{:02}\", value)"
echo "4. /* TODO: {::?} */    -> format!(\"{:?}\", value)"
echo "5. /* TODO: {::.1$} */  -> format!(\"{:.1$}\", value, precision)"
