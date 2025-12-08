# [0049] // TODO: Replace Process stub with crate::process::Proc when all usages are updated

**File:** `kernel/src/types/stubs.rs`
**Line:** 141
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
138:     }
139: }
140: 
141: // TODO: Replace Process stub with crate::process::Proc when all usages are updated
142: 
143: // Memory address type
144: #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
