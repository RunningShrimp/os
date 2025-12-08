# [0350] // TODO: Implement getpgid syscall - get process group ID

**File:** `kernel/src/syscalls/process.rs`
**Line:** 626
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
623: }
624: 
625: fn sys_getpgid(_args: &[u64]) -> SyscallResult {
626:     // TODO: Implement getpgid syscall - get process group ID
627:     Err(SyscallError::NotSupported)
628: }
629: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
