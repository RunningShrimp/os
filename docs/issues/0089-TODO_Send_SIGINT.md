# [0089] // TODO: Send SIGINT

**File:** `kernel/src/drivers/mod.rs`
**Line:** 203
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Driver Engineer
**Suggested Estimate (hours):** 36
**Suggested Labels:** `high;todo`

## Context

```
200:         }
201:         // Ctrl-C
202:         0x03 => {
203:             // TODO: Send SIGINT
204:         }
205:         // Ctrl-D (EOF)
206:         0x04 => {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
