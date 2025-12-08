# [0020] // TODO: Implement setting real-time clock

**File:** `kernel/src/posix/timer.rs`
**Line:** 432
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
429:         return EPERM; // Only real-time clock can be set
430:     }
431: 
432:     // TODO: Implement setting real-time clock
433:     EPERM
434: }
435: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
