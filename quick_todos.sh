#!/bin/bash
# Find easy-to-fix TODOs that can be quickly resolved

echo "=== Easy TODOs to Fix ==="
echo ""

echo "1. Simple stub implementations that can use existing code:"
grep -rn "TODO.*Implement.*get\|TODO.*Implement.*find" kernel/src/subsystems/syscalls/ --include="*.rs" | head -10

echo ""
echo "2. TODOs that can be replaced with 'Not supported' errors:"
grep -rn "TODO.*implement" kernel/src/posix/ --include="*.rs" | grep -i "shared\|pshared" | head -5

echo ""
echo "3. Obvious obsolete TODOs (feature already exists):"
grep -rn "TODO.*Implement.*table" kernel/src/subsystems/ --include="*.rs" | head -5

echo ""
echo "4. Simple TODOs that just need error returns:"
grep -rn "TODO.*Implement.*proper" kernel/src/ --include="*.rs" | head -5
