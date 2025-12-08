# [0343] // TODO: Implement proper physical page tracking for aarch64

**File:** `kernel/src/syscalls/process.rs`
**Line:** 534
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
531:                     unsafe {
532:                         if crate::mm::vm::unmap_page(pagetable, cleanup_va).is_ok() {
533:                             // Note: AArch64 unmap_page doesn't return PA, so we can't free here
534:                             // TODO: Implement proper physical page tracking for aarch64
535:                         }
536:                     }
537:                     #[cfg(target_arch = "x86_64")]
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
