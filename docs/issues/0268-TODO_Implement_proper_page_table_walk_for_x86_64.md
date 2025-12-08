# [0268] // TODO: Implement proper page table walk for x86_64

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 558
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
555:         #[cfg(target_arch = "x86_64")]
556:         {
557:             // For x86_64, check if page is mapped
558:             // TODO: Implement proper page table walk for x86_64
559:             synced_pages += 1;
560:         }
561:         
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
