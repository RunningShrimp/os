# [0356] // TODO: Add TLS setup for other architectures

**File:** `kernel/src/process/thread.rs`
**Line:** 1014
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
1011:                         thread.fs_base = thread.tls_base;
1012:                     }
1013:                 }
1014:                 // TODO: Add TLS setup for other architectures
1015:             }
1016: 
1017:             // Perform context switch (simplified)
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
