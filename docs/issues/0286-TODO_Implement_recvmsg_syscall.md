# [0286] // TODO: Implement recvmsg syscall

**File:** `kernel/src/syscalls/network/data.rs`
**Line:** 252
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
249: 
250: /// Receive message
251: pub fn sys_recvmsg(_args: &[u64]) -> super::super::common::SyscallResult {
252:     // TODO: Implement recvmsg syscall
253:     Err(SyscallError::NotSupported)
254: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
