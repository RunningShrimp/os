# [0331] // TODO: Implement sched_yield syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 425
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
422: }
423: 
424: fn sys_sched_yield(_args: &[u64]) -> SyscallResult {
425:     // TODO: Implement sched_yield syscall
426:     Err(SyscallError::NotSupported)
427: }
428: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
