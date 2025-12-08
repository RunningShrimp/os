# [0032] // TODO: Sync and cleanup

**File:** `kernel/src/vfs/ext4.rs`
**Line:** 208
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
205:     }
206:     
207:     fn unmount(&self) -> VfsResult<()> {
208:         // TODO: Sync and cleanup
209:         self.sync()
210:     }
211: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
