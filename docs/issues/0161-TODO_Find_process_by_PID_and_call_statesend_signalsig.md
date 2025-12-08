# [0161] // TODO: Find process by PID and call state.send_signal(sig)

**File:** `kernel/src/ipc/signal.rs`
**Line:** 726
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
723:     // For now, just a stub that would be called from syscall handler
724:     let _ = (pid, sig);
725:     
726:     // TODO: Find process by PID and call state.send_signal(sig)
727:     
728:     Ok(())
729: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
