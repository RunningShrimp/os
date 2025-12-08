# [0190] // TODO: Read directory entries

**File:** `kernel/src/fs/fs_impl.rs`
**Line:** 537
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
534:     pub fn list_dir(&self, dir_inum: u32) -> Vec<(String, u32)> {
535:         let entries = Vec::new();
536:         
537:         // TODO: Read directory entries
538:         let _ = dir_inum;
539:         
540:         entries
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
