#!/bin/bash

set -e

PACKAGES=("nos-api" "nos-syscalls" "nos-services" "nos-error-handling")

echo "Starting unused import cleanup..."

for pkg in "${PACKAGES[@]}"; do
    echo ""
    echo "=== Processing $pkg ==="

    # Run cargo check and capture warnings
    echo "Running cargo check for $pkg..."
    cargo check --package "$pkg" 2>&1 | tee "/tmp/${pkg}-warnings.log" | grep -i "unused" || echo "No unused warnings found for $pkg"

    # Extract unused imports from warnings
    grep -E "warning: unused|warning: .*is unused" "/tmp/${pkg}-warnings.log" > "/tmp/${pkg}-unused-only.log" || true

    if [ -s "/tmp/${pkg}-unused-only.log" ]; then
        echo "Found unused imports in $pkg:"
        cat "/tmp/${pkg}-unused-only.log"
    else
        echo "No unused imports to fix in $pkg"
    fi
done

echo ""
echo "=== Summary ==="
echo "Check /tmp for detailed logs:"
for pkg in "${PACKAGES[@]}"; do
    echo "  /tmp/${pkg}-unused-only.log"
done
