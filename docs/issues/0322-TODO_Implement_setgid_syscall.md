# [0322] // TODO: Implement setgid syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 313
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
310: }
311: 
312: fn sys_setgid(_args: &[u64]) -> SyscallResult {
313:     // TODO: Implement setgid syscall
314:     Err(SyscallError::NotSupported)
315: }
316: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
