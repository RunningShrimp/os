# [0308] // TODO: Re-register timer with kernel timer system

**File:** `kernel/src/syscalls/glib.rs`
**Line:** 445
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
442:             let current_expiration = self.next_expiration.load(Ordering::SeqCst);
443:             let next_expiration = current_expiration + interval_ns;
444:             self.next_expiration.store(next_expiration, Ordering::SeqCst);
445:             // TODO: Re-register timer with kernel timer system
446:         } else {
447:             // One-shot timer, disarm it
448:             self.armed.store(false, Ordering::SeqCst);
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
