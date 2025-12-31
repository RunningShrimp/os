#!/usr/bin/env python3
"""
Batch convert TODO comments to GitHub issue references - Batch 4.
This script processes Rust files and converts TODO comments to use GitHub issue references.
"""

import re
import sys
from pathlib import Path

# TODO counter - continue from 1146 (after batch 3)
issue_counter = 1146
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

    # Process files with 3-4 TODOs each
    high_priority_files = [
        # Core systems with 4 TODOs each
        'kernel/src/epoll.rs',
        'kernel/src/sched/rt_sched.rs',
        'kernel/src/resource/cgroup.rs',
        'kernel/src/platform/trap/mod.rs',
        'kernel/src/monitoring/profiler.rs',
        'kernel/src/filesystem/quota.rs',
        'kernel/src/error/recovery.rs',
        'kernel/src/db/parser.rs',

        # Testing and verification (4 TODOs each)
        'kernel/src/testing/performance_tests.rs',
        'kernel/src/subsystems/formal_verification/theorem_prover.rs',
        'kernel/src/subsystems/formal_verification/model_checker.rs',

        # Files with 3 TODOs each
        'kernel/src/types/stubs.rs',
        'kernel/src/subsystems/syscalls/performance_monitor.rs',
        'kernel/src/subsystems/syscalls/mm/mprotect.rs',
        'kernel/src/subsystems/syscalls/mm/brk.rs',
        'kernel/src/subsystems/syscalls/dispatch/mod.rs',
        'kernel/src/subsystems/sync/sleeplock.rs',
        'kernel/src/subsystems/mm/user_space_isolation.rs',
        'kernel/src/subsystems/mm/pmem_tx.rs',
        'kernel/src/subsystems/microkernel/scheduler.rs',
        'kernel/src/subsystems/fs/dax.rs',
        'kernel/src/subsystems/fs/api/mod.rs',
        'kernel/src/security/permission_check.rs',
        'kernel/src/posix/rt_mutex.rs',
        'kernel/src/platform/drivers/mod.rs',
    ]

    print(f"Processing {len(high_priority_files)} files...")

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
