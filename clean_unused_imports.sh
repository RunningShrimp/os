#!/bin/bash

# Script to check for unused imports in the specified directories

echo "Checking nos-api..."
cargo check --package nos-api 2>&1 | grep -E "warning:.*unused" > /tmp/nos-api-unused.txt

echo "Checking nos-syscalls..."
cargo check --package nos-syscalls 2>&1 | grep -E "warning:.*unused" > /tmp/nos-syscalls-unused.txt

echo "Checking nos-services..."
cargo check --package nos-services 2>&1 | grep -E "warning:.*unused" > /tmp/nos-services-unused.txt

echo "Checking nos-error-handling..."
cargo check --package nos-error-handling 2>&1 | grep -E "warning:.*unused" > /tmp/nos-error-handling-unused.txt

echo "===== nos-api unused imports ====="
cat /tmp/nos-api-unused.txt

echo ""
echo "===== nos-syscalls unused imports ====="
cat /tmp/nos-syscalls-unused.txt

echo ""
echo "===== nos-services unused imports ====="
cat /tmp/nos-services-unused.txt

echo ""
echo "===== nos-error-handling unused imports ====="
cat /tmp/nos-error-handling-unused.txt
