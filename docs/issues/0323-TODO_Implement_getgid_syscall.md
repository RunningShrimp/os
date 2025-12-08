# [0323] // TODO: Implement getgid syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 318
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
315: }
316: 
317: fn sys_getgid(_args: &[u64]) -> SyscallResult {
318:     // TODO: Implement getgid syscall
319:     Err(SyscallError::NotSupported)
320: }
321: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
