# [0335] // TODO: Implement sched_getscheduler syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 445
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
442: }
443: 
444: fn sys_sched_getscheduler(_args: &[u64]) -> SyscallResult {
445:     // TODO: Implement sched_getscheduler syscall
446:     Err(SyscallError::NotSupported)
447: }
448: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
