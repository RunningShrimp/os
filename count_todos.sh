#!/bin/bash
# Count and categorize TODO/FIXME/XXX comments

echo "=== TODO/FIXME/XXX Comment Analysis ==="
echo ""

# Count total TODOs (all variations)
TODO_COUNT=$(grep -r "TODO\|FIXME\|XXX" kernel/src/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')

echo "Total TODO/FIXME/XXX comments: $TODO_COUNT"
echo ""

# Count by type
echo "--- Breakdown by Type ---"
TODO_SIMPLE=$(grep -r "TODO:" kernel/src/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
FIXME_COUNT=$(grep -r "FIXME:" kernel/src/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
XXX_COUNT=$(grep -r "XXX:" kernel/src/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')

echo "TODO:   $TODO_SIMPLE"
echo "FIXME:  $FIXME_COUNT"
echo "XXX:    $XXX_COUNT"
echo ""

# Count by module
echo "--- Top 10 Modules with Most TODOs ---"
grep -r "TODO\|FIXME\|XXX" kernel/src/ --include="*.rs" 2>/dev/null | \
    sed 's|kernel/src/||' | \
    sed 's|/.*$||' | \
    sort | uniq -c | sort -rn | head -10

echo ""
echo "--- Sample TODOs (first 10) ---"
grep -r "TODO\|FIXME\|XXX" kernel/src/ --include="*.rs" 2>/dev/null | head -10 | \
    sed 's|^|  |'

