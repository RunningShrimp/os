# [0348] // TODO: Implement uptime syscall - get system uptime

**File:** `kernel/src/syscalls/process.rs`
**Line:** 616
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
613: }
614: 
615: fn sys_uptime(_args: &[u64]) -> SyscallResult {
616:     // TODO: Implement uptime syscall - get system uptime
617:     Err(SyscallError::NotSupported)
618: }
619: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
