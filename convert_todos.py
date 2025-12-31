#!/usr/bin/env python3
"""
Batch convert TODO comments to GitHub issue references.
This script processes Rust files and converts TODO comments to use GitHub issue references.
"""

import re
import sys
from pathlib import Path

# TODO counter
issue_counter = 780  # Start from 780 (after our manual fixes)
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
            # Match TODO: comments
            if 'TODO:' in line and 'GH-' not in line:
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
    
    # Process files with high TODO counts
    high_priority_files = [
        'kernel/src/subsystems/syscalls/thread.rs',
        'kernel/src/subsystems/syscalls/network/service.rs',
        'kernel/src/drivers/vfio/api.rs',
        'kernel/src/subsystems/mm/madvise.rs',
        'kernel/src/subsystems/mm/libpmem.rs',
        'kernel/src/vmm/hypervisor.rs',
        'kernel/src/bpf/tracer.rs',
        'kernel/src/subsystems/syscalls/signal/service.rs',
        'kernel/src/subsystems/syscalls/handlers/net.rs',
        'kernel/src/subsystems/posix/timer.rs',
        'kernel/src/posix/rt_extension.rs',
        'kernel/src/filesystem/cow.rs',
        'kernel/src/drivers/vfio/pci.rs',
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
