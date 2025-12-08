# [0344] // TODO: Implement proper unmapping for x86_64

**File:** `kernel/src/syscalls/process.rs`
**Line:** 540
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
537:                     #[cfg(target_arch = "x86_64")]
538:                     {
539:                         // x86_64 unmap implementation needed
540:                         // TODO: Implement proper unmapping for x86_64
541:                     }
542:                 }
543:                 return Err(SyscallError::OutOfMemory);
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
