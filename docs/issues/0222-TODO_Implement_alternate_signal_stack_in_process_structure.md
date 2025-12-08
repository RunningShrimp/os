# [0222] // TODO: Implement alternate signal stack in process structure

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 388
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
385:         return Err(SyscallError::BadAddress);
386:     }
387: 
388:     // TODO: Implement alternate signal stack in process structure
389:     // For now, just return ENOTSUP
390: 
391:     // Get old stack info if requested
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
