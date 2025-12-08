# [0313] // TODO: Store path in file structure or lookup from inode

**File:** `kernel/src/syscalls/fs.rs`
**Line:** 150
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
147:         
148:         // Get the path from the VFS file (if available)
149:         // For now, we'll need to track the path separately
150:         // TODO: Store path in file structure or lookup from inode
151:     }
152:     
153:     // Update process's current working directory
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
