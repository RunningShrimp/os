# [0316] // TODO: Implement access syscall

**File:** `kernel/src/syscalls/fs.rs`
**Line:** 1229
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
1226: }
1227: 
1228: fn sys_access(_args: &[u64]) -> SyscallResult {
1229:     // TODO: Implement access syscall
1230:     Err(SyscallError::NotSupported)
1231: }
1232: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
