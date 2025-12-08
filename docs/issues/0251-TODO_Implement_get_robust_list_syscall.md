# [0251] // TODO: Implement get_robust_list syscall

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 897
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
894: }
895: 
896: fn sys_get_robust_list(_args: &[u64]) -> SyscallResult {
897:     // TODO: Implement get_robust_list syscall
898:     Err(SyscallError::NotSupported)
899: }
900: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
