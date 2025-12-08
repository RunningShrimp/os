# [0225] // TODO: Implement timeout and blocking wait

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 683
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
680:         return Ok(signal as u64);
681:     }
682: 
683:     // TODO: Implement timeout and blocking wait
684:     // For now, return EAGAIN if no signal is pending
685:     Err(SyscallError::WouldBlock)
686: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
