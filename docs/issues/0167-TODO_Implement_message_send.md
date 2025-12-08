# [0167] // TODO: Implement message send

**File:** `kernel/src/ipc/mod.rs`
**Line:** 160
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
157: 
158: /// Send message to queue
159: pub fn msg_send(queue_id: u32, msg: &IpcMessage) -> bool {
160:     // TODO: Implement message send
161:     true
162: }
163: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
