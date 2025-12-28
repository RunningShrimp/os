#!/usr/bin/env python3
"""
Comprehensive fix for alloc::format! usage
"""
import re
import os
import sys
from pathlib import Path

def extract_format_args(args_str):
    """Extract format string and arguments from format! macro call"""
    # Remove surrounding quotes from format string
    match = re.match(r'"([^"]*(?:\\.[^"]*)*)"', args_str)
    if not match:
        return None, None

    fmt_str = match.group(1)
    remaining = args_str[match.end():].strip()

    # Parse arguments
    args = []
    depth = 0
    current_arg = ""

    for char in remaining:
        if char in '({[':
            depth += 1
            current_arg += char
        elif char in ')}]':
            depth -= 1
            current_arg += char
            if depth == 0 and current_arg.strip().startswith(','):
                args.append(current_arg.strip(',').strip())
                current_arg = ""
            elif depth < 0:
                break
        elif char == ',' and depth == 0:
            if current_arg.strip():
                args.append(current_arg.strip())
            current_arg = ""
        else:
            current_arg += char

    if current_arg.strip() and not current_arg.strip().startswith(','):
        args.append(current_arg.strip().rstrip(','))

    return fmt_str, args

def build_string_from_format(fmt_str, args):
    """Build string concatenation from format string and args"""
    result_parts = []
    arg_idx = 0

    # Split by format specifiers
    parts = re.split(r'\{[^}]*\}', fmt_str)

    for i, part in enumerate(parts):
        if part:
            result_parts.append(f'alloc::string::String::from("{part}")')

        # Add argument if there's a placeholder
        spec_match = re.search(r'\{([^}]*)\}', fmt_str[len(''.join(parts[:i+1])):] if i < len(parts)-1 else '')
        if spec_match and arg_idx < len(args):
            spec = spec_match.group(1)
            arg = args[arg_idx].strip()
            arg_idx += 1

            if not spec:
                # Simple {}
                result_parts.append(f'&{arg}.to_string()')
            elif spec == '?':
                # Debug {:?}
                result_parts.append(f'&{arg}.to_string()')
            elif spec == 'x':
                # Hex {:x}
                result_parts.append(f'&alloc::format::lower_hex({arg})')
            elif re.match(r'\.\d+', spec):
                # Precision like {.2}
                result_parts.append(f'&format_float({arg}, {spec[1:]})')
            else:
                # Unknown spec, keep as-is
                result_parts.append(f'/* TODO: {{:{spec}}} */ &{arg}.to_string()')

    # Concatenate all parts
    if len(result_parts) == 0:
        return 'alloc::string::String::new()'
    elif len(result_parts) == 1:
        return result_parts[0]
    else:
        # Chain + operations
        return ' + '.join(result_parts)

def fix_line(line):
    """Fix a single line containing format! calls"""
    # Pattern: .push_str(&alloc::format!(...))
    if '.push_str(&' in line and 'format!' in line:
        match = re.search(r'\.push_str\(&&?(alloc::(?:alloc::)*(?:format)!\([^)]+\))\)', line)
        if match:
            format_call = match.group(1)
            inner = re.search(r'format!\(([^)]+)\)', format_call)
            if inner:
                args_str = inner.group(1)
                fmt_str, args = extract_format_args(args_str)
                if fmt_str is not None:
                    replacement = build_string_from_format(fmt_str, args)
                    prefix = line[:match.start()]
                    suffix = line[match.end():]
                    return f'{prefix}.push_str({replacement}){suffix}'

    # Pattern: let x = alloc::format!(...)
    match = re.search(r'let\s+(\w+)\s*=\s*alloc::(?:alloc::)*format!\(([^)]+)\)', line)
    if match:
        var_name = match.group(1)
        args_str = match.group(2)
        fmt_str, args = extract_format_args(args_str)
        if fmt_str is not None:
            replacement = build_string_from_format(fmt_str, args)
            prefix = line[:match.start()]
            suffix = line[match.end():]
            return f'{prefix}let mut {var_name} = {replacement}{suffix}'

    # Pattern: return Err(alloc::format!(...))
    match = re.search(r'return\s+Err\((alloc::(?:alloc::)*format!\([^)]+\))\)', line)
    if match:
        format_call = match.group(1)
        inner = re.search(r'format!\(([^)]+)\)', format_call)
        if inner:
            args_str = inner.group(1)
            fmt_str, args = extract_format_args(args_str)
            if fmt_str is not None:
                replacement = build_string_from_format(fmt_str, args)
                prefix = line[:match.start()]
                suffix = line[match.end():]
                return f'{prefix}return Err({replacement}){suffix}'

    # Pattern: description: alloc::format!(...)
    match = re.search(r'description:\s*alloc::(?:alloc::)*format!\(([^)]+)\)', line)
    if match:
        args_str = match.group(1)
        fmt_str, args = extract_format_args(args_str)
        if fmt_str is not None:
            replacement = build_string_from_format(fmt_str, args)
            prefix = line[:match.start()]
            suffix = line[match.end():]
            return f'{prefix}description: {replacement}{suffix}'

    # Pattern: alloc::format!("string only") in any context
    line = re.sub(
        r'alloc::(?:alloc::)*format!\("([^"]+)"\)',
        r'alloc::string::String::from("\1")',
        line
    )

    # Pattern: alloc::format!(...)
    match = re.search(r'alloc::(?:alloc::)*format!\(([^)]+)\)', line)
    if match:
        args_str = match.group(1)
        fmt_str, args = extract_format_args(args_str)
        if fmt_str is not None:
            replacement = build_string_from_format(fmt_str, args)
            return line[:match.start()] + replacement + line[match.end():]

    return line

def fix_file(filepath):
    """Fix a single file"""
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            lines = f.readlines()

        original = lines[:]
        fixed_lines = []

        for line in lines:
            fixed_line = fix_line(line.rstrip('\n'))
            fixed_lines.append(fixed_line + '\n')

        if fixed_lines != original:
            with open(filepath, 'w', encoding='utf-8') as f:
                f.writelines(fixed_lines)
            return True
        return False
    except Exception as e:
        print(f"Error processing {filepath}: {e}", file=sys.stderr)
        return False

def main():
    kernel_src = Path('/Users/wangbiao/Desktop/project/nos/kernel/src')

    # Find all non-backup .rs files
    fixed_count = 0
    total_files = 0

    for rs_file in kernel_src.rglob('*.rs'):
        # Skip backup files
        if any(rs_file.suffix.endswith(ext) for ext in ['.bak', '.bak2', '.bak3', '.orig']):
            continue

        if 'format!' in rs_file.read_text(errors='ignore'):
            total_files += 1
            if fix_file(rs_file):
                fixed_count += 1
                print(f"✓ Fixed: {rs_file.relative_to(kernel_src)}")
            else:
                print(f"- No changes: {rs_file.relative_to(kernel_src)}")

    print(f"\nTotal files processed: {fixed_count}/{total_files}")

if __name__ == '__main__':
    main()
