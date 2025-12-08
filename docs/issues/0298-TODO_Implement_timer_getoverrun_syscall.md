# [0298] // TODO: Implement timer_getoverrun syscall

**File:** `kernel/src/syscalls/time.rs`
**Line:** 473
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
470: }
471: 
472: fn sys_timer_getoverrun(_args: &[u64]) -> SyscallResult {
473:     // TODO: Implement timer_getoverrun syscall
474:     Err(SyscallError::NotSupported)
475: }
476: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
