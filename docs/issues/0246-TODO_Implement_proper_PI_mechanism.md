# [0246] // TODO: Implement proper PI mechanism

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 718
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
715:     }
716:     
717:     // If futex is contended, implement priority inheritance
718:     // TODO: Implement proper PI mechanism
719:     // For now, just sleep on the futex address
720:     
721:     let channel = uaddr | 0xf0000000;
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
