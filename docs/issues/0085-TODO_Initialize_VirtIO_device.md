# [0085] // TODO: Initialize VirtIO device

**File:** `kernel/src/drivers/mod.rs`
**Line:** 101
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Driver Engineer
**Suggested Estimate (hours):** 36
**Suggested Labels:** `high;todo`

## Context

```
98: #[allow(dead_code)]
99: impl VirtioBlk {
100:     pub fn new(base: usize) -> Option<Self> {
101:         // TODO: Initialize VirtIO device
102:         Some(Self {
103:             base,
104:             capacity: 0,
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
