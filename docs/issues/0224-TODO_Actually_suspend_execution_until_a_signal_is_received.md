# [0224] // TODO: Actually suspend execution until a signal is received

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 419
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
416:     // pause() suspends execution until a signal is delivered
417:     // It always returns -1 with EINTR
418: 
419:     // TODO: Actually suspend execution until a signal is received
420:     // For now, just return EINTR immediately
421: 
422:     Err(SyscallError::Interrupted)
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
