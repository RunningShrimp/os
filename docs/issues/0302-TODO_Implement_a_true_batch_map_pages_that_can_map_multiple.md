# [0302] // TODO: Implement a true batch map_pages that can map multiple non-contiguous pages.

**File:** `kernel/src/syscalls/advanced_mmap.rs`
**Line:** 302
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
299:             // Note: This expects page table walk to be done once for the entire range
300:             // and map all contiguous pages at once. We'll use map_page in a loop but
301:             // this is still more efficient than the original code since we did allocation first.
302:             // TODO: Implement a true batch map_pages that can map multiple non-contiguous pages.
303:             map_pages(proc.pagetable, va, pa_start, aligned_length, perm)
304:         };
305:         
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
