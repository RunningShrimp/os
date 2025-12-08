# [0015] // TODO: Add current time to relative time

**File:** `kernel/src/posix/timer.rs`
**Line:** 308
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
305:         new_spec.it_value
306:     } else {
307:         // Convert relative time to absolute time
308:         // TODO: Add current time to relative time
309:         new_spec.it_value
310:     };
311: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
