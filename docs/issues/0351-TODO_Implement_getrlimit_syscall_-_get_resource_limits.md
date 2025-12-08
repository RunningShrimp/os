# [0351] // TODO: Implement getrlimit syscall - get resource limits

**File:** `kernel/src/syscalls/process.rs`
**Line:** 631
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
628: }
629: 
630: fn sys_getrlimit(_args: &[u64]) -> SyscallResult {
631:     // TODO: Implement getrlimit syscall - get resource limits
632:     Err(SyscallError::NotSupported)
633: }
634: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
