# [0242] // TODO: Implement actual namespace joining

**File:** `kernel/src/syscalls/thread.rs`
**Line:** 449
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
446:     };
447:     
448:     // Join the namespace
449:     // TODO: Implement actual namespace joining
450:     crate::println!("[setns] Process {} joining namespace {:?}", pid, ns_type);
451:     
452:     Ok(0)
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
