# [0139] if let Some(scheduler) = self.cpu_schedulers.get_mut(0) { // TODO: CPU selection

**File:** `kernel/src/microkernel/scheduler.rs`
**Line:** 235
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
232: 
233:         // Add to ready queue if becoming ready
234:         if state == ThreadState::Runnable {
235:             if let Some(scheduler) = self.cpu_schedulers.get_mut(0) { // TODO: CPU selection
236:                 if !scheduler.ready_queue.contains(&tid) {
237:                     scheduler.enqueue(tid)?;
238:                 }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
