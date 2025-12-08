# [0179] // For now, return None as placeholder

**File:** `kernel/src/graphics/surface.rs`
**Line:** 277
**Marker:** placeholder
**Suggested Priority:** Low
**Suggested Owner Role:** Graphics Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `low;placeholder`

## Context

```
274:     /// Get surface by ID (returns a reference - caller must handle locking)
275:     pub fn get_surface(&self, id: SurfaceId) -> Option<alloc::sync::Arc<Mutex<Surface>>> {
276:         // In real implementation, surfaces would be stored as Arc<Mutex<Surface>>
277:         // For now, return None as placeholder
278:         None
279:     }
280:     
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
