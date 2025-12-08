# [0290] // TODO: Implement settimeofday syscall

**File:** `kernel/src/syscalls/time.rs`
**Line:** 84
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
81: }
82: 
83: fn sys_settimeofday(_args: &[u64]) -> SyscallResult {
84:     // TODO: Implement settimeofday syscall
85:     Err(SyscallError::NotSupported)
86: }
87: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
