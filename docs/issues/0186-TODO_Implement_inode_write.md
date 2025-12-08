# [0186] // TODO: Implement inode write

**File:** `kernel/src/fs/fs_impl.rs`
**Line:** 361
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
358: 
359:     /// Write data to inode
360:     pub fn write(&mut self, _dev: &impl BlockDevice, _src: &[u8], _off: usize) -> usize {
361:         // TODO: Implement inode write
362:         0
363:     }
364: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
