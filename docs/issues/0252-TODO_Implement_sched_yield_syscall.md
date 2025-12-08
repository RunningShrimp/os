# [0252] // TODO: Implement sched_yield syscall

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 902
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
899: }
900: 
901: fn sys_sched_yield(_args: &[u64]) -> SyscallResult {
902:     // TODO: Implement sched_yield syscall
903:     Err(SyscallError::NotSupported)
904: }
905: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
