# [0269] // TODO: Add file descriptor field to MemoryRegion if needed

**File:** `kernel/src/syscalls/memory.rs`
**Line:** 649
**Marker:** TODO
**Suggested Priority:** Critical
**Suggested Owner Role:** Kernel Engineer
**Suggested Estimate (hours):** 40
**Suggested Labels:** `critical;todo`

## Context

```
646: 
647:     // Check if region is anonymous (we only support anonymous mappings for now)
648:     // For now, we assume anonymous mappings (file-backed mappings not supported yet)
649:     // TODO: Add file descriptor field to MemoryRegion if needed
650: 
651:     let result_addr = if aligned_new_size <= aligned_old_size {
652:         // Shrinking the mapping
```

## Recommended next steps
- Confirm the owner and adjust scope estimate\- Add unit/integration tests to cover intended behavior
- Produce a PR that either implements the missing behavior or documents a migration if it's a stub
