#!/bin/bash
# Analyze module dependencies in kernel/src/subsystems/syscalls

echo "=== Module Dependency Analysis ==="
echo ""
echo "Finding all use statements in syscalls modules..."
echo ""

# Find all use statements that reference other syscall modules
grep -rh "^use crate::subsystems::syscalls" kernel/src/subsystems/syscalls/ --include="*.rs" | \
    sed 's/.*use crate::subsystems::syscalls:://' | \
    sed 's/::.*$//' | \
    sed 's/;//' | \
    sort | uniq -c | sort -rn

echo ""
echo "=== Deep Nesting Analysis ==="
echo ""
echo "Directories with depth >= 4:"
find kernel/src/subsystems/syscalls -type d | awk -F/ '{print NF-2, $0}' | sort -rn | head -10

echo ""
echo "=== Files in Deepest Directories ==="
echo ""
echo "implementation/handlers/mm:"
ls -1 kernel/src/subsystems/syscalls/implementation/handlers/mm/*.rs 2>/dev/null | wc -l
echo ""

echo "implementation/handlers/fs:"
ls -1 kernel/src/subsystems/syscalls/implementation/handlers/fs.rs 2>/dev/null
echo ""

echo "security/access_control:"
ls -1 kernel/src/subsystems/syscalls/security/access_control/*.rs 2>/dev/null | wc -l
echo ""

echo "=== Module Structure ==="
echo ""
find kernel/src/subsystems/syscalls/ -type d -maxdepth 3 | sort
