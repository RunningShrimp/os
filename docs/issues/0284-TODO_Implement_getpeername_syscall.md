# [0284] // TODO: Implement getpeername syscall

**File:** `kernel/src/syscalls/network/options.rs`
**Line:** 68
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
65: 
66: /// Get peer name
67: pub fn sys_getpeername(_args: &[u64]) -> super::super::common::SyscallResult {
68:     // TODO: Implement getpeername syscall
69:     Err(SyscallError::NotSupported)
70: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
