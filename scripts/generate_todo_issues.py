#!/usr/bin/env python3
"""
Parse docs/TODO_GREP_OUTPUT.txt and generate:
 - docs/issues/todos.csv
 - docs/issues/<0001-...>.md (per-item issue template)
 - docs/issues/README.md (import instructions)

Designed to be short and fast (won't block the terminal).
"""
import os
import csv
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INPUT = ROOT / 'docs' / 'TODO_GREP_OUTPUT.txt'
OUTDIR = ROOT / 'docs' / 'issues'
OUTCSV = OUTDIR / 'todos.csv'

# Simple heuristics for suggested priority/owner/estimate
PRIORITY_MAP = [
    (re.compile(r'syscalls|process|thread|signal|futex|sched|execve|fork|waitpid', re.I), 'Critical', 'Kernel Engineer', 40),
    (re.compile(r'vfs|fs|inode|directory|ext4', re.I), 'Critical', 'Filesystems Engineer', 40),
    (re.compile(r'memory|mmap|mincore|mlock|munlock|page|mm\b|advanced_mmap', re.I), 'High', 'Memory/MM Engineer', 32),
    (re.compile(r'security|permission|seccomp|selinux|aslr|smap|smep|cap', re.I), 'Critical', 'Security Engineer', 40),
    (re.compile(r'driver|virtio|device_manager|driver binding|probe', re.I), 'High', 'Driver Engineer', 36),
    (re.compile(r'zero_copy|io_uring|splice|sendfile|performance|benchmark', re.I), 'Medium', 'Performance Engineer', 24),
    (re.compile(r'graphics|gui|input|hit testing', re.I), 'Low', 'Graphics Engineer', 16),
    (re.compile(r'test|placeholder|integration_test|bench', re.I), 'Low', 'QA/Tester', 8),
]

def guess_props(path, text):
    ctx = f"{path} {text}"
    for rx, pr, owner, hrs in PRIORITY_MAP:
        if rx.search(ctx):
            return pr, owner, hrs
    return 'Medium', 'Engineer', 16


def ensure_outdir():
    OUTDIR.mkdir(parents=True, exist_ok=True)


def parse_line(line):
    # Expected format: file:line:    // text
    # But content may contain colons, so split only on first two ':'
    parts = line.split(':', 2)
    if len(parts) < 3:
        return None
    file_path = parts[0].strip()
    line_no = parts[1].strip()
    content = parts[2].rstrip() if parts[2] else ''
    # Normalize
    try:
        line_no = int(line_no)
    except ValueError:
        line_no = 0
    return file_path, line_no, content


def sanitize_title(text):
    # Make short title safe for filenames
    t = re.sub(r"[^0-9A-Za-z-_ ]+", '', text).strip()
    t = re.sub(r"[ \t]+", ' ', t)
    if len(t) > 60:
        t = t[:60].rstrip()
    if not t:
        return 'todo'
    return t.replace(' ', '_')


def main():
    if not INPUT.exists():
        print('Input file not found:', INPUT)
        return

    ensure_outdir()

    items = []
    with INPUT.open('r', encoding='utf-8', errors='replace') as f:
        for ln in f:
            ln = ln.rstrip('\n')
            if not ln:
                continue
            parsed = parse_line(ln)
            if not parsed:
                continue
            file_path, line_no, content = parsed
            # Determine type marker by searching for TODO/FIXME/STUB/hack/placeholder
            marker = None
            for m in ['TODO', 'FIXME', 'STUB', 'placeholder', 'Temporary', 'hack']:
                if m in content:
                    marker = m
                    break
            marker = marker or 'NOTE'
            # Read context lines around the match (3 lines before/after) if file exists
            ctx_snippet = ''
            abs_file = ROOT / file_path
            if abs_file.exists():
                try:
                    all_lines = abs_file.read_text(encoding='utf-8', errors='replace').splitlines()
                    start = max(0, line_no - 4)
                    end = min(len(all_lines), line_no + 3)
                    snippet = all_lines[start:end]
                    ctx_snippet = '\n'.join(f"{i+1}: {s}" for i, s in enumerate(snippet, start=start))
                except Exception as e:
                    ctx_snippet = f"(failed to read file: {e})"
            else:
                ctx_snippet = '(file not found)'

            suggested_priority, suggested_owner_role, suggested_estimate_hours = guess_props(file_path, content)
            suggested_labels = [suggested_priority.lower(), marker.lower()]

            items.append({
                'file': file_path,
                'line': line_no,
                'marker': marker,
                'content': content.strip(),
                'context': ctx_snippet,
                'priority': suggested_priority,
                'owner': suggested_owner_role,
                'estimate_hours': suggested_estimate_hours,
                'labels': ';'.join(suggested_labels),
            })

    # Write CSV
    with OUTCSV.open('w', encoding='utf-8', newline='') as csvf:
        writer = csv.DictWriter(csvf, fieldnames=['id','file','line','marker','content','context','priority','owner','estimate_hours','labels'])
        writer.writeheader()
        for i, item in enumerate(items, start=1):
            item_row = dict(item)
            item_row['id'] = i
            writer.writerow(item_row)

    # Create per-item markdown files
    for i, item in enumerate(items, start=1):
        idstr = f"{i:04d}"
        safe = sanitize_title(item['content'][:80])
        filename = OUTDIR / f"{idstr}-{safe}.md"
        title = item['content'] if item['content'] else f"TODO in {item['file']}:{item['line']}"
        body = f"""# [{idstr}] {title}\n\n**File:** `{item['file']}`\n**Line:** {item['line']}\n**Marker:** {item['marker']}\n**Suggested Priority:** {item['priority']}\n**Suggested Owner Role:** {item['owner']}\n**Suggested Estimate (hours):** {item['estimate_hours']}\n**Suggested Labels:** `{item['labels']}`\n\n## Context\n\n```
{item['context']}
```
\n## Recommended next steps\n- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior\n- Produce a PR that either implements the missing behavior or documents a migration if it's a stub\n"""
        try:
            filename.write_text(body, encoding='utf-8')
        except Exception as e:
            print('Failed writing', filename, e)

    # README with import instructions
    readme = OUTDIR / 'README.md'
    readme.write_text('''# TODO Issues Import

This directory contains:
- `todos.csv` — a CSV with all discovered TODO-like items (id, file, line, content, context, priority, owner, estimate_hours, labels)
- `<id>-... .md` — per-item issue templates ready to paste into your Issue tracker

How to import to GitHub Issues (example using gh CLI):
1) Review and adjust `todos.csv` if needed
2) For each line, use `gh issue create` with title and body from corresponding markdown file

Example:
```bash
# Create issue for item 0001
gh issue create --title "[0001] <short title>" --body-file docs/issues/0001-... .md --label "priority: critical"
```

Or bulk-create using GitHub API scripts — ensure to rate-limit and check permissions.
''', encoding='utf-8')

    print(f'Done: {len(items)} items -> {OUTCSV}\nPer-item markdown files under {OUTDIR}')

if __name__ == '__main__':
    main()
