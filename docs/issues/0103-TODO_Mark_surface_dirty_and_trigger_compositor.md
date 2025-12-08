# [0103] // TODO: Mark surface dirty and trigger compositor

**File:** `kernel/src/web/engine.rs`
**Line:** 156
**Marker:** TODO
**Suggested Priority:** Medium
**Suggested Owner Role:** Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `medium;todo`

## Context

```
153:         if let Some(surface_id) = self.surface_id {
154:             // Mark surface as dirty
155:             let surface_manager = get_surface_manager();
156:             // TODO: Mark surface dirty and trigger compositor
157:             composite_frame()?;
158:         }
159:         
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
