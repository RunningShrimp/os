# [0300] // TODO: Implement proper timed send with blocking and timeout

**File:** `kernel/src/syscalls/mqueue.rs`
**Line:** 261
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
258:     let timeout = unsafe { core::ptr::read(timeout_bytes.as_ptr() as *const crate::posix::Timespec) };
259:     
260:     // For now, implement as non-blocking send with timeout validation
261:     // TODO: Implement proper timed send with blocking and timeout
262:     if timeout.tv_sec < 0 || timeout.tv_nsec < 0 {
263:         return Err(SyscallError::InvalidArgument);
264:     }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
