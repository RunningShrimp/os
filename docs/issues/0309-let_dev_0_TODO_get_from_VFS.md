# [0309] let dev = 0; // TODO: get from VFS

**File:** `kernel/src/syscalls/glib.rs`
**Line:** 625
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
622:         let wd = self.next_wd.fetch_add(1, Ordering::SeqCst) as i32;
623: 
624:         // Get inode info (simplified - in real implementation would get from VFS)
625:         let dev = 0; // TODO: get from VFS
626:         let ino = 0; // TODO: get from VFS
627: 
628:         let watch = WatchDescriptor {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
