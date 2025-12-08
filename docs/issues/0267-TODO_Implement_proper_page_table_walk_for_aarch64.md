# [0267] // TODO: Implement proper page table walk for aarch64

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 551
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
548:         #[cfg(target_arch = "aarch64")]
549:         {
550:             // For aarch64, check if page is mapped
551:             // TODO: Implement proper page table walk for aarch64
552:             synced_pages += 1;
553:         }
554:         
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
