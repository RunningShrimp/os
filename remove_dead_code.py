#!/usr/bin/env python3
"""
Automatically remove dead code from Rust codebase.
"""

import re
import sys
from pathlib import Path

def parse_dead_code_warnings(cargo_output):
    """Parse cargo check output in short format."""
    dead_code_items = []

    for line in cargo_output.split('\n'):
        # Format: file:line:col: warning: type `name` is never used
        match = re.match(r'^([^:]+):(\d+):\d+:\s*warning:\s*(\w+)\s+`([^`]+)`\s+is never used', line)
        if match:
            file_path = match.group(1)
            line_num = int(match.group(2))
            item_type = match.group(3)
            item_name = match.group(4)

            dead_code_items.append({
                'type': item_type,
                'name': item_name,
                'file': file_path,
                'line': line_num
            })

    return dead_code_items

def find_end_brace_line(lines, start_line):
    """Find the line with the matching closing brace."""
    brace_count = 0
    in_block = False

    for i in range(start_line, len(lines)):
        brace_count += lines[i].count('{')
        brace_count -= lines[i].count('}')

        if brace_count > 0:
            in_block = True

        if in_block and brace_count == 0:
            return i + 1

    return start_line + 1

def remove_dead_code_item(file_path, line_num, item_name, item_type):
    """Remove a dead code item from a file."""
    try:
        with open(file_path, 'r') as f:
            lines = f.readlines()

        if line_num < 1 or line_num > len(lines):
            print(f"  Warning: Line {line_num} out of range")
            return False

        start_line = line_num - 1  # Convert to 0-indexed

        # Check if there's an attribute on previous lines (#[...])
        attr_lines = 0
        for i in range(start_line - 1, -1, -1):
            if i < len(lines) and lines[i].strip().startswith('#['):
                attr_lines += 1
            else:
                break

        actual_start = start_line - attr_lines

        # Find end based on type
        if item_type in ['function', 'method', 'associated function']:
            # Find end of function
            end_line = find_end_brace_line(lines, start_line)
            new_lines = lines[:actual_start] + lines[end_line:]

        elif item_type in ['struct', 'enum', 'trait']:
            # Find end of struct/enum/trait definition
            end_line = find_end_brace_line(lines, start_line)
            new_lines = lines[:actual_start] + lines[end_line:]

        elif item_type in ['constant', 'static']:
            # These are usually single line, but check for multi-line
            if ';' in lines[start_line]:
                # Single line
                new_lines = lines[:actual_start] + lines[start_line + 1:]
            else:
                # Multi-line, find semicolon
                for i in range(start_line, len(lines)):
                    if ';' in lines[i]:
                        new_lines = lines[:actual_start] + lines[i + 1:]
                        break
                else:
                    new_lines = lines[:actual_start] + lines[start_line + 1:]

        elif item_type in ['type']:
            # Type alias
            if ';' in lines[start_line]:
                new_lines = lines[:actual_start] + lines[start_line + 1:]
            else:
                new_lines = lines[:actual_start] + lines[start_line + 1:]

        else:
            print(f"  Warning: Unknown type {item_type}")
            return False

        # Write back
        with open(file_path, 'w') as f:
            f.writelines(new_lines)

        print(f"  ✓ Removed {item_type} `{item_name}`")
        return True

    except Exception as e:
        print(f"  ✗ Error: {e}")
        return False

def main():
    print("Reading cargo check output...")
    with open('/tmp/cargo_check_output.txt', 'r') as f:
        cargo_output = f.read()

    print("Parsing dead code warnings...")
    dead_code_items = parse_dead_code_warnings(cargo_output)

    print(f"Found {len(dead_code_items)} dead code items")

    if len(dead_code_items) == 0:
        print("No dead code found!")
        return

    # Group by file and process in reverse line order
    files_to_process = {}
    for item in dead_code_items:
        file_path = item['file']
        if file_path not in files_to_process:
            files_to_process[file_path] = []
        files_to_process[file_path].append(item)

    print(f"\nProcessing {len(files_to_process)} files...\n")

    # Process each file
    total_removed = 0
    for file_path, items in sorted(files_to_process.items()):
        full_path = Path('/Users/wangbiao/Desktop/project/nos') / file_path
        if not full_path.exists():
            print(f"⚠ File not found: {full_path}")
            continue

        # Sort items by line number in descending order
        items.sort(key=lambda x: x['line'], reverse=True)

        print(f"📄 {file_path} ({len(items)} items)")
        removed = sum(1 for item in items if remove_dead_code_item(str(full_path), item['line'], item['name'], item['type']))
        total_removed += removed
        print(f"   → Removed {removed}/{len(items)} items\n")

    print(f"✅ Done! Removed {total_removed} dead code items total")

if __name__ == '__main__':
    main()
