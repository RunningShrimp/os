# [0332] // TODO: Implement sched_get_priority_max syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 430
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
427: }
428: 
429: fn sys_sched_get_priority_max(_args: &[u64]) -> SyscallResult {
430:     // TODO: Implement sched_get_priority_max syscall
431:     Err(SyscallError::NotSupported)
432: }
433: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
