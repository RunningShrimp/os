# [0285] // TODO: Implement sendmsg syscall

**File:** `kernel/src/syscalls/network/data.rs`
**Line:** 246
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
243: 
244: /// Send message
245: pub fn sys_sendmsg(_args: &[u64]) -> super::super::common::SyscallResult {
246:     // TODO: Implement sendmsg syscall
247:     Err(SyscallError::NotSupported)
248: }
249: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
