# [0192] // TODO: Write to inode

**File:** `kernel/src/fs/file.rs`
**Line:** 413
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
410:                 }
411:             }
412:             FileType::Inode => {
413:                 // TODO: Write to inode
414:                 if let Some(_inum) = self.inode {
415:                     buf.len() as isize
416:                 } else {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
