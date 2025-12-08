# [0319] Err(SyscallError::IoError) // Temporary fix until we have proper cancellation error

**File:** `kernel/src/syscalls/aio.rs`
**Line:** 546
**Marker:** Temporary
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;temporary`

## Context

```
543:         AioStatus::Cancelled => {
544:             // Operation was cancelled
545:             AIO_OPERATIONS.lock().remove(&operation_id);
546:             Err(SyscallError::IoError) // Temporary fix until we have proper cancellation error
547:         }
548:     }
549: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
