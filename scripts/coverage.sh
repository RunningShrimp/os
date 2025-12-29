#!/bin/bash

# Tarpaulin Coverage Script
# This script runs test coverage analysis using cargo-tarpaulin

set -e

echo "Running tarpaulin coverage analysis..."
echo "======================================="

# Run tarpaulin with workspace coverage
cargo tarpaulin --workspace --out Html --out Xml -- --test-threads=1

echo "======================================="
echo "Coverage report generated!"
echo "HTML report: coverage/index.html"
echo "XML report: cobertura.xml"
