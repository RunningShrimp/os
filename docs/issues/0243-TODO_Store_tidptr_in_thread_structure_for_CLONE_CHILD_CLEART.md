# [0243] // TODO: Store tidptr in thread structure for CLONE_CHILD_CLEARTID

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 469
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
466:         let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
467:         
468:         // Store tidptr for clearing on thread exit
469:         // TODO: Store tidptr in thread structure for CLONE_CHILD_CLEARTID
470:         
471:         Ok(pid as u64)
472:     } else {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
