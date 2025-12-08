# [0320] // TODO: Implement setuid syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 303
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
300: }
301: 
302: fn sys_setuid(_args: &[u64]) -> SyscallResult {
303:     // TODO: Implement setuid syscall
304:     Err(SyscallError::NotSupported)
305: }
306: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
