# [0203] // TODO: Implement socket send

**File:** `kernel/src/services/network.rs`
**Line:** 559
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
556: 
557: /// Send data over socket
558: pub fn net_send(socket: usize, buf: *const u8, len: usize, flags: u32) -> Option<usize> {
559:     // TODO: Implement socket send
560:     None
561: }
562: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
