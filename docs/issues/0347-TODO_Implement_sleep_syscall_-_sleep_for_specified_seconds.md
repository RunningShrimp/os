# [0347] // TODO: Implement sleep syscall - sleep for specified seconds

**File:** `kernel/src/syscalls/process.rs`
**Line:** 611
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
608: }
609: 
610: fn sys_sleep(_args: &[u64]) -> SyscallResult {
611:     // TODO: Implement sleep syscall - sleep for specified seconds
612:     Err(SyscallError::NotSupported)
613: }
614: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
