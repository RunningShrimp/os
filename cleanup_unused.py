#!/usr/bin/env python3
"""
Quick script to identify potentially unused imports in Rust files.
This is a heuristic-based approach - not as accurate as cargo check.
"""

import os
import re
from pathlib import Path
from collections import defaultdict

def extract_use_statements(content):
    """Extract all use statements from Rust file content."""
    use_lines = []
    for i, line in enumerate(content.split('\n'), 1):
        if line.strip().startswith('use '):
            use_lines.append((i, line.strip()))
    return use_lines

def extract_imported_items(use_statement):
    """Extract imported items from a use statement."""
    # Remove 'use ' and ';'
    stmt = use_statement[4:].rstrip(';').strip()

    # Handle different import patterns
    if '::' in stmt:
        # Get the last part after ::
        parts = stmt.split('::')
        if '{' in stmt:
            # use foo::{bar, baz};
            inner = stmt.split('{')[1].rstrip('}')
            items = [i.strip() for i in inner.split(',')]
            return items
        else:
            # use foo::Bar;
            return [parts[-1]]
    else:
        # use foo;
        return [stmt]

def check_item_usage(content, item):
    """Check if an item is used in the content."""
    # Remove comments first
    content_no_comments = re.sub(r'//.*', '', content)
    content_no_comments = re.sub(r'/\*.*?\*/', '', content_no_comments, flags=re.DOTALL)

    # Check for item usage (as a word, not part of another word)
    pattern = r'\b' + re.escape(item) + r'\b'
    return bool(re.search(pattern, content_no_comments))

def analyze_file(filepath):
    """Analyze a Rust file for potentially unused imports."""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        return None

    use_statements = extract_use_statements(content)
    unused = []

    for line_num, use_stmt in use_statements:
        items = extract_imported_items(use_stmt)
        all_used = True

        for item in items:
            # Skip common items that are hard to detect
            if item in ['*', 'self', 'super', 'crate']:
                continue

            # Check if item is used in the rest of the file
            if not check_item_usage(content, item):
                all_used = False
                break

        if not all_used:
            unused.append((line_num, use_stmt))

    return unused

def main():
    base_dirs = [
        'nos-api/src',
        'nos-syscalls/src',
        'nos-services/src',
        'nos-error-handling/src'
    ]

    print("Scanning for potentially unused imports...\n")
    print("=" * 80)

    for base_dir in base_dirs:
        if not os.path.exists(base_dir):
            continue

        print(f"\n## {base_dir}")
        print("-" * 80)

        for rust_file in Path(base_dir).rglob('*.rs'):
            unused = analyze_file(rust_file)
            if unused:
                rel_path = os.path.relpath(rust_file)
                print(f"\n{rel_path}:")
                for line_num, use_stmt in unused:
                    print(f"  Line {line_num}: {use_stmt}")

    print("\n" + "=" * 80)
    print("\nNOTE: This is a heuristic check. Some items may be false positives.")
    print("Please verify with 'cargo check' before removing.")

if __name__ == '__main__':
    main()
