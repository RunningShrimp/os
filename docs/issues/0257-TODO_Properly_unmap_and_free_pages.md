# [0257] // TODO: Properly unmap and free pages

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 94
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
91:         proc.sz = addr;
92:     } else if addr < old_sz {
93:         // Shrinking break - for now, just update size (simplified)
94:         // TODO: Properly unmap and free pages
95:         proc.sz = addr;
96:     }
97: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
