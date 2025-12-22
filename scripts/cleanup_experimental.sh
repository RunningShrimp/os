#!/bin/bash
# Cleanup Experimental Code Script
#
# This script scans the codebase for experimental/temporary files containing
# keywords like "enhanced", "optimized", "minimal" and generates a report
# to help identify files that need to be cleaned up or merged.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/temp/cleanup_analysis"
REPORT_FILE="$OUTPUT_DIR/experimental_files_report.md"
CSV_FILE="$OUTPUT_DIR/experimental_files.csv"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo -e "${GREEN}Scanning for experimental files...${NC}"

# Keywords to search for
KEYWORDS=("enhanced" "optimized" "minimal" "experimental" "temporary" "temp" "stub" "placeholder")

# File patterns to search
PATTERNS=("*.rs" "*.c" "*.h")

# Initialize arrays
declare -a ENHANCED_FILES
declare -a OPTIMIZED_FILES
declare -a MINIMAL_FILES
declare -a EXPERIMENTAL_FILES

# Function to check if file contains keyword
contains_keyword() {
    local file="$1"
    local keyword="$2"
    if grep -qi "$keyword" "$file" 2>/dev/null; then
        return 0
    fi
    return 1
}

# Function to check filename
filename_contains() {
    local file="$1"
    local keyword="$2"
    local basename=$(basename "$file")
    if echo "$basename" | grep -qi "$keyword"; then
        return 0
    fi
    return 1
}

# Function to analyze file
analyze_file() {
    local file="$1"
    local rel_path="${file#$PROJECT_ROOT/}"
    
    # Check filename
    if filename_contains "$file" "enhanced"; then
        ENHANCED_FILES+=("$rel_path")
        return
    fi
    
    if filename_contains "$file" "optimized"; then
        OPTIMIZED_FILES+=("$rel_path")
        return
    fi
    
    if filename_contains "$file" "minimal"; then
        MINIMAL_FILES+=("$rel_path")
        return
    fi
    
    # Check file content
    if contains_keyword "$file" "enhanced"; then
        ENHANCED_FILES+=("$rel_path")
    fi
    
    if contains_keyword "$file" "optimized"; then
        OPTIMIZED_FILES+=("$rel_path")
    fi
    
    if contains_keyword "$file" "minimal"; then
        MINIMAL_FILES+=("$rel_path")
    fi
    
    if contains_keyword "$file" "experimental" || contains_keyword "$file" "temporary" || contains_keyword "$file" "temp"; then
        EXPERIMENTAL_FILES+=("$rel_path")
    fi
}

# Scan files
echo "Scanning Rust files..."
while IFS= read -r -d '' file; do
    analyze_file "$file"
done < <(find "$PROJECT_ROOT" -type f \( -name "*.rs" -o -name "*.c" -o -name "*.h" \) -not -path "*/target/*" -not -path "*/.git/*" -not -path "*/node_modules/*" -print0)

# Remove duplicates
ENHANCED_FILES=($(printf '%s\n' "${ENHANCED_FILES[@]}" | sort -u))
OPTIMIZED_FILES=($(printf '%s\n' "${OPTIMIZED_FILES[@]}" | sort -u))
MINIMAL_FILES=($(printf '%s\n' "${MINIMAL_FILES[@]}" | sort -u))
EXPERIMENTAL_FILES=($(printf '%s\n' "${EXPERIMENTAL_FILES[@]}" | sort -u))

# Generate CSV report
echo "Generating CSV report..."
cat > "$CSV_FILE" <<EOF
Category,File Path,Status,Recommendation
EOF

for file in "${ENHANCED_FILES[@]}"; do
    echo "enhanced,$file,needs_review,Evaluate and merge into main implementation" >> "$CSV_FILE"
done

for file in "${OPTIMIZED_FILES[@]}"; do
    echo "optimized,$file,needs_review,Evaluate performance improvements and merge if beneficial" >> "$CSV_FILE"
done

for file in "${MINIMAL_FILES[@]}"; do
    echo "minimal,$file,needs_review,Check if still needed or can be removed" >> "$CSV_FILE"
done

for file in "${EXPERIMENTAL_FILES[@]}"; do
    echo "experimental,$file,needs_review,Review and remove if obsolete" >> "$CSV_FILE"
done

# Generate Markdown report
echo "Generating Markdown report..."
cat > "$REPORT_FILE" <<EOF
# Experimental Files Cleanup Report

Generated: $(date)

## Summary

- **Enhanced files**: ${#ENHANCED_FILES[@]}
- **Optimized files**: ${#OPTIMIZED_FILES[@]}
- **Minimal files**: ${#MINIMAL_FILES[@]}
- **Experimental files**: ${#EXPERIMENTAL_FILES[@]}
- **Total files to review**: $((${#ENHANCED_FILES[@]} + ${#OPTIMIZED_FILES[@]} + ${#MINIMAL_FILES[@]} + ${#EXPERIMENTAL_FILES[@]}))

## Enhanced Files (${#ENHANCED_FILES[@]})

These files contain "enhanced" in their name or content. They should be evaluated
and merged into the main implementation if they provide value.

EOF

for file in "${ENHANCED_FILES[@]}"; do
    echo "- \`$file\`" >> "$REPORT_FILE"
done

cat >> "$REPORT_FILE" <<EOF

## Optimized Files (${#OPTIMIZED_FILES[@]})

These files contain "optimized" in their name or content. They should be evaluated
for performance improvements and merged if beneficial.

EOF

for file in "${OPTIMIZED_FILES[@]}"; do
    echo "- \`$file\`" >> "$REPORT_FILE"
done

cat >> "$REPORT_FILE" <<EOF

## Minimal Files (${#MINIMAL_FILES[@]})

These files contain "minimal" in their name or content. They should be checked
if still needed or can be removed.

EOF

for file in "${MINIMAL_FILES[@]}"; do
    echo "- \`$file\`" >> "$REPORT_FILE"
done

cat >> "$REPORT_FILE" <<EOF

## Experimental Files (${#EXPERIMENTAL_FILES[@]})

These files contain "experimental", "temporary", or "temp" keywords. They should
be reviewed and removed if obsolete.

EOF

for file in "${EXPERIMENTAL_FILES[@]}"; do
    echo "- \`$file\`" >> "$REPORT_FILE"
done

cat >> "$REPORT_FILE" <<EOF

## Recommendations

1. **File System Enhanced Files**: 
   - \`kernel/src/subsystems/fs/ext4_enhanced.rs\`
   - \`kernel/src/subsystems/fs/ext4_enhanced_impl.rs\`
   - \`kernel/src/subsystems/fs/ext4_enhanced_impl2.rs\`
   - **Action**: Choose one as the official implementation (recommend ext4_enhanced_impl2), merge valuable features, and delete others.

2. **Network Enhanced Files**:
   - \`kernel/src/subsystems/net/routing_enhanced.rs\`
   - \`kernel/src/subsystems/net/enhanced_network.rs\`
   - \`kernel/src/subsystems/net/tcp_enhanced.rs\`
   - \`kernel/src/subsystems/net/icmp_enhanced.rs\`
   - **Action**: Merge enhancements into main network modules.

3. **Process Optimized Files**:
   - \`kernel/src/subsystems/process/lock_optimized.rs\`
   - **Action**: Evaluate lock optimizations and merge if beneficial.

4. **Memory Optimized Files**:
   - \`kernel/src/mm/optimized_page_allocator.rs\`
   - **Action**: Evaluate allocator optimizations and merge if beneficial.

5. **System Call Optimized Files**:
   - \`nos-syscalls/src/optimized_syscall_path.rs\`
   - \`kernel/src/syscall/optimized_arg_handler.rs\`
   - **Action**: These may be part of the fast-path optimization already implemented. Review and remove if redundant.

## Next Steps

1. Review each file in this report
2. Determine which features are valuable and should be merged
3. Create merge plan for each category
4. Execute merges and remove redundant files
5. Update documentation and tests

EOF

echo -e "${GREEN}Report generated successfully!${NC}"
echo -e "  - Markdown report: ${YELLOW}$REPORT_FILE${NC}"
echo -e "  - CSV report: ${YELLOW}$CSV_FILE${NC}"
echo ""
echo -e "${GREEN}Summary:${NC}"
echo -e "  Enhanced files: ${#ENHANCED_FILES[@]}"
echo -e "  Optimized files: ${#OPTIMIZED_FILES[@]}"
echo -e "  Minimal files: ${#MINIMAL_FILES[@]}"
echo -e "  Experimental files: ${#EXPERIMENTAL_FILES[@]}"
echo -e "  Total: $((${#ENHANCED_FILES[@]} + ${#OPTIMIZED_FILES[@]} + ${#MINIMAL_FILES[@]} + ${#EXPERIMENTAL_FILES[@]}))"

