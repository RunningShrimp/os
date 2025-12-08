# [0307] // TODO: Register timer with kernel timer system for actual expiration handling

**File:** `kernel/src/syscalls/glib.rs`
**Line:** 400
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
397:             self.expiration_count.store(0, Ordering::SeqCst);
398:         }
399: 
400:         // TODO: Register timer with kernel timer system for actual expiration handling
401: 
402:         Ok(old_spec)
403:     }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
