# [0017] // TODO: Get monotonic clock

**File:** `kernel/src/posix/timer.rs`
**Line:** 397
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
394:             Timespec::new(0, 0)
395:         }
396:         crate::posix::CLOCK_MONOTONIC => {
397:             // TODO: Get monotonic clock
398:             Timespec::new(0, 0)
399:         }
400:         crate::posix::CLOCK_PROCESS_CPUTIME_ID => {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
