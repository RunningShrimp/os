# TODO Issues Import

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
