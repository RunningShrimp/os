# [0345] // TODO: Implement proper physical page tracking for aarch64

**File:** `kernel/src/syscalls/process.rs`
**Line:** 585
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
582:             unsafe {
583:                 if crate::mm::vm::unmap_page(pagetable, va).is_ok() {
584:                     // Note: AArch64 unmap_page doesn't return PA, so we can't free here
585:                     // TODO: Implement proper physical page tracking for aarch64
586:                 }
587:             }
588: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
