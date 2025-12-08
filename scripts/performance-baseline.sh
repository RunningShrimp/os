#!/bin/bash

# Performance Baseline Management Script
# Manages performance baselines and detects regressions

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

BASELINE_DIR=".baselines"
BENCHMARK_RESULTS_DIR="target/criterion"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Create baseline directory if it doesn't exist
ensure_baseline_dir() {
    if [ ! -d "$BASELINE_DIR" ]; then
        mkdir -p "$BASELINE_DIR"
        print_status "Created baseline directory: $BASELINE_DIR"
    fi
}

# Save current benchmark results as baseline
save_baseline() {
    local baseline_name="${1:-current}"
    
    ensure_baseline_dir
    
    print_status "Saving baseline: $baseline_name"
    
    if [ ! -d "$BENCHMARK_RESULTS_DIR" ]; then
        print_error "Benchmark results not found. Run benchmarks first."
        exit 1
    fi
    
    # Copy benchmark results to baseline directory
    cp -r "$BENCHMARK_RESULTS_DIR" "$BASELINE_DIR/$baseline_name"
    
    # Save metadata
    cat > "$BASELINE_DIR/$baseline_name/metadata.json" <<EOF
{
    "name": "$baseline_name",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "git_commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
    "git_branch": "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown')"
}
EOF
    
    print_success "Baseline saved: $baseline_name"
}

# Load baseline for comparison
load_baseline() {
    local baseline_name="${1:-current}"
    
    if [ ! -d "$BASELINE_DIR/$baseline_name" ]; then
        print_error "Baseline not found: $baseline_name"
        exit 1
    fi
    
    echo "$BASELINE_DIR/$baseline_name"
}

# Compare current results with baseline
compare_with_baseline() {
    local baseline_name="${1:-current}"
    local threshold="${2:-0.10}"  # Default 10% threshold
    
    print_status "Comparing with baseline: $baseline_name"
    
    local baseline_path=$(load_baseline "$baseline_name")
    
    if [ ! -d "$BENCHMARK_RESULTS_DIR" ]; then
        print_error "Current benchmark results not found. Run benchmarks first."
        exit 1
    fi
    
    # Extract benchmark results from criterion output
    # This is a simplified comparison - in production, use criterion's comparison tools
    print_status "Comparing benchmark results..."
    
    # Check if criterion comparison is available
    if command -v cargo-criterion &> /dev/null; then
        cargo criterion --baseline "$baseline_name" --threshold "$threshold"
    else
        print_warning "cargo-criterion not found. Using basic comparison."
        print_status "Baseline: $baseline_path"
        print_status "Current:  $BENCHMARK_RESULTS_DIR"
        print_warning "Install cargo-criterion for detailed comparison: cargo install cargo-criterion"
    fi
}

# List available baselines
list_baselines() {
    print_status "Available baselines:"
    
    if [ ! -d "$BASELINE_DIR" ]; then
        print_warning "No baselines found"
        return
    fi
    
    for baseline in "$BASELINE_DIR"/*; do
        if [ -d "$baseline" ]; then
            local name=$(basename "$baseline")
            if [ -f "$baseline/metadata.json" ]; then
                local timestamp=$(grep -o '"timestamp": "[^"]*"' "$baseline/metadata.json" | cut -d'"' -f4)
                local commit=$(grep -o '"git_commit": "[^"]*"' "$baseline/metadata.json" | cut -d'"' -f4)
                echo "  $name - $timestamp ($commit)"
            else
                echo "  $name"
            fi
        fi
    done
}

# Check for performance regressions
check_regressions() {
    local baseline_name="${1:-current}"
    local threshold="${2:-0.10}"  # 10% performance degradation threshold
    
    print_status "Checking for performance regressions..."
    
    # Run benchmarks
    print_status "Running benchmarks..."
    cargo bench -p kernel --features kernel_tests -- --save-baseline current
    
    # Compare with baseline
    compare_with_baseline "$baseline_name" "$threshold"
    
    # Exit with error if regressions found (simplified check)
    # In production, parse criterion output and check for regressions
    print_success "Regression check completed"
}

# Main command handler
case "${1:-help}" in
    save)
        save_baseline "${2:-current}"
        ;;
    load)
        load_baseline "${2:-current}"
        ;;
    compare)
        compare_with_baseline "${2:-current}" "${3:-0.10}"
        ;;
    list)
        list_baselines
        ;;
    check)
        check_regressions "${2:-current}" "${3:-0.10}"
        ;;
    *)
        echo "Usage: $0 {save|load|compare|list|check} [baseline_name] [threshold]"
        echo ""
        echo "Commands:"
        echo "  save [name]     - Save current benchmark results as baseline"
        echo "  load [name]     - Load baseline path"
        echo "  compare [name] [threshold] - Compare current results with baseline"
        echo "  list            - List all available baselines"
        echo "  check [name] [threshold] - Run benchmarks and check for regressions"
        exit 1
        ;;
esac



