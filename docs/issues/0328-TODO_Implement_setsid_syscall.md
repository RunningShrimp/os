# [0328] // TODO: Implement setsid syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 410
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
407: }
408: 
409: fn sys_setsid(_args: &[u64]) -> SyscallResult {
410:     // TODO: Implement setsid syscall
411:     Err(SyscallError::NotSupported)
412: }
413: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
