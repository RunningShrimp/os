# [0189] // TODO: Implement directory link

**File:** `kernel/src/fs/fs_impl.rs`
**Line:** 529
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
526: 
527:     /// Create a new directory entry
528:     pub fn dirlink(&self, _dir_inum: u32, _name: &str, _inum: u32) -> bool {
529:         // TODO: Implement directory link
530:         false
531:     }
532: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
