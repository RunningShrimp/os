# [0027] shm_atime: 0, // TODO: Track attach time

**File:** `kernel/src/posix/shm.rs`
**Line:** 404
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
401:             *buf = ShmidDs {
402:                 shm_perm: seg_guard.perm,
403:                 shm_segsz: seg_guard.size,
404:                 shm_atime: 0, // TODO: Track attach time
405:                 shm_dtime: seg_guard.last_detach_time,
406:                 shm_ctime: seg_guard.creation_time,
407:                 shm_cpid: seg_guard.creator_pid,
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
