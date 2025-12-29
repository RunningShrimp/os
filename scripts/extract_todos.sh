#!/bin/bash
# Extract TODO comments from NOS kernel source
# Usage: ./extract_todos.sh [options]
#
# Options:
#   --format=markdown   Output in Markdown format (default)
#   --format=github     Output in GitHub issue format
#   --priority=P0       Filter by priority (P0, P1, P2, P3)
#   --module=vfs        Filter by module name
#   --help              Show this help message

set -e

# Default values
FORMAT="markdown"
PRIORITY=""
MODULE=""
OUTPUT_FILE="todos.md"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --format=*)
            FORMAT="${1#*=}"
            ;;
        --priority=*)
            PRIORITY="${1#*=}"
            ;;
        --module=*)
            MODULE="${1#*=}"
            ;;
        --output=*)
            OUTPUT_FILE="${1#*=}"
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --format=markdown   Output in Markdown format (default)"
            echo "  --format=github     Output in GitHub issue format"
            echo "  --priority=P0       Filter by priority (P0, P1, P2, P3)"
            echo "  --module=vfs        Filter by module name"
            echo "  --output=FILE       Output file (default: todos.md)"
            echo "  --help              Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
    shift
done

# Create output header
if [ "$FORMAT" = "markdown" ]; then
    cat > "$OUTPUT_FILE" << 'HEADER'
# NOS Kernel TODO Comments

This document contains all TODO comments extracted from the NOS kernel source code.

**Generated:** $(date)
**Total TODOs:** $(grep -r "TODO" kernel/src --include="*.rs" | wc -l | tr -d ' ')

## Priority Legend

- **P0** - Critical: Bugs, security issues, correctness problems
- **P1** - High: Core features, performance, important functionality
- **P2** - Medium: Nice-to-have features, enhancements
- **P3** - Low: Documentation, minor cleanup, optimizations

---

HEADER
elif [ "$FORMAT" = "github" ]; then
    cat > "$OUTPUT_FILE" << 'HEADER'
# NOS Kernel TODO Items - GitHub Issues Format

This file contains TODO comments formatted for creating GitHub issues.

**Generated:** $(date)
**Total TODOs:** $(grep -r "TODO" kernel/src --include="*.rs" | wc -l | tr -d ' ')

---

HEADER
fi

# Extract TODOs
if [ -n "$MODULE" ]; then
    FILTER="grep \"$MODULE\""
else
    FILTER="cat"
fi

grep -rn "TODO" kernel/src --include="*.rs" -B1 -A1 | \
    while IFS= read -r line; do
        if [[ $line =~ ^kernel/src/(.+):([0-9]+):(.*)$ ]]; then
            file="${BASH_REMATCH[1]}"
            lineno="${BASH_REMATCH[2]}"
            content="${BASH_REMATCH[3]}"
            
            # Extract module name
            module=$(echo "$file" | cut -d'/' -f1)
            
            if [ "$FORMAT" = "markdown" ]; then
                echo "### [$module] $file:$lineno" >> "$OUTPUT_FILE"
                echo "\`\`\`rust" >> "$OUTPUT_FILE"
                echo "$content" >> "$OUTPUT_FILE"
                echo "\`\`\`" >> "$OUTPUT_FILE"
                echo "" >> "$OUTPUT_FILE"
            elif [ "$FORMAT" = "github" ]; then
                echo "---" >> "$OUTPUT_FILE"
                echo "**Title:** TODO in $module ($file:$lineno)" >> "$OUTPUT_FILE"
                echo "" >> "$OUTPUT_FILE"
                echo "**File:** \`$file:$lineno\`" >> "$OUTPUT_FILE"
                echo "**Module:** $module" >> "$OUTPUT_FILE"
                echo "" >> "$OUTPUT_FILE"
                echo "**Description:**" >> "$OUTPUT_FILE"
                echo "\`\`\`" >> "$OUTPUT_FILE"
                echo "$content" >> "$OUTPUT_FILE"
                echo "\`\`\`" >> "$OUTPUT_FILE"
                echo "" >> "$OUTPUT_FILE"
            fi
        fi
    done

echo "TODO extraction complete: $OUTPUT_FILE"
echo "Total TODOs extracted: $(grep -c '^###' "$OUTPUT_FILE" 2>/dev/null || echo 0)"
