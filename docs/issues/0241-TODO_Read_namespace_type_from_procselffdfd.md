# [0241] // TODO: Read namespace type from /proc/self/fd/{fd}

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 433
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
430:     // Determine namespace type from nstype or fd
431:     let ns_type = if nstype == 0 {
432:         // Determine from namespace file path
433:         // TODO: Read namespace type from /proc/self/fd/{fd}
434:         return Err(SyscallError::NotSupported);
435:     } else {
436:         // Map nstype to NamespaceType
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
