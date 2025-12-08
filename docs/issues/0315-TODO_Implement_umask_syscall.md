# [0315] // TODO: Implement umask syscall

**File:** `kernel/src/syscalls/fs.rs`
**Line:** 1056
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
1053: }
1054: 
1055: fn sys_umask(_args: &[u64]) -> SyscallResult {
1056:     // TODO: Implement umask syscall
1057:     Err(SyscallError::NotSupported)
1058: }
1059: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
