# [0157] failed_allocations: 0, // TODO: Track failures

**File:** `kernel/src/mm/allocator.rs`
**Line:** 202
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Memory/MM Engineer
**Suggested Estimate (hours):** 32
**Suggested Labels:** `high;todo`

## Context

```
199:             total_deallocations: 0, // TODO: Track deallocations
200:             current_allocated_bytes: buddy_stats.allocated + slab_stats.allocated,
201:             peak_allocated_bytes: buddy_stats.allocated + slab_stats.allocated, // TODO: Track peak
202:             failed_allocations: 0, // TODO: Track failures
203:         }
204:     }
205: }
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
