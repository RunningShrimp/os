# [0341] // TODO: Implement capget syscall

**File:** `kernel/src/syscalls/process.rs`
**Line:** 475
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
472: }
473: 
474: fn sys_capget(_args: &[u64]) -> SyscallResult {
475:     // TODO: Implement capget syscall
476:     Err(SyscallError::NotSupported)
477: }
478: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
