# [0088] // TODO: Handle backspace

**File:** `kernel/src/drivers/mod.rs`
**Line:** 199
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Driver Engineer
**Suggested Estimate (hours):** 36
**Suggested Labels:** `high;todo`

## Context

```
196:     match c {
197:         // Backspace
198:         0x7F | 0x08 => {
199:             // TODO: Handle backspace
200:         }
201:         // Ctrl-C
202:         0x03 => {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
