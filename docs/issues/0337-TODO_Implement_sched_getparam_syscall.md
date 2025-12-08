# [0337] // TODO: Implement sched_getparam syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 455
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
452: }
453: 
454: fn sys_sched_getparam(_args: &[u64]) -> SyscallResult {
455:     // TODO: Implement sched_getparam syscall
456:     Err(SyscallError::NotSupported)
457: }
458: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
