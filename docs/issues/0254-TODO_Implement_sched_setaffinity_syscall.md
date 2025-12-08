# [0254] // TODO: Implement sched_setaffinity syscall

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 912
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
909: }
910: 
911: fn sys_sched_setaffinity(_args: &[u64]) -> SyscallResult {
912:     // TODO: Implement sched_setaffinity syscall
913:     Err(SyscallError::NotSupported)
914: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
