# [0033] // TODO: Sync inode to disk

**File:** `kernel/src/vfs/ext4.rs`
**Line:** 528
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
525:     }
526:     
527:     fn sync(&self) -> VfsResult<()> {
528:         // TODO: Sync inode to disk
529:         Ok(())
530:     }
531: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
