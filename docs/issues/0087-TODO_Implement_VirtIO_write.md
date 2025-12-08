# [0087] // TODO: Implement VirtIO write

**File:** `kernel/src/drivers/mod.rs`
**Line:** 122
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Driver Engineer
**Suggested Estimate (hours):** 36
**Suggested Labels:** `high;todo`

## Context

```
119:     }
120: 
121:     fn write(&self, _lba: usize, _buf: &[u8]) {
122:         // TODO: Implement VirtIO write
123:     }
124: 
125:     fn num_blocks(&self) -> usize {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
