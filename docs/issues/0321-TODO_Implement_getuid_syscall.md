# [0321] // TODO: Implement getuid syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 308
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
305: }
306: 
307: fn sys_getuid(_args: &[u64]) -> SyscallResult {
308:     // TODO: Implement getuid syscall
309:     Err(SyscallError::NotSupported)
310: }
311: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
