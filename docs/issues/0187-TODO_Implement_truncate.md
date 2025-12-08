# [0187] // TODO: Implement truncate

**File:** `kernel/src/fs/fs_impl.rs`
**Line:** 515
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
512:             inode.ref_count -= 1;
513:             if inode.ref_count == 0 && inode.nlink == 0 {
514:                 // Truncate and free inode
515:                 // TODO: Implement truncate
516:                 inode.itype = InodeType::Free;
517:             }
518:         }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
