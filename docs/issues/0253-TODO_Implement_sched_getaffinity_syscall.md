# [0253] // TODO: Implement sched_getaffinity syscall

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 907
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
904: }
905: 
906: fn sys_sched_getaffinity(_args: &[u64]) -> SyscallResult {
907:     // TODO: Implement sched_getaffinity syscall
908:     Err(SyscallError::NotSupported)
909: }
910: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
