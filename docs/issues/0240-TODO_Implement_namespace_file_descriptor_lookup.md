# [0240] // TODO: Implement namespace file descriptor lookup

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 426
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
423:     let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
424:     
425:     // Look up file descriptor to get namespace path
426:     // TODO: Implement namespace file descriptor lookup
427:     // For now, return not supported
428:     crate::println!("[setns] Process {} attempting to join namespace via fd {}", pid, fd);
429:     
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
