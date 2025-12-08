# [0275] // TODO: Implement ifinfo syscall

**File:** `kernel/src/syscalls/network/interface.rs`
**Line:** 14
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
11: 
12: /// Get network interface information
13: pub fn sys_ifinfo(_args: &[u64]) -> super::super::common::SyscallResult {
14:     // TODO: Implement ifinfo syscall
15:     Err(SyscallError::NotSupported)
16: }
17: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
