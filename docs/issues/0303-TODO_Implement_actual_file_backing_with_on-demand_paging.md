# [0303] // TODO: Implement actual file backing with on-demand paging

**File:** `kernel/src/syscalls/advanced_mmap.rs`
**Line:** 321
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
318:         }
319:     } else {
320:         // For file-backed mappings, map directly (we'll allocate pages on demand)
321:         // TODO: Implement actual file backing with on-demand paging
322:         let map_result = unsafe {
323:             map_pages(proc.pagetable, va, 0, aligned_length, perm)
324:         };
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
