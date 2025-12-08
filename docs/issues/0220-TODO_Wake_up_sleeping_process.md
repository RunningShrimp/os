# [0220] // TODO: Wake up sleeping process

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 55
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
52:             
53:             // Wake up process if it's sleeping
54:             if proc.state == crate::process::ProcState::Sleeping {
55:                 // TODO: Wake up sleeping process
56:                 // This would involve signaling to scheduler
57:             }
58:             
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
