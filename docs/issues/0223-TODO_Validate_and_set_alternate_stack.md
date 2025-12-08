# [0223] // TODO: Validate and set alternate stack

**File:** `kernel/src/syscalls/signal.rs`
**Line:** 408
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
405:                 .map_err(|_| SyscallError::BadAddress)?;
406:         }
407: 
408:         // TODO: Validate and set alternate stack
409:         // For now, just ignore
410:     }
411: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
