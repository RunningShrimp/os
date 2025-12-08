# [0023] // TODO: Implement actual signal sending

**File:** `kernel/src/posix/mqueue.rs`
**Line:** 231
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
228:                     // Send signal to process
229:                     crate::println!("[mqueue] Sending signal {} to process {}", 
230:                         notify_info.notify_sig, notify_info.notify_pid);
231:                     // TODO: Implement actual signal sending
232:                 }
233:             }
234:             MQ_PIPE => {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
