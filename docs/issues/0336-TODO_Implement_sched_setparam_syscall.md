# [0336] // TODO: Implement sched_setparam syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 450
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
447: }
448: 
449: fn sys_sched_setparam(_args: &[u64]) -> SyscallResult {
450:     // TODO: Implement sched_setparam syscall
451:     Err(SyscallError::NotSupported)
452: }
453: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
