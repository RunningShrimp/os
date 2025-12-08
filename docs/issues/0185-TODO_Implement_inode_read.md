# [0185] // TODO: Implement inode read

**File:** `kernel/src/fs/fs_impl.rs`
**Line:** 355
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
352: impl Inode {
353:     /// Read data from inode
354:     pub fn read(&self, _dev: &impl BlockDevice, _dst: &mut [u8], _off: usize) -> usize {
355:         // TODO: Implement inode read
356:         0
357:     }
358: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
