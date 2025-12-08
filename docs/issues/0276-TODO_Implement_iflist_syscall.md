# [0276] // TODO: Implement iflist syscall

**File:** `kernel/src/syscalls/network/interface.rs`
**Line:** 20
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
17: 
18: /// List network interfaces
19: pub fn sys_iflist(_args: &[u64]) -> super::super::common::SyscallResult {
20:     // TODO: Implement iflist syscall
21:     Err(SyscallError::NotSupported)
22: }
23: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
