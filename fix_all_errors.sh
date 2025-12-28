#!/bin/bash
# Batch fix compilation errors in NOS kernel

echo "Running cargo check to count errors..."
cargo check 2>&1 | grep "error: could not compile" || true

# Get error counts by type
echo ""
echo "Error breakdown:"
cargo check 2>&1 | grep "^error\[E" | sed 's/.*\[E/E/' | sed 's/\].*//' | sort | uniq -c | sort -rn | head -20

echo ""
echo "Total errors:"
cargo check 2>&1 | grep "due to.*previous errors" | sed 's/.*due to \([0-9]*\).*/\1/'
