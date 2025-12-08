# [0333] // TODO: Implement sched_get_priority_min syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 435
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
432: }
433: 
434: fn sys_sched_get_priority_min(_args: &[u64]) -> SyscallResult {
435:     // TODO: Implement sched_get_priority_min syscall
436:     Err(SyscallError::NotSupported)
437: }
438: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
