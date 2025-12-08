# [0236] // TODO: Implement execve syscall

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 309
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
306: }
307: 
308: fn sys_execve(_args: &[u64]) -> SyscallResult {
309:     // TODO: Implement execve syscall
310:     Err(SyscallError::NotSupported)
311: }
312: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
