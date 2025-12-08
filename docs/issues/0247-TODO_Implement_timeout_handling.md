# [0247] // TODO: Implement timeout handling

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 725
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
722:     
723:     // Handle timeout if provided
724:     if timeout != 0 {
725:         // TODO: Implement timeout handling
726:         // For now, sleep indefinitely
727:     }
728:     
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
