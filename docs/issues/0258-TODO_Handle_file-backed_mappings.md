# [0258] // TODO: Handle file-backed mappings

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 252
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
249:         
250:         Ok(target_addr as u64)
251:     } else {
252:         // TODO: Handle file-backed mappings
253:         Err(SyscallError::NotSupported)
254:     }
255: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
