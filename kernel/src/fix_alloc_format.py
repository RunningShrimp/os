#!/usr/bin/env python3
"""
Fix all alloc::format! usage to be no_std compatible
"""
import re
import os
import sys

def fix_format_macros(content):
    """Fix various alloc::format! patterns"""
    lines = content.split('\n')
    result = []

    for line in lines:
        # Pattern 1: alloc::alloc::format!(...) -> replace with appropriate string building
        # Pattern 2: alloc::format!(...) -> replace
        # Pattern 3: use alloc::format -> remove

        # Simple string concatenation patterns
        if 'alloc::alloc::format!(' in line or 'alloc::format!(' in line:
            # Check if this is a macro call
            match = re.search(r'(alloc::(?:alloc::)?format!\([^)]+\))', line)
            if match:
                format_call = match.group(1)
                # Extract the format string and arguments
                inner = re.search(r'format!\(([^)]+)\)', format_call)
                if inner:
                    args = inner.group(1)
                    # Split by comma to get format string and params
                    parts = [p.strip() for p in args.split(',', 1)]
                    if len(parts) == 1:
                        # Just a string, simple case
                        new_str = parts[0].strip('"\'')
                        replacement = f'alloc::string::String::from({new_str})'
                    else:
                        # Has format parameters, need more complex handling
                        fmt_str = parts[0].strip('"\'')
                        replacement = handle_format_string(fmt_str, parts[1] if len(parts) > 1 else '')

                    line = line.replace(format_call, replacement)

        result.append(line)

    return '\n'.join(result)

def handle_format_string(fmt_str, args):
    """Handle format string with placeholders"""
    # For simple cases like "Error: {}"
    if '{}' in fmt_str or '{:?}' in fmt_str:
        parts = fmt_str.split('{}')
        if len(parts) == 2:
            # Simple single placeholder
            return f'alloc::string::String::from("{parts[0]}") + &{args}.to_string() + &alloc::string::String::from("{parts[1]}")'

    # For numeric formatting like {:.2}
    if '{:' in fmt_str:
        # This needs manual formatting
        return f'/* TODO: Manual formatting needed for {fmt_str} */ alloc::string::String::from("{fmt_str}")'

    # Default: return as-is with comment
    return f'/* TODO: Fix format: {fmt_str} with {args} */ alloc::string::String::from("{fmt_str}")'

def fix_file(filepath):
    """Fix a single file"""
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()

        original = content

        # Fix macro definitions
        content = re.sub(
            r'alloc::(?:alloc::)?format!\(\$\(([^)]*)\)\*\)',
            r'core::format_args!($(\1)*)',
            content
        )

        # Fix simple format calls in expressions
        # Pattern: return Err(alloc::format!("Error: {}", err))
        content = re.sub(
            r'return Err\(alloc::(?:alloc::)?format!\(([^,]+),\s*([^)]+)\)\)',
            r'return Err(alloc::string::String::from(\1) + &\2.to_string())',
            content
        )

        # Pattern: .push_str(&alloc::format!("...", ...))
        content = re.sub(
            r'\.push_str\(&&?alloc::(?:alloc::)?format!\(([^,]+),\s*([^)]+)\)\)',
            r'.push_str(\1); .push_str(&\2.to_string())',
            content
        )

        # Pattern: description: alloc::format!("...")
        content = re.sub(
            r'description:\s*alloc::(?:alloc::)?format!\(([^)]+)\)',
            r'description: alloc::string::String::from(\1)',
            content
        )

        # Pattern: simple string format with one variable
        content = re.sub(
            r'alloc::(?:alloc::)?format!\("([^"]+)",\s*([^)]+)\)',
            r'alloc::string::String::from("\1") + &\2.to_string()',
            content
        )

        # Pattern: alloc::format!("string only")
        content = re.sub(
            r'alloc::(?:alloc::)?format!\("([^"]+)"\)',
            r'alloc::string::String::from("\1")',
            content
        )

        # Pattern: hex format {:x}
        content = re.sub(
            r'alloc::(?:alloc::)?format!\("([^"]*)\{(:x)\}([^"]*)",\s*([^)]+)\)',
            r'alloc::string::String::from("\1") + &alloc::format::lower_hex(\4) + &alloc::string::String::from("\3")',
            content
        )

        if content != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}", file=sys.stderr)
        return False

def main():
    kernel_src = '/Users/wangbiao/Desktop/project/nos/kernel/src'

    # Find all .rs files
    fixed_count = 0
    for root, dirs, files in os.walk(kernel_src):
        for file in files:
            if file.endswith('.rs') and not file.endswith('.bak') and not file.endswith('.bak2') and not file.endswith('.bak3'):
                filepath = os.path.join(root, file)
                if fix_file(filepath):
                    fixed_count += 1
                    print(f"Fixed: {filepath}")

    print(f"\nTotal files fixed: {fixed_count}")

if __name__ == '__main__':
    main()
