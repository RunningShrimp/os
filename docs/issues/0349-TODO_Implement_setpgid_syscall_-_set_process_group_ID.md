# [0349] // TODO: Implement setpgid syscall - set process group ID

**File:** `kernel/src/syscalls/process.rs`
**Line:** 621
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
618: }
619: 
620: fn sys_setpgid(_args: &[u64]) -> SyscallResult {
621:     // TODO: Implement setpgid syscall - set process group ID
622:     Err(SyscallError::NotSupported)
623: }
624: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
