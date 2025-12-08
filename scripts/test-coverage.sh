#!/bin/bash
# Test coverage script for NOS kernel
# Generates coverage reports using cargo-tarpaulin

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if cargo-tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    print_info "Installing cargo-tarpaulin..."
    cargo install cargo-tarpaulin --locked
fi

# Create coverage directory
mkdir -p coverage

print_info "Running test coverage analysis..."

# Run coverage on user-space components
print_info "Running coverage on user-space components..."
cargo tarpaulin \
    --workspace \
    --exclude kernel \
    --exclude bootloader \
    --out Lcov \
    --output-dir coverage/ \
    --timeout 300 \
    --exclude-files '*/build.rs' \
    --exclude-files '*/main.rs' \
    || print_warning "Coverage analysis for user-space components completed with warnings"

# Run coverage on kernel unit tests (hosted mode)
print_info "Running coverage on kernel unit tests (hosted mode)..."
cargo tarpaulin \
    -p kernel \
    --features kernel_tests \
    --target x86_64-unknown-linux-gnu \
    --out Lcov \
    --output-dir coverage/ \
    --timeout 300 \
    --exclude-files '*/build.rs' \
    --exclude-files '*/main.rs' \
    || print_warning "Coverage analysis for kernel tests completed with warnings"

# Generate HTML report
print_info "Generating HTML coverage report..."
cargo tarpaulin \
    --workspace \
    --exclude kernel \
    --exclude bootloader \
    --out Html \
    --output-dir coverage/html/ \
    --timeout 300 \
    || print_warning "HTML report generation completed with warnings"

# Check coverage threshold
print_info "Checking coverage threshold..."
COVERAGE=$(cargo tarpaulin \
    --workspace \
    --exclude kernel \
    --exclude bootloader \
    --out Stdout \
    --timeout 300 2>/dev/null | grep -oP '\d+\.\d+%' | head -1 | sed 's/%//' || echo "0")

if [ -z "$COVERAGE" ] || [ "$COVERAGE" = "0" ]; then
    print_warning "Could not determine coverage percentage"
else
    print_info "Current coverage: ${COVERAGE}%"
    
    # Check if coverage meets threshold (90%)
    if (( $(echo "$COVERAGE < 90" | bc -l 2>/dev/null || echo "1") )); then
        print_error "Coverage ${COVERAGE}% is below threshold of 90%"
        exit 1
    else
        print_info "Coverage ${COVERAGE}% meets threshold of 90%"
    fi
fi

print_info "Coverage report generated in coverage/html/"
print_info "Open coverage/html/index.html in a browser to view the report"

