# [0237] // TODO: Implement exit syscall

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 314
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
311: }
312: 
313: fn sys_exit(_args: &[u64]) -> SyscallResult {
314:     // TODO: Implement exit syscall
315:     Err(SyscallError::NotSupported)
316: }
317: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
