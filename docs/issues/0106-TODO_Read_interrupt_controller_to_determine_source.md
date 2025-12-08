# [0106] // TODO: Read interrupt controller to determine source

**File:** `kernel/src/trap/mod.rs`
**Line:** 152
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
149:     }
150:     
151:     fn handle_irq() {
152:         // TODO: Read interrupt controller to determine source
153:         crate::time::timer_interrupt();
154:     }
155: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
