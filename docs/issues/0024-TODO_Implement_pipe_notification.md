# [0024] // TODO: Implement pipe notification

**File:** `kernel/src/posix/mqueue.rs`
**Line:** 235
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
232:                 }
233:             }
234:             MQ_PIPE => {
235:                 // TODO: Implement pipe notification
236:                 crate::println!("[mqueue] Pipe notification not implemented yet");
237:             }
238:             _ => {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
