# [0016] // TODO: Get real-time clock

**File:** `kernel/src/posix/timer.rs`
**Line:** 393
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
390: 
391:     let current_time = match clock_id {
392:         crate::posix::CLOCK_REALTIME => {
393:             // TODO: Get real-time clock
394:             Timespec::new(0, 0)
395:         }
396:         crate::posix::CLOCK_MONOTONIC => {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
