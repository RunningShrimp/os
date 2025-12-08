# [0221] // TODO: Block until a signal is delivered

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 352
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
349:     // Save current mask and set new mask
350:     signals.suspend(new_mask);
351: 
352:     // TODO: Block until a signal is delivered
353:     // For now, just restore mask and return EINTR
354:     signals.restore_mask();
355: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
