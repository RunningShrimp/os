# [0227] // TODO: Block until a signal is delivered

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 786
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
783:     // Save current mask and set new mask
784:     signals.suspend(new_mask);
785: 
786:     // TODO: Block until a signal is delivered
787:     // For now, just restore mask and return EINTR
788:     signals.restore_mask();
789: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
