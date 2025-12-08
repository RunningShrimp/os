# [0143] // TODO: Decrement reference count on old page and free if zero

**File:** `kernel/src/mm/vm.rs`
**Line:** 1308
**Marker:** TODO
**Suggested Priority:** High
**Suggested Owner Role:** Memory/MM Engineer
**Suggested Estimate (hours):** 32
**Suggested Labels:** `high;todo`

## Context

```
1305:     // Flush TLB for this address
1306:     flush_tlb_page(va);
1307:     
1308:     // TODO: Decrement reference count on old page and free if zero
1309:     let _ = pagetable;
1310:     
1311:     PageFaultResult::Handled
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
