# [0201] // TODO: Implement socket accept

**File:** `kernel/src/services/network.rs`
**Line:** 547
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
544: 
545: /// Accept incoming connection
546: pub fn net_accept(socket: usize, addr: *mut u8, addr_len: *mut usize) -> Option<usize> {
547:     // TODO: Implement socket accept
548:     None
549: }
550: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
