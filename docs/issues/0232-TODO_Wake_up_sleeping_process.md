# [0232] // TODO: Wake up sleeping process

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 890
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
887:             
888:             // Wake up process if it's sleeping
889:             if proc.state == crate::process::ProcState::Sleeping {
890:                 // TODO: Wake up sleeping process
891:                 // This would involve signaling to scheduler
892:             }
893:             
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
