# [0317] // TODO: Implement pivot_root syscall

**File:** `kernel/src/syscalls/fs.rs`
**Line:** 1418
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
1415: }
1416: 
1417: fn sys_pivot_root(_args: &[u64]) -> SyscallResult {
1418:     // TODO: Implement pivot_root syscall
1419:     Err(SyscallError::NotSupported)
1420: }
1421: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
