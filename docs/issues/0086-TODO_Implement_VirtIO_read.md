# [0086] // TODO: Implement VirtIO read

**File:** `kernel/src/drivers/mod.rs`
**Line:** 118
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Driver Engineer
**Suggested Estimate (hours):** 36
**Suggested Labels:** `high;todo`

## Context

```
115: 
116: impl BlockDevice for VirtioBlk {
117:     fn read(&self, _lba: usize, _buf: &mut [u8]) {
118:         // TODO: Implement VirtIO read
119:     }
120: 
121:     fn write(&self, _lba: usize, _buf: &[u8]) {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
