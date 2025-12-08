# [0234] // TODO: Implement fork syscall

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 299
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
296: }
297: 
298: fn sys_fork(_args: &[u64]) -> SyscallResult {
299:     // TODO: Implement fork syscall
300:     Err(SyscallError::NotSupported)
301: }
302: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
