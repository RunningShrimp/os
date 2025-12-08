# [0262] // TODO: Implement mlock syscall

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 465
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
462: }
463: 
464: fn sys_mlock(args: &[u64]) -> SyscallResult {
465:     // TODO: Implement mlock syscall
466:     Err(SyscallError::NotSupported)
467: }
468: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
