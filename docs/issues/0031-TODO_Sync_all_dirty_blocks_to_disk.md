# [0031] // TODO: Sync all dirty blocks to disk

**File:** `kernel/src/vfs/ext4.rs`
**Line:** 191
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Filesystems Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
188:     }
189:     
190:     fn sync(&self) -> VfsResult<()> {
191:         // TODO: Sync all dirty blocks to disk
192:         Ok(())
193:     }
194:     
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
