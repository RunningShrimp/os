# [0018] // TODO: Get process CPU time

**File:** `kernel/src/posix/timer.rs`
**Line:** 401
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
398:             Timespec::new(0, 0)
399:         }
400:         crate::posix::CLOCK_PROCESS_CPUTIME_ID => {
401:             // TODO: Get process CPU time
402:             Timespec::new(0, 0)
403:         }
404:         crate::posix::CLOCK_THREAD_CPUTIME_ID => {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
