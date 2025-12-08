# [0182] // TODO: Implement hit testing

**File:** `kernel/src/graphics/input.rs`
**Line:** 225
**Marker:** TODO
**Suggested Priority:** Low
**Suggested Owner Role:** Graphics Engineer
**Suggested Estimate (hours):** 16
**Suggested Labels:** `low;todo`

## Context

```
222:                 self.mouse_y.store(*y as u32, Ordering::Release);
223:                 
224:                 // Find surface under cursor and send event
225:                 // TODO: Implement hit testing
226:                 if let Some(surface_id) = self.find_surface_at(*x, *y) {
227:                     self.send_to_surface(surface_id, event)?;
228:                 }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
