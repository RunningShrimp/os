# [0188] // TODO: Implement directory lookup

**File:** `kernel/src/fs/fs_impl.rs`
**Line:** 523
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
520: 
521:     /// Look up directory entry
522:     pub fn dirlookup(&self, _dir_inum: u32, _name: &str) -> Option<u32> {
523:         // TODO: Implement directory lookup
524:         None
525:     }
526: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
