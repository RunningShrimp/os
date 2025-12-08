# [0238] // TODO: Implement wait4 syscall

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 319
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
316: }
317: 
318: fn sys_wait4(_args: &[u64]) -> SyscallResult {
319:     // TODO: Implement wait4 syscall
320:     Err(SyscallError::NotSupported)
321: }
322: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
