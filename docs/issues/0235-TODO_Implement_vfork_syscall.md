# [0235] // TODO: Implement vfork syscall

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 304
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
301: }
302: 
303: fn sys_vfork(_args: &[u64]) -> SyscallResult {
304:     // TODO: Implement vfork syscall
305:     Err(SyscallError::NotSupported)
306: }
307: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
