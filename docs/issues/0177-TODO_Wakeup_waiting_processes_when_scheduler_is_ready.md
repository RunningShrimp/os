# [0177] // TODO: Wakeup waiting processes when scheduler is ready

**File:** `kernel/src/sync/mod.rs`
**Line:** 619
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
616:     fn drop(&mut self) {
617:         self.lock.holder.store(0, Ordering::Relaxed);
618:         self.lock.locked.store(false, Ordering::Release);
619:         // TODO: Wakeup waiting processes when scheduler is ready
620:         // This would involve calling the scheduler to wakeup processes waiting on this lock
621:         crate::println!("[sync] SleepLock released - would wakeup waiting processes");
622:     }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
