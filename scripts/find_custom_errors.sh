#!/bin/bash
# Find and categorize custom error types in NOS kernel
# Usage: ./find_custom_errors.sh

set -e

echo "# NOS Kernel Custom Error Types Analysis"
echo ""
echo "**Generated:** $(date)"
echo ""

# Count error enums
ERROR_ENUMS=$(grep -rn "enum.*Error" kernel/src --include="*.rs" | wc -l | tr -d ' ')
ERROR_STRUCTS=$(grep -rn "pub struct.*Error" kernel/src --include="*.rs" | wc -l | tr -d ' ')

echo "## Summary"
echo ""
echo "- Custom Error Enums: $ERROR_ENUMS"
echo "- Custom Error Structs: $ERROR_STRUCTS"
echo "- Total Custom Error Types: $((ERROR_ENUMS + ERROR_STRUCTS))"
echo ""

echo "## Error Enums by Module"
echo ""
grep -rn "enum.*Error" kernel/src --include="*.rs" | \
    sed 's/kernel\/src\///' | \
    sed 's/\([^:]*\):\([0-9]*\):\(.*\)/- **\1**:\2 - \3/' | \
    sort

echo ""
echo "## Error Structs by Module"
echo ""
grep -rn "pub struct.*Error" kernel/src --include="*.rs" | \
    sed 's/kernel\/src\///' | \
    sed 's/\([^:]*\):\([0-9]*\):\(.*\)/- **\1**:\2 - \3/' | \
    sort

echo ""
echo "## Modules with Multiple Error Types"
echo ""
for file in $(grep -rl "enum.*Error\|pub struct.*Error" kernel/src --include="*.rs" -l | sed 's|kernel/src/||' | sed 's|/.*||' | sort -u); do
    count=$(grep "enum.*Error\|pub struct.*Error" "kernel/src/$file"*.rs 2>/dev/null | wc -l | tr -d ' ')
    if [ "$count" -gt "1" ]; then
        echo "- **$file**: $count error type(s)"
    fi
done

echo ""
echo "## Recommended Migration Order"
echo ""
echo "### Priority 0 (Critical)"
echo "1. **vfs** - VfsError, JournalError"
echo "2. **memory** - MemoryError"
echo "3. **posix** - SecurityError"
echo ""
echo "### Priority 1 (High)"
echo "4. **libc** - CLibError, LibcError"
echo "5. **subsystems/syscalls** - ValidationError"
echo "6. **platform/drivers** - NvmeError, UsbError"
echo ""
echo "### Priority 2 (Medium)"
echo "7. **compat** - ComError"
echo "8. **time** - SystemTimeError"
echo "9. **subsystems/formal_verification** - TypeError"
echo ""
echo "### Priority 3 (Low)"
echo "10. Test-related errors"
echo "11. Debug/analysis errors"
