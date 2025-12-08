# [0294] // TODO: Implement alarm syscall

**File:** `kernel/src/syscalls/time.rs`
**Line:** 297
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
294: }
295: 
296: fn sys_alarm(_args: &[u64]) -> SyscallResult {
297:     // TODO: Implement alarm syscall
298:     Err(SyscallError::NotSupported)
299: }
300: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
