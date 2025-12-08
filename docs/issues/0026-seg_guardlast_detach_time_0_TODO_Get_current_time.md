# [0026] seg_guard.last_detach_time = 0; // TODO: Get current time

**File:** `kernel/src/posix/shm.rs`
**Line:** 355
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
352: 
353:     // Update segment statistics
354:     seg_guard.nattch = seg_guard.nattch.saturating_sub(1);
355:     seg_guard.last_detach_time = 0; // TODO: Get current time
356: 
357:     // If segment is marked for removal and has no more attachments, remove it
358:     if seg_guard.remove_pending && seg_guard.nattch == 0 {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
