# [0140] if let Some(scheduler) = self.cpu_schedulers.get_mut(0) { // TODO: CPU selection

**File:** `kernel/src/microkernel/scheduler.rs`
**Line:** 302
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
299:                         tcb.wake_time = None;
300: 
301:                         // Add to ready queue
302:                         if let Some(scheduler) = self.cpu_schedulers.get_mut(0) { // TODO: CPU selection
303:                             let _ = scheduler.enqueue(*tid);
304:                         }
305:                     }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
