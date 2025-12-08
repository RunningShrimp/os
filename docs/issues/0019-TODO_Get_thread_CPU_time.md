# [0019] // TODO: Get thread CPU time

**File:** `kernel/src/posix/timer.rs`
**Line:** 405
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
402:             Timespec::new(0, 0)
403:         }
404:         crate::posix::CLOCK_THREAD_CPUTIME_ID => {
405:             // TODO: Get thread CPU time
406:             Timespec::new(0, 0)
407:         }
408:         _ => return EINVAL,
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
