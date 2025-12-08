# [0176] // TODO: Call scheduler yield when available

**File:** `kernel/src/sync/mod.rs`
**Line:** 572
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
569: 
570:             // After many spins, yield to reduce CPU contention
571:             if spin_count > 1000 {
572:                 // TODO: Call scheduler yield when available
573:                 spin_count = 0;
574:             }
575:         }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
