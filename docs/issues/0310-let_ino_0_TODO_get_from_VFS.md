# [0310] let ino = 0; // TODO: get from VFS

**File:** `kernel/src/syscalls/glib.rs`
**Line:** 626
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
623: 
624:         // Get inode info (simplified - in real implementation would get from VFS)
625:         let dev = 0; // TODO: get from VFS
626:         let ino = 0; // TODO: get from VFS
627: 
628:         let watch = WatchDescriptor {
629:             wd,
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
