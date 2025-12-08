# [0270] // TODO: Implement x86_64 unmap

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 713
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
710: 
711:         #[cfg(target_arch = "x86_64")]
712:         {
713:             // TODO: Implement x86_64 unmap
714:         }
715:     }
716: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
