# [0226] // TODO: Wake up sleeping process

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 731
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
728: 
729:             // Wake up process if it's sleeping
730:             if proc.state == crate::process::ProcState::Sleeping {
731:                 // TODO: Wake up sleeping process
732:                 // This would involve signaling to scheduler
733:             }
734: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
