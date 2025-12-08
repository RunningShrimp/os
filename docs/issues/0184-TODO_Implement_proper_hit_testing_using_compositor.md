# [0184] // TODO: Implement proper hit testing using compositor

**File:** `kernel/src/graphics/input.rs`
**Line:** 260
**Marker:** TODO
**Suggested Priority:** Low
**Suggested Owner Role:** Graphics Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `low;todo`

## Context

```
257:     
258:     /// Find surface at coordinates (simplified)
259:     fn find_surface_at(&self, _x: f32, _y: f32) -> Option<SurfaceId> {
260:         // TODO: Implement proper hit testing using compositor
261:         None
262:     }
263:     
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
