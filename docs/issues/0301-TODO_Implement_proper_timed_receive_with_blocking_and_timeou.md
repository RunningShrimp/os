# [0301] // TODO: Implement proper timed receive with blocking and timeout

**File:** `kernel/src/syscalls/mqueue.rs`
**Line:** 368
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
365:     let timeout = unsafe { core::ptr::read(timeout_bytes.as_ptr() as *const crate::posix::Timespec) };
366:     
367:     // For now, implement as non-blocking receive with timeout validation
368:     // TODO: Implement proper timed receive with blocking and timeout
369:     if timeout.tv_sec < 0 || timeout.tv_nsec < 0 {
370:         return Err(SyscallError::InvalidArgument);
371:     }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
