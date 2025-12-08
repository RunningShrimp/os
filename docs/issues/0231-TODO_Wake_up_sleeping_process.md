# [0231] // TODO: Wake up sleeping process

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 860
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
857: 
858:             // Wake up process if it's sleeping
859:             if proc.state == crate::process::ProcState::Sleeping {
860:                 // TODO: Wake up sleeping process
861:                 // This would involve signaling to scheduler
862:             }
863: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
