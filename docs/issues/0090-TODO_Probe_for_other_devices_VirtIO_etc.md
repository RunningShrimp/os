# [0090] // TODO: Probe for other devices (VirtIO, etc.)

**File:** `kernel/src/drivers/mod.rs`
**Line:** 299
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Driver Engineer
**Suggested Estimate (hours):** 36
**Suggested Labels:** `high;todo`

## Context

```
296:     // RAM disk is always available
297:     crate::println!("drivers: ramdisk {} blocks", RamDisk.num_blocks());
298:     
299:     // TODO: Probe for other devices (VirtIO, etc.)
300:     
301:     #[cfg(target_arch = "aarch64")]
302:     {
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
