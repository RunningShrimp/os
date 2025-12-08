# [0142] // TODO: Recursively free page table pages

**File:** `kernel/src/mm/vm.rs`
**Line:** 836
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Memory/MM Engineer
**Suggested Estimate (hours):** 32
**Suggested Labels:** `high;todo`

## Context

```
833:         return;
834:     }
835:     
836:     // TODO: Recursively free page table pages
837:     unsafe { kfree(pagetable as *mut u8); }
838: }
839: 
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
