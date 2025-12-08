# [0342] // TODO: Implement capset syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 480
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
477: }
478: 
479: fn sys_capset(_args: &[u64]) -> SyscallResult {
480:     // TODO: Implement capset syscall
481:     Err(SyscallError::NotSupported)
482: }
483: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
