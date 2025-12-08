# [0164] // TODO: Implement shared memory detach

**File:** `kernel/src/ipc/mod.rs`
**Line:** 142
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Memory/MM Engineer
**Suggested Estimate (hours):** 32
**Suggested Labels:** `high;todo`

## Context

```
139: 
140: /// Detach from shared memory region
141: pub fn shm_detach(addr: usize) -> bool {
142:     // TODO: Implement shared memory detach
143:     true
144: }
145: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
