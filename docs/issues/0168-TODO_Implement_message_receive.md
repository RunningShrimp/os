# [0168] // TODO: Implement message receive

**File:** `kernel/src/ipc/mod.rs`
**Line:** 166
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
163: 
164: /// Receive message from queue
165: pub fn msg_recv(queue_id: u32, msg_type: u32) -> Option<IpcMessage> {
166:     // TODO: Implement message receive
167:     None
168: }
169: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
