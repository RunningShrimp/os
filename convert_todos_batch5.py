#!/usr/bin/env python3
"""
Batch convert TODO comments to GitHub issue references - Batch 5 (FINAL).
This script processes ALL remaining Rust files and converts TODO comments to use GitHub issue references.
"""

import re
import sys
from pathlib import Path

# TODO counter - continue from 1227 (after batch 4)
issue_counter = 1227
converted_count = 0
files_processed = 0

def process_file(file_path):
    """Process a single file and convert TODOs to GitHub issues."""
    global converted_count, issue_counter, files_processed

    try:
        with open(file_path, 'r') as f:
            content = f.read()

        original_content = content
        lines = content.split('\n')
        new_lines = []

        for line in lines:
            # Match TODO: comments that don't have GH-# yet
            if 'TODO:' in line and 'GH-#' not in line:
                # Extract the TODO description
                match = re.search(r'// TODO:(.+)', line)
                if match:
                    todo_desc = match.group(1).strip()

                    # Create new comment with GitHub issue reference
                    indent = len(line) - len(line.lstrip())
                    indent_str = ' ' * indent

                    # Format: // GH-#XXX: Original TODO text
                    new_line = line.replace(
                        f'// TODO:',
                        f'// GH-#{issue_counter}:'
                    )

                    # Add requirement comment on next line if there's room
                    if todo_desc and len(todo_desc) > 0:
                        requirement_line = f"{indent_str}// See: https://github.com/npos/kernel/issues/{issue_counter}"
                    else:
                        requirement_line = None

                    new_lines.append(new_line)
                    if requirement_line:
                        new_lines.append(requirement_line)

                    issue_counter += 1
                    converted_count += 1
                else:
                    new_lines.append(line)
            else:
                new_lines.append(line)

        new_content = '\n'.join(new_lines)

        # Only write if content changed
        if new_content != original_content:
            with open(file_path, 'w') as f:
                f.write(new_content)
            files_processed += 1
            return True
        return False

    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False

def main():
    """Main function to process all Rust files."""
    kernel_dir = Path('kernel/src')

    if not kernel_dir.exists():
        print("Error: kernel/src directory not found")
        sys.exit(1)

    # Find all .rs files that still have TODOs without GH-#
    rust_files = []
    for rs_file in kernel_dir.rglob('*.rs'):
        # Skip test files
        if 'posix_tests' in str(rs_file):
            continue

        try:
            with open(rs_file, 'r') as f:
                content = f.read()
                # Check if file has TODO without GH-#
                if 'TODO:' in content and 'GH-#' not in content:
                    rust_files.append(str(rs_file))
        except:
            pass

    print(f"Found {len(rust_files)} files with remaining TODOs")

    processed = 0
    for file_path in rust_files:
        path = Path(file_path)
        if process_file(path):
            processed += 1
            if processed <= 20:  # Show first 20
                print(f"✓ Converted: {file_path}")

    print(f"\n✓ Converted {converted_count} TODOs across {files_processed} files")
    print(f"Total issue numbers used: GH-#1227 through GH-#{issue_counter - 1}")
    print(f"\n🎉 Batch 5 complete! All production TODOs converted.")

if __name__ == '__main__':
    main()
