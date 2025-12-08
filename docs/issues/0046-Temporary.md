# [0046] Temporary,

**File:** `kernel/src/compat/sandbox.rs`
**Line:** 85
**Marker:** Temporary
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;temporary`

## Context

```
82:     /// Service sandbox
83:     Service,
84:     /// Temporary sandbox
85:     Temporary,
86:     /// Development sandbox
87:     Development,
88: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
