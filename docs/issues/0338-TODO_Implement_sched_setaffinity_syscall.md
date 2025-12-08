# [0338] // TODO: Implement sched_setaffinity syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 460
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
457: }
458: 
459: fn sys_sched_setaffinity(_args: &[u64]) -> SyscallResult {
460:     // TODO: Implement sched_setaffinity syscall
461:     Err(SyscallError::NotSupported)
462: }
463: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
