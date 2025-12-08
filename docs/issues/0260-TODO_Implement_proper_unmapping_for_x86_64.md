# [0260] // TODO: Implement proper unmapping for x86_64

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 319
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
316:         {
317:             // x86_64 implementation would go here
318:             // For now, just increment count
319:             // TODO: Implement proper unmapping for x86_64
320:             unmapped_count += 1;
321:         }
322: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
