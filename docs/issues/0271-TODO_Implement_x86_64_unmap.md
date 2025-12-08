# [0271] // TODO: Implement x86_64 unmap

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 981
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
978: 
979:             #[cfg(target_arch = "x86_64")]
980:             {
981:                 // TODO: Implement x86_64 unmap
982:             }
983: 
984:             flush_tlb_page(va);
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
