# [0259] // TODO: Implement proper physical page tracking for aarch64

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 309
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
306:                 if crate::mm::vm::unmap_page(pagetable, current).is_ok() {
307:                     // Note: For aarch64, we would need to track physical addresses
308:                     // separately. For now, we just unmap without freeing physical memory.
309:                     // TODO: Implement proper physical page tracking for aarch64
310:                     unmapped_count += 1;
311:                 }
312:             }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
