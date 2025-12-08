# [0358] // TODO: Get from current process

**File:** `kernel/src/process/mod.rs`
**Line:** 20
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
17: 
18: /// Get current user ID
19: pub fn getuid() -> uid_t {
20:     // TODO: Get from current process
21:     0
22: }
23: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
