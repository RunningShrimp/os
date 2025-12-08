# [0263] // TODO: Implement munlock syscall

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 470
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
467: }
468: 
469: fn sys_munlock(args: &[u64]) -> SyscallResult {
470:     // TODO: Implement munlock syscall
471:     Err(SyscallError::NotSupported)
472: }
473: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
