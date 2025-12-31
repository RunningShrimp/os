#!/usr/bin/env python3
"""
Batch convert TODO comments to GitHub issue references - Batch 3.
This script processes Rust files and converts TODO comments to use GitHub issue references.
"""

import re
import sys
from pathlib import Path

# TODO counter - continue from 1060 (after batch 2)
issue_counter = 1060
converted_count = 0

def process_file(file_path):
    """Process a single file and convert TODOs to GitHub issues."""
    global converted_count, issue_counter

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

    # Process files with moderate TODO counts (4-6 TODOs each)
    high_priority_files = [
        # Core helpers (6 TODOs)
        'kernel/src/helpers.rs',

        # Syscalls services (5 TODOs each)
        'kernel/src/subsystems/syscalls/zero_copy.rs',
        'kernel/src/subsystems/syscalls/services/mod.rs',
        'kernel/src/subsystems/syscalls/services/traits.rs',

        # Network (5 TODOs)
        'kernel/src/subsystems/net/enhanced_network.rs',

        # Memory management (5 TODOs each)
        'kernel/src/subsystems/mm/vm/lock.rs',
        'kernel/src/subsystems/mm/vm/protection.rs',
        'kernel/src/subsystems/mm/vm/mmap.rs',
        'kernel/src/subsystems/mm/api/stats.rs',
        'kernel/src/subsystems/mm/api/page.rs',
        'kernel/src/subsystems/mm/nvdimm.rs',
        'kernel/src/mm/virt_mem.rs',

        # Filesystem (5 TODOs each)
        'kernel/src/subsystems/fs/vfs/ext4.rs',
        'kernel/src/subsystems/fs/mod.rs',

        # IPC (4 TODOs)
        'kernel/src/subsystems/posix/shm.rs',

        # Legacy support (4 TODOs each)
        'kernel/src/subsystems/syscalls/glib_legacy.rs',
        'kernel/src/subsystems/syscalls/dispatch/traits.rs',

        # Virtualization (4 TODOs)
        'kernel/src/virtio/mmio.rs',

        # Monitoring (5 TODOs)
        'kernel/src/monitoring/examples/integration_example.rs',
    ]

    print(f"Processing {len(high_priority_files)} high-priority files...")

    processed = 0
    for file_path in high_priority_files:
        path = Path(file_path)
        if path.exists():
            if process_file(path):
                processed += 1
                print(f"✓ Converted: {file_path}")

    print(f"\nConverted {converted_count} TODOs across {processed} files")
    print(f"Next issue number: {issue_counter}")

if __name__ == '__main__':
    main()
