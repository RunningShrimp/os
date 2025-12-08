# [0283] // TODO: Implement getsockname syscall

**File:** `kernel/src/syscalls/network/options.rs`
**Line:** 62
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
59: 
60: /// Get socket name
61: pub fn sys_getsockname(_args: &[u64]) -> super::super::common::SyscallResult {
62:     // TODO: Implement getsockname syscall
63:     Err(SyscallError::NotSupported)
64: }
65: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
