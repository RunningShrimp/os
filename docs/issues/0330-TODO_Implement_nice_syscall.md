# [0330] // TODO: Implement nice syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 420
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
417: }
418: 
419: fn sys_nice(_args: &[u64]) -> SyscallResult {
420:     // TODO: Implement nice syscall
421:     Err(SyscallError::NotSupported)
422: }
423: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
