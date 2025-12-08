# [0202] // TODO: Implement socket connect

**File:** `kernel/src/services/network.rs`
**Line:** 553
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
550: 
551: /// Connect to a remote address
552: pub fn net_connect(socket: usize, addr: *const u8, addr_len: usize) -> bool {
553:     // TODO: Implement socket connect
554:     false
555: }
556: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
