# [0311] // TODO: Implement getrandom syscall

**File:** `kernel/src/syscalls/glib.rs`
**Line:** 1030
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
1027: // Placeholder implementations - to be replaced with actual syscall logic
1028: 
1029: fn sys_getrandom(_args: &[u64]) -> SyscallResult {
1030:     // TODO: Implement getrandom syscall
1031:     Err(SyscallError::NotSupported)
1032: }
1033: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
