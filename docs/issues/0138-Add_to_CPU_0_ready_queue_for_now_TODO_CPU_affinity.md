# [0138] // Add to CPU 0 ready queue for now (TODO: CPU affinity)

**File:** `kernel/src/microkernel/scheduler.rs`
**Line:** 200
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
197:         let mut table = self.thread_table.lock();
198:         table.insert(tid, tcb);
199: 
200:         // Add to CPU 0 ready queue for now (TODO: CPU affinity)
201:         if self.cpu_schedulers.len() > 0 {
202:             self.cpu_schedulers[0].enqueue(tid).map_err(|_| {
203:                 // Remove from table if enqueue fails
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
