#!/usr/bin/env python3
"""
Fix multiline alloc::alloc::format! calls
"""
import re
import os
from pathlib import Path

def fix_multiline_format_in_file(filepath):
    """Fix multiline format! calls in a file"""
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()

        original = content

        # Find and fix multiline format! blocks
        # Pattern: alloc::alloc::format!(\n    "...",\n    ...\n)
        pattern = r'alloc::alloc::format!\(\s*\n([^)]+)\n\s*\)'

        def replacer(match):
            body = match.group(1)
            lines = [line.strip().rstrip(',') for line in body.split('\n') if line.strip()]
            if not lines:
                return match.group(0)

            fmt_str = lines[0].strip('"').strip("'")
            args = [line.strip() for line in lines[1:]]

            # Build string
            result_parts = []
            arg_idx = 0

            # Split by format specifiers
            parts = re.split(r'\{[^}]*\}', fmt_str)

            for i, part in enumerate(parts):
                if part:
                    result_parts.append(f'alloc::string::String::from("{part}")')

                if i < len(parts) - 1:
                    spec_match = re.search(r'\{([^}]*)\}', fmt_str[len(''.join(parts[:i+1])):][:20])
                    if spec_match and arg_idx < len(args):
                        spec = spec_match.group(1)
                        arg = args[arg_idx].strip()
                        arg_idx += 1

                        if not spec or spec == '?':
                            result_parts.append(f'&{arg}.to_string()')
                        else:
                            result_parts.append(f'/* TODO: {{:{spec}}} */ &{arg}.to_string()')

            return ' + '.join(result_parts) if result_parts else 'alloc::string::String::new()'

        content = re.sub(pattern, replacer, content, flags=re.MULTILINE)

        if content != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}")
        return False

def main():
    kernel_src = Path('/Users/wangbiao/Desktop/project/nos/kernel/src')

    # Files with remaining issues
    files_to_fix = [
        'subsystems/syscalls/dispatch/dispatcher.rs',
        'subsystems/syscalls/security/syscall_validator.rs',
        'subsystems/syscalls/services/dispatcher.rs',
        'subsystems/perf/profiler.rs',
        'subsystems/drivers/basic_drivers.rs',
        'subsystems/drivers/pci_device_manager.rs',
    ]

    for file_path in files_to_fix:
        full_path = kernel_src / file_path
        if full_path.exists():
            if fix_multiline_format_in_file(full_path):
                print(f"✓ Fixed: {file_path}")
            else:
                print(f"- No changes: {file_path}")

if __name__ == '__main__':
    main()
