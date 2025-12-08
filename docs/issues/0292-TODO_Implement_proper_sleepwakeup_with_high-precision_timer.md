# [0292] // TODO: Implement proper sleep/wakeup with high-precision timer

**File:** `kernel/src/syscalls/time.rs`
**Line:** 266
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
263:         }
264:     } else {
265:         // Longer sleep: use sleep/wakeup mechanism
266:         // TODO: Implement proper sleep/wakeup with high-precision timer
267:         crate::time::sleep_ms(sleep_ns / 1_000_000);
268:     }
269:     
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
