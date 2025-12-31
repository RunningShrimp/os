#!/usr/bin/env python3
"""
Batch convert TODO comments to GitHub issue references - Batch 2.
This script processes Rust files and converts TODO comments to use GitHub issue references.
"""

import re
import sys
from pathlib import Path

# TODO counter - continue from 937 (after batch 1)
issue_counter = 937
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

    # Find all .rs files
    rust_files = list(kernel_dir.rglob('*.rs'))
    print(f"Found {len(rust_files)} Rust files")

    # Process files with high TODO counts (excluding test files for now)
    high_priority_files = [
        # IDS subsystem (high priority security)
        'kernel/src/ids/host_ids/detector.rs',

        # Memory management (core functionality)
        'kernel/src/subsystems/syscalls/memory/advanced_mmap.rs',
        'kernel/src/subsystems/mm/memory_isolation.rs',

        # Filesystem (core functionality)
        'kernel/src/subsystems/syscalls/fs/handlers.rs',
        'kernel/src/subsystems/fs/pmfs.rs',
        'kernel/src/filesystem/lfs.rs',
        'kernel/src/filesystem/journaling.rs',

        # Drivers (hardware interface)
        'kernel/src/drivers/vfio/device.rs',
        'kernel/src/drivers/vfio/iommu.rs',

        # IRQ (interrupt handling)
        'kernel/src/irq/threaded_irq.rs',

        # Resource management
        'kernel/src/resource/mod.rs',

        # Process management
        'kernel/src/subsystems/process/vfork.rs',

        # POSIX timers
        'kernel/src/posix/rt_timer.rs',

        # Signal handling
        'kernel/src/subsystems/syscalls/signal/handlers.rs',

        # Service management
        'kernel/src/subsystems/syscalls/service.rs',
        'kernel/src/subsystems/syscalls/handlers.rs',
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
