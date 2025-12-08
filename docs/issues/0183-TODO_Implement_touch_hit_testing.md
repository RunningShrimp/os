# [0183] // TODO: Implement touch hit testing

**File:** `kernel/src/graphics/input.rs`
**Line:** 240
**Marker:** TODO
**Suggested Priority:** Low
**Suggested Owner Role:** Graphics Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `low;todo`

## Context

```
237:             }
238:             InputEvent::TouchStart { .. } | InputEvent::TouchMove { .. } | InputEvent::TouchEnd { .. } => {
239:                 // Touch events go to surface under touch point
240:                 // TODO: Implement touch hit testing
241:             }
242:         }
243:         
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
