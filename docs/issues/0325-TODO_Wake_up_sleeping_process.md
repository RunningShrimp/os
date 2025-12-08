# [0325] // TODO: Wake up sleeping process

**File:** `kernel/src/syscalls/process.rs`
**Line:** 384
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
381:             
382:             // Wake up process if it's sleeping
383:             if proc.state == crate::process::ProcState::Sleeping {
384:                 // TODO: Wake up sleeping process
385:                 // This would involve signaling the scheduler
386:             }
387:             
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
