# [0155] total_deallocations: 0, // TODO: Track deallocations

**File:** `kernel/src/mm/allocator.rs`
**Line:** 199
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Memory/MM Engineer
**Suggested Estimate (hours):** 32
**Suggested Labels:** `high;todo`

## Context

```
196:         
197:         crate::mm::traits::AllocatorStats {
198:             total_allocations,
199:             total_deallocations: 0, // TODO: Track deallocations
200:             current_allocated_bytes: buddy_stats.allocated + slab_stats.allocated,
201:             peak_allocated_bytes: buddy_stats.allocated + slab_stats.allocated, // TODO: Track peak
202:             failed_allocations: 0, // TODO: Track failures
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
